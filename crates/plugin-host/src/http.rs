// SPDX-License-Identifier: AGPL-3.0-or-later

//! The shared HTTP client (docs/architecture/plugins.md#source-context): per-host rate limiting,
//! conditional requests (the caller sends the validators), retries with backoff, timeouts,
//! response size limits, SSRF protection after redirects and DNS resolution, declared hosts only,
//! and a generic User-Agent. It never carries subscriber or operator data.

use std::collections::HashMap;
use std::io::{Seek, SeekFrom, Write};
use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use reqwest::dns::{Addrs, Name, Resolve, Resolving};
use secure_notify_core::ports::{HttpError, HttpRequest, HttpResponse};
use secure_notify_core::sources::{BoxFuture, Download, SourceHttp};
use tokio::time::Instant;
use url::Url;

use crate::ssrf::is_public;

/// Client settings.
#[derive(Debug, Clone)]
pub struct HttpSettings {
    /// The User-Agent header.
    pub user_agent: String,
    /// Timeout of one request, including reading the body.
    pub timeout: Duration,
    /// Timeout of one bulk download, including reading the body.
    pub download_timeout: Duration,
    /// Directory for downloads; the system temporary directory when `None`.
    pub temp_dir: Option<PathBuf>,
    /// Largest accepted response body.
    pub max_bytes: usize,
    /// Minimum time between two requests to the same host.
    pub min_interval_per_host: Duration,
    /// Retries after a transport error, a timeout, `429`, or a `5xx` status.
    pub retries: u32,
    /// Delay before the first retry; doubled for each further retry.
    pub retry_delay: Duration,
    /// Redirects followed per request.
    pub max_redirects: usize,
    /// Allows loopback addresses. Only for tests against a local server; no setting enables it.
    pub allow_loopback: bool,
}

impl HttpSettings {
    /// Defaults with the generic User-Agent `secure-notify/<version> (+<repository URL>)`.
    pub fn new(version: &str, repository_url: &str) -> Self {
        Self {
            user_agent: format!("secure-notify/{version} (+{repository_url})"),
            timeout: Duration::from_secs(60),
            download_timeout: Duration::from_mins(30),
            temp_dir: None,
            max_bytes: 50 * 1024 * 1024,
            min_interval_per_host: Duration::from_secs(1),
            retries: 2,
            retry_delay: Duration::from_secs(2),
            max_redirects: 5,
            allow_loopback: false,
        }
    }
}

fn allowed(ip: IpAddr, allow_loopback: bool) -> bool {
    is_public(ip) || (allow_loopback && ip.is_loopback())
}

/// Resolves names and drops every address the client must not reach, so a public name that
/// resolves to a private address is refused at connection time.
struct SafeResolver {
    allow_loopback: bool,
}

impl Resolve for SafeResolver {
    fn resolve(&self, name: Name) -> Resolving {
        let allow_loopback = self.allow_loopback;
        Box::pin(async move {
            let host = format!("{}:0", name.as_str());
            let resolved: Vec<SocketAddr> = tokio::net::lookup_host(host).await?.collect();
            let safe: Vec<SocketAddr> = resolved
                .into_iter()
                .filter(|a| allowed(a.ip(), allow_loopback))
                .collect();
            if safe.is_empty() {
                let error: Box<dyn std::error::Error + Send + Sync> =
                    format!("{} resolves only to refused addresses", name.as_str()).into();
                return Err(error);
            }
            let addrs: Addrs = Box::new(safe.into_iter());
            Ok(addrs)
        })
    }
}

/// The shared client. Sources get a [`ScopedHttp`] restricted to their declared hosts.
pub struct SharedHttpClient {
    client: reqwest::Client,
    settings: HttpSettings,
    last_request: Mutex<HashMap<String, Instant>>,
}

impl SharedHttpClient {
    /// Builds the client: rustls with embedded web PKI roots, no automatic redirects (each hop
    /// is checked here), the safe resolver.
    pub fn new(settings: HttpSettings) -> Result<Arc<Self>, String> {
        let client = reqwest::Client::builder()
            .user_agent(settings.user_agent.clone())
            .timeout(settings.timeout)
            .redirect(reqwest::redirect::Policy::none())
            .dns_resolver(Arc::new(SafeResolver {
                allow_loopback: settings.allow_loopback,
            }))
            .https_only(false)
            .build()
            .map_err(|e| format!("cannot build the HTTP client: {e}"))?;
        Ok(Arc::new(Self {
            client,
            settings,
            last_request: Mutex::new(HashMap::new()),
        }))
    }

