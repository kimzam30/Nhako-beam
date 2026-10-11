//! The engine bridge. Each command here is one method of the app's `Engine` contract, and every
//! engine event goes to the window as `nhako://event` with the same JSON the mock engine produces.
//!
//! Every command is `async`: Tauri runs plain commands on the main thread, outside the async
//! runtime, and many engine methods start background tasks that need it.

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use nhako_core::{
    Connection, Engine, EngineConfig, EngineEvent, FileItem, IncomingRequest, Network, OwnServer, PairingCode, RedeemResult, ServerCheck, Snapshot,
    TransferDirection, TransferStatus, check_server as check, files,
    store::{self, Store},
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State, async_runtime::JoinHandle};
use tauri_plugin_opener::OpenerExt;
use tokio::sync::broadcast::error::RecvError;
use tracing::warn;

use crate::shell;

/// Every engine event, as JSON.
pub const EVENT: &str = "nhako://event";
/// The window should redraw from this snapshot (after the engine restarted, or the window fell behind).
pub const RESYNC: &str = "nhako://resync";

pub struct Host {
    config_dir: PathBuf,
    engine: Mutex<Option<Engine>>,
    forward: Mutex<Option<JoinHandle<()>>>,
    /// Starting, stopping and restarting happen one at a time.
    lifecycle: tokio::sync::Mutex<()>,
    /// Files the window picked or dropped: the app only sees ids, the engine needs paths.
    files: Mutex<HashMap<String, PathBuf>>,
    file_seq: AtomicU64,
}

impl Host {
    pub fn new(config_dir: PathBuf) -> Self {
        Self {
            config_dir,
            engine: Mutex::new(None),
            forward: Mutex::new(None),
            lifecycle: tokio::sync::Mutex::new(()),
            files: Mutex::new(HashMap::new()),
            file_seq: AtomicU64::new(0),
        }
    }

    fn running(&self) -> Option<Engine> {
        self.engine.lock().expect("engine lock").clone()
    }

    fn engine(&self) -> Result<Engine, String> {
        self.running().ok_or_else(|| "Nhako Beam is still starting. Try again in a moment.".to_string())
    }

    /// The saved settings, read straight from disk while the engine isn't running (during onboarding).
    /// Only call it with `lifecycle` held: the first open creates the device key, and two at once
    /// could read a half-written key or make two keys.
    fn store(&self) -> Result<Store, String> {
        Store::open(&self.config_dir, store::default_save_dir()).map_err(|e| format!("Couldn't read the settings: {e}"))
    }

    /// Uses the running engine, or the saved settings on disk when it isn't running. Holds
    /// `lifecycle`, so the engine can't start (and open the same files) halfway through.
    async fn settings<T>(&self, running: impl FnOnce(&Engine) -> T, stored: impl FnOnce(&mut Store) -> Result<T, String>) -> Result<T, String> {
        let _g = self.lifecycle.lock().await;
        match self.running() {
            Some(e) => Ok(running(&e)),
            None => stored(&mut self.store()?),
        }
    }

    async fn start(&self, app: &AppHandle) -> Result<Engine, String> {
        let engine = Engine::start(EngineConfig::new(&self.config_dir)).await.map_err(|e| e.to_string())?;
        let task = tauri::async_runtime::spawn(forward(app.clone(), engine.clone()));
        *self.forward.lock().expect("forward lock") = Some(task);
        *self.engine.lock().expect("engine lock") = Some(engine.clone());
        Ok(engine)
    }

    pub async fn stop(&self) {
        let _g = self.lifecycle.lock().await;
        self.stop_locked().await;
    }

    async fn stop_locked(&self) {
        if let Some(task) = self.forward.lock().expect("forward lock").take() {
            task.abort();
        }
        let engine = self.engine.lock().expect("engine lock").take();
        if let Some(e) = engine {
            let _ = tokio::time::timeout(Duration::from_secs(4), e.stop()).await;
        }
    }
}

/// Passes engine events to the window, and raises a notification when the window isn't looked at.
async fn forward(app: AppHandle, engine: Engine) {
    let mut events = engine.subscribe();
    let mut notified = HashSet::new();
    loop {
        match events.recv().await {
            Ok(event) => {
                notify(&app, &engine, &event, &mut notified);
                let _ = app.emit(EVENT, &event);
            }
            Err(RecvError::Lagged(_)) => {
                let _ = app.emit(RESYNC, engine.snapshot());
            }
            Err(RecvError::Closed) => break,
        }
    }
}

