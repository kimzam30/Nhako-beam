//! Your own server (see `relay/`): reading the settings, choosing a proxy, and checking that the
//! relay and the address lookup server answer.

use std::{
    net::IpAddr,
    time::{Duration, Instant},
};

use iroh::{RelayUrl, SecretKey, dns::DnsResolver};
use iroh_relay::{
    client::{ClientBuilder, ConnectError},
    protos::handshake,
    tls::{CaTlsConfig, default_provider},
};
use url::Url;

use crate::types::{CheckResult, OwnServer, ServerCheck};

/// How long a server check waits for an answer.
pub const CHECK_TIMEOUT: Duration = Duration::from_secs(8);

/// Your own server, checked and ready to use.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnNetwork {
    pub relay: RelayUrl,
    pub token: Option<String>,
    /// Where to publish and look up addresses. `None` uses n0's public lookup.
    pub lookup: Option<Url>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SettingError {
    #[error("relay address: {0}")]
    Relay(String),
    #[error("lookup address: {0}")]
    Lookup(String),
    #[error("token: {0}")]
    Token(String),
}

impl OwnNetwork {
    pub fn parse(s: &OwnServer) -> Result<Self, SettingError> {
        Ok(Self {
            relay: parse_relay(&s.relay).map_err(SettingError::Relay)?,
            token: parse_token(s.token.as_deref()).map_err(SettingError::Token)?,
            lookup: match s.lookup.as_deref().map(str::trim) {
                None | Some("") => None,
                Some(raw) => Some(parse_lookup(raw).map_err(SettingError::Lookup)?),
            },
        })
    }
}

fn parse_http_url(raw: &str) -> Result<Url, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("it's empty".into());
    }
    if !raw.contains("://") {
        return Err("start it with http:// or https://".into());
    }
    let url = Url::parse(raw).map_err(|e| e.to_string())?;
    if !matches!(url.scheme(), "http" | "https") {
        return Err(format!("{}:// can't be used, start it with http:// or https://", url.scheme()));
    }
    if url.host_str().is_none_or(str::is_empty) {
        return Err("it has no host name or IP address".into());
    }
    if url.query().is_some() || url.fragment().is_some() || !url.username().is_empty() || url.password().is_some() {
        return Err("remove the user name and anything after ? or #".into());
    }
    Ok(url)
}

/// A relay address is just the server: `http(s)://host[:port]`.
pub fn parse_relay(raw: &str) -> Result<RelayUrl, String> {
    let url = parse_http_url(raw)?;
    if url.path() != "/" {
        return Err("a relay address has no path, use just the server, for example http://192.168.1.20:3340".into());
    }
    Ok(RelayUrl::from(url))
}

/// The lookup server's pkarr address. A bare server gets `/pkarr`, where `relay/` serves it.
pub fn parse_lookup(raw: &str) -> Result<Url, String> {
    let mut url = parse_http_url(raw)?;
    if url.path() == "/" {
        url.set_path("/pkarr");
    }
    Ok(url)
}

/// Tokens go in an HTTP header, so: printable ASCII, no spaces, at most 512 characters.
pub fn parse_token(raw: Option<&str>) -> Result<Option<String>, String> {
    let Some(t) = raw.map(str::trim).filter(|t| !t.is_empty()) else {
        return Ok(None);
    };
    if t.len() > 512 {
        return Err("it's longer than 512 characters".into());
    }
    if !t.bytes().all(|b| (0x21..=0x7e).contains(&b)) {
        return Err("use only letters, digits and symbols, with no spaces".into());
    }
    Ok(Some(t.to_string()))
}

/// The proxy from `HTTP(S)_PROXY`, unless `NO_PROXY` says to go straight to `host`.
///
/// iroh's own `proxy_from_env` doesn't read `NO_PROXY`, which matters for a relay on your own
/// network: it must not be sent through a company or VPN proxy that can't reach it.
pub fn proxy_for(host: &str) -> Option<Url> {
    let proxy = ["HTTP_PROXY", "http_proxy", "HTTPS_PROXY", "https_proxy"]
        .iter()
        .find_map(|k| std::env::var(k).ok().and_then(|v| Url::parse(&v).ok()))?;
    let no_proxy = std::env::var("NO_PROXY").or_else(|_| std::env::var("no_proxy")).unwrap_or_default();
    (!bypasses_proxy(host, &no_proxy)).then_some(proxy)
}

