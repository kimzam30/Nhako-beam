//! The engine: one iroh endpoint, the device list, transfers and requests, and the public API
//! that mirrors the app's `Engine` contract (`desktop/src/lib/engine/types.ts`).

use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    path::{Path, PathBuf},
    str::FromStr,
    sync::{Arc, Mutex, MutexGuard},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use iroh::{
    Endpoint, EndpointId,
    address_lookup::MemoryLookup,
    endpoint::{Connection, RecvStream, SendStream},
};
use n0_future::StreamExt;
use tokio::{
    sync::{broadcast, mpsc, oneshot},
    task::JoinHandle,
};
use tracing::{debug, info, warn};

use crate::{
    codekeys::{KdfCost, fingerprint},
    files,
    net::{self, Network},
    pair::HostedCode,
    server::SettingError,
    session::{self, End},
    store::{self, Store, TrustedDevice},
    types::{self, *},
    wire::{self, Answer, ClientHello, Ctrl, DeclineReason, DeviceInfo, FileMeta, FileState, Intent, Offer, ServerHello, TransferId},
};

/// How the engine is set up. `EngineConfig::new` gives the real defaults.
#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub config_dir: PathBuf,
    /// Overrides the saved "where files go" setting for this run.
    pub save_dir: Option<PathBuf>,
    /// Where files go on a fresh install, when the platform knows better than the default
    /// (Android: Download/Nhako Beam). A folder picked later is kept.
    pub default_save_dir: Option<PathBuf>,
    /// Overrides the saved connection setting for this run. `None` uses the saved one.
    pub network: Option<Network>,
    /// Never use direct paths, only the relay. For testing a relay.
    pub relay_only: bool,
    pub mdns: bool,
    pub kdf: KdfCost,
    /// How long to keep trying after a connection is lost.
    pub reconnect_for: Duration,
    /// How long entering a code keeps looking for the device showing it.
    pub code_lookup_for: Duration,
    /// Tests: resolve addresses from a shared table instead of the network.
    pub memory_lookup: Option<MemoryLookup>,
    /// Tests: cap the sender at this many bytes per second, so a pause can land mid-file.
    #[doc(hidden)]
    pub test_rate_limit: Option<u64>,
}

