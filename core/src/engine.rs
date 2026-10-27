//! The engine: one iroh endpoint, the device list, transfers and requests, and the public API
//! that mirrors the app's `Engine` contract (`desktop/src/lib/engine/types.ts`).

use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
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
    beacon::{self, Beacon},
    codekeys::{KdfCost, fingerprint},
    files,
    history::{History, Record},
    net::{self, Network},
    pair::HostedCode,
    server::SettingError,
    session::{self, End},
    store::{self, Store, TrustedDevice},
    text,
    types::{self, *},
    wake,
    wire::{self, Answer, ClientHello, Ctrl, DeclineReason, DeviceInfo, FileMeta, FileState, Folders, HelloExt, Intent, Offer, ServerHello, TransferId},
};

/// A file to send: where it is, the name it's sent under, and for a file inside a folder being
/// sent, the folder it's in (`Photos/2024`, starting with the sent folder's own name).
#[derive(Debug, Clone)]
pub struct Outgoing {
    pub path: PathBuf,
    pub name: String,
    pub folder: Option<String>,
}

/// The longest text sent as text, in bytes of UTF-8 (64 KiB). Longer text goes as a file.
pub const TEXT_MAX: usize = 64 * 1024;
/// The most files one offer can hold; the receiver refuses more.
pub const FILES_MAX: usize = 100_000;
/// Incoming connections handled at once (transfers, pings, trust asks). Plenty for real use.
const CONNECTIONS_MAX: usize = 256;

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
    /// Look for nearby devices with mDNS. Tests and `--no-nearby` turn it off. The "Find nearby
    /// devices" setting turns both this and the beacon off.
    pub mdns: bool,
    /// Nhako Beam's own nearby announcements (`beacon.rs`). `None` turns them off.
    pub beacon: Option<beacon::Config>,
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
    /// The wake service for phones that are asleep (`wake.rs`). `None` turns waking off.
    pub wake_url: Option<url::Url>,
    /// How long to keep dialling after asking a device to wake up.
    pub wake_for: Duration,
    /// GitHub's list of releases, looked at once a day for a newer version. `None` in builds that
    /// get updates from a store, and in the command line tool.
    pub update_url: Option<url::Url>,
    /// How long a shared link works (Share via link).
    pub share_for: Duration,
    /// Tests: the address Share via link serves on, instead of this device's own on the network.
    #[doc(hidden)]
    pub test_share_ip: Option<std::net::IpAddr>,
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
            beacon: Some(beacon::Config::default()),
            kdf: KdfCost::REAL,
            reconnect_for: Duration::from_secs(60),
            code_lookup_for: Duration::from_secs(20),
            memory_lookup: None,
            test_rate_limit: None,
            wake_url: wake::default_url(),
            wake_for: wake::WAKE_FOR,
            update_url: crate::update::default_url(),
            share_for: crate::share::SHARE_FOR,
            test_share_ip: None,
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
    #[error("that text is too long to send as text")]
    TextTooLong,
    #[error("that device isn't nearby")]
    NotNearby,
    /// Share via link needs a local network (Wi-Fi, a cable or a hotspot).
    #[error("not on a local network")]
    NoLan,
}

pub(crate) type FrameRx = mpsc::Receiver<Result<Vec<u8>, wire::WireError>>;
pub(crate) type CmdTx = mpsc::UnboundedSender<Cmd>;
pub(crate) type CmdRx = mpsc::UnboundedReceiver<Cmd>;

/// How dialling a device turned out.
enum Reached {
    Yes((Connection, SendStream, FrameRx)),
    No(String),
    /// The user cancelled while it was being woken up.
    Cancelled,
}

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
    /// The folder it goes in, as it was when the request was accepted.
    pub dir: Arc<Path>,
    /// The name it's saved under, made safe. The real one gets " (1)" if that's taken.
    pub name: String,
    /// Where it's saved (or will be). Empty until its part file is made.
    pub final_path: PathBuf,
    /// Its part file, made when its data starts to arrive. A request of thousands of files
    /// doesn't fill the folder with empty files, or wait to make them all, before anything moves.
    pub part: Option<PathBuf>,
    pub size: u64,
    pub verified: bool,
}

impl DstFile {
    /// Picks a free name and makes the part file, unless that's done already. Blocking.
    pub fn claim(&mut self) -> std::io::Result<()> {
        if self.part.is_none() {
            // A folder inside one being received is made when its first file arrives.
            std::fs::create_dir_all(&self.dir)?;
            let (final_path, part) = files::reserve(&self.dir, &self.name)?;
            self.final_path = final_path;
            self.part = Some(part);
        }
        Ok(())
    }
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
    /// Milliseconds bytes spent moving, across pauses and reconnects (for History's average speed).
    moved_ms: u64,
    /// When the latest session started. A small file can finish before the first progress report,
    /// so its moving time is the session's.
    session_start: Option<u64>,
    /// Confirmed bytes per file, in file order, and what the app was last told.
    have: Vec<u64>,
    shown: Vec<u64>,
    /// Hash states along the file being moved, so a resume doesn't re-read it all.
    pub marks: session::Marks,
    /// Sender of text: the copy of it on disk, sent to devices without text. Deleted when it ends.
    pub temp: Option<PathBuf>,
    /// Receiver of folders: the folders made for them in the save folder, cleared out if cancelled.
    pub folders: Vec<PathBuf>,
}

impl Tx {
    /// Records confirmed progress and updates the 3 second speed average.
    pub fn set_have(&mut self, have: &[u64]) {
        let mut done = 0;
        for (i, f) in self.t.files.iter().enumerate() {
            let b = have.get(i).copied().unwrap_or(0).min(f.size);
            self.have[i] = b;
            if let Some(v) = self.t.file_bytes.get_mut(&f.id) {
                *v = b;
            }
            done += b;
        }
        self.note_done(done);
    }

    /// Records one file's confirmed bytes, without going through every file.
    pub fn set_file(&mut self, i: usize, bytes: u64) {
        let Some(f) = self.t.files.get(i) else { return };
        let b = bytes.min(f.size);
        let old = std::mem::replace(&mut self.have[i], b);
        if let Some(v) = self.t.file_bytes.get_mut(&f.id) {
            *v = b;
        }
        self.note_done(self.t.done_bytes - old + b);
    }

    /// Nothing new confirmed: keeps the speed average moving (down to 0 when stalled).
    pub fn tick(&mut self) {
        self.note_done(self.t.done_bytes);
    }

