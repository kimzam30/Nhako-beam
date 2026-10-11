//! "Send with Nhako Beam" in the file manager's right-click menu, and the `--send` option it uses.
//!
//! - Windows: an Explorer menu entry for the current user (`HKCU\Software\Classes\*\shell\NhakoBeam`).
//!   On Windows 11 it is under "Show more options". The uninstaller removes it.
//! - Linux: a Files (Nautilus) script, in the right-click menu under Scripts.
//!
//! Both run `nhako-beam --send FILE...`. When the app is already running, the new launch hands the
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
    /// `None` where there's no menu to add (other platforms).
    pub manager: Option<FileManager>,
    pub on: bool,
}

pub fn state(data_dir: &Path) -> MenuState {
    MenuState {
        manager: imp::MANAGER,
        on: imp::installed(data_dir),
    }
}

/// Adds or removes the menu entry, pointing it at this copy of the app.
pub fn set(data_dir: &Path, on: bool) -> std::io::Result<()> {
    let exe = std::env::current_exe()?;
    if on { imp::install(data_dir, &exe) } else { imp::remove(data_dir) }
}

/// Keeps an existing entry pointing at this copy of the app (it may have moved after an update).
pub fn refresh(data_dir: &Path) {
    if imp::installed(data_dir)
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
    /// Also deleted by the uninstaller (src-tauri/windows/hooks.nsh): keep the two in step.
    const KEY: &str = r"Software\Classes\*\shell\NhakoBeam";

    pub fn installed(_: &Path) -> bool {
        RegKey::predef(HKEY_CURRENT_USER).open_subkey(format!(r"{KEY}\command")).is_ok()
    }

    pub fn install(_: &Path, exe: &Path) -> io::Result<()> {
        let exe = exe.display();
        let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(KEY)?;
        key.set_value("MUIVerb", &"Send with Nhako Beam")?;
        key.set_value("Icon", &format!("\"{exe}\",0"))?;
        // Show it for any number of selected files (Explorer hides verbs past 15 otherwise).
        key.set_value("MultiSelectModel", &"Player")?;
        let (cmd, _) = key.create_subkey("command")?;
        cmd.set_value("", &format!("\"{exe}\" {} \"%1\"", super::SEND_ARG))?;
        Ok(())
    }

    pub fn remove(_: &Path) -> io::Result<()> {
        match RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(KEY) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use std::{io, path::Path, path::PathBuf};

    use super::FileManager;

    pub const MANAGER: Option<FileManager> = Some(FileManager::Files);
    const NAME: &str = "Send with Nhako Beam";

    fn script(data_dir: &Path) -> PathBuf {
        data_dir.join("nautilus").join("scripts").join(NAME)
    }

    pub fn installed(data_dir: &Path) -> bool {
        script(data_dir).is_file()
    }

    pub fn install(data_dir: &Path, exe: &Path) -> io::Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let path = script(data_dir);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let exe = exe.display().to_string().replace('\'', r"'\''");
        let body = format!(
            "#!/bin/sh\n# Added by Nhako Beam (Settings > On this computer). Sends the files selected in Files.\nexec '{exe}' {} \"$@\"\n",
            super::SEND_ARG
        );
        std::fs::write(&path, body)?;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
    }

    pub fn remove(data_dir: &Path) -> io::Result<()> {
        match std::fs::remove_file(script(data_dir)) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            other => other,
        }
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
        set(&dir, false).unwrap();
        assert!(!state(&dir).on);
        set(&dir, false).unwrap();
        let _ = std::fs::remove_dir_all(dir);
    }
}
