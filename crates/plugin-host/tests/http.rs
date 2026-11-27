// SPDX-License-Identifier: AGPL-3.0-or-later

//! The shared HTTP client against a local test server (docs/architecture/plugins.md#source-context,
//! docs/architecture/security.md#threats-and-mitigations). The server listens on loopback, so
//! these tests allow loopback; every other refused range stays refused.

#![allow(clippy::unwrap_used)]

use std::sync::Arc;
use std::time::Duration;

use secure_notify_core::ports::{HttpError, HttpRequest};
use secure_notify_plugin_host::{HttpSettings, SharedHttpClient};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn client(max_bytes: usize) -> Arc<SharedHttpClient> {
    let mut settings = HttpSettings::new("0.0.0-test", "https://example.org/source");
    settings.allow_loopback = true;
    settings.retry_delay = Duration::ZERO;
    settings.min_interval_per_host = Duration::ZERO;
    settings.max_bytes = max_bytes;
    SharedHttpClient::new(settings).unwrap()
}

fn get(url: String) -> HttpRequest {
    HttpRequest {
        url,
        headers: Vec::new(),
    }
}

#[tokio::test]
async fn requests_carry_the_generic_user_agent_and_conditional_validators() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/feed"))
        .and(header(
            "user-agent",
            "secure-notify/0.0.0-test (+https://example.org/source)",
        ))
        .and(header("if-none-match", "\"v1\""))
        .respond_with(ResponseTemplate::new(304))
        .expect(1)
        .mount(&server)
        .await;
    let http = client(1024).scoped(vec!["127.0.0.1".to_string()]);
    let mut request = get(format!("{}/feed", server.uri()));
    request
        .headers
        .push(("If-None-Match".to_string(), "\"v1\"".to_string()));
    let response = http.get(request).await.unwrap();
    assert_eq!(response.status, 304);
}

#[tokio::test]
async fn undeclared_hosts_are_refused_without_a_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;
    let http = client(1024).scoped(vec!["nodejs.org".to_string()]);
    let error = http
        .get(get(format!("{}/feed", server.uri())))
        .await
        .unwrap_err();
    assert!(
        matches!(error, HttpError::Refused(ref m) if m.contains("not declared")),
        "{error}"
    );
}

#[tokio::test]
async fn redirects_are_followed_only_to_declared_public_hosts() {
    let server = MockServer::start().await;
    for (from, to) in [
        ("/same", "/final"),
        ("/undeclared", "https://example.com/x"),
        ("/private", "http://10.0.0.1/x"),
        ("/metadata", "http://169.254.169.254/latest/meta-data/"),
        ("/scheme", "file:///etc/passwd"),
        ("/ftp", "ftp://127.0.0.1/x"),
    ] {
        Mock::given(path(from))
            .respond_with(ResponseTemplate::new(302).insert_header("location", to))
            .mount(&server)
            .await;
    }
    Mock::given(path("/final"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&server)
        .await;
    let http = client(1024).scoped(vec!["127.0.0.1".to_string()]);
    let ok = http
        .get(get(format!("{}/same", server.uri())))
        .await
        .unwrap();
    assert_eq!(ok.body, b"ok");
    for from in ["/undeclared", "/private", "/metadata", "/scheme", "/ftp"] {
        let error = http
            .get(get(format!("{}{from}", server.uri())))
            .await
            .unwrap_err();
        assert!(matches!(error, HttpError::Refused(_)), "{from}: {error}");
    }
}

#[tokio::test]
async fn private_addresses_are_refused_even_when_declared() {
    // A private address is refused before connecting, whatever the manifest declares.
    let mut settings = HttpSettings::new("0.0.0-test", "https://example.org/source");
    settings.retries = 0;
    let http = SharedHttpClient::new(settings)
        .unwrap()
        .scoped(vec!["127.0.0.1".to_string(), "10.0.0.1".to_string()]);
    for url in ["http://127.0.0.1:9/", "http://10.0.0.1/"] {
        let error = http.get(get(url.to_string())).await.unwrap_err();
        assert!(matches!(error, HttpError::Refused(_)), "{url}: {error}");
    }
}

#[tokio::test]
async fn responses_over_the_size_limit_are_refused() {
    let server = MockServer::start().await;
    Mock::given(path("/big"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'x'; 2048]))
        .mount(&server)
        .await;
    let http = client(1024).scoped(vec!["127.0.0.1".to_string()]);
    let error = http
        .get(get(format!("{}/big", server.uri())))
        .await
        .unwrap_err();
    assert_eq!(error, HttpError::TooLarge(1024));
}

#[tokio::test]
async fn server_errors_are_retried() {
    let server = MockServer::start().await;
    Mock::given(path("/flaky"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(2)
        .mount(&server)
        .await;
    Mock::given(path("/flaky"))
        .respond_with(ResponseTemplate::new(200).set_body_string("recovered"))
        .mount(&server)
        .await;
    let http = client(1024).scoped(vec!["127.0.0.1".to_string()]);
    let response = http
        .get(get(format!("{}/flaky", server.uri())))
        .await
        .unwrap();
    assert_eq!(response.body, b"recovered");
}
