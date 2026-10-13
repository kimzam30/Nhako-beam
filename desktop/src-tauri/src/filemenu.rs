//! "Send with Nhako Beam" in the file manager's right-click menu, and the `--send` option it uses.
//!
//! - Windows: Explorer menu entries for the current user, one for files
//!   (`HKCU\Software\Classes\*\shell\NhakoBeam`) and one for folders (`...\Directory\shell\NhakoBeam`).
//!   On Windows 11 they are under "Show more options". The uninstaller removes them.
//! - Linux: a script for Files (Nautilus) and Nemo, under Scripts in the right-click menu, and a
//!   Dolphin service menu. Each file manager that isn't installed simply never reads its copy.
//!
//! All of them run `nhako-beam --send PATH...`, for files and folders alike. When the app is already running, the new launch hands the
//! files to it (single instance) and exits.

use std::path::{Path, PathBuf};

use serde::Serialize;

/// `nhako-beam --send FILE...`: open with these files ready to send.
pub const SEND_ARG: &str = "--send";

/// The files after `--send`. Relative paths are resolved against `cwd` (the folder the file
/// manager ran us in), and `file://` URLs are accepted too.
pub fn send_paths(args: &[String], cwd: &Path) -> Vec<PathBuf> {
    let Some(i) = args.iter().position(|a| a == SEND_ARG) else {
        return vec![];
    };
    args[i + 1..]
        .iter()
        .filter(|a| !a.starts_with("--"))
        .filter_map(|a| {
            if a.starts_with("file://") {
                return tauri::Url::parse(a).ok()?.to_file_path().ok();
            }
            let p = PathBuf::from(a);
            Some(if p.is_absolute() { p } else { cwd.join(p) })
        })
        .collect()
}

/// Which file manager the menu entry is for, so the app can name it.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FileManager {
    #[cfg_attr(not(windows), allow(dead_code))]
    Explorer,
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    Files,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuState {
    /// `None` where there's no menu to add (other platforms, and a Flatpak).
    pub manager: Option<FileManager>,
    pub on: bool,
    /// Running as a Flatpak: the menu and "Start when you log in" would point inside the sandbox,
    /// where the file manager and the desktop can't start the app, so Settings leaves both out.
    pub sandboxed: bool,
}

/// Running inside a Flatpak (Flathub's build).
pub fn sandboxed() -> bool {
    std::env::var_os("FLATPAK_ID").is_some()
}

pub fn state(data_dir: &Path) -> MenuState {
    let sandboxed = sandboxed();
    MenuState {
        manager: if sandboxed { None } else { imp::MANAGER },
        on: !sandboxed && imp::installed(data_dir),
        sandboxed,
    }
}

/// Adds or removes the menu entry, pointing it at this copy of the app.
pub fn set(data_dir: &Path, on: bool) -> std::io::Result<()> {
    if sandboxed() {
        return Err(std::io::Error::new(std::io::ErrorKind::Unsupported, "not from inside a Flatpak"));
    }
    let exe = std::env::current_exe()?;
    if on { imp::install(data_dir, &exe) } else { imp::remove(data_dir) }
}

/// Keeps an existing entry pointing at this copy of the app (it may have moved after an update).
pub fn refresh(data_dir: &Path) {
    if !sandboxed()
        && imp::installed(data_dir)
        && let Err(e) = set(data_dir, true)
    {
        tracing::warn!("couldn't update the file manager menu: {e}");
    }
}

#[cfg(windows)]
mod imp {
    use std::{io, path::Path};

    use winreg::{RegKey, enums::HKEY_CURRENT_USER};

    use super::FileManager;

    pub const MANAGER: Option<FileManager> = Some(FileManager::Explorer);
    /// Files, then folders. Also deleted by the uninstaller (src-tauri/windows/hooks.nsh): keep
    /// the two in step.
    pub const KEYS: [&str; 2] = [r"Software\Classes\*\shell\NhakoBeam", r"Software\Classes\Directory\shell\NhakoBeam"];