    /// A view of the client that may reach only `hosts`.
    pub fn scoped(self: &Arc<Self>, hosts: Vec<String>) -> ScopedHttp {
        ScopedHttp {
            shared: Arc::clone(self),
            hosts,
        }
    }

    /// Waits until the host may be contacted again.
    async fn pace(&self, host: &str) {
        let wait = {
            let mut last = self
                .last_request
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let now = Instant::now();
            let next = last
                .get(host)
                .map_or(now, |t| *t + self.settings.min_interval_per_host);
            let start = next.max(now);
            last.insert(host.to_string(), start);
            start - now
        };
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
    }
}

/// The shared client restricted to a set of declared hosts.
pub struct ScopedHttp {
    shared: Arc<SharedHttpClient>,
    hosts: Vec<String>,
}

impl ScopedHttp {
    /// Checks scheme, declared host, and literal addresses of a URL.
    fn check(&self, url: &Url) -> Result<String, HttpError> {
        if !matches!(url.scheme(), "http" | "https") {
            return Err(HttpError::Refused(format!(
                "scheme `{}` is not allowed",
                url.scheme()
            )));
        }
        let host = url
            .host_str()
            .ok_or_else(|| HttpError::Refused("URL without host".to_string()))?;
        let bare = host.trim_start_matches('[').trim_end_matches(']');
        if let Ok(ip) = bare.parse::<IpAddr>()
            && !allowed(ip, self.shared.settings.allow_loopback)
        {
            return Err(HttpError::Refused(format!("address {ip} is not allowed")));
        }
        if !self.hosts.iter().any(|h| h.eq_ignore_ascii_case(host)) {
            return Err(HttpError::Refused(format!("host {host} is not declared")));
        }
        Ok(host.to_ascii_lowercase())
    }

    async fn send_once(
        &self,
        url: &Url,
        headers: &[(String, String)],
        max: u64,
        timeout: Duration,
        body: &mut dyn BodySink,
    ) -> Result<Hop, HttpError> {
        let host = self.check(url)?;
        self.shared.pace(&host).await;
        let mut request = self.shared.client.get(url.clone()).timeout(timeout);
        for (name, value) in headers {
            request = request.header(name.as_str(), value.as_str());
        }
        let mut response = request.send().await.map_err(transport)?;
        let status = response.status().as_u16();
        let response_headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .filter_map(|(n, v)| Some((n.as_str().to_string(), v.to_str().ok()?.to_string())))
            .collect();
        if (300..400).contains(&status) && status != 304 {
            let location = response_headers
                .iter()
                .find(|(n, _)| n == "location")
                .map(|(_, v)| v.clone())
                .ok_or_else(|| {
                    HttpError::Transport(format!("redirect {status} without Location"))
                })?;
            let next = url
                .join(&location)
                .map_err(|e| HttpError::Transport(format!("bad redirect: {e}")))?;
            return Ok(Hop::Redirect(next));
        }
        let limit = usize::try_from(max).unwrap_or(usize::MAX);
        if response.content_length().is_some_and(|len| len > max) {
            return Err(HttpError::TooLarge(limit));
        }
        let mut written: u64 = 0;
        while let Some(chunk) = response.chunk().await.map_err(transport)? {
            written += chunk.len() as u64;
            if written > max {
                return Err(HttpError::TooLarge(limit));
            }
            body.write(&chunk)?;
        }
        Ok(Hop::Done(status, response_headers))
    }

    async fn send_following_redirects(
        &self,
        request: &HttpRequest,
        max: u64,
        timeout: Duration,
        body: &mut dyn BodySink,
    ) -> Result<(u16, Vec<(String, String)>), HttpError> {
        let mut url = Url::parse(&request.url)
            .map_err(|e| HttpError::Refused(format!("invalid URL: {e}")))?;
        for _ in 0..=self.shared.settings.max_redirects {
            match self
                .send_once(&url, &request.headers, max, timeout, body)
                .await?
            {
                Hop::Done(status, headers) => return Ok((status, headers)),
                Hop::Redirect(next) => url = next,
            }
        }
        Err(HttpError::Transport("too many redirects".to_string()))
    }