fn notify(app: &AppHandle, engine: &Engine, event: &EngineEvent, notified: &mut HashSet<String>) {
    let name = |id: &str| {
        engine
            .devices()
            .into_iter()
            .find(|d| d.id == id)
            .map(|d| d.name)
            .unwrap_or_else(|| "A device".into())
    };
    match event {
        EngineEvent::Incoming { request } => shell::notify_if_away(
            app,
            &incoming_title(&name(&request.device_id), request),
            "Open Nhako Beam to accept or decline.",
        ),
        // Once per transfer: a finished transfer can be reported more than once.
        EngineEvent::Transfer { transfer }
            if transfer.direction == TransferDirection::Receive && transfer.status == TransferStatus::Done && notified.insert(transfer.id.clone()) =>
        {
            let what = files_phrase(transfer.files.len(), transfer.files.first().map(|f| f.name.as_str()));
            shell::notify_if_away(
                app,
                &format!("Got {what} from {}", name(&transfer.device_id)),
                "Saved to your Nhako Beam folder.",
            );
        }
        _ => {}
    }
}

fn incoming_title(device: &str, r: &IncomingRequest) -> String {
    format!(
        "{device} wants to send you {}",
        files_phrase(r.files.len(), r.files.first().map(|f| f.name.as_str()))
    )
}

fn files_phrase(n: usize, first: Option<&str>) -> String {
    match (n, first) {
        (1, Some(name)) => name.to_string(),
        (n, _) => format!("{n} files"),
    }
}

// ---------- Starting ----------

/// What the window needs when it loads.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Started {
    snapshot: Snapshot,
    me: SelfInfo,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfInfo {
    name: String,
    save_dir: String,
    fingerprint: Option<String>,
    version: String,
}

/// Starts the engine (once). `name` is the name picked during onboarding, saved before the
/// network starts so other devices never see a default name.
#[tauri::command]
pub async fn engine_start(app: AppHandle, host: State<'_, Host>, name: Option<String>) -> Result<Started, String> {
    let _g = host.lifecycle.lock().await;
    if let Some(name) = name.filter(|n| !n.trim().is_empty())
        && host.running().is_none()
    {
        let mut s = host.store()?;
        s.state.name = name.trim().chars().filter(|c| !c.is_control()).take(32).collect();
        s.save().map_err(|e| e.to_string())?;
    }
    let engine = match host.running() {
        Some(e) => e,
        None => host.start(&app).await?,
    };
    Ok(Started {
        snapshot: engine.snapshot(),
        me: self_info(&app, &engine),
    })
}

#[tauri::command]
pub async fn engine_stop(host: State<'_, Host>) -> Result<(), String> {
    host.stop().await;
    Ok(())
}

fn self_info(app: &AppHandle, engine: &Engine) -> SelfInfo {
    let me = engine.me();
    SelfInfo {
        name: me.name,
        save_dir: me.save_dir.to_string_lossy().into_owned(),
        fingerprint: Some(me.fingerprint),
        version: app.package_info().version.to_string(),
    }
}

#[tauri::command]
pub async fn get_self(app: AppHandle, host: State<'_, Host>) -> Result<SelfInfo, String> {
    host.settings(
        |e| self_info(&app, e),
        |s| {
            Ok(SelfInfo {
                name: s.state.name.clone(),
                save_dir: s.state.save_dir.to_string_lossy().into_owned(),
                fingerprint: None,
                version: app.package_info().version.to_string(),
            })
        },
    )
    .await
}

// ---------- Files ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Picked {
    files: Vec<FileItem>,
    /// Folders and things that couldn't be read. Folders aren't sent in this version.
    skipped: usize,
}

/// Turns picked or dropped paths into the app's file items, and remembers which path each id is.
#[tauri::command]
pub async fn files_from_paths(host: State<'_, Host>, paths: Vec<PathBuf>) -> Result<Picked, String> {
    let mut out = Vec::new();
    let mut skipped = 0;
    let mut known = host.files.lock().expect("files lock");
    for p in paths {
        match std::fs::metadata(&p) {
            Ok(m) if m.is_file() => {
                let id = format!("p{}", host.file_seq.fetch_add(1, Ordering::Relaxed));
                let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into());
                out.push(FileItem {
                    id: id.clone(),
                    mime: files::mime_for(&name),
                    name,
                    size: m.len(),
                });
                known.insert(id, p);
            }
            _ => skipped += 1,
        }
    }
    Ok(Picked { files: out, skipped })
}

