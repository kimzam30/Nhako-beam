//! "A new version is out": at most once a day, one request to GitHub's public list of Nhako Beam's
//! releases. Nothing about you or your devices is sent (GitHub sees the address it comes from, as
//! with any web page). It never downloads or installs anything; the app shows a row with a link.
//!
//! Settings > About turns it off. Builds that get their updates from a store (F-Droid, Flathub)
//! don't check at all (`EngineConfig::update_url` is `None`).

use std::time::Duration;

use iroh_relay::tls::{CaTlsConfig, default_provider};
use serde::Deserialize;
use url::Url;

use crate::types::UpdateInfo;

/// GitHub's list of releases, newest first. Pre-releases are in it (every 0.x release is one),
/// which the "latest release" address would leave out.
pub const RELEASES_URL: &str = "https://api.github.com/repos/kimzam30/Nhako-beam/releases?per_page=10";
/// How often to look.
pub const CHECK_EVERY: Duration = Duration::from_secs(24 * 60 * 60);
/// After a failed look (offline, GitHub busy), the next try.
pub const RETRY_AFTER: Duration = Duration::from_secs(60 * 60);
const TIMEOUT: Duration = Duration::from_secs(15);

/// The release list, unless this build gets updates elsewhere: `NHAKO_NO_UPDATE_CHECK` set when
/// it was built (the F-Droid build), or running as a Flatpak (Flathub updates it).
pub fn default_url() -> Option<Url> {
    if option_env!("NHAKO_NO_UPDATE_CHECK").is_some() || std::env::var_os("FLATPAK_ID").is_some() {
        return None;
    }
    Url::parse(RELEASES_URL).ok()
}

/// `0.2.10` as numbers, from a version or a tag (`v0.2.10`). `None` for anything else.
fn parse(v: &str) -> Option<(u64, u64, u64)> {
    let mut parts = v.trim().trim_start_matches('v').split('.');
    let out = (parts.next()?.parse().ok()?, parts.next()?.parse().ok()?, parts.next()?.parse().ok()?);
    parts.next().is_none().then_some(out)
}

/// Whether `candidate` is a newer version than `current`.
pub fn newer(candidate: &str, current: &str) -> bool {
    matches!((parse(candidate), parse(current)), (Some(a), Some(b)) if a > b)
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
}

/// The newest release in GitHub's list that's newer than `current`, if any.
pub fn pick(json: &str, current: &str) -> Result<Option<UpdateInfo>, String> {
    let list: Vec<Release> = serde_json::from_str(json).map_err(|e| format!("unexpected answer: {e}"))?;
    let best = list
        .into_iter()
        .filter(|r| !r.draft && parse(&r.tag_name).is_some() && r.html_url.starts_with("https://github.com/"))
        .max_by_key(|r| parse(&r.tag_name));
    Ok(best.filter(|r| newer(&r.tag_name, current)).map(|r| UpdateInfo {
        version: r.tag_name.trim_start_matches('v').to_string(),
        url: r.html_url,
    }))
}

/// Looks for a newer release than `current`.
pub async fn check(url: &Url, current: &str) -> Result<Option<UpdateInfo>, String> {
    let tls = CaTlsConfig::default().client_config(default_provider()).map_err(|e| format!("TLS: {e}"))?;
    let client = reqwest::Client::builder()
        .use_preconfigured_tls(tls)
        .timeout(TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;
    let res = client
        .get(url.as_str())
        .header("accept", "application/vnd.github+json")
        // GitHub wants one; the version is the only thing it says.
        .header("user-agent", format!("NhakoBeam/{current}"))
        .send()
        .await
        .map_err(|e| e.without_url().to_string())?;
    if !res.status().is_success() {
        return Err(format!("HTTP {}", res.status().as_u16()));
    }
    let body = res.text().await.map_err(|e| e.without_url().to_string())?;
    pick(&body, current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions() {
        assert!(newer("v0.2.0", "0.1.0"));
        assert!(newer("0.10.0", "0.9.9"));
        assert!(!newer("v0.1.0", "0.1.0"));
        assert!(!newer("v0.1.0", "0.2.0"));
        assert!(!newer("v1.0.0-rc1", "0.1.0"), "only plain versions");
        assert!(!newer("latest", "0.1.0"));
    }

    #[test]
    fn picks_the_newest_published_release() {
        let json = r#"[
            {"tag_name": "v0.4.0", "html_url": "https://github.com/kimzam30/Nhako-beam/releases/tag/v0.4.0", "draft": true},
            {"tag_name": "v0.2.0", "html_url": "https://github.com/kimzam30/Nhako-beam/releases/tag/v0.2.0", "prerelease": true},
            {"tag_name": "v0.3.0", "html_url": "https://github.com/kimzam30/Nhako-beam/releases/tag/v0.3.0", "prerelease": true},
            {"tag_name": "v0.9.0", "html_url": "https://evil.example/v0.9.0"}
        ]"#;
        let got = pick(json, "0.1.0").unwrap().unwrap();
        assert_eq!(got.version, "0.3.0");
        assert_eq!(got.url, "https://github.com/kimzam30/Nhako-beam/releases/tag/v0.3.0");
        assert_eq!(pick(json, "0.3.0").unwrap(), None);
        assert_eq!(pick("[]", "0.1.0").unwrap(), None);
        assert!(pick(r#"{"message": "API rate limit exceeded"}"#, "0.1.0").is_err());
    }
}