    /// Retries `attempt` after transport errors, timeouts, `429`, and `5xx`.
    async fn with_retries<T, F, Fut>(&self, mut attempt: F) -> Result<T, HttpError>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<(u16, T), HttpError>>,
    {
        let settings = &self.shared.settings;
        let mut delay = settings.retry_delay;
        let mut tries = 0;
        loop {
            let result = attempt().await;
            let retryable = match &result {
                Ok((status, _)) => *status == 429 || *status >= 500,
                Err(HttpError::Timeout | HttpError::Transport(_)) => true,
                Err(HttpError::Refused(_) | HttpError::TooLarge(_)) => false,
            };
            if !retryable || tries >= settings.retries {
                return result.map(|(_, value)| value);
            }
            tries += 1;
            tokio::time::sleep(delay).await;
            delay *= 2;
        }
    }

    /// Performs a `GET` with retries.
    pub async fn get(&self, request: HttpRequest) -> Result<HttpResponse, HttpError> {
        let max = self.shared.settings.max_bytes as u64;
        let timeout = self.shared.settings.timeout;
        self.with_retries(|| async {
            let mut body = Vec::new();
            let (status, headers) = self
                .send_following_redirects(&request, max, timeout, &mut body)
                .await?;
            Ok((
                status,
                HttpResponse {
                    status,
                    headers,
                    body,
                },
            ))
        })
        .await
    }

    /// Performs a `GET` with retries, streaming the body to an anonymous temporary file.
    pub async fn download(
        &self,
        request: HttpRequest,
        max_bytes: u64,
    ) -> Result<Download, HttpError> {
        let timeout = self.shared.settings.download_timeout;
        let temp_dir = self.shared.settings.temp_dir.clone();
        self.with_retries(|| async {
            let file = match &temp_dir {
                Some(dir) => tempfile::tempfile_in(dir),
                None => tempfile::tempfile(),
            }
            .map_err(|e| HttpError::Transport(format!("cannot create a temporary file: {e}")))?;
            let mut sink = FileSink { file, len: 0 };
            let (status, headers) = self
                .send_following_redirects(&request, max_bytes, timeout, &mut sink)
                .await?;
            let FileSink { mut file, len } = sink;
            file.flush()
                .and_then(|()| file.seek(SeekFrom::Start(0)).map(|_| ()))
                .map_err(|e| HttpError::Transport(format!("temporary file: {e}")))?;
            Ok((
                status,
                Download {
                    status,
                    headers,
                    file,
                    len,
                },
            ))
        })
        .await
    }
}

/// Where a response body goes.
trait BodySink: Send {
    fn write(&mut self, chunk: &[u8]) -> Result<(), HttpError>;
}

impl BodySink for Vec<u8> {
    fn write(&mut self, chunk: &[u8]) -> Result<(), HttpError> {
        self.extend_from_slice(chunk);
        Ok(())
    }
}

struct FileSink {
    file: std::fs::File,
    len: u64,
}

impl BodySink for FileSink {
    fn write(&mut self, chunk: &[u8]) -> Result<(), HttpError> {
        self.len += chunk.len() as u64;
        self.file
            .write_all(chunk)
            .map_err(|e| HttpError::Transport(format!("temporary file: {e}")))
    }
}

enum Hop {
    Done(u16, Vec<(String, String)>),
    Redirect(Url),
}

fn transport(error: reqwest::Error) -> HttpError {
    if error.is_timeout() {
        HttpError::Timeout
    } else {
        // Never include the URL query or headers, only the error kind and message.
        HttpError::Transport(error.without_url().to_string())
    }
}

impl SourceHttp for ScopedHttp {
    fn get(&self, request: HttpRequest) -> BoxFuture<'_, Result<HttpResponse, HttpError>> {
        Box::pin(ScopedHttp::get(self, request))
    }

    fn download(
        &self,
        request: HttpRequest,
        max_bytes: u64,
    ) -> BoxFuture<'_, Result<Download, HttpError>> {
        Box::pin(ScopedHttp::download(self, request, max_bytes))
    }
}
