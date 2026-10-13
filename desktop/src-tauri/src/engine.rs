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
    Connection, Engine, EngineConfig, EngineEvent, FileItem, IncomingRequest, Network, Outgoing, OwnServer, PairingCode, RedeemResult, ServerCheck, Snapshot,
    Transfer, TransferDirection, TransferStatus, WhoCanSend, check_server as check, engine_limits, files,
    store::{self, Store},
    text,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, async_runtime::JoinHandle};
use tauri_plugin_clipboard_manager::ClipboardExt;
use tauri_plugin_dialog::DialogExt;
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
    /// Files the window picked or dropped: the app only sees ids, the engine needs paths (and,
    /// for a file inside a folder being sent, the folder it's in).
    files: Mutex<HashMap<String, (PathBuf, Option<String>)>>,
    file_seq: AtomicU64,
    /// Paths you handed the app yourself (the file dialog, a drop, "Send with Nhako Beam") and a
    /// save folder you chose, not yet used. The window can only send or save to these, so a
    /// script that somehow got into the page couldn't send any file it names.
    offered: Mutex<HashSet<PathBuf>>,
    /// Set by the real-app test (`NHAKO_TEST_ANY_PATH=1`), which can't drag real files in.
    any_path: bool,
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
            offered: Mutex::new(HashSet::new()),
            any_path: std::env::var_os("NHAKO_TEST_ANY_PATH").is_some_and(|v| v == "1"),
        }
    }

    /// Remembers paths you handed the app yourself (see `offered`).
    pub fn offer(&self, paths: impl IntoIterator<Item = PathBuf>) {
        let mut offered = self.offered.lock().expect("offered lock");
        // Drops that never get staged shouldn't pile up for ever.
        if offered.len() > 10_000 {
            offered.clear();
        }
        offered.extend(paths);
    }

    /// Takes `path` out of the offered set. A drop reaches the window through an event, so its
    /// paths may still be on their way here: wait a moment for them.
    async fn take_offered(&self, path: &Path) -> bool {
        if self.any_path {
            return true;
        }
        for _ in 0..20 {
            if self.offered.lock().expect("offered lock").remove(path) {
                return true;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        warn!("refused a path the window wasn't given: {}", path.display());
        false
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

    /// Saves a setting that only takes effect when the engine starts, then restarts it (refused
    /// while transfers are running). Without a running engine, it's written to disk for next time.
    async fn save_and_restart(
        &self,
        app: &AppHandle,
        running: impl FnOnce(&Engine) -> Result<(), String>,
        stored: impl FnOnce(&mut Store),
    ) -> Result<(), String> {
        let _g = self.lifecycle.lock().await;
        let Some(engine) = self.running() else {
            let mut s = self.store()?;
            stored(&mut s);
            return s.save().map_err(|e| e.to_string());
        };
        if engine.transfers().iter().any(|t| !t.status.is_final()) {
            return Err("Wait for the transfers in progress to finish (or cancel them), then save again.".into());
        }
        running(&engine)?;
        self.stop_locked().await;
        let engine = self.start(app).await?;
        let _ = app.emit(RESYNC, engine.snapshot());
        Ok(())
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
/// Also tells the engine whether a window is on screen, so it checks on devices less often while
/// the app sits in the tray.
async fn forward(app: AppHandle, engine: Engine) {
    let mut events = engine.subscribe();
    let mut notified = HashSet::new();
    let mut look = tokio::time::interval(Duration::from_secs(5));
    let mut on_screen = None;
    loop {
        tokio::select! {
            event = events.recv() => match event {
                Ok(event) => {
                    notify(&app, &engine, &event, &mut notified);
                    let _ = app.emit(EVENT, &event);
                }
                Err(RecvError::Lagged(_)) => {
                    let _ = app.emit(RESYNC, engine.snapshot());
                }
                Err(RecvError::Closed) => break,
            },
            _ = look.tick() => {
                let now = shell::on_screen(&app);
                if on_screen != Some(now) {
                    on_screen = Some(now);
                    engine.set_foreground(now);
                }
            }
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
        EngineEvent::Incoming { request } => {
            let id = request.id.clone();
            let body = match &request.text {
                Some(t) => preview(t),
                None => "Accept to save it in your Nhako Beam folder.".into(),
            };
            shell::notify_with_actions(
                app,
                &incoming_title(&name(&request.device_id), request),
                &body,
                &[("accept", "Accept"), ("decline", "Decline")],
                move |app, action| answer_from_notification(app, &id, action),
            );
        }
        // Text from your own devices with "Accept files automatically" on goes straight to the
        // clipboard; the window shows the same toast when it's open. Text you accepted on a card
        // is copied from there.
        EngineEvent::Transfer { transfer }
            if transfer.direction == TransferDirection::Receive
                && transfer.status == TransferStatus::Done
                && transfer.text.is_some()
                && notified.insert(transfer.id.clone()) =>
        {
            let auto = engine.devices().iter().any(|d| d.id == transfer.device_id && d.auto_accept);
            if !auto {
                return;
            }
            let body = transfer.text.clone().unwrap_or_default();
            if let Err(e) = app.clipboard().write_text(body.clone()) {
                warn!("couldn't copy received text: {e}");
                return;
            }
            let link = text::is_link(&body).then(|| body.trim().to_string());
            let actions: &[(&str, &str)] = if link.is_some() { &[("open", "Open link")] } else { &[] };
            shell::notify_with_actions(
                app,
                &format!("Copied {} from {}", text_phrase(transfer), name(&transfer.device_id)),
                &preview(&body),
                actions,
                move |app, action| match (action, &link) {
                    ("open", Some(url)) => {
                        let _ = app.opener().open_url(url, None::<&str>);
                    }
                    ("default", _) => shell::show_window(app),
                    _ => {}
                },
            );
        }
        EngineEvent::TrustAsk { request } => {
            shell::notify_with_actions(
                app,
                &format!("{} wants to add this computer to its devices", name(&request.device_id)),
                "Open Nhako Beam to check that both screens show the same code.",
                &[],
                |app, action| {
                    if action == "default" {
                        shell::show_window(app);
                    }
                },
            );
        }
        // Once per transfer: a finished transfer can be reported more than once.
        EngineEvent::Transfer { transfer }
            if transfer.direction == TransferDirection::Receive && transfer.status == TransferStatus::Done && notified.insert(transfer.id.clone()) =>
        {
            let what = match top_folder(&transfer.files) {
                Some(folder) => format!("the folder {folder}"),
                None => files_phrase(transfer.files.len(), transfer.files.first().map(|f| f.name.as_str())),
            };
            let id = transfer.id.clone();
            shell::notify_with_actions(
                app,
                &format!("Got {what} from {}", name(&transfer.device_id)),
                "Saved to your Nhako Beam folder.",
                &[("open", "Open"), ("reveal", "Show in folder")],
                move |app, action| match action {
                    "open" | "reveal" => {
                        if let Some(engine) = app.state::<Host>().running()
                            && let Err(e) = open_saved(app, &engine, &id, action == "reveal")
                        {
                            warn!("{e}");
                            shell::show_window(app);
                        }
                    }
                    "default" => shell::show_window(app),
                    _ => {}
                },
            );
        }
        _ => {}
    }
}

/// Accept or Decline on a request's notification. A request that was already answered, withdrawn
/// or timed out opens the window instead, so nothing happens silently.
fn answer_from_notification(app: &AppHandle, request_id: &str, action: &str) {
    let engine = app.state::<Host>().running();
    let pending = engine.as_ref().is_some_and(|e| e.requests().iter().any(|r| r.id == request_id));
    match (action, engine) {
        ("accept", Some(e)) if pending => {
            e.accept(request_id, false, false);
        }
        ("decline", Some(e)) if pending => e.decline(request_id),
        ("__closed", _) => {}
        _ => shell::show_window(app),
    }
}

fn incoming_title(device: &str, r: &IncomingRequest) -> String {
    let what = match &r.text {
        Some(t) if text::is_link(t) => "a link".to_string(),
        Some(_) => "some text".to_string(),
        None => match top_folder(&r.files) {
            Some(folder) => format!("the folder {folder}"),
            None => files_phrase(r.files.len(), r.files.first().map(|f| f.name.as_str())),
        },
    };
    format!("{device} wants to send you {what}")
}

fn text_phrase(t: &Transfer) -> &'static str {
    if t.text.as_deref().is_some_and(text::is_link) { "a link" } else { "text" }
}

/// The first line or so of a text, for a notification.
fn preview(t: &str) -> String {
    let line = t.trim().lines().next().unwrap_or_default();
    let short: String = line.chars().take(80).collect();
    if short.len() < t.trim().len() { format!("{short}…") } else { short }
}

/// The one folder every file is in, when a single folder was sent.
fn top_folder(files: &[FileItem]) -> Option<&str> {
    let first = files.first()?.folder.as_deref()?.split('/').next()?;
    files
        .iter()
        .all(|f| f.folder.as_deref().and_then(|d| d.split('/').next()) == Some(first))
        .then_some(first)
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
    if let Some(name) = name.map(|n| nhako_core::text::device_name(&n)).filter(|n| !n.is_empty())
        && host.running().is_none()
    {
        let mut s = host.store()?;
        s.state.name = name;
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
    /// Things that couldn't be read, and empty folders.
    skipped: usize,
}

/// Turns dropped paths, or files from "Send with Nhako Beam", into the app's file items. Only
/// paths you handed the app yourself are taken (see `Host::offered`); others count as skipped.
#[tauri::command]
pub async fn files_from_paths(host: State<'_, Host>, paths: Vec<PathBuf>) -> Result<Picked, String> {
    let mut allowed = Vec::new();
    let mut refused = 0;
    for p in paths {
        if host.take_offered(&p).await {
            allowed.push(p);
        } else {
            refused += 1;
        }
    }
    let mut picked = stage(&host, allowed);
    picked.skipped += refused;
    Ok(picked)
}

/// The native "choose files" dialog, over the window that asked. `None` when cancelled.
#[tauri::command]
pub async fn pick_files(window: tauri::Window, host: State<'_, Host>) -> Result<Option<Picked>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title("Choose files to send")
        .pick_files(move |p| {
            let _ = tx.send(p);
        });
    let paths: Vec<PathBuf> = rx
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|p| p.into_path().ok())
        .collect();
    Ok((!paths.is_empty()).then(|| stage(&host, paths)))
}

/// The native "choose folder" dialog for folders to send, over the window that asked. `None`
/// when cancelled.
#[tauri::command]
pub async fn pick_folders(window: tauri::Window, host: State<'_, Host>) -> Result<Option<Picked>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    window
        .dialog()
        .file()
        .set_parent(&window)
        .set_title("Choose folders to send")
        .pick_folders(move |p| {
            let _ = tx.send(p);
        });
    let paths: Vec<PathBuf> = rx
        .await
        .ok()
        .flatten()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|p| p.into_path().ok())
        .collect();
    Ok((!paths.is_empty()).then(|| stage(&host, paths)))
}

/// The native "choose folder" dialog for where received files go, starting at `current`.
/// The chosen folder can then be saved with `set_save_dir`. `None` when cancelled.
#[tauri::command]
pub async fn pick_save_dir(window: tauri::Window, host: State<'_, Host>, current: Option<PathBuf>) -> Result<Option<PathBuf>, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let mut dialog = window.dialog().file().set_parent(&window).set_title("Where should received files go?");
    if let Some(dir) = current.filter(|d| d.is_dir()) {
        dialog = dialog.set_directory(dir);
    }
    dialog.pick_folder(move |p| {
        let _ = tx.send(p);
    });
    let dir = rx.await.ok().flatten().and_then(|p| p.into_path().ok());
    if let Some(dir) = &dir {
        host.offer([dir.clone()]);
    }
    Ok(dir)
}

fn stage(host: &Host, paths: Vec<PathBuf>) -> Picked {
    let mut out = Vec::new();
    let mut skipped = 0;
    let mut known = host.files.lock().expect("files lock");
    let mut add = |out: &mut Vec<FileItem>, p: PathBuf, size: u64, folder: Option<String>| {
        let id = format!("p{}", host.file_seq.fetch_add(1, Ordering::Relaxed));
        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into());
        out.push(FileItem {
            id: id.clone(),
            mime: files::mime_for(&name),
            name,
            size,
            folder: folder.clone(),
        });
        known.insert(id, (p, folder));
    };
    for p in paths {
        match std::fs::metadata(&p) {
            Ok(m) if m.is_file() => add(&mut out, p, m.len(), None),
            // A folder goes with everything inside it, keeping its shape.
            Ok(m) if m.is_dir() => match files::walk_folder(&p, engine_limits::FILES_MAX) {
                Ok(inside) if !inside.is_empty() => {
                    for (file, folder) in inside {
                        match std::fs::metadata(&file) {
                            Ok(m) => add(&mut out, file, m.len(), Some(folder)),
                            Err(_) => skipped += 1,
                        }
                    }
                }
                _ => skipped += 1,
            },
            _ => skipped += 1,
        }
    }
    Picked { files: out, skipped }
}

#[tauri::command]
pub async fn send(host: State<'_, Host>, device_id: String, file_ids: Vec<String>) -> Result<String, String> {
    let out: Vec<Outgoing> = {
        let known = host.files.lock().expect("files lock");
        file_ids
            .iter()
            .filter_map(|id| known.get(id).cloned())
            .map(|(path, folder)| Outgoing {
                name: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into()),
                path,
                folder,
            })
            .collect()
    };
    if out.len() != file_ids.len() {
        return Err("Some of those files can't be found any more. Add them again.".into());
    }
    host.engine()?.send_files(&device_id, &out).map_err(|e| match e {
        nhako_core::EngineError::File { path, .. } => format!("Couldn't read {}. It may have been moved or deleted.", display_name(&path)),
        nhako_core::EngineError::UnknownDevice => "That device isn't known any more.".into(),
        other => other.to_string(),
    })
}

/// Sends text: a note, a link, a clipboard. Returns the transfer id.
#[tauri::command]
pub async fn send_text(host: State<'_, Host>, device_id: String, text: String) -> Result<String, String> {
    host.engine()?.send_text(&device_id, &text).map_err(|e| match e {
        nhako_core::EngineError::TextTooLong => "That's too long to send as text. Save it in a file and send that.".into(),
        nhako_core::EngineError::NoFiles => "There's nothing to send.".into(),
        nhako_core::EngineError::UnknownDevice => "That device isn't known any more.".into(),
        other => other.to_string(),
    })
}

/// What's on the clipboard as text, for "Send clipboard". Empty when there's none.
#[tauri::command]
pub async fn read_clipboard(app: AppHandle) -> Result<String, String> {
    Ok(app.clipboard().read_text().unwrap_or_default())
}

/// Puts text on the clipboard ("Copy" on text that arrived). Works with the window in the background too.
#[tauri::command]
pub async fn write_clipboard(app: AppHandle, text: String) -> Result<(), String> {
    app.clipboard().write_text(text).map_err(|e| {
        warn!("couldn't copy: {e}");
        "Couldn't copy it.".to_string()
    })
}

#[tauri::command]
pub async fn ask_trust(host: State<'_, Host>, device_id: String) -> Result<(), String> {
    host.engine()?.ask_trust(&device_id).map_err(|e| match e {
        nhako_core::EngineError::NotNearby => "That device isn't nearby any more.".into(),
        _ => "That device isn't known any more.".into(),
    })
}

#[tauri::command]
pub async fn cancel_trust(host: State<'_, Host>, device_id: String) -> Result<(), String> {
    host.engine()?.cancel_trust(&device_id);
    Ok(())
}

#[tauri::command]
pub async fn answer_trust(host: State<'_, Host>, device_id: String, yes: bool) -> Result<(), String> {
    host.engine()?.answer_trust(&device_id, yes);
    Ok(())
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
    open_saved(&app, &host.engine()?, &transfer_id, reveal)
}

fn open_saved(app: &AppHandle, engine: &Engine, transfer_id: &str, reveal: bool) -> Result<(), String> {
    let saved: Vec<PathBuf> = engine.saved_files(transfer_id).into_iter().filter(|p| p.exists()).collect();
    let Some(first) = saved.first() else {
        return Err("The files aren't there any more. They may have been moved or deleted.".into());
    };
    // A folder that arrived: open it, or show it in the save folder.
    let save_dir = engine.me().save_dir;
    let shared = shared_folder(&saved, &save_dir);
    let opener = app.opener();
    let result = match (&shared, reveal) {
        (Some(folder), true) => opener.reveal_item_in_dir(folder),
        (Some(folder), false) => opener.open_path(folder.to_string_lossy(), None::<&str>),
        (None, true) => opener.reveal_item_in_dir(first),
        (None, false) if saved.len() == 1 => opener.open_path(first.to_string_lossy(), None::<&str>),
        (None, false) => {
            let dir = first.parent().unwrap_or(first);
            opener.open_path(dir.to_string_lossy(), None::<&str>)
        }
    };
    result.map_err(|e| {
        warn!("couldn't open {}: {e}", first.display());
        "Couldn't open it. There may be no app set up for this kind of file.".to_string()
    })
}

/// The folder inside `save_dir` that holds every saved file, when they arrived as one folder.
fn shared_folder(saved: &[PathBuf], save_dir: &Path) -> Option<PathBuf> {
    let top = |p: &PathBuf| -> Option<PathBuf> {
        let rest = p.strip_prefix(save_dir).ok()?;
        let mut parts = rest.components();
        let first = parts.next()?;
        // Only a file inside a folder there, not one saved straight in it.
        parts.next()?;
        Some(save_dir.join(first))
    };
    let first = top(saved.first()?)?;
    saved.iter().all(|p| top(p).as_ref() == Some(&first)).then_some(first)
}

#[tauri::command]
pub async fn set_save_dir(host: State<'_, Host>, dir: PathBuf) -> Result<(), String> {
    if !dir.is_absolute() || !host.take_offered(&dir).await {
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
            let name: String = nhako_core::text::device_name(&name);
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
    host.save_and_restart(
        &app,
        |e| e.set_connection(connection.clone()).map_err(|e| e.to_string()),
        |s| s.state.connection = connection.clone(),
    )
    .await
}

#[tauri::command]
pub async fn get_lan_discovery(host: State<'_, Host>) -> Result<bool, String> {
    host.settings(|e| e.lan_discovery(), |s| Ok(s.state.lan_discovery)).await
}

/// Turns looking for nearby devices on or off. Like the server setting, the engine restarts.
#[tauri::command]
pub async fn set_lan_discovery(app: AppHandle, host: State<'_, Host>, on: bool) -> Result<(), String> {
    host.save_and_restart(&app, |e| e.set_lan_discovery(on).map_err(|e| e.to_string()), |s| s.state.lan_discovery = on)
        .await
}

/// Who can send to me. Applies to the next request, so no restart.
#[tauri::command]
pub async fn get_who_can_send(host: State<'_, Host>) -> Result<WhoCanSend, String> {
    host.settings(|e| e.who_can_send(), |s| Ok(s.state.who_can_send)).await
}

#[tauri::command]
pub async fn set_who_can_send(host: State<'_, Host>, who: WhoCanSend) -> Result<(), String> {
    host.settings(
        |e| e.set_who_can_send(who).map_err(|e| e.to_string()),
        |s| {
            s.state.who_can_send = who;
            Ok(s.save().map_err(|e| e.to_string()))
        },
    )
    .await?
}

#[tauri::command]
pub async fn clear_history(host: State<'_, Host>) -> Result<(), String> {
    host.engine()?.clear_history();
    Ok(())
}

/// Settings > About > Copy diagnostics: the network and nearby discovery report, with recent logs.
#[tauri::command]
pub async fn diagnostics(app: AppHandle, host: State<'_, Host>) -> Result<String, String> {
    let line = format!("desktop {}", app.package_info().version);
    Ok(match host.running() {
        Some(e) => e.diagnostics(&line),
        None => nhako_core::diagnostics_without_engine(&line),
    })
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
                folder: None,
            }],
            total_bytes: 1,
            received_at: 0,
            text: None,
        };
        assert_eq!(incoming_title("Aisyah's phone", &r), "Aisyah's phone wants to send you holiday.mp4");
        assert_eq!(files_phrase(3, Some("a")), "3 files");
        let mut folder = r.clone();
        folder.files[0].folder = Some("Trip/day 2".into());
        assert_eq!(incoming_title("Aisyah's phone", &folder), "Aisyah's phone wants to send you the folder Trip");
        let mut link = r.clone();
        link.text = Some("https://example.com".into());
        assert_eq!(incoming_title("Aisyah's phone", &link), "Aisyah's phone wants to send you a link");
        assert_eq!(preview("first line\nsecond"), "first line…");
        assert_eq!(preview("  short  "), "short");
    }

    #[test]
    fn a_folder_that_arrived_opens_as_one() {
        let save = PathBuf::from("/home/k/Downloads/Nhako Beam");
        let inside = vec![save.join("Trip/a.jpg"), save.join("Trip/day 2/b.jpg")];
        assert_eq!(shared_folder(&inside, &save), Some(save.join("Trip")));
        assert_eq!(shared_folder(&[save.join("a.jpg")], &save), None, "a file on its own");
        assert_eq!(shared_folder(&[save.join("Trip/a.jpg"), save.join("b.jpg")], &save), None);
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