pub(crate) fn bypasses_proxy(host: &str, no_proxy: &str) -> bool {
    let host = host.trim_start_matches('[').trim_end_matches(']').to_ascii_lowercase();
    let ip: Option<IpAddr> = host.parse().ok();
    no_proxy.split(',').map(str::trim).filter(|e| !e.is_empty()).any(|entry| {
        if entry == "*" {
            return true;
        }
        if let Some((net, bits)) = entry.split_once('/') {
            let (Ok(net), Ok(bits), Some(ip)) = (net.parse::<IpAddr>(), bits.parse::<u32>(), ip) else {
                return false;
            };
            return in_cidr(ip, net, bits);
        }
        if let (Ok(e), Some(ip)) = (entry.trim_start_matches('[').trim_end_matches(']').parse::<IpAddr>(), ip) {
            return e == ip;
        }
        let domain = entry.trim_start_matches("*.").trim_start_matches('.').to_ascii_lowercase();
        host == domain || host.ends_with(&format!(".{domain}"))
    })
}

fn in_cidr(ip: IpAddr, net: IpAddr, bits: u32) -> bool {
    match (ip, net) {
        (IpAddr::V4(ip), IpAddr::V4(net)) if bits <= 32 => {
            let mask = u32::MAX.checked_shl(32 - bits).unwrap_or(0);
            u32::from(ip) & mask == u32::from(net) & mask
        }
        (IpAddr::V6(ip), IpAddr::V6(net)) if bits <= 128 => {
            let mask = u128::MAX.checked_shl(128 - bits).unwrap_or(0);
            u128::from(ip) & mask == u128::from(net) & mask
        }
        _ => false,
    }
}

/// Checks both servers at once. Never fails: problems come back inside the result.
pub async fn check_server(s: &OwnServer) -> ServerCheck {
    let relay = async {
        let url = match parse_relay(&s.relay) {
            Ok(u) => u,
            Err(reason) => return CheckResult::Invalid { reason },
        };
        match parse_token(s.token.as_deref()) {
            Ok(token) => check_relay(&url, token.as_deref()).await,
            Err(reason) => CheckResult::Invalid {
                reason: format!("token: {reason}"),
            },
        }
    };
    let lookup = async {
        let raw = s.lookup.as_deref().map(str::trim).filter(|l| !l.is_empty())?;
        Some(match parse_lookup(raw) {
            Ok(url) => check_lookup(&url).await,
            Err(reason) => CheckResult::Invalid { reason },
        })
    };
    let (relay, lookup) = tokio::join!(relay, lookup);
    ServerCheck { relay, lookup }
}

fn elapsed_ms(start: Instant) -> u32 {
    start.elapsed().as_millis().try_into().unwrap_or(u32::MAX)
}

fn nothing_there() -> CheckResult {
    CheckResult::Unreachable {
        reason: "nothing answered at that address. Is the server running, and is the port right?".into(),
    }
}

fn no_answer() -> CheckResult {
    CheckResult::Unreachable {
        reason: format!("no answer within {} seconds", CHECK_TIMEOUT.as_secs()),
    }
}

/// Opens a real relay session with a throwaway key, so a wrong token shows up as such.
pub async fn check_relay(url: &RelayUrl, token: Option<&str>) -> CheckResult {
    let tls = match CaTlsConfig::default().client_config(default_provider()) {
        Ok(t) => t,
        Err(e) => {
            return CheckResult::Unreachable {
                reason: format!("couldn't set up TLS: {e}"),
            };
        }
    };
    let mut b = ClientBuilder::new(url.clone(), SecretKey::generate(), DnsResolver::new()).tls_client_config(tls);
    if let Some(t) = token {
        b = b.auth_token(t);
    }
    if let Some(p) = url.host_str().and_then(proxy_for) {
        b = b.proxy_url(p);
    }
    let start = Instant::now();
    match tokio::time::timeout(CHECK_TIMEOUT, b.connect()).await {
        Err(_) => no_answer(),
        Ok(Ok(client)) => {
            let ms = elapsed_ms(start);
            drop(client);
            CheckResult::Ok { ms }
        }
        Ok(Err(ConnectError::Handshake {
            source: handshake::Error::ServerDeniedAuth { .. },
            ..
        })) => CheckResult::TokenRejected,
        Ok(Err(ConnectError::Dial { .. })) => nothing_there(),
        Ok(Err(e)) => CheckResult::Unreachable {
            reason: format!("something answered, but not like an iroh relay ({e})"),
        },
    }
}

