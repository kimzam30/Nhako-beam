//! The Nhako Beam engine for the Android app, through UniFFI.
//!
//! The call surface is typed (UniFFI generates the Kotlin), but the data that crosses is the same
//! JSON the desktop app reads (`desktop/src/lib/engine/types.ts`): events, the snapshot, codes,
//! server checks. One contract for every app, and Kotlin parses it with kotlinx.serialization.
//!
//! Calls block (the engine has its own runtime here), so Kotlin calls them off the main thread.
//! Events arrive on an engine thread through [`EventListener`].

use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use nhako_core::{Connection, Engine, EngineConfig, EngineEvent, Network, Outgoing, OwnServer, WhoCanSend, check_server, store};
use serde::Serialize;
use tokio::{runtime::Runtime, sync::broadcast::error::RecvError, task::JoinHandle};

uniffi::setup_scaffolding!();

/// A problem, in plain language, ready to show.
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum NhakoError {
    /// `reason` (not `message`, which Kotlin exceptions already have) is plain language, ready to show.
    #[error("{reason}")]
    Failed { reason: String },
}

fn failed(reason: impl Into<String>) -> NhakoError {
    NhakoError::Failed { reason: reason.into() }
}

/// Gets every engine event as JSON (`{"type":"transfer","transfer":{...}}`), on an engine thread.
#[uniffi::export(foreign)]
pub trait EventListener: Send + Sync {
    fn on_event(&self, json: String);
}

/// A file to send. `path` can be `/proc/self/fd/N` for a file shared from another app, which must
/// stay open until the transfer is finished, or a `content://` address for a file in a folder
/// picked in the system picker, opened through [`FileOpener`] when it's needed. `name` is what the
/// other device sees; `folder` is the folder it's in, for a file inside a folder being sent.
#[derive(Debug, Clone, uniffi::Record)]
pub struct SendFile {
    pub path: String,
    pub name: String,
    pub folder: Option<String>,
}

/// Opens `content://` addresses for the engine, one file at a time as each is sent, so a folder
/// of thousands of files never holds thousands open.
#[uniffi::export(foreign)]
pub trait FileOpener: Send + Sync {
    /// Opens it for reading and hands over the descriptor (the engine closes it), or -1.
    fn open(&self, uri: String) -> i32;
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SelfInfo {
    name: String,
    save_dir: String,
    fingerprint: Option<String>,
    version: String,
}

#[derive(uniffi::Object)]
pub struct NhakoEngine {
    rt: Runtime,
    config_dir: PathBuf,
    default_save_dir: PathBuf,
    listener: Arc<dyn EventListener>,
    engine: Mutex<Option<Engine>>,
    forward: Mutex<Option<JoinHandle<()>>>,
    /// Starting, stopping and restarting happen one at a time, and settings on disk are only
    /// touched under it (the first open creates the device key).
    lifecycle: Mutex<()>,
    /// The app is on screen. Kept here too, so an engine started later starts with it.
    foreground: AtomicBool,
}

#[uniffi::export]
impl NhakoEngine {
    /// `config_dir`: the app's private files folder. `default_save_dir`: where received files go on
    /// a fresh install (Download/Nhako Beam).
    #[uniffi::constructor]
    pub fn new(config_dir: String, default_save_dir: String, listener: Arc<dyn EventListener>) -> Result<Arc<Self>, NhakoError> {
        init_logging(Path::new(&config_dir));
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("nhako")
            .enable_all()
            .build()
            .map_err(|e| failed(format!("Couldn't start: {e}")))?;
        Ok(Arc::new(Self {
            rt,
            config_dir: config_dir.into(),
            default_save_dir: default_save_dir.into(),
            listener,
            engine: Mutex::new(None),
            forward: Mutex::new(None),
            lifecycle: Mutex::new(()),
            foreground: AtomicBool::new(true),
        }))
    }

    /// Starts the engine (once) and returns the snapshot JSON to draw from. `name` is the name
    /// picked during onboarding, saved before the network starts.
    pub fn start(&self, name: Option<String>) -> Result<String, NhakoError> {
        let _g = self.lifecycle.lock().expect("lifecycle lock");
        if let Some(name) = name.map(|n| clean_name(&n)).filter(|n| !n.is_empty())
            && self.running().is_none()
        {
            let mut s = self.store()?;
            s.state.name = name;
            s.save().map_err(|e| failed(e.to_string()))?;
        }
        let engine = match self.running() {
            Some(e) => e,
            None => self.start_locked()?,
        };
        json(&engine.snapshot())
    }

