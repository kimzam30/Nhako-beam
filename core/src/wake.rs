//! Waking a phone that Android has put to sleep, so it can take files while the app is closed.
//!
//! A phone shares its wake token (a Firebase Cloud Messaging token) with its trusted devices in
//! the hello (`wire::HelloExt`). When one of them can't reach it, it asks the wake service (the
//! Cloudflare Worker in `wake/`) to push an empty "wake up" message to that token; the phone starts
//! its engine, and the sender keeps dialling for a while. The push carries nothing else: no names,
//! no files, not even who is sending.

use std::time::Duration;

use iroh_relay::tls::{CaTlsConfig, default_provider};
use url::Url;

/// How long a sender keeps trying after asking a phone to wake up.
pub const WAKE_FOR: Duration = Duration::from_secs(45);
/// How long the wake service gets to answer.
const TIMEOUT: Duration = Duration::from_secs(10);

/// The wake service: `NHAKO_WAKE_URL` when this program runs, or when it was built.
/// `None` turns waking off (no service set up yet).
pub fn default_url() -> Option<Url> {
    std::env::var("NHAKO_WAKE_URL")
        .ok()
        .or_else(|| option_env!("NHAKO_WAKE_URL").map(str::to_string))
        .and_then(|s| parse_url(&s))
}

/// Only `https://` (or `http://` to a local test server) with a host.
pub fn parse_url(raw: &str) -> Option<Url> {
    let url = Url::parse(raw.trim()).ok()?;
    let local = url.host_str().is_some_and(|h| h == "localhost" || h == "127.0.0.1" || h == "[::1]");
    match url.scheme() {
        "https" => Some(url),
        "http" if local => Some(url),
        _ => None,
    }
}

/// Tokens are what Firebase hands out: letters, digits and `-_:`, a few hundred characters.
pub fn valid_token(token: &str) -> bool {
    (32..=4096).contains(&token.len()) && token.bytes().all(|b| b.is_ascii_alphanumeric() || b"-_:".contains(&b))
}

#[derive(Debug, thiserror::Error)]
pub enum WakeError {
    /// The token is no longer valid (the app was uninstalled, or its data cleared): forget it.
    #[error("the wake token is no longer valid")]
    Gone,
    #[error("the wake service turned it down (HTTP {0})")]
    Refused(u16),
    #[error("couldn't reach the wake service: {0}")]
    Unreachable(String),
}

/// Asks the wake service to wake the device with this token.
pub async fn send(url: &Url, token: &str) -> Result<(), WakeError> {
    let tls = CaTlsConfig::default()
        .client_config(default_provider())
        .map_err(|e| WakeError::Unreachable(format!("TLS: {e}")))?;
    let client = reqwest::Client::builder()
        .use_preconfigured_tls(tls)
        .timeout(TIMEOUT)
        .build()
        .map_err(|e| WakeError::Unreachable(e.to_string()))?;
    let body = serde_json::json!({ "token": token }).to_string();
    let res = client
        .post(url.as_str())
        .header("content-type", "application/json")
        .body(body)
        .send()
        .await
        .map_err(|e| WakeError::Unreachable(e.without_url().to_string()))?;
    match res.status().as_u16() {
        200..=299 => Ok(()),
        404 | 410 => Err(WakeError::Gone),
        code => Err(WakeError::Refused(code)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_https_or_a_local_test_server() {
        assert!(parse_url("https://nhako-wake.example.workers.dev/wake").is_some());
        assert!(parse_url("http://127.0.0.1:8787/wake").is_some());
        assert!(parse_url("http://wake.example.com/wake").is_none());
        assert!(parse_url("ftp://x").is_none());
        assert!(parse_url("nonsense").is_none());
    }

    #[test]
    fn tokens_look_like_firebase_ones() {
        assert!(valid_token(&format!("dQw4w9WgXcQ:APA91b{}", "x".repeat(140))));
        assert!(!valid_token("short"));
        assert!(!valid_token(&format!("{}\"}}", "a".repeat(40))), "nothing that could break out of JSON");
    }
}