    fn note_done(&mut self, done: u64) {
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
        if let Some(since) = self.t.moving_since {
            self.moved_ms += now_ms().saturating_sub(since);
        }
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
    peer: EndpointId,
    /// From a trusted device, or one connected with a code.
    yours: bool,
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
    /// The status each transfer's History entry was last written with.
    recorded: HashMap<TransferId, TransferStatus>,
    /// When each device seen nearby (mDNS or the beacon) was last checked, so announcements
    /// repeating every few seconds don't each start a connection.
    nearby_checked: HashMap<EndpointId, Instant>,
    /// When each device that isn't yours asked to send, over the last minute.
    stranger_offers: HashMap<EndpointId, Vec<Instant>>,
    /// Trust asks from other devices, waiting for an answer here.
    pub(crate) trust_asks: HashMap<EndpointId, crate::trust::Asked>,
    /// Trust asks this device made, still going. Dropping one stops it.
    pub(crate) trust_asking: HashMap<EndpointId, oneshot::Sender<()>>,
    /// When each device asked to be trusted here, over the last minute.
    pub(crate) trust_ask_times: HashMap<EndpointId, Vec<Instant>>,
    /// The link being shared (Share via link). Dropping it stops its server.
    pub(crate) share: Option<crate::share::Hosted>,
    /// When the update check last tried, during this run.
    update_tried: Option<Instant>,
}

pub(crate) struct Inner {
    pub cfg: EngineConfig,
    /// The network actually in use.
    pub network: Network,
    pub endpoint: Endpoint,
    /// Addresses learned while pairing, so the first send doesn't wait on public lookups.
    pub learned: MemoryLookup,
    pub mdns: Option<iroh_mdns_address_lookup::MdnsAddressLookup>,
    pub beacon: Option<Arc<Beacon>>,
    /// Why nearby discovery is missing a part, for Copy diagnostics.
    pub nearby_problems: NearbyProblems,
    pub store: Mutex<Store>,
    history: Mutex<History>,
    pub st: Mutex<St>,
    events: broadcast::Sender<EngineEvent>,
    tasks: Mutex<Vec<JoinHandle<()>>>,
    /// The task running the shown code's endpoint, so `stop` can let it close cleanly.
    pub code_task: Mutex<Option<JoinHandle<()>>>,
    /// The app is on screen (see `set_foreground`).
    foreground: std::sync::atomic::AtomicBool,
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

/// How often the beacon says "I'm here" once settled.
const ANNOUNCE_EVERY: Duration = Duration::from_secs(30);
/// The gap between the asks in a burst.
const ASK_AGAIN: Duration = Duration::from_secs(2);
/// How often the beacon looks for new network adapters (Wi-Fi joined, cable plugged in).
const ADAPTERS_EVERY: Duration = Duration::from_secs(10);
/// A device whose beacon hasn't been heard for this long (three announcements missed) is checked again.
const QUIET_AFTER: Duration = Duration::from_secs(100);
/// The most devices the beacon keeps track of.
const MAX_HEARD: usize = 256;
/// A device seen nearby again within this time isn't checked again.
const RECHECK_NEARBY: Duration = Duration::from_secs(20);
/// Trusted devices are checked this often while the app is on screen.
const RECHECK_TRUSTED: Duration = Duration::from_secs(60);
/// Both of the above while the app is in the background (Android) or the tray (desktop): nothing
/// shows the device list then, and each check wakes the radio. Coming back checks them at once.
const RECHECK_IN_BACKGROUND: Duration = Duration::from_secs(300);
/// With Who can send set to Everyone nearby, a device that isn't yours can ask this many times a
/// minute, one request at a time...
const STRANGER_OFFERS_PER_MINUTE: usize = 3;
/// ...and requests from devices that aren't yours wait at most this many at once.
const STRANGER_REQUESTS_WAITING: usize = 5;

/// What's missing from nearby discovery, and why (for Copy diagnostics).
#[derive(Debug, Clone, Default)]
pub(crate) struct NearbyProblems {
    pub mdns: Option<String>,
    /// mDNS runs, but couldn't get UDP port 5353 for IPv4.
    pub mdns_v4: Option<String>,
    pub beacon: Option<String>,
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
        // Each needs both: the run allowing it (tests and --no-nearby turn them off) and the setting.
        let mdns = cfg.mdns && store.state.lan_discovery;
        let use_beacon = cfg.beacon.is_some() && store.state.lan_discovery;
        let bound = net::bind(net::EndpointOpts {
            secret: store.secret.clone(),
            alpns: vec![wire::ALPN.to_vec()],
            network: &network,
            relay_only: cfg.relay_only,
            mdns,
            memory: cfg.memory_lookup.as_ref(),
        })
        .await
        .map_err(|e| EngineError::Bind(e.to_string()))?;
        let mut nearby_problems = NearbyProblems::default();
        if let Some(e) = &bound.mdns_error {
            warn!("mDNS unavailable: {e}");
            nearby_problems.mdns = Some(e.clone());
        }
        if bound.mdns.is_some()
            && let Err(e) = beacon::probe_mdns_v4()
        {
            // The mDNS library carries on over IPv6 alone without saying so (Windows: the DNS Client
            // service holds the port). The beacon still finds devices over IPv4.
            warn!("mDNS can't use IPv4 here (UDP port 5353: {e}); the beacon looks for nearby devices instead");
            nearby_problems.mdns_v4 = Some(e);
        }
        let beacon = match (&cfg.beacon, use_beacon) {
            (Some(bc), true) => {
                let port = bound.endpoint.bound_sockets().into_iter().find(|a| a.is_ipv4()).map(|a| a.port());
                match port.map(|port| Beacon::open(bc.clone(), bound.endpoint.id(), port)) {
                    Some(Ok(b)) => Some(b),
                    Some(Err(e)) => {
                        warn!("the beacon couldn't start: {e}");
                        nearby_problems.beacon = Some(e.to_string());
                        None
                    }
                    None => {
                        nearby_problems.beacon = Some("no IPv4 socket to announce".into());
                        None
                    }
                }
            }
            _ => None,
        };

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
        // Blocked only when nothing that was meant to look for nearby devices could start.
        let discovery = if (mdns || use_beacon) && bound.mdns.is_none() && beacon.is_none() {
            DiscoveryState::Blocked
        } else {
            DiscoveryState::Searching
        };
        let history = History::open(&cfg.config_dir);
        // Copies of text from the last run: transfers don't outlive the app.
        let _ = std::fs::remove_dir_all(cfg.config_dir.join(OUTGOING_DIR));
        let inner = Inner {
            cfg,
            network,
            endpoint: bound.endpoint,
            learned,
            mdns: bound.mdns,
            beacon,
            nearby_problems,
            store: Mutex::new(store),
            history: Mutex::new(history),
            st: Mutex::new(St {
                devices,
                transfers: HashMap::new(),
                requests: HashMap::new(),
                code: None,
                discovery,
                free_space: None,
                server_problem: None,
                recorded: HashMap::new(),
                nearby_checked: HashMap::new(),
                stranger_offers: HashMap::new(),
                trust_asks: HashMap::new(),
                trust_asking: HashMap::new(),
                trust_ask_times: HashMap::new(),
                share: None,
                update_tried: None,
            }),
            events,
            tasks: Mutex::new(Vec::new()),
            code_task: Mutex::new(None),
            foreground: std::sync::atomic::AtomicBool::new(true),
        };
        let engine = Self { inner: Arc::new(inner) };
        info!(id = %engine.id().fmt_short(), "engine started");

        engine.spawn_bg(engine.clone().accept_loop());
        engine.spawn_bg(engine.clone().mdns_loop());
        engine.spawn_bg(engine.clone().beacon_loop());
        engine.spawn_bg(engine.clone().housekeeping_loop());
        engine.rescan();
        Ok(engine)
    }

    /// Closes the network and stops background work.
    pub async fn stop(&self) {
        self.cancel_code();
        self.stop_link();
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
            history: self.history(),
            trust_asks: self.trust_asks(),
            link_share: self.link_share(),
            update: self.update_found(),
        }
    }

    /// Finished transfers, newest first.
    pub fn history(&self) -> Vec<HistoryEntry> {
        self.history_lock().entries()
    }

    /// Forgets every History entry. Received files stay where they are.
    pub fn clear_history(&self) {
        self.history_lock().clear();
    }

