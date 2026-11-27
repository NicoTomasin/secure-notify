// SPDX-License-Identifier: AGPL-3.0-or-later

//! Configuration, read once at the binary boundary (docs/engineering/configuration.md): the
//! environment (a `.env` file locally) for secrets and runtime values, and the optional instance
//! config file `secure-notify.toml` for instance-wide behavior
//! (docs/operations/self-hosting.md#configuration).

use std::fmt;
use std::io;
use std::path::{Path, PathBuf};

use jiff::SignedDuration;
use secure_notify_core::Secret;
use secure_notify_core::time::parse_duration;
use serde::{Deserialize, Deserializer};

/// Default of `SOURCE_URL`: the canonical repository URL, declared once in the workspace manifest.
pub(crate) const DEFAULT_SOURCE_URL: &str = env!("CARGO_PKG_REPOSITORY");
/// Default of `DATA_DIR`.
pub(crate) const DEFAULT_DATA_DIR: &str = "data";
/// Instance config file looked up in the data directory when `CONFIG_FILE` is not set.
pub(crate) const CONFIG_FILE_NAME: &str = "secure-notify.toml";

/// Every setting the binary reads, validated.
#[derive(Debug, Clone)]
pub(crate) struct Settings {
    /// `DATA_DIR`: database and backups.
    pub(crate) data_dir: PathBuf,
    /// `LOG_LEVEL`.
    pub(crate) log_level: tracing::Level,
    /// `SOURCE_URL`: the source code link shown by `/about`.
    pub(crate) source_url: String,
    /// `TELEGRAM_BOT_TOKEN`.
    pub(crate) telegram_bot_token: Option<Secret>,
    /// The instance config file, or defaults.
    pub(crate) instance: InstanceConfig,
}

/// Instance-wide behavior from `secure-notify.toml`.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub(crate) struct InstanceConfig {
    /// `[correlation]`.
    pub(crate) correlation: CorrelationConfig,
    /// `[retention]`.
    pub(crate) retention: RetentionConfig,
}

/// `[correlation]` (docs/architecture/deduplication.md#grace-window).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub(crate) struct CorrelationConfig {
    /// `grace_window`: delay before a new or changed story can be dispatched.
    #[serde(deserialize_with = "duration")]
    pub(crate) grace_window: SignedDuration,
}

impl Default for CorrelationConfig {
    fn default() -> Self {
        Self {
            grace_window: SignedDuration::from_mins(5),
        }
    }
}

/// `[retention]` (docs/architecture/deduplication.md#retention).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub(crate) struct RetentionConfig {
    /// `events`: how long detailed data is kept.
    #[serde(deserialize_with = "positive_duration")]
    pub(crate) events: SignedDuration,
    /// `dedup`: how long minimal deduplication records are kept.
    #[serde(deserialize_with = "positive_duration")]
    pub(crate) dedup: SignedDuration,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            events: SignedDuration::from_hours(7 * 24),
            dedup: SignedDuration::from_hours(365 * 24),
        }
    }
}

fn duration<'de, D: Deserializer<'de>>(deserializer: D) -> Result<SignedDuration, D::Error> {
    let text = String::deserialize(deserializer)?;
    parse_duration(&text).map_err(serde::de::Error::custom)
}

fn positive_duration<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<SignedDuration, D::Error> {
    let value = duration(deserializer)?;
    if value.is_positive() {
        Ok(value)
    } else {
        Err(serde::de::Error::custom("must be longer than zero"))
    }
}

/// An invalid setting. The message names the setting and never contains a secret value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConfigError {
    /// The setting, such as `LOG_LEVEL` or the config file path.
    pub(crate) setting: String,
    /// What is wrong.
    pub(crate) message: String,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid setting {}: {}", self.setting, self.message)
    }
}

impl std::error::Error for ConfigError {}

fn error(setting: impl Into<String>, message: impl Into<String>) -> ConfigError {
    ConfigError {
        setting: setting.into(),
        message: message.into(),
    }
}

/// Where settings come from; the real one reads the process environment and files, tests pass
/// explicit values.
pub(crate) trait Source {
    /// The environment variable `name`, if set.
    fn var(&self, name: &str) -> Option<String>;
    /// Reads a file.
    fn read(&self, path: &Path) -> io::Result<String>;
}

/// The process environment and file system.
pub(crate) struct ProcessSource;

impl Source for ProcessSource {
    fn var(&self, name: &str) -> Option<String> {
        std::env::var(name).ok()
    }