#[tauri::command]
pub async fn send(host: State<'_, Host>, device_id: String, file_ids: Vec<String>) -> Result<String, String> {
    let paths: Vec<PathBuf> = {
        let known = host.files.lock().expect("files lock");
        file_ids.iter().filter_map(|id| known.get(id).cloned()).collect()
    };
    if paths.len() != file_ids.len() {
        return Err("Some of those files can't be found any more. Add them again.".into());
    }
    host.engine()?.send(&device_id, &paths).map_err(|e| match e {
        nhako_core::EngineError::File { path, .. } => format!("Couldn't read {}. It may have been moved or deleted.", display_name(&path)),
        nhako_core::EngineError::UnknownDevice => "That device isn't known any more.".into(),
        other => other.to_string(),
    })
}

fn display_name(p: &Path) -> String {
    p.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| p.to_string_lossy().into_owned())
}

/// Opens what a received transfer saved: the file itself (or its folder when there are several),
/// or the folder with the file selected when `reveal` is set.
#[tauri::command]
pub async fn open_received(app: AppHandle, host: State<'_, Host>, transfer_id: String, reveal: bool) -> Result<(), String> {
    let saved: Vec<PathBuf> = host.engine()?.saved_files(&transfer_id).into_iter().filter(|p| p.exists()).collect();
    let Some(first) = saved.first() else {
        return Err("The files aren't there any more. They may have been moved or deleted.".into());
    };
    let opener = app.opener();
    let result = if reveal {
        opener.reveal_item_in_dir(first)
    } else if saved.len() == 1 {
        opener.open_path(first.to_string_lossy(), None::<&str>)
    } else {
        let dir = first.parent().unwrap_or(first);
        opener.open_path(dir.to_string_lossy(), None::<&str>)
    };
    result.map_err(|e| {
        warn!("couldn't open {}: {e}", first.display());
        "Couldn't open it. There may be no app set up for this kind of file.".to_string()
    })
}

#[tauri::command]
pub async fn set_save_dir(host: State<'_, Host>, dir: PathBuf) -> Result<(), String> {
    if !dir.is_absolute() {
        return Err("Pick a folder.".into());
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("Can't use that folder: {e}"))?;
    host.settings(
        |e| e.set_save_dir(&dir),
        |s| {
            s.state.save_dir = dir.clone();
            s.save().map_err(|e| e.to_string())
        },
    )
    .await
}

// ---------- Transfers ----------

#[tauri::command]
pub async fn rescan(host: State<'_, Host>) -> Result<(), String> {
    host.engine()?.rescan();
    Ok(())
}

#[tauri::command]
pub async fn accept(host: State<'_, Host>, request_id: String, trust: bool, auto_accept: bool) -> Result<Option<String>, String> {
    Ok(host.engine()?.accept(&request_id, trust, auto_accept))
}

#[tauri::command]
pub async fn decline(host: State<'_, Host>, request_id: String) -> Result<(), String> {
    host.engine()?.decline(&request_id);
    Ok(())
}

#[tauri::command]
pub async fn pause(host: State<'_, Host>, transfer_id: String) -> Result<(), String> {
    host.engine()?.pause(&transfer_id);
    Ok(())
}

#[tauri::command]
pub async fn resume(host: State<'_, Host>, transfer_id: String) -> Result<(), String> {
    host.engine()?.resume(&transfer_id);
    Ok(())
}

#[tauri::command]
pub async fn cancel(host: State<'_, Host>, transfer_id: String) -> Result<(), String> {
    host.engine()?.cancel(&transfer_id);
    Ok(())
}

#[tauri::command]
pub async fn retry(host: State<'_, Host>, transfer_id: String) -> Result<(), String> {
    host.engine()?.retry(&transfer_id);
    Ok(())
}

// ---------- Pairing and devices ----------

#[tauri::command]
pub async fn create_code(host: State<'_, Host>) -> Result<PairingCode, String> {
    Ok(host.engine()?.create_code())
}