impl EngineConfig {
    pub fn new(config_dir: impl Into<PathBuf>) -> Self {
        Self {
            config_dir: config_dir.into(),
            save_dir: None,
            default_save_dir: None,
            network: None,
            relay_only: false,
            mdns: true,
            kdf: KdfCost::REAL,
            reconnect_for: Duration::from_secs(60),
            code_lookup_for: Duration::from_secs(20),
            memory_lookup: None,
            test_rate_limit: None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error(transparent)]
    Store(#[from] store::StoreError),
    #[error("couldn't start networking: {0}")]
    Bind(String),
    #[error("the server setting can't be used: {0}")]
    Setting(#[from] SettingError),
    #[error("unknown device")]
    UnknownDevice,
    #[error("nothing to send")]
    NoFiles,
    #[error("can't read {path}: {reason}")]
    File { path: PathBuf, reason: String },
}

pub(crate) type FrameRx = mpsc::Receiver<Result<Vec<u8>, wire::WireError>>;
pub(crate) type CmdTx = mpsc::UnboundedSender<Cmd>;
pub(crate) type CmdRx = mpsc::UnboundedReceiver<Cmd>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Cmd {
    Pause,
    Resume,
    Cancel,
    /// Tests: stop with a generic error, as if something went wrong on this side.
    Fail,
}

#[derive(Debug, Clone)]
pub(crate) struct SrcFile {
    pub path: PathBuf,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

#[derive(Debug, Clone)]
pub(crate) struct DstFile {
    pub final_path: PathBuf,
    pub part: PathBuf,
    pub size: u64,
    pub verified: bool,
}

#[derive(Debug, Clone)]
pub(crate) enum TxSide {
    Send(Vec<SrcFile>),
    Recv(Vec<DstFile>),
}

pub(crate) struct Tx {
    pub t: Transfer,
    pub peer: EndpointId,
    pub side: TxSide,
    /// Control channel of the live session (or of the waiting phase).
    pub cmd: Option<CmdTx>,
    /// The live session's connection.
    pub conn: Option<Connection>,
    /// Bumped every time a new session or reconnect loop takes over, so stale ones can tell.
    pub epoch: u64,
    last_emit: Option<Instant>,
    samples: VecDeque<(Instant, u64)>,
}

impl Tx {
    /// Records confirmed progress and updates the 3 second speed average.
    pub fn set_have(&mut self, have: &[u64]) {
        let mut done = 0;
        for (i, f) in self.t.files.iter().enumerate() {
            let b = have.get(i).copied().unwrap_or(0).min(f.size);
            self.t.file_bytes.insert(f.id.clone(), b);
            done += b;
        }
        let now = Instant::now();
        if done > self.t.done_bytes && self.t.moving_since.is_none() {
            self.t.moving_since = Some(now_ms());
        }
        self.t.done_bytes = done;
        self.samples.push_back((now, done));
        while self.samples.len() > 2 && now.duration_since(self.samples[0].0) > Duration::from_secs(3) {
            self.samples.pop_front();
        }
        self.t.speed = match (self.samples.front(), self.samples.back()) {
            (Some(a), Some(b)) if b.0 > a.0 && b.1 >= a.1 => (b.1 - a.1) as f64 / b.0.duration_since(a.0).as_secs_f64(),
            _ => 0.0,
        };
    }

    pub fn stop_moving(&mut self) {
        self.t.speed = 0.0;
        self.t.moving_since = None;
        self.samples.clear();
    }
}

#[derive(Debug, Default, Clone)]
pub(crate) struct Dev {
    pub info: Option<DeviceInfo>,
    /// Seen through mDNS or reached over a same-network address.
    pub nearby: bool,
    /// Paired with a code during this run.
    pub paired: bool,
    pub reachable: bool,
    pub route: Option<Route>,
}

enum Decision {
    Accept { trust: bool, auto_accept: bool },
    Decline,
}

struct Pending {
    decide: Option<oneshot::Sender<Decision>>,
    /// What the request card shows. `None` while an auto-accepted request starts.
    request: Option<IncomingRequest>,
}

pub(crate) struct St {
    pub devices: HashMap<EndpointId, Dev>,
    pub transfers: HashMap<TransferId, Tx>,
    requests: HashMap<TransferId, Pending>,
    pub code: Option<HostedCode>,
    pub discovery: DiscoveryState,
    pub free_space: Option<u64>,
    /// Your own server's last known state (always `None` on public servers).
    pub server_problem: Option<ServerProblem>,
}

pub(crate) struct Inner {
    pub cfg: EngineConfig,
    /// The network actually in use.
    pub network: Network,
    pub endpoint: Endpoint,
    /// Addresses learned while pairing, so the first send doesn't wait on public lookups.
    pub learned: MemoryLookup,
    pub mdns: Option<iroh_mdns_address_lookup::MdnsAddressLookup>,
    pub store: Mutex<Store>,
    pub st: Mutex<St>,
    events: broadcast::Sender<EngineEvent>,
    tasks: Mutex<Vec<JoinHandle<()>>>,
    /// The task running the shown code's endpoint, so `stop` can let it close cleanly.
    pub code_task: Mutex<Option<JoinHandle<()>>>,
}

/// The transfer engine. Cheap to clone; all clones share one engine.
#[derive(Clone)]
pub struct Engine {
    pub(crate) inner: Arc<Inner>,
}

/// What this device is.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Me {
    pub id: String,
    pub name: String,
    pub kind: DeviceKind,
    pub os: OsPlatform,
    pub fingerprint: String,
    pub save_dir: PathBuf,
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

pub fn device_id(id: &EndpointId) -> String {
    id.to_string()
}

pub fn parse_device_id(s: &str) -> Option<EndpointId> {
    EndpointId::from_str(s.trim()).ok()
}

impl Engine {
    /// Opens the config folder, binds the network and starts discovery.
    pub async fn start(cfg: EngineConfig) -> Result<Self, EngineError> {
        let default_save = cfg
            .save_dir
            .clone()
            .or_else(|| cfg.default_save_dir.clone())
            .unwrap_or_else(store::default_save_dir);
        let mut store = Store::open(&cfg.config_dir, default_save)?;
        if let Some(dir) = &cfg.save_dir {
            store.state.save_dir = dir.clone();
        }
        let network = match &cfg.network {
            Some(n) => n.clone(),
            None => Network::from_connection(&store.state.connection)?,
        };
        let bound = net::bind(net::EndpointOpts {
            secret: store.secret.clone(),
            alpns: vec![wire::ALPN.to_vec()],
            network: &network,
            relay_only: cfg.relay_only,
            mdns: cfg.mdns,
            memory: cfg.memory_lookup.as_ref(),
        })
        .await
        .map_err(|e| EngineError::Bind(e.to_string()))?;
        if let Some(e) = &bound.mdns_error {
            warn!("mDNS unavailable, nearby devices won't show up: {e}");
        }

        let learned = MemoryLookup::new();
        bound
            .endpoint
            .address_lookup()
            .map_err(|e| EngineError::Bind(e.to_string()))?
            .add(learned.clone());
        let (events, _) = broadcast::channel(4096);
        let mut devices = HashMap::new();
        for d in &store.state.trusted {
            devices.insert(
                d.id,
                Dev {
                    info: Some(DeviceInfo {
                        name: d.name.clone(),
                        kind: d.kind,
                        os: d.os,
                    }),
                    ..Default::default()
                },
            );
        }
        let discovery = if cfg.mdns && bound.mdns.is_none() {
            DiscoveryState::Blocked
        } else {
            DiscoveryState::Searching
        };
        let inner = Inner {
            cfg,
            network,
            endpoint: bound.endpoint,
            learned,
            mdns: bound.mdns,
            store: Mutex::new(store),
            st: Mutex::new(St {
                devices,
                transfers: HashMap::new(),
                requests: HashMap::new(),
                code: None,
                discovery,
                free_space: None,
                server_problem: None,
            }),
            events,
            tasks: Mutex::new(Vec::new()),
            code_task: Mutex::new(None),
        };
        let engine = Self { inner: Arc::new(inner) };
        info!(id = %engine.id().fmt_short(), "engine started");

        engine.spawn_bg(engine.clone().accept_loop());
        engine.spawn_bg(engine.clone().mdns_loop());
        engine.spawn_bg(engine.clone().housekeeping_loop());
        engine.rescan();
        Ok(engine)
    }

    /// Closes the network and stops background work.
    pub async fn stop(&self) {
        self.cancel_code();
        let code_task = self.inner.code_task.lock().expect("code task lock").take();
        if let Some(t) = code_task {
            let _ = tokio::time::timeout(Duration::from_secs(3), t).await;
        }
        for h in self.inner.tasks.lock().expect("tasks lock").drain(..) {
            h.abort();
        }
        self.inner.endpoint.close().await;
    }

    pub fn subscribe(&self) -> broadcast::Receiver<EngineEvent> {
        self.inner.events.subscribe()
    }

    pub fn id(&self) -> EndpointId {
        self.inner.endpoint.id()
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.inner.endpoint
    }

    pub fn me(&self) -> Me {
        let s = self.store();
        Me {
            id: device_id(&self.id()),
            name: s.state.name.clone(),
            kind: s.state.kind,
            os: OsPlatform::current(),
            fingerprint: fingerprint(&self.id()),
            save_dir: s.state.save_dir.clone(),
        }
    }

    /// Waits until the relay connection is up (or `timeout` passes). Returns true if online.
    pub async fn online(&self, timeout: Duration) -> bool {
        tokio::time::timeout(timeout, self.inner.endpoint.online()).await.is_ok()
    }

    pub fn devices(&self) -> Vec<Device> {
        let ids: Vec<EndpointId> = self.st().devices.keys().copied().collect();
        let mut out: Vec<Device> = ids.iter().filter_map(|id| self.device_snapshot(id)).collect();
        out.sort_by(|a, b| b.trusted.cmp(&a.trusted).then(b.reachable.cmp(&a.reachable)).then(a.name.cmp(&b.name)));
        out
    }

    pub fn transfers(&self) -> Vec<Transfer> {
        let mut out: Vec<Transfer> = self.st().transfers.values().map(|t| t.t.clone()).collect();
        out.sort_by_key(|t| t.started_at);
        out
    }

    pub fn transfer(&self, id: &str) -> Option<Transfer> {
        let tid = wire::parse_transfer_id(id)?;
        self.st().transfers.get(&tid).map(|t| t.t.clone())
    }

    /// Requests waiting for an answer on this device.
    pub fn requests(&self) -> Vec<IncomingRequest> {
        let mut out: Vec<IncomingRequest> = self.st().requests.values().filter_map(|p| p.request.clone()).collect();
        out.sort_by_key(|r| r.received_at);
        out
    }

    /// Everything the app needs to draw itself after (re)loading.
    pub fn snapshot(&self) -> Snapshot {
        let (discovery, free_space, server_problem) = {
            let st = self.st();
            (st.discovery, st.free_space, st.server_problem)
        };
        Snapshot {
            discovery,
            devices: self.devices(),
            transfers: self.transfers(),
            requests: self.requests(),
            code: self.current_code(),
            free_space,
            server_problem,
        }
    }

    /// Where a received transfer's finished files are on disk (only the ones that arrived and checked out).
    pub fn saved_files(&self, transfer_id: &str) -> Vec<PathBuf> {
        let Some(id) = wire::parse_transfer_id(transfer_id) else { return vec![] };
        match self.st().transfers.get(&id).map(|t| &t.side) {
            Some(TxSide::Recv(files)) => files.iter().filter(|f| f.verified).map(|f| f.final_path.clone()).collect(),
            _ => vec![],
        }
    }

    /// Your own server's last known state. Always `None` on public servers.
    pub fn server_problem(&self) -> Option<ServerProblem> {
        self.st().server_problem
    }

    /// Checks your own relay now, remembers the answer and tells the app if it changed.
    /// Does nothing on public servers. Returns the problem, if any.
    pub async fn check_own_server(&self) -> Option<ServerProblem> {
        let Network::Own(own) = &self.inner.network else { return None };
        let problem = match crate::server::check_relay(&own.relay, own.token.as_deref()).await {
            CheckResult::Ok { .. } => None,
            CheckResult::TokenRejected => Some(ServerProblem::TokenRejected),
            CheckResult::Unreachable { .. } | CheckResult::Invalid { .. } => Some(ServerProblem::Unreachable),
        };
        let changed = {
            let mut st = self.st();
            let changed = st.server_problem != problem;
            st.server_problem = problem;
            changed
        };
        if changed {
            match problem {
                Some(p) => warn!(?p, "your own server has a problem"),
                None => info!("your own server is fine again"),
            }
            self.emit(EngineEvent::Server { problem });
        }
        problem
    }

    /// Looks for nearby devices again, re-checks trusted ones and your own server.
    pub fn rescan(&self) {
        if matches!(self.inner.network, Network::Own(_)) {
            let e = self.clone();
            self.spawn_bg(async move {
                e.check_own_server().await;
            });
        }
        let e = self.clone();
        self.spawn_bg(async move {
            if e.st().discovery != DiscoveryState::Blocked {
                e.set_discovery(DiscoveryState::Searching);
            }
            let trusted: Vec<EndpointId> = e.store().state.trusted.iter().map(|d| d.id).collect();
            let nearby: Vec<EndpointId> = e.st().devices.iter().filter(|(_, d)| d.nearby).map(|(id, _)| *id).collect();
            let mut checks = n0_future::FuturesUnordered::new();
            for id in trusted.into_iter().chain(nearby) {
                checks.push(e.clone().check_device(id));
            }
            let settle = tokio::time::sleep(Duration::from_secs(3));
            tokio::pin!(settle);
            loop {
                tokio::select! {
                    next = checks.next() => if next.is_none() { break },
                    _ = &mut settle => {}
                }
            }
            if e.st().discovery == DiscoveryState::Searching {
                e.set_discovery(DiscoveryState::Idle);
            }
        });
    }

    /// Sends files to a device. Returns the transfer id.
    pub fn send(&self, device: &str, paths: &[PathBuf]) -> Result<String, EngineError> {
        let named: Vec<(PathBuf, String)> = paths
            .iter()
            .map(|p| {
                (
                    p.clone(),
                    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into()),
                )
            })
            .collect();
        self.send_named(device, &named)
    }

    /// Sends files under the names given, for paths that don't carry a name. Android shares files
    /// as open handles (`/proc/self/fd/N`), which must stay open until the transfer is finished.
    pub fn send_named(&self, device: &str, files: &[(PathBuf, String)]) -> Result<String, EngineError> {
        let peer = parse_device_id(device).ok_or(EngineError::UnknownDevice)?;
        if peer == self.id() {
            return Err(EngineError::UnknownDevice);
        }
        if files.is_empty() {
            return Err(EngineError::NoFiles);
        }
        let mut src = Vec::new();
        let mut items = Vec::new();
        for (i, (p, name)) in files.iter().enumerate() {
            let meta = std::fs::metadata(p).map_err(|e| EngineError::File {
                path: p.clone(),
                reason: e.to_string(),
            })?;
            if !meta.is_file() {
                return Err(EngineError::File {
                    path: p.clone(),
                    reason: "not a file".into(),
                });
            }
            let name = files::safe_name(name);
            items.push(FileItem {
                id: format!("f{i}"),
                mime: files::mime_for(&name),
                name,
                size: meta.len(),
            });
            src.push(SrcFile {
                path: p.clone(),
                size: meta.len(),
                modified: meta.modified().ok(),
            });
        }
        let id: TransferId = rand::random();
        let route = self.st().devices.get(&peer).and_then(|d| d.route).unwrap_or(Route::Relayed);
        let tx = new_tx(id, TransferDirection::Send, peer, items, route, TxSide::Send(src), TransferStatus::Connecting);
        self.st().transfers.insert(id, tx);
        self.emit_tx(&id, true);
        let e = self.clone();
        self.spawn_bg(async move { e.drive_send(id).await });
        Ok(wire::transfer_id_str(&id))
    }

    /// Accepts an incoming request. Returns the transfer id.
    pub fn accept(&self, request_id: &str, trust: bool, auto_accept: bool) -> Option<String> {
        let id = wire::parse_transfer_id(request_id)?;
        let decide = self.st().requests.get_mut(&id)?.decide.take()?;
        decide.send(Decision::Accept { trust, auto_accept }).ok()?;
        Some(request_id.to_string())
    }

    pub fn decline(&self, request_id: &str) {
        let Some(id) = wire::parse_transfer_id(request_id) else { return };
        if let Some(d) = self.st().requests.get_mut(&id).and_then(|p| p.decide.take()) {
            let _ = d.send(Decision::Decline);
        }
    }

    pub fn pause(&self, transfer_id: &str) {
        self.send_cmd(transfer_id, Cmd::Pause);
    }

    pub fn resume(&self, transfer_id: &str) {
        self.send_cmd(transfer_id, Cmd::Resume);
    }

    pub fn cancel(&self, transfer_id: &str) {
        let Some(id) = wire::parse_transfer_id(transfer_id) else { return };
        let has_session = {
            let st = self.st();
            st.transfers.get(&id).map(|t| t.cmd.as_ref().is_some_and(|c| c.send(Cmd::Cancel).is_ok()))
        };
        if has_session == Some(false) {
            // Failed or reconnecting without a live session: just mark it cancelled here.
            let parts = self.with_tx(&id, |tx| {
                if tx.t.status.is_final() {
                    return vec![];
                }
                tx.epoch += 1;
                tx.t.status = TransferStatus::Cancelled;
                tx.t.cancelled_by = Some(Side::Me);
                tx.t.finished_at = Some(now_ms());
                tx.stop_moving();
                match &tx.side {
                    TxSide::Recv(files) => files.iter().map(|f| f.part.clone()).collect(),
                    TxSide::Send(_) => vec![],
                }
            });
            for p in parts.unwrap_or_default() {
                let _ = std::fs::remove_file(p);
            }
            self.emit_tx(&id, true);
        }
    }

    /// Picks a failed transfer up from the last confirmed byte.
    pub fn retry(&self, transfer_id: &str) {
        let Some(id) = wire::parse_transfer_id(transfer_id) else { return };
        let dir = self.with_tx(&id, |tx| {
            if tx.t.status != TransferStatus::Failed {
                return None;
            }
            tx.t.status = TransferStatus::Connecting;
            tx.t.error = None;
            tx.t.paused_by = None;
            tx.epoch += 1;
            Some((tx.t.direction, tx.epoch))
        });
        let Some(Some((direction, epoch))) = dir else { return };
        self.emit_tx(&id, true);
        let e = self.clone();
        match direction {
            TransferDirection::Send => self.spawn_bg(async move { e.reconnect_send(id, epoch).await }),
            TransferDirection::Receive => self.spawn_bg(async move { e.pull(id, epoch).await }),
        }
    }

    pub fn set_trusted(&self, device: &str, trusted: bool) {
        let Some(id) = parse_device_id(device) else { return };
        {
            let mut s = self.store();
            if trusted {
                if s.trusted(&id).is_none() {
                    let info = self.st().devices.get(&id).and_then(|d| d.info.clone());
                    let Some(info) = info else { return };
                    s.state.trusted.push(TrustedDevice {
                        id,
                        name: info.name,
                        kind: info.kind,
                        os: info.os,
                        auto_accept: false,
                        added_at: now_ms(),
                    });
                }
            } else {
                s.state.trusted.retain(|d| d.id != id);
            }
            if let Err(e) = s.save() {
                warn!("couldn't save trusted devices: {e}");
            }
        }
        self.emit_device(&id);
    }

    pub fn set_auto_accept(&self, device: &str, on: bool) {
        let Some(id) = parse_device_id(device) else { return };
        {
            let mut s = self.store();
            let Some(d) = s.trusted_mut(&id) else { return };
            d.auto_accept = on;
            if let Err(e) = s.save() {
                warn!("couldn't save trusted devices: {e}");
            }
        }
        self.emit_device(&id);
    }

    pub fn rename_self(&self, name: &str) {
        let name: String = name.trim().chars().filter(|c| !c.is_control()).take(32).collect();
        if name.is_empty() {
            return;
        }
        let mut s = self.store();
        s.state.name = name;
        if let Err(e) = s.save() {
            warn!("couldn't save the device name: {e}");
        }
    }

    /// The saved connection setting.
    pub fn connection(&self) -> types::Connection {
        self.store().state.connection.clone()
    }

    /// The network this engine is running on (the saved setting, or the override it started with).
    pub fn network(&self) -> &Network {
        &self.inner.network
    }

    /// Checks and saves the connection setting. It takes effect the next time the engine starts,
    /// so the app restarts the engine after saving.
    pub fn set_connection(&self, c: types::Connection) -> Result<(), EngineError> {
        Network::from_connection(&c)?;
        let mut s = self.store();
        s.state.connection = c;
        s.save()?;
        Ok(())
    }

    pub fn set_save_dir(&self, dir: &Path) {
        let mut s = self.store();
        s.state.save_dir = dir.to_path_buf();
        if let Err(e) = s.save() {
            warn!("couldn't save the save folder: {e}");
        }
    }

    /// Tests: drop a live transfer's connection as if the network went away.
    #[doc(hidden)]
    pub fn test_drop_connection(&self, transfer_id: &str) {
        self.test_close(transfer_id, iroh::endpoint::VarInt::from_u32(99), b"test drop");
    }

    /// Tests: stop a live transfer with a generic error, so it can be retried.
    #[doc(hidden)]
    pub fn test_fail_connection(&self, transfer_id: &str) {
        self.send_cmd(transfer_id, Cmd::Fail);
    }

    fn test_close(&self, transfer_id: &str, code: iroh::endpoint::VarInt, reason: &[u8]) {
        let Some(id) = wire::parse_transfer_id(transfer_id) else { return };
        if let Some(conn) = self.with_tx(&id, |tx| tx.conn.clone()).flatten() {
            conn.close(code, reason);
        }
    }

    // ---------- internals ----------

    pub(crate) fn st(&self) -> MutexGuard<'_, St> {
        self.inner.st.lock().expect("engine state lock")
    }

    pub(crate) fn store(&self) -> MutexGuard<'_, Store> {
        self.inner.store.lock().expect("store lock")
    }

    pub(crate) fn emit(&self, e: EngineEvent) {
        let _ = self.inner.events.send(e);
    }

    pub(crate) fn spawn_bg(&self, fut: impl Future<Output = ()> + Send + 'static) {
        let h = tokio::spawn(fut);
        let mut tasks = self.inner.tasks.lock().expect("tasks lock");
        tasks.retain(|t| !t.is_finished());
        tasks.push(h);
    }

    pub(crate) fn me_info(&self) -> DeviceInfo {
        let s = self.store();
        DeviceInfo {
            name: s.state.name.clone(),
            kind: s.state.kind,
            os: OsPlatform::current(),
        }
    }

    pub(crate) fn save_dir(&self) -> PathBuf {
        self.store().state.save_dir.clone()
    }

    pub(crate) fn with_tx<R>(&self, id: &TransferId, f: impl FnOnce(&mut Tx) -> R) -> Option<R> {
        self.st().transfers.get_mut(id).map(f)
    }

    /// Emits the transfer. Progress-only updates are limited to one every 100 ms.
    pub(crate) fn emit_tx(&self, id: &TransferId, force: bool) {
        let snap = {
            let mut st = self.st();
            let Some(tx) = st.transfers.get_mut(id) else { return };
            let now = Instant::now();
            if !force && tx.last_emit.is_some_and(|t| now.duration_since(t) < Duration::from_millis(100)) {
                return;
            }
            tx.last_emit = Some(now);
            tx.t.clone()
        };
        self.emit(EngineEvent::Transfer { transfer: snap });
    }

    fn send_cmd(&self, transfer_id: &str, cmd: Cmd) {
        let Some(id) = wire::parse_transfer_id(transfer_id) else { return };
        if let Some(c) = self.st().transfers.get(&id).and_then(|t| t.cmd.clone()) {
            let _ = c.send(cmd);
        }
    }

    fn set_discovery(&self, state: DiscoveryState) {
        self.st().discovery = state;
        self.emit(EngineEvent::Discovery { state });
    }

    pub(crate) fn device_snapshot(&self, id: &EndpointId) -> Option<Device> {
        let trusted = self.store().trusted(id).cloned();
        let st = self.st();
        let dev = st.devices.get(id);
        let info = dev.and_then(|d| d.info.clone()).or_else(|| {
            trusted.as_ref().map(|t| DeviceInfo {
                name: t.name.clone(),
                kind: t.kind,
                os: t.os,
            })
        })?;
        let reachable = dev.is_some_and(|d| d.reachable);
        Some(Device {
            id: device_id(id),
            name: info.name,
            kind: info.kind,
            os: info.os,
            trusted: trusted.is_some(),
            auto_accept: trusted.is_some_and(|t| t.auto_accept),
            reachable,
            route: if reachable { dev.and_then(|d| d.route) } else { None },
            fingerprint: fingerprint(id),
        })
    }

    pub(crate) fn emit_device(&self, id: &EndpointId) {
        if let Some(device) = self.device_snapshot(id) {
            self.emit(EngineEvent::Device { device });
        }
    }

    /// Records what we learned about a peer from a hello. Keeps trusted names fresh.
    pub(crate) fn note_peer(&self, id: EndpointId, info: DeviceInfo, route: Route) {
        let changed = {
            let mut st = self.st();
            let d = st.devices.entry(id).or_default();
            let before = (d.info.clone(), d.reachable, d.route, d.nearby);
            d.info = Some(info.clone());
            d.reachable = true;
            d.route = Some(route);
            if route == Route::Nearby {
                d.nearby = true;
            }
            before != (d.info.clone(), d.reachable, d.route, d.nearby)
        };
        {
            let mut s = self.store();
            if let Some(t) = s.trusted_mut(&id)
                && (t.name != info.name || t.kind != info.kind || t.os != info.os)
            {
                t.name = info.name;
                t.kind = info.kind;
                t.os = info.os;
                let _ = s.save();
            }
        }
        if changed {
            self.emit_device(&id);
        }
    }

    fn peer_allowed(&self, id: &EndpointId, conn: &Connection) -> bool {
        if self.store().trusted(id).is_some() {
            return true;
        }
        let known = self.st().devices.get(id).is_some_and(|d| d.paired || d.nearby);
        known || net::is_lan_conn(conn)
    }

    async fn check_device(self, id: EndpointId) {
        let me = self.me_info();
        match net::ping(&self.inner.endpoint, id, me, Duration::from_secs(8)).await {
            Ok((info, route)) => self.note_peer(id, info, route),
            Err(err) => {
                debug!(peer = %id.fmt_short(), %err, "device check failed");
                self.mark_unreachable(&id);
            }
        }
    }

    fn mark_unreachable(&self, id: &EndpointId) {
        let trusted = self.store().trusted(id).is_some();
        let (changed, remove) = {
            let mut st = self.st();
            let Some(d) = st.devices.get_mut(id) else { return };
            let known = d.info.is_some();
            let changed = d.reachable && known;
            d.reachable = false;
            d.route = None;
            let remove = !trusted && !d.paired;
            if remove {
                st.devices.remove(id);
            }
            (changed, remove && known)
        };
        if remove {
            self.emit(EngineEvent::DeviceLost { device_id: device_id(id) });
        } else if changed {
            self.emit_device(id);
        }
    }

    async fn accept_loop(self) {
        while let Some(incoming) = self.inner.endpoint.accept().await {
            let e = self.clone();
            tokio::spawn(async move {
                let conn = match incoming.accept() {
                    Ok(accepting) => match accepting.await {
                        Ok(c) => c,
                        Err(err) => return debug!(%err, "handshake failed"),
                    },
                    Err(err) => return debug!(%err, "incoming connection failed"),
                };
                if conn.alpn() != wire::ALPN {
                    conn.close(wire::close::PROTOCOL, b"alpn");
                    return;
                }
                if let Err(err) = e.handle_conn(conn).await {
                    debug!(%err, "incoming connection ended");
                }
            });
        }
    }

    async fn mdns_loop(self) {
        let Some(mdns) = self.inner.mdns.clone() else { return };
        let mut events = mdns.subscribe().await;
        while let Some(ev) = events.next().await {
            match ev {
                iroh_mdns_address_lookup::DiscoveryEvent::Discovered { endpoint_info, .. } => {
                    let id = endpoint_info.endpoint_id;
                    if id == self.id() {
                        continue;
                    }
                    self.st().devices.entry(id).or_default().nearby = true;
                    tokio::spawn(self.clone().check_device(id));
                }
                iroh_mdns_address_lookup::DiscoveryEvent::Expired { endpoint_id } => {
                    if let Some(d) = self.st().devices.get_mut(&endpoint_id) {
                        d.nearby = false;
                    }
                    tokio::spawn(self.clone().check_device(endpoint_id));
                }
                _ => {}
            }
        }
    }

    /// Free space every 15 s, trusted devices every 60 s.
    async fn housekeeping_loop(self) {
        let mut tick = 0u64;
        loop {
            let dir = self.save_dir();
            if let Some(bytes) = tokio::task::spawn_blocking(move || files::free_space(&dir)).await.ok().flatten() {
                let changed = {
                    let mut st = self.st();
                    let changed = st.free_space != Some(bytes);
                    st.free_space = Some(bytes);
                    changed
                };
                if changed {
                    self.emit(EngineEvent::FreeSpace { bytes });
                }
            }
            // Your own server: every 30 s while something is wrong, every 5 minutes otherwise.
            let server_every = if self.st().server_problem.is_some() { 2 } else { 20 };
            if tick > 0 && tick.is_multiple_of(server_every) && matches!(self.inner.network, Network::Own(_)) {
                let e = self.clone();
                tokio::spawn(async move {
                    e.check_own_server().await;
                });
            }
            if tick > 0 && tick.is_multiple_of(4) {
                let trusted: Vec<EndpointId> = self.store().state.trusted.iter().map(|d| d.id).collect();
                for id in trusted {
                    tokio::spawn(self.clone().check_device(id));
                }
            }
            tick += 1;
            tokio::time::sleep(Duration::from_secs(15)).await;
        }
    }

    // ---------- incoming ----------

    async fn handle_conn(&self, conn: Connection) -> Result<(), String> {
        let peer = conn.remote_id();
        let (mut send, recv) = conn.accept_bi().await.map_err(|e| e.to_string())?;
        let mut frames = spawn_frame_reader(recv);
        let hello: ClientHello = next_frame(&mut frames, Duration::from_secs(10)).await?;
        if hello.version != wire::VERSION {
            conn.close(wire::close::PROTOCOL, b"version");
            return Err(format!("unsupported version {}", hello.version));
        }
        if !self.peer_allowed(&peer, &conn) {
            conn.close(wire::close::NOT_ALLOWED, b"not allowed");
            return Err("not allowed".into());
        }
        self.note_peer(peer, hello.info.clone(), net::route_of(&conn));
        wire::write_frame(
            &mut send,
            &ServerHello {
                version: wire::VERSION,
                info: self.me_info(),
            },
        )
        .await
        .map_err(|e| e.to_string())?;
        match hello.intent {
            Intent::Ping => {
                let _ = send.finish();
                let _ = tokio::time::timeout(Duration::from_secs(5), conn.closed()).await;
                Ok(())
            }
            Intent::Offer(offer) => self.handle_offer(conn, send, frames, peer, offer).await,
            Intent::Resume { id } => {
                let ok = self
                    .with_tx(&id, |tx| {
                        tx.peer == peer && tx.t.direction == TransferDirection::Receive && !tx.t.status.is_final()
                    })
                    .unwrap_or(false);
                if !ok {
                    let _ = wire::write_frame(&mut send, &Answer::Decline(DeclineReason::Unknown)).await;
                    conn.close(wire::close::DECLINED, b"unknown transfer");
                    return Err("resume for unknown transfer".into());
                }
                let files = self.receiver_offsets(&id);
                wire::write_frame(&mut send, &Answer::Accept { files }).await.map_err(|e| e.to_string())?;
                self.run_session(id, conn, send, frames, None).await;
                Ok(())
            }
            Intent::Pull { id, files } => {
                let ok = self
                    .with_tx(&id, |tx| {
                        tx.peer == peer && tx.t.direction == TransferDirection::Send && !tx.t.status.is_final()
                    })
                    .unwrap_or(false);
                if !ok {
                    let _ = wire::write_frame(&mut send, &Answer::Decline(DeclineReason::Unknown)).await;
                    conn.close(wire::close::DECLINED, b"unknown transfer");
                    return Err("pull for unknown transfer".into());
                }
                wire::write_frame(&mut send, &Answer::Accept { files: files.clone() })
                    .await
                    .map_err(|e| e.to_string())?;
                self.run_session(id, conn, send, frames, Some(files)).await;
                Ok(())
            }
        }
    }

    async fn handle_offer(&self, conn: Connection, mut send: SendStream, mut frames: FrameRx, peer: EndpointId, offer: Offer) -> Result<(), String> {
        let valid = !offer.files.is_empty() && offer.files.len() <= 100_000 && offer.files.iter().try_fold(0u64, |a, f| a.checked_add(f.size)).is_some();
        if !valid || self.st().transfers.contains_key(&offer.id) || self.st().requests.contains_key(&offer.id) {
            let _ = wire::write_frame(&mut send, &Answer::Decline(DeclineReason::Unknown)).await;
            conn.close(wire::close::DECLINED, b"bad offer");
            return Err("bad offer".into());
        }
        let items: Vec<FileItem> = offer
            .files
            .iter()
            .enumerate()
            .map(|(i, f)| {
                let name = files::safe_name(&f.name);
                FileItem {
                    id: format!("f{i}"),
                    mime: files::mime_for(&name),
                    name,
                    size: f.size,
                }
            })
            .collect();
        let total: u64 = items.iter().map(|f| f.size).sum();
        let request_id = wire::transfer_id_str(&offer.id);
        let auto = self.store().trusted(&peer).is_some_and(|t| t.auto_accept);

        let (decide_tx, decide_rx) = oneshot::channel();
        let request = (!auto).then(|| IncomingRequest {
            id: request_id.clone(),
            device_id: device_id(&peer),
            files: items.clone(),
            total_bytes: total,
            received_at: now_ms(),
        });
        self.st().requests.insert(
            offer.id,
            Pending {
                decide: Some(decide_tx),
                request: request.clone(),
            },
        );
        match request {
            None => {
                let _ = self.accept(&request_id, false, true);
            }
            Some(request) => self.emit(EngineEvent::Incoming { request }),
        }

        let decision = tokio::select! {
            d = decide_rx => d.unwrap_or(Decision::Decline),
            frame = frames.recv() => {
                // The sender withdrew (or the connection dropped) while we were deciding.
                self.st().requests.remove(&offer.id);
                if matches!(frame.as_ref().map(|f| f.as_ref().ok().and_then(|b| postcard::from_bytes::<Ctrl>(b).ok())), Some(Some(Ctrl::Withdraw)) | None | Some(None)) {
                    self.emit(EngineEvent::IncomingWithdrawn { request_id });
                }
                return Ok(());
            }
        };
        self.st().requests.remove(&offer.id);

        let (trust, auto_accept) = match decision {
            Decision::Decline => {
                let _ = wire::write_frame(&mut send, &Answer::Decline(DeclineReason::User)).await;
                let _ = send.finish();
                let _ = tokio::time::timeout(Duration::from_secs(3), conn.closed()).await;
                return Ok(());
            }
            Decision::Accept { trust, auto_accept } => (trust, auto_accept),
        };
        if trust {
            self.set_trusted(&device_id(&peer), true);
            if auto_accept {
                self.set_auto_accept(&device_id(&peer), true);
            }
        }

        // Reserve names and create empty part files, so later transfers pick other names.
        let dir = self.save_dir();
        let mut dst = Vec::new();
        let mut taken = Vec::new();
        let created = std::fs::create_dir_all(&dir).and_then(|_| {
            for f in &items {
                let final_path = files::unique_path(&dir, &f.name, &taken);
                let part = files::part_path(&final_path);
                std::fs::File::create(&part)?;
                taken.push(final_path.clone());
                dst.push(DstFile {
                    final_path,
                    part,
                    size: f.size,
                    verified: false,
                });
            }
            Ok(())
        });
        let route = net::route_of(&conn);
        let mut tx = new_tx(
            offer.id,
            TransferDirection::Receive,
            peer,
            items,
            route,
            TxSide::Recv(dst.clone()),
            TransferStatus::Transferring,
        );
        if let Err(err) = created {
            warn!(%err, "can't save incoming files");
            for d in &dst {
                let _ = std::fs::remove_file(&d.part);
            }
            tx.t.status = TransferStatus::Failed;
            tx.t.error = Some(if files::is_disk_full(&err) {
                TransferError::DiskFull
            } else {
                TransferError::Permission
            });
            self.st().transfers.insert(offer.id, tx);
            self.emit_tx(&offer.id, true);
            let _ = wire::write_frame(&mut send, &Answer::Decline(DeclineReason::Unknown)).await;
            conn.close(wire::close::DECLINED, b"can't save");
            return Ok(());
        }
        self.st().transfers.insert(offer.id, tx);
        self.emit_tx(&offer.id, true);
        wire::write_frame(
            &mut send,
            &Answer::Accept {
                files: vec![FileState::Have(0); dst.len()],
            },
        )
        .await
        .map_err(|e| e.to_string())?;
        self.run_session(offer.id, conn, send, frames, None).await;
        Ok(())
    }

    /// What the receiver already has for each file: verified, or the part file length.
    pub(crate) fn receiver_offsets(&self, id: &TransferId) -> Vec<FileState> {
        let files = self.with_tx(id, |tx| match &tx.side {
            TxSide::Recv(f) => f.clone(),
            TxSide::Send(_) => vec![],
        });
        files
            .unwrap_or_default()
            .iter()
            .map(|f| {
                if f.verified {
                    FileState::Verified
                } else {
                    FileState::Have(std::fs::metadata(&f.part).map(|m| m.len().min(f.size)).unwrap_or(0))
                }
            })
            .collect()
    }

    // ---------- outgoing ----------

    async fn drive_send(self, id: TransferId) {
        let Some((peer, metas)) = self.with_tx(&id, |tx| {
            (
                tx.peer,
                tx.t.files
                    .iter()
                    .map(|f| FileMeta {
                        name: f.name.clone(),
                        size: f.size,
                    })
                    .collect::<Vec<_>>(),
            )
        }) else {
            return;
        };
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
        let Some(epoch) = self.with_tx(&id, |tx| {
            tx.cmd = Some(cmd_tx);
            tx.epoch += 1;
            tx.epoch
        }) else {
            return;
        };
        let opened = self.open(peer, Intent::Offer(Offer { id, files: metas })).await;
        let (conn, mut send, mut frames) = match opened {
            Ok(x) => x,
            Err(err) => {
                debug!(%err, "couldn't reach the receiver");
                self.mark_unreachable(&peer);
                return self.fail(&id, TransferError::PeerGone);
            }
        };
        self.with_tx(&id, |tx| {
            tx.t.status = TransferStatus::Waiting;
            tx.t.route = net::route_of(&conn);
        });
        self.emit_tx(&id, true);

        let long_wait = tokio::time::sleep(Duration::from_secs(60));
        tokio::pin!(long_wait);
        let answer = loop {
            tokio::select! {
                frame = frames.recv() => break frame,
                cmd = cmd_rx.recv() => if matches!(cmd, Some(Cmd::Cancel) | None) {
                    let _ = wire::write_frame(&mut send, &Ctrl::Withdraw).await;
                    conn.close(wire::close::CANCELLED, b"withdrawn");
                    return self.finish(&id, epoch, End::Cancelled(Side::Me));
                },
                _ = &mut long_wait, if !self.with_tx(&id, |t| t.t.waiting_long).unwrap_or(true) => {
                    self.with_tx(&id, |tx| tx.t.waiting_long = true);
                    self.emit_tx(&id, true);
                }
            }
        };
        match answer.map(|r| r.and_then(|b| postcard::from_bytes::<Answer>(&b).map_err(Into::into))) {
            Some(Ok(Answer::Accept { files })) => {
                self.with_tx(&id, |tx| {
                    tx.t.status = TransferStatus::Transferring;
                    tx.t.waiting_long = false;
                });
                self.emit_tx(&id, true);
                self.run_session_with(id, conn, send, frames, Some(files), cmd_rx, epoch).await;
            }
            Some(Ok(Answer::Decline(reason))) => {
                conn.close(wire::close::DONE, b"");
                self.with_tx(&id, |tx| {
                    tx.t.status = TransferStatus::Declined;
                    tx.t.error = (reason == DeclineReason::NoSpace).then_some(TransferError::DiskFullPeer);
                    tx.t.finished_at = Some(now_ms());
                    tx.cmd = None;
                });
                self.emit_tx(&id, true);
            }
            other => {
                debug!(answer = ?other.map(|r| r.map(|_| ())), "no answer from the receiver");
                self.fail(&id, TransferError::PeerGone)
            }
        }
    }

    /// Dials a peer with an intent and reads its hello. Returns the connection and control stream.
    pub(crate) async fn open(&self, peer: EndpointId, intent: Intent) -> Result<(Connection, SendStream, FrameRx), String> {
        let conn = self.connect(peer, Duration::from_secs(20)).await?;
        let (mut send, recv) = conn.open_bi().await.map_err(|e| e.to_string())?;
        let mut frames = spawn_frame_reader(recv);
        wire::write_frame(
            &mut send,
            &ClientHello {
                version: wire::VERSION,
                info: self.me_info(),
                intent,
            },
        )
        .await
        .map_err(|e| e.to_string())?;
        let hello: ServerHello = next_frame(&mut frames, Duration::from_secs(10)).await?;
        self.note_peer(peer, hello.info, net::route_of(&conn));
        Ok((conn, send, frames))
    }

    /// Connects on nhako/1. If the peer's address isn't known yet (its lookup record can take a
    /// few seconds to appear), keeps asking for up to 10 seconds.
    pub(crate) async fn connect(&self, peer: EndpointId, timeout: Duration) -> Result<Connection, String> {
        let deadline = tokio::time::Instant::now() + timeout;
        let lookup_until = tokio::time::Instant::now() + Duration::from_secs(10);
        loop {
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            match tokio::time::timeout(left, self.inner.endpoint.connect(peer, wire::ALPN)).await {
                Ok(Ok(conn)) => return Ok(conn),
                Ok(Err(iroh::endpoint::ConnectError::Connect {
                    source: iroh::endpoint::ConnectWithOptsError::NoAddress { .. },
                    ..
                })) if tokio::time::Instant::now() < lookup_until => {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
                Ok(Err(e)) => return Err(e.to_string()),
                Err(_) => return Err("timed out".into()),
            }
        }
    }

    /// Sender side: redial with `Resume` until it works or `reconnect_for` passes.
    async fn reconnect_send(self, id: TransferId, epoch: u64) {
        let Some(peer) = self.with_tx(&id, |tx| tx.peer) else { return };
        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel();
        self.with_tx(&id, |tx| tx.cmd = Some(cmd_tx));
        let deadline = Instant::now() + self.inner.cfg.reconnect_for;
        loop {
            if self.with_tx(&id, |tx| tx.epoch != epoch).unwrap_or(true) {
                return;
            }
            if let Ok((conn, mut send, mut frames)) = self.open(peer, Intent::Resume { id }).await {
                match next_frame::<Answer>(&mut frames, Duration::from_secs(10)).await {
                    Ok(Answer::Accept { files }) => {
                        self.with_tx(&id, |tx| tx.t.status = TransferStatus::Transferring);
                        self.emit_tx(&id, true);
                        return self.run_session_with(id, conn, send, frames, Some(files), cmd_rx, epoch).await;
                    }
                    _ => {
                        let _ = send.finish();
                        conn.close(wire::close::DONE, b"");
                        return self.fail(&id, TransferError::PeerGone);
                    }
                }
            }
            if Instant::now() >= deadline {
                return self.fail(&id, TransferError::PeerGone);
            }
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(2)) => {}
                cmd = cmd_rx.recv() => if matches!(cmd, Some(Cmd::Cancel) | None) {
                    return self.finish(&id, epoch, End::Cancelled(Side::Me));
                }
            }
        }
    }