    fn read(&self, path: &Path) -> io::Result<String> {
        std::fs::read_to_string(path)
    }
}

/// A non-empty environment variable; empty values count as unset.
fn var(source: &dyn Source, name: &str) -> Option<String> {
    source.var(name).filter(|v| !v.trim().is_empty())
}

/// A secret from `NAME` or from the file named by `NAME_FILE`, never both.
fn secret(source: &dyn Source, name: &str) -> Result<Option<Secret>, ConfigError> {
    let file_name = format!("{name}_FILE");
    match (var(source, name), var(source, &file_name)) {
        (Some(_), Some(_)) => Err(error(
            name,
            format!("set either {name} or {file_name}, not both"),
        )),
        (Some(value), None) => Ok(Some(Secret::new(value.trim()))),
        (None, Some(path)) => {
            let content = source
                .read(Path::new(&path))
                .map_err(|e| error(&file_name, format!("cannot read {path}: {e}")))?;
            let value = content.trim();
            if value.is_empty() {
                Err(error(&file_name, format!("{path} is empty")))
            } else {
                Ok(Some(Secret::new(value)))
            }
        }
        (None, None) => Ok(None),
    }
}

fn log_level(source: &dyn Source) -> Result<tracing::Level, ConfigError> {
    let Some(value) = var(source, "LOG_LEVEL") else {
        return Ok(tracing::Level::INFO);
    };
    match value.trim().to_ascii_lowercase().as_str() {
        "error" => Ok(tracing::Level::ERROR),
        "warn" => Ok(tracing::Level::WARN),
        "info" => Ok(tracing::Level::INFO),
        "debug" => Ok(tracing::Level::DEBUG),
        "trace" => Ok(tracing::Level::TRACE),
        _ => Err(error(
            "LOG_LEVEL",
            format!("`{value}` is not one of error, warn, info, debug, trace"),
        )),
    }
}

fn source_url(source: &dyn Source) -> Result<String, ConfigError> {
    let Some(value) = var(source, "SOURCE_URL") else {
        return Ok(DEFAULT_SOURCE_URL.to_string());
    };
    let value = value.trim().to_string();
    let lower = value.to_ascii_lowercase();
    let valid = ["http://", "https://"]
        .iter()
        .any(|scheme| lower.starts_with(scheme) && lower.len() > scheme.len())
        && !value.chars().any(char::is_whitespace);
    if valid {
        Ok(value)
    } else {
        Err(error(
            "SOURCE_URL",
            format!("`{value}` is not an http or https URL"),
        ))
    }
}

fn instance_config(source: &dyn Source, data_dir: &Path) -> Result<InstanceConfig, ConfigError> {
    let (path, required) = match var(source, "CONFIG_FILE") {
        Some(path) => (PathBuf::from(path), true),
        None => (data_dir.join(CONFIG_FILE_NAME), false),
    };
    let text = match source.read(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == io::ErrorKind::NotFound && !required => {
            return Ok(InstanceConfig::default());
        }
        Err(e) => {
            return Err(error(
                "CONFIG_FILE",
                format!("cannot read {}: {e}", path.display()),
            ));
        }
    };
    toml::from_str(&text).map_err(|e| error(path.display().to_string(), e.to_string()))
}