    /// Closes the network properly, so other devices see this one leave right away.
    pub fn stop(&self) {
        let _g = self.lifecycle.lock().expect("lifecycle lock");
        self.stop_locked();
    }

    pub fn snapshot(&self) -> Result<String, NhakoError> {
        json(&self.engine()?.snapshot())
    }

    /// `{name, saveDir, fingerprint, version}`, also before the engine has started.
    pub fn get_self(&self) -> Result<String, NhakoError> {
        let version = env!("CARGO_PKG_VERSION").to_string();
        self.settings(
            |e| {
                let me = e.me();
                SelfInfo {
                    name: me.name,
                    save_dir: me.save_dir.to_string_lossy().into_owned(),
                    fingerprint: Some(me.fingerprint),
                    version: version.clone(),
                }
            },
            |s| {
                Ok(SelfInfo {
                    name: s.state.name.clone(),
                    save_dir: s.state.save_dir.to_string_lossy().into_owned(),
                    fingerprint: None,
                    version: version.clone(),
                })
            },
        )
        .and_then(|info| json(&info))
    }

    /// The app came on screen or went to the background: devices are checked less often there.
    pub fn set_foreground(&self, on: bool) -> Result<(), NhakoError> {
        self.foreground.store(on, Ordering::Relaxed);
        if let Some(e) = self.running() {
            self.in_rt(|| e.set_foreground(on));
        }
        Ok(())
    }

    pub fn rescan(&self) -> Result<(), NhakoError> {
        let e = self.engine()?;
        self.in_rt(|| e.rescan());
        Ok(())
    }

    /// Returns the transfer id.
    pub fn send(&self, device_id: String, files: Vec<SendFile>) -> Result<String, NhakoError> {
        let e = self.engine()?;
        let out: Vec<Outgoing> = files
            .into_iter()
            .map(|f| Outgoing {
                path: PathBuf::from(f.path),
                name: f.name,
                folder: f.folder,
            })
            .collect();
        self.in_rt(|| e.send_files(&device_id, &out)).map_err(send_error)
    }

    /// Sends text (a note, a link, a clipboard). Returns the transfer id.
    pub fn send_text(&self, device_id: String, text: String) -> Result<String, NhakoError> {
        let e = self.engine()?;
        self.in_rt(|| e.send_text(&device_id, &text)).map_err(send_error)
    }

    /// Lets the engine open `content://` addresses (files in a picked folder). Set once.
    pub fn set_file_opener(&self, opener: Arc<dyn FileOpener>) {
        nhako_core::files::set_content_opener(Box::new(move |uri| {
            let fd = opener.open(uri.to_string());
            if fd < 0 {
                return Err(std::io::Error::other("couldn't open it"));
            }
            #[cfg(unix)]
            {
                use std::os::fd::FromRawFd;
                // SAFETY: the app detached this descriptor and handed it over; nothing else closes it.
                Ok(unsafe { std::fs::File::from_raw_fd(fd) })
            }
            #[cfg(not(unix))]
            Err(std::io::Error::other("not on this platform"))
        }));
    }

    /// Asks a nearby device to trust this one, and to be trusted back (see `Engine::ask_trust`).
    pub fn ask_trust(&self, device_id: String) -> Result<(), NhakoError> {
        let e = self.engine()?;
        self.in_rt(|| e.ask_trust(&device_id)).map_err(|err| match err {
            nhako_core::EngineError::NotNearby => failed("That device isn't nearby any more."),
            _ => failed("That device isn't known any more."),
        })
    }

    pub fn cancel_trust(&self, device_id: String) -> Result<(), NhakoError> {
        let e = self.engine()?;
        self.in_rt(|| e.cancel_trust(&device_id));
        Ok(())
    }

    pub fn answer_trust(&self, device_id: String, yes: bool) -> Result<(), NhakoError> {
        let e = self.engine()?;
        self.in_rt(|| e.answer_trust(&device_id, yes));
        Ok(())
    }

