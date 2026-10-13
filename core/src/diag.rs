//! Copy diagnostics: a plain-text report on this device's network and nearby discovery, with the
//! last lines of the log, to paste into a bug report. Device names, file names and internet
//! addresses are left out, so it can be posted publicly.

use std::{fmt::Write, time::Instant};

use crate::{
    Engine,
    engine::now_ms,
    logfile,
    net::{self, Network},
    types::{DiscoveryState, OsPlatform},
};

/// How much of the log goes into the report.
const LOG_TAIL: usize = 48 * 1024;

impl Engine {
    /// The diagnostics report. `app` is the app's own line, for example "desktop 0.1.0".
    pub fn diagnostics(&self, app: &str) -> String {
        let mut o = String::new();
        let _ = self.write_diagnostics(&mut o, app);
        o
    }

    fn write_diagnostics(&self, o: &mut String, app: &str) -> std::fmt::Result {
        let ep = &self.inner.endpoint;
        writeln!(o, "Nhako Beam diagnostics")?;
        writeln!(o, "App: {app} (engine {})", env!("CARGO_PKG_VERSION"))?;
        writeln!(o, "System: {:?} {}", OsPlatform::current(), std::env::consts::ARCH)?;
        writeln!(o, "Time: {}", utc(now_ms()))?;
        writeln!(o, "Device: {} ({:?})", self.id().fmt_short(), self.store().state.kind)?;
        writeln!(o)?;

        writeln!(o, "Connection")?;
        let servers = match &self.inner.network {
            Network::N0 => "public servers".to_string(),
            Network::Own(own) => format!("your own server ({})", own.relay.host_str().unwrap_or("?")),
            Network::LocalOnly => "same network only".to_string(),
        };
        writeln!(o, "  Servers: {servers}")?;
        let addr = ep.addr();
        let relay = addr.relay_urls().next().map(|_| "connected").unwrap_or("not connected");
        writeln!(o, "  Relay: {relay}")?;
        let sockets: Vec<String> = ep.bound_sockets().iter().map(|a| a.to_string()).collect();
        writeln!(o, "  Listening on: {}", sockets.join(", "))?;
        let (lan, internet): (Vec<&std::net::SocketAddr>, Vec<_>) = addr.ip_addrs().partition(|a| net::is_lan(a.ip()));
        let lan: Vec<String> = lan.iter().map(|a| a.to_string()).collect();
        writeln!(o, "  Same-network addresses: {}", if lan.is_empty() { "none".into() } else { lan.join(", ") })?;
        writeln!(o, "  Internet addresses: {} (not shown)", internet.len())?;
        writeln!(o)?;

        writeln!(o, "Nearby discovery")?;
        let (state, devices) = {
            let st = self.st();
            (st.discovery, st.devices.len())
        };
        writeln!(o, "  Setting: {}", if self.lan_discovery() { "on" } else { "off" })?;
        writeln!(o, "  State: {}", discovery_name(state))?;
        let p = &self.inner.nearby_problems;
        let mdns = match (&self.inner.mdns, &p.mdns) {
            (Some(_), _) => match &p.mdns_v4 {
                None => "running (IPv4 and IPv6)".to_string(),
                Some(e) => format!("running on IPv6 only: UDP port 5353 is taken for IPv4 ({e})"),
            },
            (None, Some(e)) => format!("couldn't start: {e}"),
            (None, None) => "off".to_string(),
        };
        writeln!(o, "  mDNS: {mdns}")?;
        match &self.inner.beacon {
            Some(b) => {
                let s = b.status();
                writeln!(o, "  Beacon: listening on UDP {}", s.port)?;
                if s.adapters.is_empty() {
                    writeln!(o, "    Adapters: none listed (sending on the default one)")?;
                }
                for a in &s.adapters {
                    writeln!(o, "    Adapter: {} {}/{}", a.name, a.addr, a.prefix)?;
                }
                let heard = s.last_heard.map(|t| format!("{} s ago", Instant::now().duration_since(t).as_secs()));
                writeln!(o, "    Sent: {}, failed: {}", s.sent, s.send_failures)?;
                if let Some(e) = &s.last_send_error {
                    writeln!(o, "    Last send error: {e}")?;
                }
                writeln!(o, "    Heard: {} (last {})", s.received, heard.as_deref().unwrap_or("never"))?;
            }
            None => match &p.beacon {
                Some(e) => writeln!(o, "  Beacon: couldn't start: {e}")?,
                None => writeln!(o, "  Beacon: off")?,
            },
        }
        writeln!(o)?;

        writeln!(o, "Devices ({devices})")?;
        for d in self.devices() {
            let route = d.route.map(|r| format!("{r:?}")).unwrap_or_else(|| "-".into());
            let id = crate::parse_device_id(&d.id).map(|i| i.fmt_short().to_string()).unwrap_or_default();
            writeln!(
                o,
                "  {id} {:?}/{:?}: {}{}, route {route}",
                d.os,
                d.kind,
                if d.trusted { "trusted, " } else { "" },
                if d.reachable { "reachable" } else { "not reachable" },
            )?;
        }
        writeln!(o)?;

        write_log(o)
    }
}

/// The report when the engine isn't running (it couldn't start, or is restarting): the app line,
/// the system and the log.
pub fn diagnostics_without_engine(app: &str) -> String {
    let mut o = String::new();
    let _ = writeln!(o, "Nhako Beam diagnostics");
    let _ = writeln!(o, "App: {app} (engine {})", env!("CARGO_PKG_VERSION"));
    let _ = writeln!(o, "System: {:?} {}", OsPlatform::current(), std::env::consts::ARCH);
    let _ = writeln!(o, "Time: {}", utc(now_ms()));
    let _ = writeln!(o, "The engine isn't running.\n");
    let _ = write_log(&mut o);
    o
}

fn write_log(o: &mut String) -> std::fmt::Result {
    match logfile::installed() {
        Some(log) => {
            writeln!(o, "Recent log ({})", log.path().display())?;
            o.push_str(&log.tail(LOG_TAIL));
        }
        None => writeln!(o, "No log file")?,
    }
    Ok(())
}

fn discovery_name(s: DiscoveryState) -> &'static str {
    match s {
        DiscoveryState::Searching => "searching",
        DiscoveryState::Idle => "idle",
        DiscoveryState::Blocked => "blocked",
        DiscoveryState::Offline => "offline",
    }
}

/// `2026-10-13 09:41:07 UTC` from Unix milliseconds.
fn utc(ms: u64) -> String {
    let secs = ms / 1000;
    let (days, rem) = ((secs / 86_400) as i64, secs % 86_400);
    // Howard Hinnant's days-to-civil.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02} UTC", rem / 3600, rem % 3600 / 60, rem % 60)
}

#[cfg(test)]
mod tests {
    #[test]
    fn utc() {
        assert_eq!(super::utc(0), "1970-01-01 00:00:00 UTC");
        assert_eq!(super::utc(1_791_884_467_000), "2026-10-13 09:41:07 UTC");
        assert_eq!(super::utc(951_782_400_000), "2000-02-29 00:00:00 UTC");
    }
}