/// Reads and validates every setting.
pub(crate) fn load(source: &dyn Source) -> Result<Settings, ConfigError> {
    let data_dir = PathBuf::from(var(source, "DATA_DIR").unwrap_or(DEFAULT_DATA_DIR.to_string()));
    Ok(Settings {
        log_level: log_level(source)?,
        source_url: source_url(source)?,
        telegram_bot_token: secret(source, "TELEGRAM_BOT_TOKEN")?,
        instance: instance_config(source, &data_dir)?,
        data_dir,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[derive(Default)]
    struct FakeSource {
        vars: HashMap<String, String>,
        files: HashMap<PathBuf, String>,
    }

    impl FakeSource {
        fn var(mut self, name: &str, value: &str) -> Self {
            self.vars.insert(name.to_string(), value.to_string());
            self
        }

        fn file(mut self, path: &str, content: &str) -> Self {
            self.files.insert(PathBuf::from(path), content.to_string());
            self
        }
    }

    impl Source for FakeSource {
        fn var(&self, name: &str) -> Option<String> {
            self.vars.get(name).cloned()
        }

        fn read(&self, path: &Path) -> io::Result<String> {
            self.files
                .get(path)
                .cloned()
                .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
        }
    }

    fn load_error(source: &FakeSource) -> ConfigError {
        load(source).expect_err("configuration must be rejected")
    }

    #[test]
    fn absent_settings_take_their_documented_defaults() {
        let settings = load(&FakeSource::default()).unwrap();
        assert_eq!(settings.data_dir, PathBuf::from("data"));
        assert_eq!(settings.log_level, tracing::Level::INFO);
        assert_eq!(
            settings.source_url,
            "https://github.com/NicoTomasin/secure-notify"
        );
        assert_eq!(settings.telegram_bot_token, None);
        // deduplication.md: grace window 5 minutes, retention one week, dedup records one year.
        assert_eq!(
            settings.instance.correlation.grace_window,
            SignedDuration::from_secs(300)
        );
        assert_eq!(
            settings.instance.retention.events,
            SignedDuration::from_secs(604_800)
        );
        assert_eq!(
            settings.instance.retention.dedup,
            SignedDuration::from_secs(31_536_000)
        );
    }

    #[test]
    fn valid_settings_are_used() {
        let source = FakeSource::default()
            .var("DATA_DIR", "/srv/sn")
            .var("LOG_LEVEL", "debug")
            .var("SOURCE_URL", "https://example.org/fork")
            .var("TELEGRAM_BOT_TOKEN", "123:abc")
            .file(
                "/srv/sn/secure-notify.toml",
                "[correlation]\ngrace_window = \"90s\"\n[retention]\nevents = \"3d\"\n",
            );
        let settings = load(&source).unwrap();
        assert_eq!(settings.data_dir, PathBuf::from("/srv/sn"));
        assert_eq!(settings.log_level, tracing::Level::DEBUG);
        assert_eq!(settings.source_url, "https://example.org/fork");
        assert_eq!(settings.telegram_bot_token.unwrap().expose(), "123:abc");
        assert_eq!(
            settings.instance.correlation.grace_window,
            SignedDuration::from_secs(90)
        );
        assert_eq!(
            settings.instance.retention.events,
            SignedDuration::from_secs(259_200)
        );
    }

    #[test]
    fn invalid_settings_are_rejected_naming_the_setting() {
        let cases = [
            (
                "LOG_LEVEL",
                FakeSource::default().var("LOG_LEVEL", "verbose"),
            ),
            (
                "SOURCE_URL",
                FakeSource::default().var("SOURCE_URL", "ftp://example.org"),
            ),
            (
                "CONFIG_FILE",
                FakeSource::default().var("CONFIG_FILE", "/missing.toml"),
            ),
            (
                "data/secure-notify.toml",
                FakeSource::default().file("data/secure-notify.toml", "[unknown]\nx = 1\n"),
            ),
            (
                "data/secure-notify.toml",
                FakeSource::default()
                    .file("data/secure-notify.toml", "[retention]\nevents = \"0d\"\n"),
            ),
        ];
        for (setting, source) in cases {
            let error = load_error(&source);
            assert_eq!(error.setting.replace('\\', "/"), setting, "{error}");
        }
    }

    #[test]
    fn empty_values_from_the_example_file_count_as_unset() {
        // `.env.example` lists every variable with an empty value.
        let source = FakeSource::default()
            .var("CONFIG_FILE", "")
            .var("SOURCE_URL", "")
            .var("TELEGRAM_BOT_TOKEN", "");
        let settings = load(&source).unwrap();
        assert_eq!(settings.telegram_bot_token, None);
        assert_eq!(settings.source_url, DEFAULT_SOURCE_URL);
    }

    #[test]
    fn secrets_are_read_from_a_file_variant() {
        let source = FakeSource::default()
            .var("TELEGRAM_BOT_TOKEN_FILE", "/run/secrets/token")
            .file("/run/secrets/token", "123:from-file\n");
        let settings = load(&source).unwrap();
        assert_eq!(
            settings.telegram_bot_token.unwrap().expose(),
            "123:from-file"
        );
    }

    #[test]
    fn setting_a_secret_and_its_file_variant_is_an_error_without_the_value() {
        let source = FakeSource::default()
            .var("TELEGRAM_BOT_TOKEN", "123:do-not-print")
            .var("TELEGRAM_BOT_TOKEN_FILE", "/run/secrets/token");
        let error = load_error(&source);
        assert_eq!(error.setting, "TELEGRAM_BOT_TOKEN");
        assert!(!error.to_string().contains("do-not-print"), "{error}");
    }
}
