// SPDX-License-Identifier: AGPL-3.0-or-later

//! The console platform (docs/engineering/local-development.md#console-platform): runs the real
//! instance on a scratch database and exchanges messages through standard input and output, so
//! every flow can be exercised without third-party accounts. Development builds only.

use std::path::Path;
use std::sync::{Arc, Mutex};

use jiff::{SignedDuration, Timestamp};
use secure_notify_conversation::{
    Attachment, ChatKind, CodeKind, Conversation, Incoming, render_plain,
};
use secure_notify_core::ports::{Clock, InstanceStore};
use secure_notify_core::subscribers::{EndpointRef, PAIRING_CODE_KEY};
use secure_notify_core::time::parse_duration;
use secure_notify_storage::Database;
use tokio::io::{AsyncBufReadExt, BufReader};

/// Platform ID of console endpoints.
pub(crate) const PLATFORM: &str = "console";

/// The development clock: real time plus the offset added by `:advance`.
#[derive(Default)]
pub(crate) struct DevClock {
    offset: Mutex<SignedDuration>,
}

impl DevClock {
    fn advance(&self, by: SignedDuration) {
        let mut offset = self
            .offset
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *offset += by;
    }
}

impl Clock for DevClock {
    fn now(&self) -> Timestamp {
        let offset = *self
            .offset
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        Timestamp::now()
            .checked_add(offset)
            .unwrap_or_else(|_| Timestamp::now())
    }
}

/// Who is talking and where.
struct Session {
    user: String,
    group: Option<String>,
    invite_code: Option<String>,
    link_code: Option<String>,
    last_output: String,
    failed: bool,
}

impl Session {
    fn chat(&self) -> (String, ChatKind) {
        match &self.group {
            Some(name) => (format!("group:{name}"), ChatKind::Group),
            None => (self.user.clone(), ChatKind::Private),
        }
    }
}

/// Writes to the transcript (standard output).
#[allow(clippy::print_stdout)]
fn print(line: &str) {
    println!("{line}");
}

/// Runs the console. Returns whether every step succeeded.
pub(crate) async fn run(
    conversation: &Conversation<Database>,
    database: &Database,
    clock: &Arc<DevClock>,
    script: Option<&Path>,
) -> Result<bool, String> {
    let mut session = Session {
        user: "console".to_string(),
        group: None,
        invite_code: None,
        link_code: None,
        last_output: String::new(),
        failed: false,
    };
    if let Some(path) = script {
        let text = tokio::fs::read_to_string(path)
            .await
            .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        for line in text.lines() {
            step(conversation, database, clock, &mut session, line).await;
        }
    } else {
        let mut lines = BufReader::new(tokio::io::stdin()).lines();
        while let Some(line) = lines
            .next_line()
            .await
            .map_err(|e| format!("cannot read standard input: {e}"))?
        {
            step(conversation, database, clock, &mut session, &line).await;
        }
    }
    Ok(!session.failed && !crate::logging::error_logged())
}

async fn step(
    conversation: &Conversation<Database>,
    database: &Database,
    clock: &Arc<DevClock>,
    session: &mut Session,
    raw: &str,
) {
    let line = raw.trim();
    if line.is_empty() || line.starts_with('#') {
        return;
    }
    let line = match substitute(database, session, line).await {
        Ok(line) => line,
        Err(problem) => return fail(session, raw, &problem),
    };
    if let Some(directive) = line.strip_prefix(':') {
        let (name, argument) = directive
            .split_once(char::is_whitespace)
            .map_or((directive, ""), |(n, a)| (n, a.trim()));
        print(&line);
        match (name, argument) {
            ("as", user) if !user.is_empty() => {
                session.user = user.to_string();
                session.group = None;
            }
            ("private", "") => session.group = None,
            ("group", name) if !name.is_empty() => session.group = Some(name.to_string()),
            ("advance", duration) => match parse_duration(duration) {
                Ok(by) => clock.advance(by),
                Err(problem) => fail(session, raw, &problem),
            },
            ("expect", text) if !text.is_empty() => {
                if !session.last_output.contains(text) {
                    let problem = format!("the last bot output does not contain `{text}`");
                    fail(session, raw, &problem);
                }
            }
            ("upload", path) if !path.is_empty() => match tokio::fs::read(path).await {
                Ok(bytes) => {
                    let filename = Path::new(path)
                        .file_name()
                        .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
                    let attachment = Attachment { filename, bytes };
                    send(conversation, session, String::new(), Some(attachment)).await;
                }
                Err(e) => fail(session, raw, &format!("cannot read {path}: {e}")),
            },
            ("ingest", _) => fail(session, raw, "this directive is not available yet"),
            _ => fail(session, raw, "unknown or incomplete directive"),
        }
        return;
    }
    send(conversation, session, line, None).await;
}

/// Sends a message (and optionally a file) from the current user and chat, and prints the reply.
async fn send(
    conversation: &Conversation<Database>,
    session: &mut Session,
    line: String,
    attachment: Option<Attachment>,
) {
    let (address, chat_kind) = session.chat();
    if let Some(file) = &attachment {
        print(&format!("[{address}] > (file {})", file.filename));
    } else {
        print(&format!("[{address}] > {line}"));
    }
    let incoming = Incoming {
        endpoint: EndpointRef {
            platform: PLATFORM.to_string(),
            address: address.clone(),
        },
        chat_kind,
        text: line.clone(),
        language_hint: None,
        attachment,
    };
    session.last_output.clear();
    if let Some(reply) = conversation.handle(&incoming).await {
        match reply
            .generated_code
            .as_ref()
            .map(|c| (c.kind, c.value.clone()))
        {
            Some((CodeKind::Invite, code)) => session.invite_code = Some(code),
            Some((CodeKind::Link, code)) => session.link_code = Some(code),
            Some((CodeKind::Pairing, _)) | None => {}
        }
        let text = render_plain(&reply);
        for (i, out) in text.lines().enumerate() {
            let prefix = if i == 0 { "<" } else { " " };
            print(&format!("[{address}] {prefix} {out}"));
        }
        session.last_output = text;
    }
}

/// Replaces `{pairing_code}`, `{invite_code}`, and `{link_code}`.
async fn substitute(database: &Database, session: &Session, line: &str) -> Result<String, String> {
    let mut line = line.to_string();
    if line.contains("{pairing_code}") {
        let code = database
            .instance_value(PAIRING_CODE_KEY)
            .await
            .map_err(|e| e.to_string())?
            .ok_or("no pairing code: the instance already has an admin")?;
        line = line.replace("{pairing_code}", &code);
    }
    for (placeholder, value) in [
        ("{invite_code}", &session.invite_code),
        ("{link_code}", &session.link_code),
    ] {
        if line.contains(placeholder) {
            let value = value
                .as_ref()
                .ok_or_else(|| format!("{placeholder} is not known yet"))?;
            line = line.replace(placeholder, value);
        }
    }
    Ok(line)
}

fn fail(session: &mut Session, line: &str, problem: &str) {
    session.failed = true;
    print(&format!("!! step failed: {line}: {problem}"));
}