    /// On when the files entry is there. `refresh` then adds the folders one, so an entry from
    /// before folders could be sent gets it too.
    pub fn installed(_: &Path) -> bool {
        RegKey::predef(HKEY_CURRENT_USER).open_subkey(format!(r"{}\command", KEYS[0])).is_ok()
    }

    pub fn install(_: &Path, exe: &Path) -> io::Result<()> {
        let exe = exe.display();
        for k in KEYS {
            let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(k)?;
            key.set_value("MUIVerb", &"Send with Nhako Beam")?;
            key.set_value("Icon", &format!("\"{exe}\",0"))?;
            // Show it for any number of selected items (Explorer hides verbs past 15 otherwise).
            key.set_value("MultiSelectModel", &"Player")?;
            let (cmd, _) = key.create_subkey("command")?;
            cmd.set_value("", &format!("\"{exe}\" {} \"%1\"", super::SEND_ARG))?;
        }
        Ok(())
    }

    pub fn remove(_: &Path) -> io::Result<()> {
        for k in KEYS {
            match RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(k) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                other => other?,
            }
        }
        Ok(())
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use std::{io, path::Path, path::PathBuf};

    use super::FileManager;

    pub const MANAGER: Option<FileManager> = Some(FileManager::Files);
    const NAME: &str = "Send with Nhako Beam";

    /// The scripts for Files (Nautilus) and Nemo (Linux Mint), and Dolphin's service menu (KDE).
    fn places(data_dir: &Path) -> [PathBuf; 3] {
        [
            data_dir.join("nautilus").join("scripts").join(NAME),
            data_dir.join("nemo").join("scripts").join(NAME),
            data_dir.join("kio").join("servicemenus").join("nhako-beam-send.desktop"),
        ]
    }

    /// On when the Files script is there. `refresh` then adds the others, so an entry from before
    /// they existed gets them too.
    pub fn installed(data_dir: &Path) -> bool {
        places(data_dir)[0].is_file()
    }

    /// Quoted for `sh`: inside single quotes, a quote is the only thing to escape.
    fn sh_quote(s: &str) -> String {
        format!("'{}'", s.replace('\'', r"'\''"))
    }

    /// Quoted for a desktop entry's Exec line: double quotes, with `"`, `` ` ``, `$` and `\`
    /// escaped, and `%` doubled.
    pub(super) fn exec_quote(s: &str) -> String {
        let mut out = String::from("\"");
        for c in s.chars() {
            match c {
                '"' | '`' | '$' | '\\' => {
                    out.push('\\');
                    out.push(c);
                }
                '%' => out.push_str("%%"),
                c => out.push(c),
            }
        }
        out.push('"');
        // The desktop entry format reads backslashes once more, so each one is written twice.
        out.replace('\\', r"\\")
    }

    pub fn install(data_dir: &Path, exe: &Path) -> io::Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let exe = exe.display().to_string();
        let script = format!(
            "#!/bin/sh\n# Added by Nhako Beam (Settings > On this computer). Sends the files and folders selected.\nexec {} {} \"$@\"\n",
            sh_quote(&exe),
            super::SEND_ARG
        );
        let service = format!(
            "[Desktop Entry]\n# Added by Nhako Beam (Settings > On this computer).\nType=Service\nMimeType=all/allfiles;inode/directory;\nX-KDE-ServiceTypes=KonqPopupMenu/Plugin\nActions=send;\n\n[Desktop Action send]\nName={NAME}\nIcon=nhako-beam\nExec={} {} %F\n",
            exec_quote(&exe),
            super::SEND_ARG
        );
        let [nautilus, nemo, dolphin] = places(data_dir);
        for (path, body) in [(nautilus, &script), (nemo, &script), (dolphin, &service)] {
            if let Some(dir) = path.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(&path, body)?;
            // Scripts must be runnable, and Dolphin only trusts service menus that are too.
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))?;
        }
        Ok(())
    }

    pub fn remove(data_dir: &Path) -> io::Result<()> {
        for path in places(data_dir) {
            match std::fs::remove_file(path) {
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                other => other?,
            }
        }
        Ok(())
    }
}