    pub fn accept(&self, request_id: String, trust: bool, auto_accept: bool) -> Result<Option<String>, NhakoError> {
        let e = self.engine()?;
        Ok(self.in_rt(|| e.accept(&request_id, trust, auto_accept)))
    }

    pub fn decline(&self, request_id: String) -> Result<(), NhakoError> {
        let e = self.engine()?;
        self.in_rt(|| e.decline(&request_id));
        Ok(())
    }

    pub fn pause(&self, transfer_id: String) -> Result<(), NhakoError> {
        let e = self.engine()?;
        self.in_rt(|| e.pause(&transfer_id));
        Ok(())
    }

    pub fn resume(&self, transfer_id: String) -> Result<(), NhakoError> {
        let e = self.engine()?;
        self.in_rt(|| e.resume(&transfer_id));
        Ok(())
    }

    pub fn cancel(&self, transfer_id: String) -> Result<(), NhakoError> {
        let e = self.engine()?;
        self.in_rt(|| e.cancel(&transfer_id));
        Ok(())
    }

    pub fn retry(&self, transfer_id: String) -> Result<(), NhakoError> {
        let e = self.engine()?;
        self.in_rt(|| e.retry(&transfer_id));
        Ok(())
    }

    /// The new code as JSON (`{words, expiresAt, uri}`).
    pub fn create_code(&self) -> Result<String, NhakoError> {
        let e = self.engine()?;
        json(&self.in_rt(|| e.create_code()))
    }

    pub fn cancel_code(&self) -> Result<(), NhakoError> {
        let e = self.engine()?;
        self.in_rt(|| e.cancel_code());
        Ok(())
    }

    /// Blocks until the other device is found or the code is given up on (about 20 s). Returns the
    /// RedeemResult JSON.
    pub fn redeem_code(&self, words: Vec<String>) -> Result<String, NhakoError> {
        let e = self.engine()?;
        json(&self.rt.block_on(async move { e.redeem_code(&words).await }))
    }

    pub fn set_trusted(&self, device_id: String, trusted: bool) -> Result<(), NhakoError> {
        let e = self.engine()?;
        self.in_rt(|| e.set_trusted(&device_id, trusted));
        Ok(())
    }

    pub fn set_auto_accept(&self, device_id: String, on: bool) -> Result<(), NhakoError> {
        let e = self.engine()?;
        self.in_rt(|| e.set_auto_accept(&device_id, on));
        Ok(())
    }

    pub fn rename_self(&self, name: String) -> Result<(), NhakoError> {
        self.settings(
            |e| e.rename_self(&name),
            |s| {
                let name = clean_name(&name);
                if name.is_empty() {
                    return Ok(());
                }
                s.state.name = name;
                s.save().map_err(|e| failed(e.to_string()))
            },
        )
    }

    pub fn set_save_dir(&self, dir: String) -> Result<(), NhakoError> {
        let dir = PathBuf::from(dir);
        std::fs::create_dir_all(&dir).map_err(|e| failed(format!("Can't use that folder: {e}")))?;
        self.settings(
            |e| e.set_save_dir(&dir),
            |s| {
                s.state.save_dir = dir.clone();
                s.save().map_err(|e| failed(e.to_string()))
            },
        )
    }

    /// The saved Connection as JSON.
    pub fn get_connection(&self) -> Result<String, NhakoError> {
        self.settings(|e| e.connection(), |s| Ok(s.state.connection.clone())).and_then(|c| json(&c))
    }

    /// Saves the Connection (JSON) and, when the engine is running, restarts it on the new
    /// servers. The new snapshot arrives as a `snapshot` event.
    pub fn set_connection(&self, connection_json: String) -> Result<(), NhakoError> {
        let connection: Connection = serde_json::from_str(&connection_json).map_err(|e| failed(format!("Not a server setting: {e}")))?;
        Network::from_connection(&connection).map_err(|e| failed(e.to_string()))?;
        self.save_and_restart(
            |e| e.set_connection(connection.clone()).map_err(|e| failed(e.to_string())),
            |s| s.state.connection = connection.clone(),
        )
    }

    /// Checks your own server before saving it. Takes an OwnServer as JSON (`{relay, token?,
    /// lookup?}`) and returns the ServerCheck JSON. Never fails: problems are in the result.
    pub fn check_server(&self, server_json: String) -> Result<String, NhakoError> {
        let own: OwnServer = serde_json::from_str(&server_json).map_err(|e| failed(format!("Not a server: {e}")))?;
        json(&self.rt.block_on(check_server(&own)))
    }

