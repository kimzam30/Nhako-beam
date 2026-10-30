//! Windows Firewall: whether Nhako Beam's own rule is there, and which profile each network is on.
//!
//! Without a rule, Windows asks the first time the app listens and ticks only Private networks.
//! On a network marked Public (a common default) nothing nearby can then reach the app, while
//! pairing by code still works through the relay (found on the owner's PC after Phase 18). The
//! installer adds the rule below with one admin prompt; the desktop app offers the same as Fix it.

use std::path::Path;

/// The rule's name. Not the app's name: Windows names the rules it makes from its own prompt after
/// the app, and those may allow only Private networks or block.
pub const RULE: &str = "Nhako Beam (nearby devices)";

/// The command (for `cmd /c`) that allows `exe` in on every network profile: it first removes every
/// rule for `exe`, including the ones Windows made from its prompt (a Block rule there would win
/// over any Allow), then adds ours. The installer runs the same (`windows/hooks.nsh`).
pub fn allow_command(exe: &Path) -> String {
    let exe = exe.display();
    format!(
        "netsh advfirewall firewall delete rule name=all program=\"{exe}\" & \
         netsh advfirewall firewall add rule name=\"{RULE}\" dir=in action=allow program=\"{exe}\" enable=yes profile=any"
    )
}

/// One network and its profile ("Public", "Private" or "DomainAuthenticated").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    pub adapter: String,
    pub category: String,
}

/// Reads `Get-NetConnectionProfile` lines of the form `adapter|category`.
pub fn parse_profiles(out: &str) -> Vec<Profile> {
    out.lines()
        .filter_map(|l| {
            let (adapter, category) = l.trim().rsplit_once('|')?;
            (!adapter.is_empty() && !category.is_empty()).then(|| Profile {
                adapter: adapter.to_string(),
                category: category.to_string(),
            })
        })
        .collect()
}

/// Whether our rule exists. `None` off Windows, or when `netsh` couldn't run.
pub fn rule_present() -> Option<bool> {
    #[cfg(windows)]
    {
        // The exit code alone: netsh's text is in the system's language.
        let out = hidden("netsh")
            .args(["advfirewall", "firewall", "show", "rule", &format!("name={RULE}")])
            .output()
            .ok()?;
        Some(out.status.success())
    }
    #[cfg(not(windows))]
    None
}

/// The profile of each connected network. Empty off Windows.
pub fn profiles() -> Vec<Profile> {
    #[cfg(windows)]
    {
        let script = "Get-NetConnectionProfile | ForEach-Object { \"$($_.InterfaceAlias)|$($_.NetworkCategory)\" }";
        match hidden("powershell").args(["-NoProfile", "-NonInteractive", "-Command", script]).output() {
            Ok(out) => parse_profiles(&String::from_utf8_lossy(&out.stdout)),
            Err(_) => Vec::new(),
        }
    }
    #[cfg(not(windows))]
    Vec::new()
}

/// Asks Windows for admin rights (one prompt) and adds the rule for `exe`. Returns once the prompt
/// is answered and the commands ran; `Ok(false)` if the rule still isn't there (the prompt was
/// declined).
pub fn allow(exe: &Path) -> std::io::Result<bool> {
    #[cfg(windows)]
    {
        // PowerShell's Start-Process is the plain way to get the elevation prompt; single quotes
        // inside its string are doubled.
        let cmd = allow_command(exe).replace('\'', "''");
        let script = format!("Start-Process -FilePath cmd -ArgumentList '/c {cmd}' -Verb RunAs -WindowStyle Hidden -Wait");
        let _ = hidden("powershell").args(["-NoProfile", "-NonInteractive", "-Command", &script]).status()?;
        Ok(rule_present() == Some(true))
    }
    #[cfg(not(windows))]
    {
        let _ = exe;
        Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "only on Windows"))
    }
}

/// A command that doesn't flash a console window.
#[cfg(windows)]
fn hidden(program: &str) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let mut c = std::process::Command::new(program);
    c.creation_flags(CREATE_NO_WINDOW);
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rule_covers_every_profile_and_clears_the_prompts_rules_first() {
        let c = allow_command(Path::new(r"C:\Users\kim\AppData\Local\Nhako Beam\nhako-beam.exe"));
        let (delete, add) = c.split_once(" & ").unwrap();
        assert!(delete.contains(r#"delete rule name=all program="C:\Users\kim\AppData\Local\Nhako Beam\nhako-beam.exe""#));
        assert!(add.contains(r#"name="Nhako Beam (nearby devices)""#));
        assert!(add.contains("dir=in action=allow"));
        assert!(add.contains("profile=any"));
    }

    #[test]
    fn reads_network_profiles() {
        let p = parse_profiles("Tailscale|Private\r\nEthernet|Public\r\n\r\nbroken\r\n");
        assert_eq!(
            p,
            vec![
                Profile {
                    adapter: "Tailscale".into(),
                    category: "Private".into()
                },
                Profile {
                    adapter: "Ethernet".into(),
                    category: "Public".into()
                },
            ]
        );
    }
}