#[cfg(not(any(windows, target_os = "linux")))]
mod imp {
    use std::{io, path::Path};

    use super::FileManager;

    pub const MANAGER: Option<FileManager> = None;

    pub fn installed(_: &Path) -> bool {
        false
    }

    pub fn install(_: &Path, _: &Path) -> io::Result<()> {
        Err(io::Error::new(io::ErrorKind::Unsupported, "no file manager menu on this system"))
    }

    pub fn remove(_: &Path) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn reads_files_after_send() {
        // Real absolute paths for whichever system runs the test (C:\... or /...).
        let base = std::env::temp_dir();
        let cwd = base.join("Pictures");
        let notes = base.join("notes.pdf");
        let spaced = base.join("a b.txt");
        let url = tauri::Url::from_file_path(&spaced).unwrap().to_string();
        assert!(url.contains("a%20b.txt"), "{url}");
        assert!(send_paths(&args(&["nhako-beam"]), &cwd).is_empty());
        assert!(send_paths(&args(&["nhako-beam", "--hidden"]), &cwd).is_empty());
        let got = send_paths(&args(&["nhako-beam", "--send", "beach.jpg", &notes.to_string_lossy(), &url]), &cwd);
        assert_eq!(got, vec![cwd.join("beach.jpg"), notes, spaced]);
    }

    #[cfg(windows)]
    #[test]
    fn explorer_entry_goes_in_and_out() {
        use winreg::{RegKey, enums::HKEY_CURRENT_USER};
        let dir = std::env::temp_dir();
        let was_on = state(&dir).on;
        set(&dir, true).unwrap();
        assert!(state(&dir).on);
        let cmd: String = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(r"Software\Classes\*\shell\NhakoBeam\command")
            .unwrap()
            .get_value("")
            .unwrap();
        assert!(cmd.ends_with(r#" --send "%1""#), "{cmd}");
        let folders: String = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey(r"Software\Classes\Directory\shell\NhakoBeam\command")
            .unwrap()
            .get_value("")
            .unwrap();
        assert_eq!(folders, cmd);
        set(&dir, false).unwrap();
        assert!(!state(&dir).on);
        set(&dir, false).unwrap();
        // Leave a developer's own machine as it was.
        if was_on {
            set(&dir, true).unwrap();
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn files_script_goes_in_and_out() {
        let dir = std::env::temp_dir().join(format!("nhako-menu-{}", std::process::id()));
        assert!(!state(&dir).on);
        set(&dir, true).unwrap();
        assert!(state(&dir).on);
        let body = std::fs::read_to_string(dir.join("nautilus/scripts/Send with Nhako Beam")).unwrap();
        assert!(body.contains("--send \"$@\""), "{body}");
        assert_eq!(std::fs::read_to_string(dir.join("nemo/scripts/Send with Nhako Beam")).unwrap(), body);
        let service = std::fs::read_to_string(dir.join("kio/servicemenus/nhako-beam-send.desktop")).unwrap();
        assert!(service.contains("inode/directory") && service.contains(" --send %F"), "{service}");
        // An older entry with only the Files script reads as on, and the others are added.
        std::fs::remove_file(dir.join("nemo/scripts/Send with Nhako Beam")).unwrap();
        assert!(state(&dir).on);
        set(&dir, false).unwrap();
        assert!(!state(&dir).on);
        set(&dir, false).unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn desktop_entry_quoting() {
        let q = imp::exec_quote;
        assert_eq!(q("/usr/bin/nhako-beam"), r#""/usr/bin/nhako-beam""#);
        assert_eq!(q("/home/a b/100%/$x"), r#""/home/a b/100%%/\\$x""#);
    }
}