    /// Where a received transfer's finished files are on disk (only the ones that arrived and checked out).
    pub fn saved_files(&self, transfer_id: &str) -> Vec<PathBuf> {
        let Some(id) = wire::parse_transfer_id(transfer_id) else { return vec![] };
        let live = match self.st().transfers.get(&id).map(|t| &t.side) {
            Some(TxSide::Recv(files)) => Some(
                files
                    .iter()
                    .filter(|f| f.verified && !f.final_path.as_os_str().is_empty())
                    .map(|f| f.final_path.clone())
                    .collect(),
            ),
            Some(TxSide::Send(_)) => Some(vec![]),
            None => None,
        };
        // From an earlier run: History remembers where they went.
        live.unwrap_or_else(|| self.history_lock().saved(transfer_id))
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
        if let Some(b) = &self.inner.beacon {
            b.ask();
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

    /// Sends files under the names given, for paths that don't carry a name (an open handle,
    /// `/proc/self/fd/N`, which must stay open until the transfer is finished). The apps use
    /// [`Engine::send_files`], which also takes folders.
    pub fn send_named(&self, device: &str, files: &[(PathBuf, String)]) -> Result<String, EngineError> {
        let out: Vec<Outgoing> = files
            .iter()
            .map(|(path, name)| Outgoing {
                path: path.clone(),
                name: name.clone(),
                folder: None,
            })
            .collect();
        self.send_files(device, &out)
    }

    /// Sends files, some of them inside folders (see [`Outgoing`]). Devices on a version without
    /// folders save every file straight in their save folder.
    pub fn send_files(&self, device: &str, files: &[Outgoing]) -> Result<String, EngineError> {
        self.start_send(device, files, None, None)
    }

    /// Sends text: a note, a link, a clipboard. It travels in the request itself, so the other
    /// device shows it before accepting; one on a version without text gets it as a `.txt` file.
    pub fn send_text(&self, device: &str, body: &str) -> Result<String, EngineError> {
        if body.trim().is_empty() {
            return Err(EngineError::NoFiles);
        }
        if body.len() > TEXT_MAX {
            return Err(EngineError::TextTooLong);
        }
        let dir = self.inner.cfg.config_dir.join(OUTGOING_DIR);
        let path = dir.join(format!("{}.txt", wire::transfer_id_str(&rand::random())));
        std::fs::create_dir_all(&dir)
            .and_then(|_| std::fs::write(&path, body))
            .map_err(|e| EngineError::File {
                path: path.clone(),
                reason: e.to_string(),
            })?;
        let name = if text::is_link(body) { "Link.txt" } else { "Text.txt" };
        let file = Outgoing {
            path: path.clone(),
            name: name.into(),
            folder: None,
        };
        self.start_send(device, &[file], Some(body.to_string()), Some(path.clone())).inspect_err(|_| {
            let _ = std::fs::remove_file(&path);
        })
    }

    fn start_send(&self, device: &str, files: &[Outgoing], text: Option<String>, temp: Option<PathBuf>) -> Result<String, EngineError> {
        let peer = parse_device_id(device).ok_or(EngineError::UnknownDevice)?;
        if peer == self.id() {
            return Err(EngineError::UnknownDevice);
        }
        if files.is_empty() {
            return Err(EngineError::NoFiles);
        }
        if files.len() > FILES_MAX {
            return Err(EngineError::File {
                path: files[0].path.clone(),
                reason: format!("more than {FILES_MAX} files"),
            });
        }
        let mut src = Vec::new();
        let mut items = Vec::new();
        for (i, f) in files.iter().enumerate() {
            let meta = files::open_to_send(&f.path).and_then(|file| file.metadata()).map_err(|e| EngineError::File {
                path: f.path.clone(),
                reason: e.to_string(),
            })?;
            if !meta.is_file() {
                return Err(EngineError::File {
                    path: f.path.clone(),
                    reason: "not a file".into(),
                });
            }
            let name = files::safe_name(&f.name);
            items.push(FileItem {
                id: format!("f{i}"),
                mime: files::mime_for(&name),
                name,
                size: meta.len(),
                folder: f.folder.as_deref().and_then(files::safe_folder).map(|p| p.join("/")),
            });
            src.push(SrcFile {
                path: f.path.clone(),
                size: meta.len(),
                modified: meta.modified().ok(),
            });
        }
        let id: TransferId = rand::random();
        let route = self.st().devices.get(&peer).and_then(|d| d.route).unwrap_or(Route::Relayed);
        let mut tx = new_tx(id, TransferDirection::Send, peer, items, route, TxSide::Send(src), TransferStatus::Connecting);
        tx.t.text = text;
        tx.temp = temp;
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
                    return (vec![], vec![]);
                }
                tx.epoch += 1;
                tx.t.status = TransferStatus::Cancelled;
                tx.t.cancelled_by = Some(Side::Me);
                tx.t.finished_at = Some(now_ms());
                tx.stop_moving();
                match &tx.side {
                    TxSide::Recv(files) => (files.iter().filter_map(|f| f.part.clone()).collect(), tx.folders.clone()),
                    TxSide::Send(_) => (vec![], vec![]),
                }
            });
            let (parts, folders) = parts.unwrap_or_default();
            clear_away(parts, folders);
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
                        wake: None,
                    });
                }
            } else {
                s.state.trusted.retain(|d| d.id != id);
            }
            if let Err(e) = s.save() {
                warn!("couldn't save trusted devices: {e}");
            }
        }
        if !trusted {
            // An untrusted device is only shown while it's in reach. One that's away goes now,
            // not at the next restart.
            let gone = {
                let mut st = self.st();
                match st.devices.get_mut(&id) {
                    Some(d) if d.reachable => {
                        d.paired = false;
                        false
                    }
                    _ => {
                        st.devices.remove(&id);
                        true
                    }
                }
            };
            if gone {
                self.emit(EngineEvent::DeviceLost { device_id: device_id(&id) });
                return;
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
        let name = text::device_name(name);
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

    /// Whether this device announces itself and looks for others on the local network.
    pub fn lan_discovery(&self) -> bool {
        self.store().state.lan_discovery
    }

    /// Saves the setting. Like the connection setting, it takes effect the next time the engine
    /// starts, so the app restarts the engine after saving.
    pub fn set_lan_discovery(&self, on: bool) -> Result<(), EngineError> {
        let mut s = self.store();
        s.state.lan_discovery = on;
        s.save()?;
        Ok(())
    }

    /// Who may send requests to this device.
    pub fn who_can_send(&self) -> WhoCanSend {
        self.store().state.who_can_send
    }

    /// Saves the setting. It applies to the next request, no restart needed.
    pub fn set_who_can_send(&self, who: WhoCanSend) -> Result<(), EngineError> {
        let mut s = self.store();
        s.state.who_can_send = who;
        s.save()?;
        Ok(())
    }

    /// Settings > About > Check for new versions. `None` when this build doesn't check (it gets
    /// updates from a store).
    pub fn update_check(&self) -> Option<bool> {
        self.inner.cfg.update_url.as_ref().map(|_| self.store().state.update_check)
    }

    /// Turns the daily check for a new version on or off. Turning it on looks soon.
    pub fn set_update_check(&self, on: bool) -> Result<(), EngineError> {
        {
            let mut s = self.store();
            s.state.update_check = on;
            if !on {
                s.state.update_found = None;
            }
            s.save()?;
        }
        if on {
            self.st().update_tried = None;
            self.maybe_check_update();
        } else {
            self.emit(EngineEvent::Update { update: None });
        }
        Ok(())
    }

    /// The newer version the last check found, if it's still newer than this one.
    pub fn update_found(&self) -> Option<UpdateInfo> {
        self.inner.cfg.update_url.as_ref()?;
        let s = self.store();
        s.state
            .update_found
            .clone()
            .filter(|u| s.state.update_check && crate::update::newer(&u.version, env!("CARGO_PKG_VERSION")))
    }

    /// Looks for a new version when it's on, a day has passed, and the last failed try was over
    /// an hour ago.
    fn maybe_check_update(&self) {
        let Some(url) = self.inner.cfg.update_url.clone() else { return };
        {
            let s = self.store();
            if !s.state.update_check || now_ms().saturating_sub(s.state.update_checked_at) < crate::update::CHECK_EVERY.as_millis() as u64 {
                return;
            }
        }
        {
            let mut st = self.st();
            if st.update_tried.is_some_and(|t| t.elapsed() < crate::update::RETRY_AFTER) {
                return;
            }
            st.update_tried = Some(Instant::now());
        }
        let e = self.clone();
        tokio::spawn(async move {
            match crate::update::check(&url, env!("CARGO_PKG_VERSION")).await {
                Ok(found) => {
                    {
                        let mut s = e.store();
                        s.state.update_checked_at = now_ms();
                        s.state.update_found = found.clone();
                        if let Err(err) = s.save() {
                            warn!("couldn't save the update check: {err}");
                        }
                    }
                    if let Some(u) = &found {
                        info!("Nhako Beam {} is out", u.version);
                    }
                    e.emit(EngineEvent::Update { update: e.update_found() });
                }
                Err(err) => debug!("couldn't check for a new version: {err}"),
            }
        });
    }

    /// This device's wake token (Android, from Firebase), or `None` when push isn't available.
    /// Trusted devices get it in the next hello.
    pub fn set_wake_token(&self, token: Option<String>) {
        let token = token.map(|t| t.trim().to_string()).filter(|t| wake::valid_token(t));
        let mut s = self.store();
        if s.state.wake_token != token {
            s.state.wake_token = token;
            if let Err(e) = s.save() {
                warn!("couldn't save the wake token: {e}");
            }
        }
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

    fn history_lock(&self) -> MutexGuard<'_, History> {
        self.inner.history.lock().expect("history lock")
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

    /// Emits the transfer. Without `force` it's a progress update (only bytes and speed changed):
    /// at most one every 100 ms, and only the files that moved since the app last heard.
    pub(crate) fn emit_tx(&self, id: &TransferId, force: bool) {
        let (event, ended) = {
            let mut st = self.st();
            let Some(tx) = st.transfers.get_mut(id) else { return };
            let now = Instant::now();
            if !force && tx.last_emit.is_some_and(|t| now.duration_since(t) < Duration::from_millis(100)) {
                return;
            }
            tx.last_emit = Some(now);
            let ended = matches!(
                tx.t.status,
                TransferStatus::Done | TransferStatus::Failed | TransferStatus::Cancelled | TransferStatus::Declined
            );
            let event = if force {
                tx.shown.clone_from(&tx.have);
                EngineEvent::Transfer { transfer: tx.t.clone() }
            } else {
                let mut file_bytes = BTreeMap::new();
                for (i, f) in tx.t.files.iter().enumerate() {
                    if tx.have[i] != tx.shown[i] {
                        tx.shown[i] = tx.have[i];
                        file_bytes.insert(f.id.clone(), tx.have[i]);
                    }
                }
                EngineEvent::TransferProgress {
                    id: tx.t.id.clone(),
                    done_bytes: tx.t.done_bytes,
                    speed: tx.t.speed,
                    moving_since: tx.t.moving_since,
                    file_bytes,
                }
            };
            (event, ended)
        };
        // History first, so whoever sees the transfer end can already find it there.
        if ended {
            self.record(id);
        }
        self.emit(event);
    }

    /// Writes the transfer's History entry, once per status it ends in.
    fn record(&self, id: &TransferId) {
        let peer_and_saved = {
            let mut st = self.st();
            let Some(tx) = st.transfers.get(id) else { return };
            let status = tx.t.status;
            let saved: Vec<PathBuf> = match &tx.side {
                TxSide::Recv(files) => files
                    .iter()
                    .filter(|f| f.verified && !f.final_path.as_os_str().is_empty())
                    .map(|f| f.final_path.clone())
                    .collect(),
                TxSide::Send(_) => vec![],
            };
            if status.is_final()
                && let Some(temp) = &tx.temp
            {
                let _ = std::fs::remove_file(temp);
            }
            let mut moved_ms = tx.moved_ms + tx.t.moving_since.filter(|_| tx.t.speed > 0.0).map(|s| now_ms().saturating_sub(s)).unwrap_or(0);
            if moved_ms == 0 && tx.t.done_bytes > 0 {
                moved_ms = tx.session_start.map(|s| now_ms().saturating_sub(s)).unwrap_or(0).max(1);
            }
            let snap = (tx.peer, tx.t.clone(), moved_ms, saved);
            if st.recorded.insert(*id, status) == Some(status) {
                return;
            }
            snap
        };
        let (peer, t, moved_ms, saved) = peer_and_saved;
        let (device_name, device_kind) = self
            .device_snapshot(&peer)
            .map(|d| (d.name, d.kind))
            .unwrap_or_else(|| ("Unknown device".into(), DeviceKind::Laptop));
        let entry = HistoryEntry {
            id: t.id,
            direction: t.direction,
            device_id: t.device_id,
            device_name,
            device_kind,
            files: t.files,
            total_bytes: t.total_bytes,
            done_bytes: t.done_bytes,
            route: t.route,
            status: t.status,
            error: t.error,
            cancelled_by: t.cancelled_by,
            started_at: t.started_at,
            finished_at: t.finished_at.unwrap_or_else(now_ms),
            average_speed: if moved_ms > 0 { t.done_bytes as f64 * 1000.0 / moved_ms as f64 } else { 0.0 },
            text: t.text,
        };
        self.history_lock().put(Record { entry: entry.clone(), saved });
        self.emit(EngineEvent::History { entry });
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
        let wakeable = !reachable && self.inner.cfg.wake_url.is_some() && trusted.as_ref().is_some_and(|t| t.wake.is_some());
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
            wakeable,
        })
    }

    pub(crate) fn emit_device(&self, id: &EndpointId) {
        if let Some(device) = self.device_snapshot(id) {
            self.emit(EngineEvent::Device { device });
        }
    }

    /// Records what we learned about a peer from a hello. Keeps trusted names fresh.
    pub(crate) fn note_peer(&self, id: EndpointId, mut info: DeviceInfo, route: Route) {
        info.name = text::device_name(&info.name);
        if info.name.is_empty() {
            info.name = "Unnamed device".into();
        }
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

    /// What this device adds to its hello for `peer`: the wake token, for trusted devices only.
    pub(crate) fn hello_ext_for(&self, peer: &EndpointId) -> HelloExt {
        let s = self.store();
        HelloExt {
            wake: s.trusted(peer).and(s.state.wake_token.clone()),
            ..HelloExt::default()
        }
    }

    /// Remembers (or forgets) a trusted device's wake token from its hello.
    pub(crate) fn note_wake(&self, peer: &EndpointId, ext: &HelloExt) {
        let token = ext.wake.clone().filter(|t| wake::valid_token(t));
        let changed = {
            let mut s = self.store();
            let Some(t) = s.trusted_mut(peer) else { return };
            if t.wake == token {
                return;
            }
            t.wake = token;
            if let Err(e) = s.save() {
                warn!("couldn't save a wake token: {e}");
            }
            true
        };
        if changed {
            self.emit_device(peer);
        }
    }

    fn wake_token_of(&self, peer: &EndpointId) -> Option<String> {
        self.store().trusted(peer).and_then(|t| t.wake.clone())
    }

    /// A trusted device, or one connected with a code while the app has been running.
    fn is_yours(&self, id: &EndpointId) -> bool {
        self.store().trusted(id).is_some() || self.st().devices.get(id).is_some_and(|d| d.paired)
    }

    /// Whether a request from `peer` may be shown: Who can send to me, then for devices that
    /// aren't yours, one request at a time and a few a minute, so a busy network can't flood you.
    /// Whether an offer from `peer` may become a request. Called with the state locked, in the same
    /// hold that adds the request, so offers arriving at once can't all slip under the limits.
    fn admit_offer(st: &mut St, peer: &EndpointId, yours: bool, who: WhoCanSend) -> Result<(), DeclineReason> {
        if yours {
            return Ok(());
        }
        if who == WhoCanSend::YourDevices {
            return Err(DeclineReason::NotAllowed);
        }
        // Every ask counts, refused ones too.
        let now = Instant::now();
        st.stranger_offers.retain(|_, times| {
            times.retain(|t| now.duration_since(*t) < Duration::from_secs(60));
            !times.is_empty()
        });
        let times = st.stranger_offers.entry(*peer).or_default();
        times.push(now);
        let too_often = times.len() > STRANGER_OFFERS_PER_MINUTE;
        let waiting = st.requests.values().filter(|p| !p.yours).count();
        if too_often || waiting >= STRANGER_REQUESTS_WAITING || st.requests.values().any(|p| p.peer == *peer) {
            return Err(DeclineReason::Unknown);
        }
        Ok(())
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
        let ext = self.hello_ext_for(&id);
        match net::ping(&self.inner.endpoint, id, me, &ext, Duration::from_secs(8)).await {
            Ok((info, theirs, route)) => {
                self.note_wake(&id, &theirs);
                self.note_peer(id, info, route);
            }
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
        let open = Arc::new(tokio::sync::Semaphore::new(CONNECTIONS_MAX));
        while let Some(incoming) = self.inner.endpoint.accept().await {
            // Past the cap, new connections are refused until some end: a flood of them can't
            // take the memory or the tasks that real transfers need.
            let Ok(permit) = open.clone().try_acquire_owned() else {
                debug!("too many connections at once, refusing one");
                incoming.refuse();
                continue;
            };
            let e = self.clone();
            tokio::spawn(async move {
                let _permit = permit;
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
                    self.saw_nearby(endpoint_info.endpoint_id);
                }
                iroh_mdns_address_lookup::DiscoveryEvent::Expired { endpoint_id } => self.lost_nearby(endpoint_id),
                _ => {}
            }
        }
    }

    /// Listens for other devices' beacons, and announces this one: right away and twice more
    /// soon after (when starting, rescanning or changing networks), then every [`ANNOUNCE_EVERY`].
    async fn beacon_loop(self) {
        let Some(b) = self.inner.beacon.clone() else { return };
        // Devices heard from, so the ones that went quiet can be checked again.
        let mut heard: HashMap<EndpointId, Instant> = HashMap::new();
        let mut replied: HashMap<std::net::IpAddr, Instant> = HashMap::new();
        let mut asks_left = 3u8;
        let mut announce_at = tokio::time::Instant::now();
        let mut adapters_at = tokio::time::Instant::now() + ADAPTERS_EVERY;
        loop {
            tokio::select! {
                got = b.recv() => {
                    let (p, from) = match got {
                        Ok(r) => r,
                        Err(e) => {
                            warn!("the beacon stopped listening: {e}");
                            return;
                        }
                    };
                    let now = Instant::now();
                    if p.ask && replied.get(&from.ip()).is_none_or(|t| now - *t > Duration::from_secs(2)) {
                        replied.retain(|_, t| now - *t < Duration::from_secs(10));
                        replied.insert(from.ip(), now);
                        b.reply(from).await;
                    }
                    // A cap, so a flood of made-up devices can't grow this without end.
                    if !heard.contains_key(&p.id) && heard.len() >= MAX_HEARD {
                        continue;
                    }
                    heard.insert(p.id, now);
                    let addr = std::net::SocketAddr::new(from.ip(), p.port);
                    self.inner.learned.add_endpoint_info(iroh::EndpointAddr::from_parts(p.id, [iroh::TransportAddr::Ip(addr)]));
                    self.saw_nearby(p.id);
                }
                _ = b.asked() => {
                    asks_left = 3;
                    announce_at = tokio::time::Instant::now();
                }
                _ = tokio::time::sleep_until(adapters_at) => {
                    adapters_at = tokio::time::Instant::now() + ADAPTERS_EVERY;
                    if b.refresh_adapters() {
                        asks_left = 3;
                        announce_at = tokio::time::Instant::now();
                    }
                }
                _ = tokio::time::sleep_until(announce_at) => {
                    b.announce(asks_left > 0).await;
                    asks_left = asks_left.saturating_sub(1);
                    announce_at = tokio::time::Instant::now() + if asks_left > 0 { ASK_AGAIN } else { ANNOUNCE_EVERY };
                    let now = Instant::now();
                    let quiet: Vec<EndpointId> = heard.iter().filter(|(_, t)| now - **t > QUIET_AFTER).map(|(id, _)| *id).collect();
                    for id in quiet {
                        heard.remove(&id);
                        self.lost_nearby(id);
                    }
                }
            }
        }
    }

    /// A device announced itself nearby (mDNS or the beacon). Checks it, unless it was just checked.
    fn saw_nearby(&self, id: EndpointId) {
        if id == self.id() {
            return;
        }
        let every = if self.is_foreground() { RECHECK_NEARBY } else { RECHECK_IN_BACKGROUND };
        let check = {
            let mut st = self.st();
            st.devices.entry(id).or_default().nearby = true;
            let now = Instant::now();
            if st.nearby_checked.get(&id).is_some_and(|t| now - *t < every) {
                false
            } else {
                if st.nearby_checked.len() > 64 {
                    st.nearby_checked.retain(|_, t| now - *t < every);
                }
                st.nearby_checked.insert(id, now);
                true
            }
        };
        if check {
            tokio::spawn(self.clone().check_device(id));
        }
    }

    /// Checks every trusted device, except those a nearby announcement got checked within `fresh`.
    fn check_trusted(&self, fresh: Duration) {
        let trusted: Vec<EndpointId> = self.store().state.trusted.iter().map(|d| d.id).collect();
        let recent: HashSet<EndpointId> = {
            let st = self.st();
            st.nearby_checked.iter().filter(|(_, t)| t.elapsed() < fresh).map(|(id, _)| *id).collect()
        };
        for id in trusted.into_iter().filter(|id| !recent.contains(id)) {
            tokio::spawn(self.clone().check_device(id));
        }
    }

    pub(crate) fn is_foreground(&self) -> bool {
        self.inner.foreground.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// The app is on screen (`true`) or in the background or tray. In the background, devices are
    /// checked every 5 minutes instead of every 20 to 60 seconds. Coming back checks them at once,
    /// so the list is fresh when you look.
    pub fn set_foreground(&self, on: bool) {
        let was = self.inner.foreground.swap(on, std::sync::atomic::Ordering::Relaxed);
        if on && !was {
            debug!("back on screen: checking devices");
            self.check_trusted(Duration::from_secs(10));
        }
    }

    /// A device stopped announcing itself. Checks whether it is still reachable some other way.
    fn lost_nearby(&self, id: EndpointId) {
        if let Some(d) = self.st().devices.get_mut(&id) {
            d.nearby = false;
        }
        tokio::spawn(self.clone().check_device(id));
    }

    /// Free space every 15 s (every minute in the background), trusted devices every 60 s (every
    /// 5 minutes in the background), skipping any just checked because it was seen nearby.
    async fn housekeeping_loop(self) {
        let mut tick = 0u64;
        let mut trusted_checked = Instant::now();
        loop {
            let foreground = self.is_foreground();
            let dir = self.save_dir();
            let free = if foreground || tick.is_multiple_of(4) {
                tokio::task::spawn_blocking(move || files::free_space(&dir)).await.ok().flatten()
            } else {
                None
            };
            if let Some(bytes) = free {
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
            self.maybe_check_update();
            let every = if foreground { RECHECK_TRUSTED } else { RECHECK_IN_BACKGROUND };
            if trusted_checked.elapsed() >= every {
                trusted_checked = Instant::now();
                self.check_trusted(every);
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
        let (hello, ext): (ClientHello, HelloExt) = next_hello(&mut frames, Duration::from_secs(10)).await?;
        if hello.version != wire::VERSION {
            conn.close(wire::close::PROTOCOL, b"version");
            return Err(format!("unsupported version {}", hello.version));
        }
        if !self.peer_allowed(&peer, &conn) {
            conn.close(wire::close::NOT_ALLOWED, b"not allowed");
            return Err("not allowed".into());
        }
        self.note_wake(&peer, &ext);
        self.note_peer(peer, hello.info.clone(), net::route_of(&conn));
        wire::write_hello(
            &mut send,
            &ServerHello {
                version: wire::VERSION,
                info: self.me_info(),
            },
            &self.hello_ext_for(&peer),
        )
        .await
        .map_err(|e| e.to_string())?;
        match hello.intent {
            Intent::Ping if ext.trust.is_some() => self.answer_trust_ask(conn, send, frames, peer, ext.trust.unwrap_or_default()).await,
            Intent::Ping => {
                let _ = send.finish();
                let _ = tokio::time::timeout(Duration::from_secs(5), conn.closed()).await;
                Ok(())
            }
            Intent::Offer(offer) => self.handle_offer(conn, send, frames, peer, offer, ext).await,
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

    /// An offer that can't be taken (malformed, or an id already in use): declined and closed.
    async fn bad_offer(conn: &Connection, send: &mut SendStream) -> Result<(), String> {
        let _ = wire::write_frame(send, &Answer::Decline(DeclineReason::Unknown)).await;
        conn.close(wire::close::DECLINED, b"bad offer");
        Err("bad offer".into())
    }

    async fn handle_offer(
        &self,
        conn: Connection,
        mut send: SendStream,
        mut frames: FrameRx,
        peer: EndpointId,
        offer: Offer,
        ext: HelloExt,
    ) -> Result<(), String> {
        let valid = !offer.files.is_empty() && offer.files.len() <= FILES_MAX && offer.files.iter().try_fold(0u64, |a, f| a.checked_add(f.size)).is_some();
        if !valid {
            return Self::bad_offer(&conn, &mut send).await;
        }
        let yours = self.is_yours(&peer);
        let who = self.who_can_send();
        // Text comes in the request itself. Its one file is the same text, for older builds; one
        // that doesn't match it isn't taken as text.
        let body = ext
            .text
            .filter(|t| offer.files.len() == 1 && t.len() as u64 == offer.files[0].size && t.len() <= TEXT_MAX)
            .map(|t| text::clean_text(&t));
        let folders = ext.folders.filter(|f| f.of.len() == offer.files.len() && body.is_none());
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
                    folder: folders.as_ref().and_then(|d| d.of(i)).and_then(files::safe_folder).map(|p| p.join("/")),
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
            text: body.clone(),
        });
        let admitted = {
            let mut st = self.st();
            if st.transfers.contains_key(&offer.id) || st.requests.contains_key(&offer.id) {
                None
            } else {
                let admitted = Self::admit_offer(&mut st, &peer, yours, who);
                if admitted.is_ok() {
                    st.requests.insert(
                        offer.id,
                        Pending {
                            peer,
                            yours,
                            decide: Some(decide_tx),
                            request: request.clone(),
                        },
                    );
                }
                Some(admitted)
            }
        };
        match admitted {
            None => return Self::bad_offer(&conn, &mut send).await,
            Some(Err(reason)) => {
                info!(peer = %peer.fmt_short(), ?reason, "refused a request");
                let _ = wire::write_frame(&mut send, &Answer::Decline(reason)).await;
                let _ = send.finish();
                let _ = tokio::time::timeout(Duration::from_secs(3), conn.closed()).await;
                return Ok(());
            }
            Some(Ok(())) => {}
        }
        match request {
            None => {
                let _ = self.accept(&request_id, false, true);
            }
            Some(request) => self.emit(EngineEvent::Incoming { request }),
        }

        let decision = tokio::select! {
            d = decide_rx => d.unwrap_or(Decision::Decline),
            _ = frames.recv() => {
                // The sender withdrew, the connection dropped, or the sender said something out of
                // turn while we were deciding: the request is over either way, so its card goes too.
                if self.st().requests.remove(&offer.id).is_some_and(|p| p.request.is_some()) {
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
        let route = net::route_of(&conn);

        if let Some(body) = body {
            return self.take_text(conn, send, frames, peer, offer.id, items, route, body).await;
        }

        // Each folder sent gets a new folder of its own here ("Photos", or "Photos (1)" when
        // that's taken). Each file gets its name and part file when its data starts
        // (DstFile::claim); the first one is made now, so a folder that can't be written to is
        // reported straight away.
        let save = self.save_dir();
        let wanted: Vec<(String, u64, Option<String>)> = items.iter().map(|f| (f.name.clone(), f.size, f.folder.clone())).collect();
        let prepared = tokio::task::spawn_blocking(move || prepare_dst(&save, wanted))
            .await
            .unwrap_or_else(|e| Err((std::io::Error::other(e), vec![])));
        let (dst, made, created) = match prepared {
            Ok((dst, made)) => (dst, made, Ok(())),
            Err((err, made)) => (vec![], made, Err(err)),
        };
        let mut tx = new_tx(
            offer.id,
            TransferDirection::Receive,
            peer,
            items,
            route,
            TxSide::Recv(dst.clone()),
            TransferStatus::Transferring,
        );
        tx.folders = made.clone();
        if let Err(err) = created {
            warn!(%err, "can't save incoming files");
            clear_away(vec![], made);
            tx.side = TxSide::Recv(vec![]);
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

    /// Accepted text: it came with the request, so nothing is written to disk. The answer says
    /// the one file is already there, and the sender finishes.
    #[allow(clippy::too_many_arguments)]
    async fn take_text(
        &self,
        conn: Connection,
        mut send: SendStream,
        mut frames: FrameRx,
        peer: EndpointId,
        id: TransferId,
        items: Vec<FileItem>,
        route: Route,
        body: String,
    ) -> Result<(), String> {
        let dst = items
            .iter()
            .map(|f| DstFile {
                dir: Arc::from(Path::new("")),
                name: f.name.clone(),
                final_path: PathBuf::new(),
                part: None,
                size: f.size,
                verified: true,
            })
            .collect();
        let mut tx = new_tx(id, TransferDirection::Receive, peer, items, route, TxSide::Recv(dst), TransferStatus::Done);
        let all: Vec<u64> = tx.t.files.iter().map(|f| f.size).collect();
        tx.set_have(&all);
        tx.t.speed = 0.0;
        tx.t.moving_since = None;
        tx.t.text = Some(body);
        tx.t.finished_at = Some(now_ms());
        self.st().transfers.insert(id, tx);
        self.emit_tx(&id, true);
        wire::write_frame(
            &mut send,
            &Answer::Accept {
                files: vec![FileState::Verified],
            },
        )
        .await
        .map_err(|e| e.to_string())?;
        // The sender answers with Done; either way it's finished here.
        let _ = tokio::time::timeout(Duration::from_secs(5), frames.recv()).await;
        conn.close(wire::close::DONE, b"");
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
                    let len = f.part.as_ref().and_then(|p| std::fs::metadata(p).ok()).map_or(0, |m| m.len());
                    FileState::Have(len.min(f.size))
                }
            })
            .collect()
    }

    // ---------- outgoing ----------

    async fn drive_send(self, id: TransferId) {
        let Some((peer, metas, extra)) = self.with_tx(&id, |tx| {
            (
                tx.peer,
                tx.t.files
                    .iter()
                    .map(|f| FileMeta {
                        name: f.name.clone(),
                        size: f.size,
                    })
                    .collect::<Vec<_>>(),
                HelloExt {
                    folders: folders_of(&tx.t.files),
                    text: tx.t.text.clone(),
                    ..HelloExt::default()
                },
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
        let opened = self
            .open_or_wake(&id, peer, Intent::Offer(Offer { id, files: metas }), &extra, &mut cmd_rx)
            .await;
        let (conn, mut send, mut frames) = match opened {
            Reached::Yes(x) => x,
            Reached::Cancelled => return self.finish(&id, epoch, End::Cancelled(Side::Me)),
            Reached::No(err) => {
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
                    tx.t.error = match reason {
                        DeclineReason::NoSpace => Some(TransferError::DiskFullPeer),
                        DeclineReason::NotAllowed => Some(TransferError::NotAllowed),
                        DeclineReason::User | DeclineReason::Unknown => None,
                    };
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

    /// Dials a peer. One that's asleep and can be woken (a phone Android paused) is asked to wake
    /// up, then dialled again until it answers or `wake_for` passes; the transfer shows `waking`
    /// meanwhile. Cancelling stops the wait.
    async fn open_or_wake(&self, id: &TransferId, peer: EndpointId, intent: Intent, extra: &HelloExt, cmds: &mut CmdRx) -> Reached {
        let url = self.inner.cfg.wake_url.clone();
        let token = url.as_ref().and_then(|_| self.wake_token_of(&peer));
        let (Some(url), Some(token)) = (url, token) else {
            return self.open(peer, intent, extra).await.map_or_else(Reached::No, Reached::Yes);
        };
        // Known to be away: wake it straight away rather than waiting out a first dial.
        let asleep = !self.st().devices.get(&peer).is_some_and(|d| d.reachable);
        if !asleep {
            match self.open(peer, intent.clone(), extra).await {
                Ok(x) => return Reached::Yes(x),
                Err(err) => debug!(%err, "not reached, trying to wake it"),
            }
        }
        self.set_waking(id, true);
        match wake::send(&url, &token).await {
            Ok(()) => info!(peer = %peer.fmt_short(), "asked it to wake up"),
            Err(wake::WakeError::Gone) => {
                // The app was uninstalled or reset there: stop offering to wake it.
                self.note_wake(&peer, &HelloExt::default());
                self.set_waking(id, false);
                return Reached::No("its wake token is no longer valid".into());
            }
            Err(err) => {
                warn!(%err, "couldn't ask it to wake up");
                self.set_waking(id, false);
                return Reached::No(err.to_string());
            }
        }
        let deadline = tokio::time::Instant::now() + self.inner.cfg.wake_for;
        let result = loop {
            let attempt = async {
                let left = deadline.saturating_duration_since(tokio::time::Instant::now()).max(Duration::from_secs(1));
                tokio::time::timeout(left, self.open(peer, intent.clone(), extra))
                    .await
                    .unwrap_or_else(|_| Err("timed out".into()))
            };
            tokio::select! {
                r = attempt => match r {
                    Ok(x) => break Reached::Yes(x),
                    Err(err) if tokio::time::Instant::now() >= deadline => break Reached::No(err),
                    Err(_) => {}
                },
                cmd = cmds.recv() => if matches!(cmd, Some(Cmd::Cancel) | None) {
                    break Reached::Cancelled;
                },
            }
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(2)) => {}
                cmd = cmds.recv() => if matches!(cmd, Some(Cmd::Cancel) | None) {
                    break Reached::Cancelled;
                },
            }
        };
        self.set_waking(id, false);
        result
    }

    fn set_waking(&self, id: &TransferId, on: bool) {
        if self.with_tx(id, |tx| std::mem::replace(&mut tx.t.waking, on) != on).unwrap_or(false) {
            self.emit_tx(id, true);
        }
    }

    /// Dials a peer with an intent and reads its hello. Returns the connection and control stream.
    /// `extra` is what goes in the hello besides the wake token (an offer's folders or text, a
    /// trust ask).
    pub(crate) async fn open(&self, peer: EndpointId, intent: Intent, extra: &HelloExt) -> Result<(Connection, SendStream, FrameRx), String> {
        let conn = self.connect(peer, Duration::from_secs(20)).await?;
        let (mut send, recv) = conn.open_bi().await.map_err(|e| e.to_string())?;
        let mut frames = spawn_frame_reader(recv);
        wire::write_hello(
            &mut send,
            &ClientHello {
                version: wire::VERSION,
                info: self.me_info(),
                intent,
            },
            &HelloExt {
                wake: self.hello_ext_for(&peer).wake,
                ..extra.clone()
            },
        )
        .await
        .map_err(|e| e.to_string())?;
        let (hello, ext): (ServerHello, HelloExt) = next_hello(&mut frames, Duration::from_secs(10)).await?;
        self.note_wake(&peer, &ext);
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
            if let Ok((conn, mut send, mut frames)) = self.open(peer, Intent::Resume { id }, &HelloExt::default()).await {
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
            for part in files.iter().filter(|f| !f.verified).filter_map(|f| f.part.as_ref()).filter(|p| !p.exists()) {
                let _ = files::create_new(part);
            }
        }
        match self.open(peer, Intent::Pull { id, files }, &HelloExt::default()).await {
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
            tx.session_start = Some(now_ms());
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
        let (parts_to_delete, folders_to_clear) = self
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
                        (vec![], vec![])
                    }
                    End::Cancelled(by) => {
                        tx.t.status = TransferStatus::Cancelled;
                        tx.t.cancelled_by = Some(by);
                        tx.t.finished_at = Some(now_ms());
                        match &tx.side {
                            TxSide::Recv(files) => (
                                files.iter().filter(|f| !f.verified).filter_map(|f| f.part.clone()).collect(),
                                tx.folders.clone(),
                            ),
                            TxSide::Send(_) => (vec![], vec![]),
                        }
                    }
                    End::Failed(err) => {
                        tx.t.status = TransferStatus::Failed;
                        tx.t.error = Some(err);
                        (vec![], vec![])
                    }
                    End::Lost => {
                        tx.t.status = TransferStatus::Reconnecting;
                        tx.t.paused_by = None;
                        (vec![], vec![])
                    }
                }
            })
            .unwrap_or_default();
        clear_away(parts_to_delete, folders_to_clear);
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
    let count = files.len();
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
            waking: false,
            text: None,
        },
        peer,
        side,
        cmd: None,
        conn: None,
        epoch: 0,
        last_emit: None,
        samples: VecDeque::new(),
        moved_ms: 0,
        session_start: None,
        have: vec![0; count],
        shown: vec![0; count],
        marks: Default::default(),
        temp: None,
        folders: vec![],
    }
}

/// The folders of files in an offer, for the hello (`None` when no file is in one).
fn folders_of(files: &[FileItem]) -> Option<Folders> {
    if files.iter().all(|f| f.folder.is_none()) {
        return None;
    }
    let mut index: HashMap<&str, u32> = HashMap::new();
    let mut out = Folders::default();
    for f in files {
        let k = match f.folder.as_deref() {
            None => 0,
            Some(d) => *index.entry(d).or_insert_with(|| {
                out.dirs.push(d.to_string());
                out.dirs.len() as u32
            }),
        };
        out.of.push(k);
    }
    Some(out)
}

/// Where copies of text being sent wait, inside the config folder (cleared at start).
const OUTGOING_DIR: &str = "outgoing";

/// Removes a cancelled transfer's part files, then the folders made for it if nothing's left in them.
fn clear_away(parts: Vec<PathBuf>, folders: Vec<PathBuf>) {
    for p in parts {
        let _ = std::fs::remove_file(p);
    }
    for f in folders {
        files::prune_empty(&f);
    }
}

/// Where each incoming file goes: the save folder, or for a file in a folder that was sent, a new
/// folder made for that folder here (see `files::reserve_folder`) and the folders inside it. Makes
/// those top folders and the first file's part file. Blocking. On failure, also returns the
/// folders it made, to clear away.
#[allow(clippy::type_complexity)]
fn prepare_dst(save: &Path, wanted: Vec<(String, u64, Option<String>)>) -> Result<(Vec<DstFile>, Vec<PathBuf>), (std::io::Error, Vec<PathBuf>)> {
    let mut made: Vec<PathBuf> = vec![];
    let mut tops: HashMap<String, PathBuf> = HashMap::new();
    let mut dirs: HashMap<String, Arc<Path>> = HashMap::new();
    let root: Arc<Path> = Arc::from(save);
    if let Err(e) = std::fs::create_dir_all(save) {
        return Err((e, made));
    }
    let mut dst = Vec::with_capacity(wanted.len());
    for (name, size, folder) in wanted {
        let dir = match folder {
            None => root.clone(),
            Some(folder) => match dirs.get(&folder) {
                Some(d) => d.clone(),
                None => {
                    let (top, rest) = folder.split_once('/').unwrap_or((&folder, ""));
                    let top_path = match tops.get(top) {
                        Some(p) => p.clone(),
                        None => match files::reserve_folder(save, top) {
                            Ok(p) => {
                                made.push(p.clone());
                                tops.insert(top.to_string(), p.clone());
                                p
                            }
                            Err(e) => return Err((e, made)),
                        },
                    };
                    let d: Arc<Path> = Arc::from(rest.split('/').filter(|p| !p.is_empty()).fold(top_path, |p, part| p.join(part)));
                    dirs.insert(folder.clone(), d.clone());
                    d
                }
            },
        };
        dst.push(DstFile {
            dir,
            name,
            final_path: PathBuf::new(),
            part: None,
            size,
            verified: false,
        });
    }
    if let Some(first) = dst.first_mut()
        && let Err(e) = first.claim()
    {
        return Err((e, made));
    }
    Ok((dst, made))
}

/// Frames read ahead of the one being handled. Each can be up to 4 MiB, so this caps what a peer
/// can make us hold per stream (QUIC's flow control holds back the rest).
const FRAMES_AHEAD: usize = 4;

/// Reads frames off a stream on its own task, so callers can `select!` on them safely.
pub(crate) fn spawn_frame_reader(mut recv: RecvStream) -> FrameRx {
    let (tx, rx) = mpsc::channel(FRAMES_AHEAD);
    tokio::spawn(async move {
        loop {
            let frame = wire::read_raw(&mut recv).await;
            let end = frame.is_err();
            if tx.send(frame).await.is_err() || end {
                return;
            }
        }
    });
    rx
}

/// The next frame as a hello with its extension.
pub(crate) async fn next_hello<T: serde::de::DeserializeOwned>(frames: &mut FrameRx, timeout: Duration) -> Result<(T, HelloExt), String> {
    match tokio::time::timeout(timeout, frames.recv()).await {
        Err(_) => Err("timed out".into()),
        Ok(None) => Err("stream closed".into()),
        Ok(Some(Err(e))) => Err(e.to_string()),
        Ok(Some(Ok(bytes))) => wire::decode_hello(&bytes).map_err(|e| e.to_string()),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codekeys::KdfCost;

    async fn engine(mem: &MemoryLookup, name: &str) -> Engine {
        let dir = std::env::temp_dir().join(format!("nhako-unit-{name}-{}", rand::random::<u32>()));
        let mut cfg = EngineConfig::new(dir.join("config"));
        cfg.update_url = None;
        cfg.save_dir = Some(dir.join("save"));
        cfg.network = Some(Network::LocalOnly);
        cfg.mdns = false;
        cfg.beacon = None;
        cfg.kdf = KdfCost::TEST;
        cfg.memory_lookup = Some(mem.clone());
        cfg.wake_url = None;
        let e = Engine::start(cfg).await.unwrap();
        e.set_who_can_send(WhoCanSend::EveryoneNearby).unwrap();
        e
    }

    fn offer(n: u8) -> Intent {
        Intent::Offer(Offer {
            id: [n; 16],
            files: vec![FileMeta {
                name: "note.txt".into(),
                size: 3,
            }],
        })
    }

    async fn wait_for(events: &mut broadcast::Receiver<EngineEvent>, mut f: impl FnMut(&EngineEvent) -> bool) {
        tokio::time::timeout(Duration::from_secs(15), async {
            loop {
                if let Ok(e) = events.recv().await
                    && f(&e)
                {
                    return;
                }
            }
        })
        .await
        .expect("the event never came");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_request_whose_sender_speaks_out_of_turn_loses_its_card() {
        let mem = MemoryLookup::new();
        let (a, b) = (engine(&mem, "a").await, engine(&mem, "b").await);
        let mut events = b.subscribe();
        let (_conn, mut send, _frames) = a.open(b.id(), offer(1), &HelloExt::default()).await.unwrap();
        wait_for(&mut events, |e| matches!(e, EngineEvent::Incoming { .. })).await;
        assert_eq!(b.requests().len(), 1);
        // Not Withdraw, and not a closed connection: something a sender shouldn't send yet.
        wire::write_frame(&mut send, &Ctrl::Done).await.unwrap();
        wait_for(&mut events, |e| matches!(e, EngineEvent::IncomingWithdrawn { .. })).await;
        assert!(b.requests().is_empty());
        a.stop().await;
        b.stop().await;
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn offers_arriving_at_once_still_get_one_card_per_device() {
        let mem = MemoryLookup::new();
        let (a, b) = (engine(&mem, "a").await, engine(&mem, "b").await);
        let mut events = b.subscribe();
        let none = HelloExt::default();
        let (x, y, z) = tokio::join!(
            a.open(b.id(), offer(1), &none),
            a.open(b.id(), offer(2), &none),
            a.open(b.id(), offer(3), &none)
        );
        let opened = [x, y, z];
        let mut held = Vec::new();
        for o in opened {
            let (conn, send, mut frames) = o.unwrap();
            // A refused offer is answered straight away; one shown as a card waits for you.
            let answer: Result<Answer, _> = next_frame(&mut frames, Duration::from_millis(1500)).await;
            held.push((conn, send, frames, answer.is_ok()));
        }
        wait_for(&mut events, |e| matches!(e, EngineEvent::Incoming { .. })).await;
        assert_eq!(b.requests().len(), 1, "one waiting request per device that isn't yours");
        assert_eq!(held.iter().filter(|h| h.3).count(), 2, "the other two were turned away");
        a.stop().await;
        b.stop().await;
    }
}