/// Asks for a record that doesn't exist: a pkarr server answers 404, anything else is suspect.
pub async fn check_lookup(url: &Url) -> CheckResult {
    let tls = match CaTlsConfig::default().client_config(default_provider()) {
        Ok(t) => t,
        Err(e) => {
            return CheckResult::Unreachable {
                reason: format!("couldn't set up TLS: {e}"),
            };
        }
    };
    let mut b = reqwest::Client::builder().use_preconfigured_tls(tls).timeout(CHECK_TIMEOUT).no_proxy();
    if let Some(p) = url.host_str().and_then(proxy_for) {
        match reqwest::Proxy::all(p.as_str()) {
            Ok(p) => b = b.proxy(p),
            Err(e) => {
                return CheckResult::Unreachable {
                    reason: format!("the proxy setting can't be used: {e}"),
                };
            }
        }
    }
    let client = match b.build() {
        Ok(c) => c,
        Err(e) => {
            return CheckResult::Unreachable {
                reason: format!("couldn't set up HTTP: {e}"),
            };
        }
    };
    let probe = format!("{}/{}", url.as_str().trim_end_matches('/'), SecretKey::generate().public().to_z32());
    let start = Instant::now();
    match client.get(&probe).send().await {
        Ok(r) if r.status() == reqwest::StatusCode::NOT_FOUND || r.status().is_success() => CheckResult::Ok { ms: elapsed_ms(start) },
        Ok(r) => CheckResult::Unreachable {
            reason: format!("it answered with HTTP {}, so it may not be a lookup server", r.status().as_u16()),
        },
        Err(e) if e.is_timeout() => no_answer(),
        Err(e) if e.is_connect() => nothing_there(),
        Err(e) => CheckResult::Unreachable {
            reason: format!("couldn't connect ({})", e.without_url()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_addresses() {
        assert_eq!(parse_relay(" http://192.168.1.20:3340 ").unwrap().to_string(), "http://192.168.1.20:3340/");
        assert_eq!(parse_relay("https://relay.example.com/").unwrap().to_string(), "https://relay.example.com/");
        assert!(parse_relay("").is_err());
        assert!(parse_relay("192.168.1.20:3340").unwrap_err().contains("http://"));
        assert!(parse_relay("ftp://x").is_err());
        assert!(parse_relay("http://x/relay").unwrap_err().contains("no path"));
        assert!(parse_relay("http://x/?token=1").is_err());
        assert!(parse_relay("http://user:pw@x/").is_err());
    }

    #[test]
    fn lookup_addresses() {
        assert_eq!(parse_lookup("http://10.0.0.2:3341").unwrap().as_str(), "http://10.0.0.2:3341/pkarr");
        assert_eq!(parse_lookup("https://dns.example.com/pkarr").unwrap().as_str(), "https://dns.example.com/pkarr");
        assert!(parse_lookup("dns.example.com").is_err());
    }

    #[test]
    fn tokens() {
        assert_eq!(parse_token(None).unwrap(), None);
        assert_eq!(parse_token(Some("  ")).unwrap(), None);
        assert_eq!(parse_token(Some(" ab-12_x ")).unwrap().as_deref(), Some("ab-12_x"));
        assert!(parse_token(Some("two words")).is_err());
        assert!(parse_token(Some("caf\u{e9}")).is_err());
        assert!(parse_token(Some(&"a".repeat(513))).is_err());
    }

    #[test]
    fn no_proxy_rules() {
        let rules = "localhost,127.0.0.0/8,::1,.example.com,192.168.0.0/16,fd00::/8";
        for host in ["localhost", "127.0.0.1", "[::1]", "relay.example.com", "example.com", "192.168.1.20", "fd12::1"] {
            assert!(bypasses_proxy(host, rules), "{host} should skip the proxy");
        }
        for host in ["relay.example.org", "10.0.0.2", "badexample.com", "2001:db8::1"] {
            assert!(!bypasses_proxy(host, rules), "{host} should use the proxy");
        }
        assert!(bypasses_proxy("anything", "*"));
        assert!(!bypasses_proxy("anything", ""));
    }
}