#[tauri::command]
pub async fn cancel_code(host: State<'_, Host>) -> Result<(), String> {
    host.engine()?.cancel_code();
    Ok(())
}

#[tauri::command]
pub async fn redeem_code(host: State<'_, Host>, words: Vec<String>) -> Result<RedeemResult, String> {
    let engine = host.engine()?;
    Ok(engine.redeem_code(&words).await)
}

#[tauri::command]
pub async fn set_trusted(host: State<'_, Host>, device_id: String, trusted: bool) -> Result<(), String> {
    host.engine()?.set_trusted(&device_id, trusted);
    Ok(())
}

#[tauri::command]
pub async fn set_auto_accept(host: State<'_, Host>, device_id: String, on: bool) -> Result<(), String> {
    host.engine()?.set_auto_accept(&device_id, on);
    Ok(())
}

#[tauri::command]
pub async fn rename_self(host: State<'_, Host>, name: String) -> Result<(), String> {
    host.settings(
        |e| e.rename_self(&name),
        |s| {
            let name: String = name.trim().chars().filter(|c| !c.is_control()).take(32).collect();
            if name.is_empty() {
                return Ok(());
            }
            s.state.name = name;
            s.save().map_err(|e| e.to_string())
        },
    )
    .await
}

// ---------- Servers ----------

#[tauri::command]
pub async fn get_connection(host: State<'_, Host>) -> Result<Connection, String> {
    host.settings(|e| e.connection(), |s| Ok(s.state.connection.clone())).await
}

/// Saves the server setting and, when the engine is running, restarts it on the new servers.
#[tauri::command]
pub async fn set_connection(app: AppHandle, host: State<'_, Host>, connection: Connection) -> Result<(), String> {
    Network::from_connection(&connection).map_err(|e| e.to_string())?;
    let _g = host.lifecycle.lock().await;
    let Some(engine) = host.running() else {
        let mut s = host.store()?;
        s.state.connection = connection;
        return s.save().map_err(|e| e.to_string());
    };
    if engine.transfers().iter().any(|t| !t.status.is_final()) {
        return Err("Wait for the transfers in progress to finish (or cancel them), then save again.".into());
    }
    engine.set_connection(connection).map_err(|e| e.to_string())?;
    host.stop_locked().await;
    let engine = host.start(&app).await?;
    let _ = app.emit(RESYNC, engine.snapshot());
    Ok(())
}

#[tauri::command]
pub async fn check_server(server: OwnServer) -> ServerCheck {
    check(&server).await
}

#[cfg(test)]
mod tests {
    use nhako_core::CheckResult;

    use super::*;

    #[test]
    fn notification_text_reads_naturally() {
        let r = IncomingRequest {
            id: "r".into(),
            device_id: "d".into(),
            files: vec![FileItem {
                id: "f0".into(),
                name: "holiday.mp4".into(),
                size: 1,
                mime: "video/mp4".into(),
            }],
            total_bytes: 1,
            received_at: 0,
        };
        assert_eq!(incoming_title("Aisyah's phone", &r), "Aisyah's phone wants to send you holiday.mp4");
        assert_eq!(files_phrase(3, Some("a")), "3 files");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn first_run_settings_open_one_at_a_time() {
        // The window asks for the name and the server setting at the same moment on first run.
        // Both open the settings, and the first open creates the device key: there must be one key.
        let dir = std::env::temp_dir().join(format!("nhako-host-{}-{}", std::process::id(), unique_suffix()));
        let host = std::sync::Arc::new(Host::new(dir.clone()));
        let opens = (0..8).map(|_| {
            let host = host.clone();
            tokio::spawn(async move { host.settings(|e| e.id().to_string(), |s| Ok(s.secret.public().to_string())).await })
        });
        let mut keys = std::collections::BTreeSet::new();
        for open in opens {
            keys.insert(open.await.unwrap().expect("settings open"));
        }
        assert_eq!(keys.len(), 1, "one device key, whatever the timing");
        let _ = std::fs::remove_dir_all(dir);
    }

    fn unique_suffix() -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos() as u64
    }

    #[test]
    fn check_result_is_the_core_type() {
        // The window reads these straight from the core, so they must keep their contract shape.
        let v = serde_json::to_value(CheckResult::TokenRejected).unwrap();
        assert_eq!(v, serde_json::json!({ "status": "tokenRejected" }));
    }
}