    pub fn get_lan_discovery(&self) -> Result<bool, NhakoError> {
        self.settings(|e| e.lan_discovery(), |s| Ok(s.state.lan_discovery))
    }

    /// Turns looking for nearby devices on or off. Like the server setting, a running engine
    /// restarts (refused while transfers are running) and the new snapshot arrives as an event.
    pub fn set_lan_discovery(&self, on: bool) -> Result<(), NhakoError> {
        self.save_and_restart(|e| e.set_lan_discovery(on).map_err(|e| failed(e.to_string())), |s| s.state.lan_discovery = on)
    }

    /// Who can send to me: `yourDevices` or `everyoneNearby`. Applies to the next request.
    pub fn get_who_can_send(&self) -> Result<String, NhakoError> {
        let who = self.settings(|e| e.who_can_send(), |s| Ok(s.state.who_can_send))?;
        Ok(serde_json::to_value(who).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default())
    }

    pub fn set_who_can_send(&self, who: String) -> Result<(), NhakoError> {
        let who: WhoCanSend = serde_json::from_value(serde_json::Value::String(who)).map_err(|e| failed(format!("Not a setting: {e}")))?;
        self.settings(
            |e| e.set_who_can_send(who).map_err(|e| failed(e.to_string())),
            |s| {
                s.state.who_can_send = who;
                Ok(s.save().map_err(|e| failed(e.to_string())))
            },
        )?
    }

    /// This phone's wake token from Firebase, or none when push isn't set up. Saved also before the
    /// engine starts; trusted devices get it in the next hello.
    pub fn set_wake_token(&self, token: Option<String>) -> Result<(), NhakoError> {
        self.settings(
            |e| e.set_wake_token(token.clone()),
            |s| {
                s.state.wake_token = token.clone().filter(|t| nhako_core::wake::valid_token(t));
                s.save().map_err(|e| failed(e.to_string()))
            },
        )
    }

    /// Whether other devices could wake this one (a wake service was built in). The app shows it in
    /// Settings > About.
    pub fn wake_available(&self) -> bool {
        nhako_core::wake::default_url().is_some()
    }

    pub fn clear_history(&self) -> Result<(), NhakoError> {
        self.engine()?.clear_history();
        Ok(())
    }

    /// Settings > About > Copy diagnostics. `app` is the app's own line, for example
    /// "android 0.1.0 (Android 15)". Works while the engine is stopped too.
    pub fn diagnostics(&self, app: String) -> String {
        match self.running() {
            Some(e) => e.diagnostics(&app),
            None => nhako_core::diagnostics_without_engine(&app),
        }
    }

    /// Where a received transfer's files are on disk (only the ones that arrived and checked out).
    pub fn saved_files(&self, transfer_id: String) -> Result<Vec<String>, NhakoError> {
        Ok(self
            .engine()?
            .saved_files(&transfer_id)
            .into_iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect())
    }
}

impl NhakoEngine {
    fn running(&self) -> Option<Engine> {
        self.engine.lock().expect("engine lock").clone()
    }

    fn engine(&self) -> Result<Engine, NhakoError> {
        self.running().ok_or_else(|| failed("Nhako Beam is still starting. Try again in a moment."))
    }

    /// Engine methods that start background work must run inside the runtime.
    fn in_rt<T>(&self, f: impl FnOnce() -> T) -> T {
        let _enter = self.rt.enter();
        f()
    }

    /// Only with `lifecycle` held.
    fn store(&self) -> Result<store::Store, NhakoError> {
        store::Store::open(&self.config_dir, self.default_save_dir.clone()).map_err(|e| failed(format!("Couldn't read the settings: {e}")))
    }

    fn settings<T>(&self, running: impl FnOnce(&Engine) -> T, stored: impl FnOnce(&mut store::Store) -> Result<T, NhakoError>) -> Result<T, NhakoError> {
        let _g = self.lifecycle.lock().expect("lifecycle lock");
        match self.running() {
            Some(e) => Ok(running(&e)),
            None => stored(&mut self.store()?),
        }
    }

