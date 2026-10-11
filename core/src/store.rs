//! What the engine keeps on disk: the device key, its name, where files go and trusted devices.
//!
//! Files in the config folder:
//! - `identity.key`: the device secret key, hex. Owner read/write only on Unix.
//! - `state.json`: everything else.

use std::{
    io::Write,
    path::{Path, PathBuf},
};

use iroh::{EndpointId, SecretKey};
use serde::{Deserialize, Serialize};

use crate::types::{Connection, DeviceKind, OsPlatform};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustedDevice {
    pub id: EndpointId,
    pub name: String,
    pub kind: DeviceKind,
    pub os: OsPlatform,
    pub auto_accept: bool,
    /// Epoch milliseconds.
    pub added_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub name: String,
    pub kind: DeviceKind,
    pub save_dir: PathBuf,
    #[serde(default)]
    pub trusted: Vec<TrustedDevice>,
    /// Public servers or your own. Takes effect the next time the engine starts.
    #[serde(default)]
    pub connection: Connection,
}

#[derive(Debug)]
pub struct Store {
    dir: PathBuf,
    pub secret: SecretKey,
    pub state: State,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("can't use the config folder {path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },
    #[error("{path} is damaged: {reason}")]
    Corrupt { path: PathBuf, reason: String },
}

fn io_err(path: &Path) -> impl FnOnce(std::io::Error) -> StoreError + '_ {
    move |source| StoreError::Io {
        path: path.to_path_buf(),
        source,
    }
}

impl Store {
    /// Opens the config folder, creating the key and defaults on first run.
    pub fn open(dir: &Path, default_save_dir: PathBuf) -> Result<Self, StoreError> {
        std::fs::create_dir_all(dir).map_err(io_err(dir))?;
        let key_path = dir.join("identity.key");
        let secret = match std::fs::read_to_string(&key_path) {
            Ok(s) => {
                let bytes: [u8; 32] = data_encoding::HEXLOWER
                    .decode(s.trim().as_bytes())
                    .ok()
                    .and_then(|b| b.try_into().ok())
                    .ok_or_else(|| StoreError::Corrupt {
                        path: key_path.clone(),
                        reason: "not a 32 byte hex key".into(),
                    })?;
                SecretKey::from_bytes(&bytes)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let key = SecretKey::generate();
                write_private(&key_path, data_encoding::HEXLOWER.encode(&key.to_bytes()).as_bytes()).map_err(io_err(&key_path))?;
                key
            }
            Err(e) => return Err(io_err(&key_path)(e)),
        };

        let state_path = dir.join("state.json");
        let state = match std::fs::read(&state_path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| StoreError::Corrupt {
                path: state_path.clone(),
                reason: e.to_string(),
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let kind = default_kind();
                State {
                    name: default_name(kind),
                    kind,
                    save_dir: default_save_dir,
                    trusted: vec![],
                    connection: Connection::Public,
                }
            }
            Err(e) => return Err(io_err(&state_path)(e)),
        };
        let store = Self {
            dir: dir.to_path_buf(),
            secret,
            state,
        };
        store.save()?;
        Ok(store)
    }

    /// Writes `state.json` atomically (write to a temp file, then rename).
    pub fn save(&self) -> Result<(), StoreError> {
        let path = self.dir.join("state.json");
        let tmp = self.dir.join("state.json.tmp");
        let json = serde_json::to_vec_pretty(&self.state).expect("state serializes");
        std::fs::write(&tmp, json).map_err(io_err(&tmp))?;
        std::fs::rename(&tmp, &path).map_err(io_err(&path))
    }

    pub fn trusted(&self, id: &EndpointId) -> Option<&TrustedDevice> {
        self.state.trusted.iter().find(|d| &d.id == id)
    }

    pub fn trusted_mut(&mut self, id: &EndpointId) -> Option<&mut TrustedDevice> {
        self.state.trusted.iter_mut().find(|d| &d.id == id)
    }
}

/// Writes a new file only this user can read. On Windows the config folder is in the user's own
/// profile (%APPDATA%), which other users can't read already.
fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut f = opts.open(path)?;
    f.write_all(bytes)?;
    f.sync_all()
}

/// Laptop if the machine has a battery, otherwise desktop. Phones set their own kind.
pub fn default_kind() -> DeviceKind {
    #[cfg(target_os = "android")]
    return DeviceKind::Phone;
    #[cfg(target_os = "linux")]
    {
        let has_battery = std::fs::read_dir("/sys/class/power_supply")
            .map(|d| d.flatten().any(|e| e.file_name().to_string_lossy().starts_with("BAT")))
            .unwrap_or(false);
        return if has_battery { DeviceKind::Laptop } else { DeviceKind::Desktop };
    }
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
        // BatteryFlag 128 means "no system battery"; 255 means Windows doesn't know.
        let mut status: SYSTEM_POWER_STATUS = unsafe { std::mem::zeroed() };
        let ok = unsafe { GetSystemPowerStatus(&mut status) } != 0;
        return if ok && status.BatteryFlag == 128 {
            DeviceKind::Desktop
        } else {
            DeviceKind::Laptop
        };
    }
    #[allow(unreachable_code)]
    DeviceKind::Laptop
}

/// "Hakim's laptop", from the login name, or just "My laptop".
pub fn default_name(kind: DeviceKind) -> String {
    let what = match kind {
        DeviceKind::Laptop => "laptop",
        DeviceKind::Desktop => "PC",
        DeviceKind::Phone => "phone",
        DeviceKind::Tablet => "tablet",
    };
    let user = std::env::var("USER").or_else(|_| std::env::var("USERNAME")).unwrap_or_default();
    let user = user.trim();
    if user.is_empty() || user == "root" {
        return format!("My {what}");
    }
    let mut chars = user.chars();
    let first = chars.next().map(|c| c.to_uppercase().collect::<String>()).unwrap_or_default();
    format!("{first}{}'s {what}", chars.as_str())
}

/// `Downloads/Nhako Beam`, or `Nhako Beam` in the home folder if there is no Downloads folder.
pub fn default_save_dir() -> PathBuf {
    let base = directories::UserDirs::new()
        .and_then(|u| u.download_dir().map(Path::to_path_buf).or_else(|| Some(u.home_dir().to_path_buf())))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("Nhako Beam")
}

/// The platform config folder, for example `~/.config/nhakobeam` on Linux.
pub fn default_config_dir() -> PathBuf {
    directories::ProjectDirs::from("io.github", "kimzam30", "nhakobeam")
        .map(|p| p.config_dir().to_path_buf())
        .unwrap_or_else(|| PathBuf::from(".nhakobeam"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_and_reopens_the_same_identity() {
        let dir = tempdir("store");
        let a = Store::open(&dir, dir.join("save")).unwrap();
        let b = Store::open(&dir, dir.join("elsewhere")).unwrap();
        assert_eq!(a.secret.public(), b.secret.public());
        assert_eq!(b.state.save_dir, dir.join("save"), "saved state wins over the default");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(dir.join("identity.key")).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }

    #[test]
    fn names_read_naturally() {
        assert!(default_name(DeviceKind::Laptop).ends_with("laptop"));
        assert!(!default_name(DeviceKind::Desktop).contains("root"));
    }

    pub(crate) fn tempdir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nhako-test-{tag}-{}", data_encoding::HEXLOWER.encode(&rand::random::<[u8; 6]>())));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