    /// Receiver side of "Try again": ask the sender to continue.
    async fn pull(self, id: TransferId, epoch: u64) {
        let Some(peer) = self.with_tx(&id, |tx| tx.peer) else { return };
        let files = self.receiver_offsets(&id);
        // Re-create part files that went missing (for example after an integrity failure).
        if let Some(TxSide::Recv(files)) = self.with_tx(&id, |tx| tx.side.clone()) {
            for f in files.iter().filter(|f| !f.verified && !f.part.exists()) {
                let _ = std::fs::File::create(&f.part);
            }
        }
        match self.open(peer, Intent::Pull { id, files }).await {
            Ok((conn, send, mut frames)) => match next_frame::<Answer>(&mut frames, Duration::from_secs(10)).await {
                Ok(Answer::Accept { .. }) => {
                    self.with_tx(&id, |tx| tx.t.status = TransferStatus::Transferring);
                    self.emit_tx(&id, true);
                    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
                    self.with_tx(&id, |tx| tx.cmd = Some(cmd_tx));
                    self.run_session_with(id, conn, send, frames, None, cmd_rx, epoch).await
                }
                _ => self.fail(&id, TransferError::PeerGone),
            },
            Err(_) => self.fail(&id, TransferError::PeerGone),
        }
    }

    async fn run_session(&self, id: TransferId, conn: Connection, send: SendStream, frames: FrameRx, start: Option<Vec<FileState>>) {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let Some(epoch) = self.with_tx(&id, |tx| {
            tx.cmd = Some(cmd_tx);
            tx.epoch += 1;
            tx.t.status = TransferStatus::Transferring;
            tx.t.error = None;
            tx.t.paused_by = None;
            tx.epoch
        }) else {
            return;
        };
        self.emit_tx(&id, true);
        self.run_session_with(id, conn, send, frames, start, cmd_rx, epoch).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn run_session_with(
        &self,
        id: TransferId,
        conn: Connection,
        send: SendStream,
        frames: FrameRx,
        start: Option<Vec<FileState>>,
        cmds: CmdRx,
        epoch: u64,
    ) {
        let Some(direction) = self.with_tx(&id, |tx| {
            tx.conn = Some(conn.clone());
            tx.t.direction
        }) else {
            return;
        };
        let watcher = tokio::spawn(watch_route(self.clone(), id, conn.clone()));
        let end = match direction {
            TransferDirection::Send => session::run_sender(self.clone(), id, conn, send, frames, start.unwrap_or_default(), cmds).await,
            TransferDirection::Receive => session::run_receiver(self.clone(), id, conn, send, frames, cmds).await,
        };
        watcher.abort();
        self.finish(&id, epoch, end);
    }

    /// Applies how a session ended. Ignores stale sessions.
    pub(crate) fn finish(&self, id: &TransferId, epoch: u64, end: End) {
        if self.with_tx(id, |tx| tx.epoch != epoch).unwrap_or(true) {
            return;
        }
        debug!(id = %wire::transfer_id_str(id), ?end, "session ended");
        let lost = matches!(end, End::Lost);
        let parts_to_delete = self
            .with_tx(id, |tx| {
                tx.stop_moving();
                tx.cmd = None;
                tx.conn = None;
                match end {
                    End::Done => {
                        tx.t.status = TransferStatus::Done;
                        tx.t.finished_at = Some(now_ms());
                        let have: Vec<u64> = tx.t.files.iter().map(|f| f.size).collect();
                        tx.set_have(&have);
                        tx.t.speed = 0.0;
                        vec![]
                    }
                    End::Cancelled(by) => {
                        tx.t.status = TransferStatus::Cancelled;
                        tx.t.cancelled_by = Some(by);
                        tx.t.finished_at = Some(now_ms());
                        match &tx.side {
                            TxSide::Recv(files) => files.iter().filter(|f| !f.verified).map(|f| f.part.clone()).collect(),
                            TxSide::Send(_) => vec![],
                        }
                    }
                    End::Failed(err) => {
                        tx.t.status = TransferStatus::Failed;
                        tx.t.error = Some(err);
                        vec![]
                    }
                    End::Lost => {
                        tx.t.status = TransferStatus::Reconnecting;
                        tx.t.paused_by = None;
                        vec![]
                    }
                }
            })
            .unwrap_or_default();
        for p in parts_to_delete {
            let _ = std::fs::remove_file(p);
        }
        self.emit_tx(id, true);
        if lost {
            let e = self.clone();
            let id = *id;
            let direction = self.with_tx(&id, |tx| tx.t.direction);
            let new_epoch = self.with_tx(&id, |tx| {
                tx.epoch += 1;
                tx.epoch
            });
            let Some(new_epoch) = new_epoch else { return };
            match direction {
                Some(TransferDirection::Send) => self.spawn_bg(async move { e.reconnect_send(id, new_epoch).await }),
                _ => {
                    // The receiver waits for the sender to come back, then gives up.
                    let wait = self.inner.cfg.reconnect_for + Duration::from_secs(5);
                    self.spawn_bg(async move {
                        tokio::time::sleep(wait).await;
                        let still = e
                            .with_tx(&id, |tx| tx.epoch == new_epoch && tx.t.status == TransferStatus::Reconnecting)
                            .unwrap_or(false);
                        if still {
                            e.fail(&id, TransferError::PeerGone);
                        }
                    })
                }
            }
        }
        if self
            .with_tx(id, |tx| tx.t.direction == TransferDirection::Receive && tx.t.status == TransferStatus::Done)
            .unwrap_or(false)
        {
            let e = self.clone();
            self.spawn_bg(async move {
                let dir = e.save_dir();
                if let Some(bytes) = tokio::task::spawn_blocking(move || files::free_space(&dir)).await.ok().flatten() {
                    e.st().free_space = Some(bytes);
                    e.emit(EngineEvent::FreeSpace { bytes });
                }
            });
        }
    }

    pub(crate) fn fail(&self, id: &TransferId, err: TransferError) {
        self.with_tx(id, |tx| {
            if tx.t.status.is_final() {
                return;
            }
            tx.t.status = TransferStatus::Failed;
            tx.t.error = Some(err);
            tx.cmd = None;
            tx.stop_moving();
        });
        self.emit_tx(id, true);
    }
}

fn new_tx(id: TransferId, direction: TransferDirection, peer: EndpointId, files: Vec<FileItem>, route: Route, side: TxSide, status: TransferStatus) -> Tx {
    let total = files.iter().map(|f| f.size).sum();
    let file_bytes: BTreeMap<String, u64> = files.iter().map(|f| (f.id.clone(), 0)).collect();
    Tx {
        t: Transfer {
            id: wire::transfer_id_str(&id),
            direction,
            device_id: device_id(&peer),
            files,
            total_bytes: total,
            done_bytes: 0,
            file_bytes,
            speed: 0.0,
            route,
            status,
            paused_by: None,
            cancelled_by: None,
            error: None,
            started_at: now_ms(),
            moving_since: None,
            finished_at: None,
            waiting_long: false,
        },
        peer,
        side,
        cmd: None,
        conn: None,
        epoch: 0,
        last_emit: None,
        samples: VecDeque::new(),
    }
}

/// Reads frames off a stream on its own task, so callers can `select!` on them safely.
pub(crate) fn spawn_frame_reader(mut recv: RecvStream) -> FrameRx {
    let (tx, rx) = mpsc::channel(64);
    tokio::spawn(async move {
        loop {
            let frame = read_raw_frame(&mut recv).await;
            let end = frame.is_err();
            if tx.send(frame).await.is_err() || end {
                return;
            }
        }
    });
    rx
}

async fn read_raw_frame(recv: &mut RecvStream) -> Result<Vec<u8>, wire::WireError> {
    let mut len = [0u8; 4];
    match recv.read_exact(&mut len).await {
        Ok(()) => {}
        Err(iroh::endpoint::ReadExactError::FinishedEarly(0)) => return Err(wire::WireError::Closed),
        Err(e) => return Err(wire::WireError::Read(e.to_string())),
    }
    let len = u32::from_be_bytes(len) as usize;
    if len > wire::MAX_FRAME {
        return Err(wire::WireError::TooLong(len));
    }
    let mut body = vec![0u8; len];
    recv.read_exact(&mut body).await.map_err(|e| wire::WireError::Read(e.to_string()))?;
    Ok(body)
}

pub(crate) async fn next_frame<T: serde::de::DeserializeOwned>(frames: &mut FrameRx, timeout: Duration) -> Result<T, String> {
    match tokio::time::timeout(timeout, frames.recv()).await {
        Err(_) => Err("timed out".into()),
        Ok(None) => Err("stream closed".into()),
        Ok(Some(Err(e))) => Err(e.to_string()),
        Ok(Some(Ok(bytes))) => postcard::from_bytes(&bytes).map_err(|e| e.to_string()),
    }
}

/// Keeps the transfer's route badge in step with the path iroh is using.
async fn watch_route(e: Engine, id: TransferId, conn: Connection) {
    let mut events = conn.path_events();
    let update = |route: Route| {
        let changed = e.with_tx(&id, |tx| std::mem::replace(&mut tx.t.route, route) != route).unwrap_or(false);
        if changed {
            e.emit_tx(&id, true);
        }
    };
    update(net::route_of(&conn));
    while let Some(ev) = events.next().await {
        if let iroh::endpoint::PathEvent::Selected { remote_addr, .. } = ev {
            update(net::route_for(&remote_addr));
        }
    }
}