    /// Saves a setting that only takes effect when the engine starts, then restarts it (refused
    /// while transfers are running). Without a running engine, it's written to disk for next time.
    fn save_and_restart(&self, running: impl FnOnce(&Engine) -> Result<(), NhakoError>, stored: impl FnOnce(&mut store::Store)) -> Result<(), NhakoError> {
        let _g = self.lifecycle.lock().expect("lifecycle lock");
        let Some(engine) = self.running() else {
            let mut s = self.store()?;
            stored(&mut s);
            return s.save().map_err(|e| failed(e.to_string()));
        };
        if engine.transfers().iter().any(|t| !t.status.is_final()) {
            return Err(failed("Wait for the transfers in progress to finish (or cancel them), then save again."));
        }
        running(&engine)?;
        self.stop_locked();
        let engine = self.start_locked()?;
        let snapshot = serde_json::json!({ "type": "snapshot", "snapshot": engine.snapshot() });
        self.listener.on_event(snapshot.to_string());
        Ok(())
    }

    /// Only with `lifecycle` held.
    fn start_locked(&self) -> Result<Engine, NhakoError> {
        let mut cfg = EngineConfig::new(&self.config_dir);
        cfg.default_save_dir = Some(self.default_save_dir.clone());
        let engine = self.rt.block_on(Engine::start(cfg)).map_err(|e| failed(e.to_string()))?;
        self.in_rt(|| engine.set_foreground(self.foreground.load(Ordering::Relaxed)));
        let listener = self.listener.clone();
        let forwarded = engine.clone();
        let task = self.rt.spawn(async move {
            let mut events = forwarded.subscribe();
            loop {
                match events.recv().await {
                    Ok(event) => {
                        if let Ok(text) = serde_json::to_string::<EngineEvent>(&event) {
                            listener.on_event(text);
                        }
                    }
                    Err(RecvError::Lagged(_)) => {
                        let snapshot = serde_json::json!({ "type": "snapshot", "snapshot": forwarded.snapshot() });
                        listener.on_event(snapshot.to_string());
                    }
                    Err(RecvError::Closed) => break,
                }
            }
        });
        *self.forward.lock().expect("forward lock") = Some(task);
        *self.engine.lock().expect("engine lock") = Some(engine.clone());
        Ok(engine)
    }

    /// Only with `lifecycle` held.
    fn stop_locked(&self) {
        if let Some(task) = self.forward.lock().expect("forward lock").take() {
            task.abort();
        }
        let engine = self.engine.lock().expect("engine lock").take();
        if let Some(e) = engine {
            self.rt.block_on(async {
                let _ = tokio::time::timeout(Duration::from_secs(4), e.stop()).await;
            });
        }
    }
}

fn send_error(err: nhako_core::EngineError) -> NhakoError {
    match err {
        nhako_core::EngineError::File { .. } => failed("Couldn't read that file. It may have been moved or deleted."),
        nhako_core::EngineError::UnknownDevice => failed("That device isn't known any more."),
        nhako_core::EngineError::TextTooLong => failed("That's too long to send as text. Save it in a file and send that."),
        nhako_core::EngineError::NoFiles => failed("There's nothing to send."),
        other => failed(other.to_string()),
    }
}

fn clean_name(name: &str) -> String {
    nhako_core::text::device_name(name)
}

fn json<T: Serialize>(value: &T) -> Result<String, NhakoError> {
    serde_json::to_string(value).map_err(|e| failed(e.to_string()))
}

/// Logs to logcat and to the log file in the app's files folder (for Copy diagnostics).
fn init_logging(config_dir: &Path) {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        use tracing_subscriber::{EnvFilter, fmt, prelude::*};
        let filter = EnvFilter::new("info,iroh=warn,iroh_relay=warn,netwatch=warn,portmapper=warn");
        let file = nhako_core::logfile::install(config_dir);
        let to_file = fmt::layer().with_ansi(false).with_writer(move || file.writer());
        #[cfg(target_os = "android")]
        let _ = tracing_subscriber::registry()
            .with(filter)
            .with(tracing_android::layer("nhako").expect("logcat layer"))
            .with(to_file)
            .try_init();
        #[cfg(not(target_os = "android"))]
        let _ = tracing_subscriber::registry().with(filter).with(fmt::layer()).with(to_file).try_init();
    });
}
