//! `nhako`: send and receive files with Nhako Beam from a terminal.

use std::{path::PathBuf, process::ExitCode, time::Duration};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use indicatif::{ProgressBar, ProgressStyle};
use nhako_core::{
    CheckResult, Connection, Device, Engine, EngineConfig, EngineEvent, Network, Outgoing, OwnNetwork, OwnServer, PairVia, RedeemFailure, RedeemResult, Route,
    ServerCheck, Transfer, TransferDirection, TransferError, TransferStatus, WhoCanSend, check_server, store, words,
};
use tokio::sync::broadcast;

/// Send files straight to your devices. No cloud, no size limits, no account.
#[derive(Parser)]
#[command(name = "nhako", version)]
struct Cli {
    /// Config folder (device key, name, trusted devices).
    #[arg(long, global = true, value_name = "DIR")]
    config_dir: Option<PathBuf>,
    /// Save received files here for this run.
    #[arg(long, global = true, value_name = "DIR")]
    save_dir: Option<PathBuf>,
    /// Same network only: no relay and no internet lookups.
    #[arg(long, global = true, conflicts_with_all = ["relay", "relay_only"])]
    local_only: bool,
    /// Use your own relay for this run, for example http://192.168.1.20:3340. To keep it, use `nhako server set`.
    #[arg(long, global = true, value_name = "URL")]
    relay: Option<String>,
    /// Token for --relay. Without it, NHAKO_RELAY_TOKEN is used, which keeps it out of your shell history.
    #[arg(long, global = true, value_name = "TOKEN", requires = "relay")]
    relay_token: Option<String>,
    /// Your own address lookup server for --relay, for example http://192.168.1.20:3341.
    #[arg(long, global = true, value_name = "URL", requires = "relay")]
    lookup: Option<String>,
    /// Only use the relay, never a direct path. For testing a relay.
    #[arg(long, global = true)]
    relay_only: bool,
    /// Don't look for devices on the same network (mDNS and the beacon).
    #[arg(long, global = true, alias = "no-mdns")]
    no_nearby: bool,
    /// Print engine events as JSON lines (for scripts).
    #[arg(long, global = true)]
    json: bool,
    /// Show debug logs.
    #[arg(short, long, global = true)]
    verbose: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Show this device's name, id and fingerprint.
    Whoami,
    /// Rename this device.
    Name {
        /// The new name, for example "Hakim's laptop".
        name: Vec<String>,
    },
    /// Send files and folders, or text. Without --to, shows a four-word code for the other device to enter.
    Send {
        #[arg(required_unless_present = "text")]
        files: Vec<PathBuf>,
        /// Send this text (a note or a link) instead of files.
        #[arg(long, conflicts_with = "files")]
        text: Option<String>,
        /// Send to a trusted or nearby device by name, fingerprint or id.
        #[arg(long, value_name = "DEVICE")]
        to: Option<String>,
        /// Save the other device as trusted after connecting with a code.
        #[arg(long)]
        trust: bool,
    },
    /// Share files, folders or text with a link, for a device without Nhako Beam on the same Wi-Fi.
    /// Each browser that opens it asks first.
    Share {
        #[arg(required_unless_present = "text")]
        files: Vec<PathBuf>,
        /// Share this text (a note or a link) instead of files.
        #[arg(long, conflicts_with = "files")]
        text: Option<String>,
        /// Let every browser in without asking.
        #[arg(short, long)]
        yes: bool,
    },
    /// Receive files. With a code, receive from the device showing it; without, wait for nearby and trusted devices.
    Receive {
        /// The four words shown on the other device.
        words: Vec<String>,
        /// Accept without asking.
        #[arg(short, long)]
        yes: bool,
        /// Save the sender as a trusted device.
        #[arg(long)]
        trust: bool,
    },
    /// Pair with a code and save each other as trusted. No words shows a code; four words enters one.
    Pair { words: Vec<String> },
    /// List nearby and trusted devices.
    Devices {
        /// Seconds to look around before listing.
        #[arg(long, default_value_t = 3)]
        wait: u64,
    },
    /// Print a report on this device's network and nearby discovery, for a bug report.
    Diagnostics {
        /// Seconds to look around first.
        #[arg(long, default_value_t = 5)]
        wait: u64,
    },
    /// Who can send to this device: your devices only (trusted, or connected with a code), or
    /// everyone nearby. Without a value, shows the current setting.
    WhoCanSend { who: Option<WhoArg> },
    /// Manage trusted devices.
    Trust {
        #[command(subcommand)]
        action: TrustAction,
    },
    /// Choose the public servers or your own (see relay/ in the repository). No action shows the current one.
    Server {
        #[command(subcommand)]
        action: Option<ServerAction>,
    },
}

#[derive(Subcommand)]
enum ServerAction {
    /// Show which servers this device uses.
    Show,
    /// Use your own server from now on. Checks it first.
    Set {
        /// Relay address, for example http://192.168.1.20:3340.
        relay: String,
        /// The relay token, if it has one. Reading it from NHAKO_RELAY_TOKEN keeps it out of your shell history.
        #[arg(long, env = "NHAKO_RELAY_TOKEN", hide_env_values = true)]
        token: Option<String>,
        /// Your own address lookup server, for example http://192.168.1.20:3341. Without it, n0's public lookup is used.
        #[arg(long)]
        lookup: Option<String>,
    },
    /// Go back to n0's free public servers.
    Public,
    /// Check that the saved server answers and accepts the token.
    Check,
}

#[derive(Clone, Copy, ValueEnum)]
enum WhoArg {
    /// Trusted devices, and devices you connect with a code.
    Yours,
    /// Anyone on the same network (you're still asked first).
    Everyone,
}

impl From<WhoArg> for WhoCanSend {
    fn from(w: WhoArg) -> Self {
        match w {
            WhoArg::Yours => Self::YourDevices,
            WhoArg::Everyone => Self::EveryoneNearby,
        }
    }
}

#[derive(Subcommand)]
enum TrustAction {
    /// Forget a trusted device. You'll need a code to connect again.
    Remove { device: String },
    /// Accept files from a trusted device without asking.
    AutoAccept {
        device: String,
        #[arg(value_parser = ["on", "off"])]
        state: String,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let filter = if cli.verbose {
        "nhako_core=debug,nhako=debug,iroh=info"
    } else {
        "nhako_core=warn,nhako=warn"
    };
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| filter.into()))
        .with_writer(std::io::stderr)
        .init();
    match run(cli).await {
        Ok(code) => code,
        Err(err) => {
            eprintln!("{err:#}");
            ExitCode::FAILURE
        }
    }
}

async fn run(cli: Cli) -> Result<ExitCode> {
    let config_dir = cli.config_dir.clone().unwrap_or_else(store::default_config_dir);
    // Server settings are changed without starting the engine, so a broken setting can always be fixed.
    if let Command::Server { action } = cli.command {
        return server_cmd(&config_dir, action.unwrap_or(ServerAction::Show)).await;
    }
    let mut cfg = EngineConfig::new(config_dir);
    cfg.save_dir = cli.save_dir.clone();
    cfg.mdns = !cli.no_nearby;
    // The command line tool is updated however it was installed.
    cfg.update_url = None;
    if cli.no_nearby {
        cfg.beacon = None;
    }
    cfg.relay_only = cli.relay_only;
    cfg.network = if let Some(relay) = &cli.relay {
        let own = OwnServer {
            relay: relay.clone(),
            token: cli.relay_token.clone().or_else(|| std::env::var("NHAKO_RELAY_TOKEN").ok()),
            lookup: cli.lookup.clone(),
        };
        Some(Network::Own(OwnNetwork::parse(&own)?))
    } else if cli.local_only {
        Some(Network::LocalOnly)
    } else {
        None
    };
    let engine = match Engine::start(cfg).await {
        Ok(e) => e,
        Err(e @ nhako_core::EngineError::Setting(_)) => {
            bail!("{e}\nFix it with `nhako server set ...`, or go back to the public servers with `nhako server public`.")
        }
        Err(e) => return Err(e).context("couldn't start Nhako Beam"),
    };
    let events = engine.subscribe();
    if cli.json {
        spawn_json_printer(engine.subscribe());
    }
    let ui = Ui { json: cli.json };
    let result = match cli.command {
        Command::Whoami => whoami(&engine, &ui),
        Command::Diagnostics { wait } => {
            tokio::time::sleep(Duration::from_secs(wait)).await;
            println!("{}", engine.diagnostics(&format!("nhako {}", env!("CARGO_PKG_VERSION"))));
            Ok(ExitCode::SUCCESS)
        }
        Command::WhoCanSend { who } => {
            if let Some(who) = who {
                engine.set_who_can_send(who.into())?;
            }
            ui.say(match engine.who_can_send() {
                WhoCanSend::YourDevices => "Only your devices can send here: trusted ones, and ones you connect with a code.",
                WhoCanSend::EveryoneNearby => "Everyone nearby can send here. You're asked before anything is saved.",
            });
            Ok(ExitCode::SUCCESS)
        }
        Command::Name { name } => {
            let name = name.join(" ");
            if name.trim().is_empty() {
                bail!("Give a name, for example: nhako name \"Hakim's laptop\"");
            }
            engine.rename_self(&name);
            ui.say(format!("This device is now called {}.", engine.me().name));
            Ok(ExitCode::SUCCESS)
        }
        Command::Send { files, text, to, trust } => send(&engine, events, &ui, files, text, to, trust).await,
        Command::Share { files, text, yes } => share(&engine, events, &ui, files, text, yes).await,
        Command::Receive { words, yes, trust } if words.is_empty() => listen(&engine, events, &ui, yes, trust).await,
        Command::Receive { words, yes, trust } => receive_code(&engine, events, &ui, &words, yes, trust).await,
        Command::Pair { words } => pair(&engine, events, &ui, &words).await,
        Command::Devices { wait } => devices(&engine, &ui, wait).await,
        Command::Trust { action } => trust_cmd(&engine, &ui, action),
        Command::Server { .. } => unreachable!("handled before the engine starts"),
    };
    engine.stop().await;
    result
}

// ---------------------------------------------------------------- output

struct Ui {
    json: bool,
}

impl Ui {
    /// Human output goes to stdout unless --json is on, where it moves to stderr.
    fn say(&self, line: impl AsRef<str>) {
        if self.json {
            eprintln!("{}", line.as_ref());
        } else {
            println!("{}", line.as_ref());
        }
    }

    fn bar(&self, total: u64) -> ProgressBar {
        if self.json {
            return ProgressBar::hidden();
        }
        let bar = ProgressBar::new(total);
        bar.set_style(
            ProgressStyle::with_template("{spinner:.blue} {msg}\n  {wide_bar:.blue/240} {bytes}/{total_bytes}  {binary_bytes_per_sec}  {eta} left")
                .expect("valid template")
                .progress_chars("━━ "),
        );
        bar.enable_steady_tick(Duration::from_millis(100));
        bar
    }
}

fn spawn_json_printer(mut rx: broadcast::Receiver<EngineEvent>) {
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(e) => println!("{}", serde_json::to_string(&e).expect("events serialize")),
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => return,
            }
        }
    });
}

fn human_bytes(n: u64) -> String {
    indicatif::HumanBytes(n).to_string()
}

fn route_word(r: Option<Route>) -> &'static str {
    match r {
        Some(Route::Nearby) => "nearby",
        Some(Route::Direct) => "direct",
        Some(Route::Relayed) => "relayed",
        None => "offline",
    }
}

fn error_text(err: Option<TransferError>, device: &str) -> String {
    match err {
        Some(TransferError::PeerGone) => format!("{device} went offline and didn't come back."),
        Some(TransferError::Network) => "The connection dropped and couldn't be restored.".into(),
        Some(TransferError::FileChanged) => "A file changed while sending. Send it again to get the latest version.".into(),
        Some(TransferError::Integrity) => "A file arrived damaged, so it wasn't saved. Run the same command again to resend it.".into(),
        Some(TransferError::RelayUnreachable) => "Can't reach the relay server. Check your connection or relay settings.".into(),
        Some(TransferError::Permission) => "Nhako Beam doesn't have permission to save here. Choose another folder with --save-dir.".into(),
        Some(TransferError::DiskFull) => "This device ran out of space.".into(),
        Some(TransferError::DiskFullPeer) => format!("{device} ran out of space."),
        Some(TransferError::NotAllowed) => format!("{device} only takes files from its own devices. Pair with a code first: nhako pair"),
        Some(TransferError::Unknown) | None => "Something went wrong. Try again, and if it keeps happening, run with --verbose.".into(),
    }
}

fn summary(t: &Transfer) -> String {
    if let Some(text) = &t.text {
        let line: String = text.trim().lines().next().unwrap_or_default().chars().take(60).collect();
        return format!("this text: \"{line}\"");
    }
    let folders: std::collections::BTreeSet<&str> = t.files.iter().filter_map(|f| f.folder.as_deref()?.split('/').next()).collect();
    if folders.len() == 1 && t.files.iter().all(|f| f.folder.is_some()) {
        return format!(
            "the folder {} ({} files, {})",
            folders.first().unwrap_or(&""),
            t.files.len(),
            human_bytes(t.total_bytes)
        );
    }
    match t.files.len() {
        1 => format!("{} ({})", t.files[0].name, human_bytes(t.total_bytes)),
        n => format!("{n} files ({})", human_bytes(t.total_bytes)),
    }
}

async fn next_event(rx: &mut broadcast::Receiver<EngineEvent>) -> Option<EngineEvent> {
    loop {
        match rx.recv().await {
            Ok(e) => return Some(e),
            Err(broadcast::error::RecvError::Lagged(_)) => continue,
            Err(_) => return None,
        }
    }
}

// ---------------------------------------------------------------- commands

fn whoami(engine: &Engine, ui: &Ui) -> Result<ExitCode> {
    let me = engine.me();
    if ui.json {
        println!("{}", serde_json::to_string(&me)?);
        return Ok(ExitCode::SUCCESS);
    }
    ui.say(format!("{}  ({:?}, {:?})", me.name, me.kind, me.os));
    ui.say(format!("Fingerprint  {}", me.fingerprint));
    ui.say(format!("Device id    {}", me.id));
    ui.say(format!("Saves to     {}", me.save_dir.display()));
    ui.say(match engine.network() {
        Network::N0 => "Servers      n0's free public servers".to_string(),
        Network::Own(own) => format!("Servers      your relay {}", own.relay),
        Network::LocalOnly => "Servers      none, same network only".to_string(),
    });
    Ok(ExitCode::SUCCESS)
}

// ---------------------------------------------------------------- server

async fn server_cmd(config_dir: &std::path::Path, action: ServerAction) -> Result<ExitCode> {
    let mut st = store::Store::open(config_dir, store::default_save_dir())?;
    match action {
        ServerAction::Show => {
            match &st.state.connection {
                Connection::Public => println!("Using n0's free public servers."),
                Connection::Own(own) => print_own(own),
            }
            Ok(ExitCode::SUCCESS)
        }
        ServerAction::Public => {
            st.state.connection = Connection::Public;
            st.save()?;
            println!("Back on n0's free public servers.");
            Ok(ExitCode::SUCCESS)
        }
        ServerAction::Check => {
            let Connection::Own(own) = st.state.connection.clone() else {
                println!("Using n0's free public servers, so there's nothing of yours to check.");
                return Ok(ExitCode::SUCCESS);
            };
            print_own(&own);
            Ok(print_check(&check_server(&own).await))
        }
        ServerAction::Set { relay, token, lookup } => {
            let own = OwnServer { relay, token, lookup };
            OwnNetwork::parse(&own)?;
            let check = check_server(&own).await;
            let code = print_check(&check);
            if check.relay == CheckResult::TokenRejected {
                bail!("Not saved: the relay turned the token down. Check the token in relay/.env and try again.");
            }
            st.state.connection = Connection::Own(own);
            st.save()?;
            if code == ExitCode::SUCCESS {
                println!("Saved. Every device that should reach this one over the internet needs the same setting.");
            } else {
                println!("Saved anyway, in case the server is just off right now. `nhako server check` tries again.");
            }
            Ok(code)
        }
    }
}

fn print_own(own: &OwnServer) {
    println!("Using your own server.");
    println!("  Relay   {}", own.relay);
    println!(
        "  Token   {}",
        if own.token.as_deref().is_some_and(|t| !t.trim().is_empty()) {
            "set"
        } else {
            "none"
        }
    );
    println!("  Lookup  {}", own.lookup.as_deref().unwrap_or("n0's public lookup"));
}

fn check_line(what: &str, r: &CheckResult) -> bool {
    match r {
        CheckResult::Ok { ms } => {
            println!("  {what}  answered in {ms} ms");
            true
        }
        CheckResult::TokenRejected => {
            println!("  {what}  answered, but turned the token down");
            false
        }
        CheckResult::Unreachable { reason } => {
            println!("  {what}  can't be reached: {reason}");
            false
        }
        CheckResult::Invalid { reason } => {
            println!("  {what}  can't be used: {reason}");
            false
        }
    }
}

fn print_check(c: &ServerCheck) -> ExitCode {
    println!("Checking...");
    let mut ok = check_line("Relay ", &c.relay);
    if let Some(l) = &c.lookup {
        ok &= check_line("Lookup", l);
    }
    if ok { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

/// Finds a device by name (exact or unique start), fingerprint, or id (or its start).
fn find_device(engine: &Engine, query: &str) -> Result<Device> {
    let q = query.trim().to_lowercase();
    let q_hex: String = q.chars().filter(|c| !c.is_whitespace()).collect();
    let all = engine.devices();
    if let Some(d) = all
        .iter()
        .find(|d| d.name.to_lowercase() == q || d.fingerprint.replace(' ', "").to_lowercase() == q_hex || d.id == q)
    {
        return Ok(d.clone());
    }
    let starts: Vec<&Device> = all
        .iter()
        .filter(|d| d.name.to_lowercase().starts_with(&q) || (q_hex.len() >= 6 && d.id.starts_with(&q_hex)))
        .collect();
    match starts.as_slice() {
        [one] => Ok((*one).clone()),
        [] => bail!("No device called \"{query}\". Run `nhako devices` to see who's around, or send with a code instead."),
        many => bail!(
            "\"{query}\" matches {} devices: {}. Be more specific.",
            many.len(),
            many.iter().map(|d| d.name.as_str()).collect::<Vec<_>>().join(", ")
        ),
    }
}

async fn send(
    engine: &Engine,
    mut events: broadcast::Receiver<EngineEvent>,
    ui: &Ui,
    files: Vec<PathBuf>,
    text: Option<String>,
    to: Option<String>,
    trust: bool,
) -> Result<ExitCode> {
    let out = outgoing(&files)?;
    if let Some(t) = &text
        && t.len() > nhako_core::engine_limits::TEXT_MAX
    {
        bail!("That text is too long to send as text. Save it in a file and send the file.");
    }
    let device = match to {
        Some(query) => {
            // Give discovery a moment, so nearby and trusted devices are known.
            tokio::time::sleep(Duration::from_secs(2)).await;
            find_device(engine, &query)?
        }
        None => {
            let Some(device) = show_code_and_wait(engine, &mut events, ui).await? else {
                return Ok(ExitCode::FAILURE);
            };
            if trust {
                engine.set_trusted(&device.id, true);
                ui.say(format!("Saved {} as a trusted device.", device.name));
            }
            device
        }
    };
    let id = match &text {
        Some(t) => engine.send_text(&device.id, t)?,
        None => engine.send_files(&device.id, &out)?,
    };
    follow_transfer(engine, &mut events, ui, &id, &device.name).await
}

/// What to send for these paths: files as they are, folders with everything inside them.
fn outgoing(files: &[PathBuf]) -> Result<Vec<Outgoing>> {
    let mut out = Vec::new();
    for f in files {
        let meta = std::fs::metadata(f).with_context(|| format!("Can't read {}", f.display()))?;
        if meta.is_dir() {
            let inside =
                nhako_core::files::walk_folder(f, nhako_core::engine_limits::FILES_MAX).with_context(|| format!("Can't send the folder {}", f.display()))?;
            if inside.is_empty() {
                bail!("{} is an empty folder. There's nothing in it to send.", f.display());
            }
            out.extend(inside.into_iter().map(|(path, folder)| Outgoing {
                name: path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                path,
                folder: Some(folder),
            }));
        } else {
            out.push(Outgoing {
                name: f.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into()),
                path: f.clone(),
                folder: None,
            });
        }
    }
    Ok(out)
}

/// `nhako share`: serves a link on this network until Ctrl+C, it ends, or you answer no to everyone.
async fn share(
    engine: &Engine,
    mut events: broadcast::Receiver<EngineEvent>,
    ui: &Ui,
    files: Vec<PathBuf>,
    text: Option<String>,
    yes: bool,
) -> Result<ExitCode> {
    let out = outgoing(&files)?;
    let share = engine.share_link(&out, text.as_deref()).map_err(|e| match e {
        nhako_core::EngineError::NoLan => anyhow::anyhow!("This computer isn't on a local network. Share via link works on the same Wi-Fi or cable."),
        nhako_core::EngineError::TextTooLong => anyhow::anyhow!("That text is too long to share as text. Save it in a file and share the file."),
        other => other.into(),
    })?;
    ui.say(format!("\nOpen this in a browser on the same Wi-Fi:\n\n  {}\n", share.url));
    ui.say(if yes {
        "Every browser that opens it can download. Ctrl+C stops sharing."
    } else {
        "You'll be asked before each browser can download. Ctrl+C stops sharing."
    });
    let mut asked = std::collections::HashSet::new();
    let mut finished = std::collections::HashMap::new();
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                engine.stop_link();
                ui.say("Stopped sharing.");
                return Ok(ExitCode::SUCCESS);
            }
            event = events.recv() => match event {
                Ok(EngineEvent::LinkShare { share: None }) => {
                    ui.say("The link has ended.");
                    return Ok(ExitCode::SUCCESS);
                }
                Ok(EngineEvent::LinkShare { share: Some(share) }) => {
                    for v in share.visitors {
                        if v.state == nhako_core::VisitorState::Asking && asked.insert(v.id.clone()) {
                            let allow = yes || confirm(format!("{} at {} wants to download. Let it in?", v.label, v.address)).await;
                            engine.answer_visitor(&v.id, allow);
                            ui.say(if allow { format!("Let in {}.", v.label) } else { format!("Turned away {}.", v.label) });
                        }
                        let seen = finished.entry(v.id.clone()).or_insert(0);
                        if v.downloads > *seen {
                            *seen = v.downloads;
                            ui.say(format!("{} ({}) finished a download.", v.label, v.address));
                        }
                    }
                }
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => return Ok(ExitCode::FAILURE),
            },
        }
    }
}

/// Shows a code and waits until a device enters it. `None` if it expired or was interrupted.
async fn show_code_and_wait(engine: &Engine, events: &mut broadcast::Receiver<EngineEvent>, ui: &Ui) -> Result<Option<Device>> {
    let code = engine.create_code();
    let words = code.words.join(" ");
    ui.say(format!("\nYour code:  {words}\n"));
    ui.say(format!(
        "On the other device, run:\n  nhako receive {words}\nor choose Receive with a code in the app.\n"
    ));
    let spinner = if ui.json { ProgressBar::hidden() } else { ProgressBar::new_spinner() };
    spinner.enable_steady_tick(Duration::from_millis(100));
    let tick = tokio::time::interval(Duration::from_secs(1));
    tokio::pin!(tick);
    loop {
        tokio::select! {
            _ = tick.tick() => {
                let left = code.expires_at.saturating_sub(now_ms()) / 1000;
                spinner.set_message(format!("Waiting for the other device. The code expires in {}:{:02}.", left / 60, left % 60));
            }
            e = next_event(events) => match e {
                Some(EngineEvent::Paired { device, check, via: PairVia::Shown }) => {
                    spinner.finish_and_clear();
                    ui.say(format!("Connected to {}. Check that both screens show {check}.", device.name));
                    return Ok(Some(device));
                }
                Some(EngineEvent::Code { code: None }) => {
                    spinner.finish_and_clear();
                    ui.say("This code expired. Run the command again for a new one.");
                    return Ok(None);
                }
                None => return Ok(None),
                _ => {}
            },
            _ = tokio::signal::ctrl_c() => {
                spinner.finish_and_clear();
                engine.cancel_code();
                ui.say("Stopped.");
                return Ok(None);
            }
        }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Shows progress for one transfer until it ends. Ctrl+C cancels it.
async fn follow_transfer(engine: &Engine, events: &mut broadcast::Receiver<EngineEvent>, ui: &Ui, id: &str, device: &str) -> Result<ExitCode> {
    // Right after accepting, the engine may not have created the transfer yet.
    let first = match engine.transfer(id) {
        Some(t) => t,
        None => {
            let found = tokio::time::timeout(Duration::from_secs(10), async {
                loop {
                    match next_event(events).await {
                        Some(EngineEvent::Transfer { transfer }) if transfer.id == id => return Some(transfer),
                        None => return None,
                        _ => {}
                    }
                }
            })
            .await;
            match found {
                Ok(Some(t)) => t,
                _ => bail!("The transfer didn't start. Try again."),
            }
        }
    };
    let bar = ui.bar(first.total_bytes);
    let mut last_status = None;
    let mut update = |t: &Transfer| -> Option<ExitCode> {
        bar.set_position(t.done_bytes);
        let sending = t.direction == TransferDirection::Send;
        if last_status != Some((t.status, t.paused_by, t.route)) {
            last_status = Some((t.status, t.paused_by, t.route));
            let what = summary(t);
            let msg = match t.status {
                TransferStatus::Connecting => format!("Connecting to {device}"),
                TransferStatus::Waiting => format!("Waiting for {device} to accept {what}"),
                TransferStatus::Transferring if sending => format!("Sending {what} to {device} ({})", route_word(Some(t.route))),
                TransferStatus::Transferring => format!("Receiving {what} from {device} ({})", route_word(Some(t.route))),
                TransferStatus::Paused if t.paused_by == Some(nhako_core::Side::Peer) => format!("{device} paused this transfer"),
                TransferStatus::Paused => "Paused".into(),
                TransferStatus::Reconnecting => "Reconnecting. Your progress is safe.".into(),
                _ => String::new(),
            };
            if !msg.is_empty() {
                bar.set_message(msg);
            }
        }
        let done = |bar: &ProgressBar, line: String, ok: bool| {
            bar.finish_and_clear();
            ui.say(line);
            Some(if ok { ExitCode::SUCCESS } else { ExitCode::FAILURE })
        };
        match t.status {
            TransferStatus::Done if sending => done(&bar, format!("Sent {} to {device}.", summary(t)), true),
            TransferStatus::Done if t.text.is_some() => {
                bar.finish_and_clear();
                ui.say(format!("Text from {device}:"));
                // The text itself goes to standard output, so it can be piped.
                println!("{}", t.text.as_deref().unwrap_or_default());
                Some(ExitCode::SUCCESS)
            }
            TransferStatus::Done => done(&bar, format!("Received {} from {device}.", summary(t)), true),
            TransferStatus::Declined if t.error == Some(TransferError::DiskFullPeer) => done(&bar, format!("{device} doesn't have enough space."), false),
            TransferStatus::Declined if t.error == Some(TransferError::NotAllowed) => done(&bar, error_text(t.error, device), false),
            TransferStatus::Declined => done(&bar, format!("{device} declined."), false),
            TransferStatus::Cancelled if t.cancelled_by == Some(nhako_core::Side::Peer) => done(&bar, format!("{device} cancelled this transfer."), false),
            TransferStatus::Cancelled => done(&bar, "You cancelled this transfer.".into(), false),
            TransferStatus::Failed => done(&bar, format!("Transfer stopped. {}", error_text(t.error, device)), false),
            _ => None,
        }
    };
    if let Some(code) = update(&first) {
        return Ok(code);
    }
    loop {
        tokio::select! {
            e = next_event(events) => match e {
                Some(EngineEvent::Transfer { transfer }) if transfer.id == id => {
                    if let Some(code) = update(&transfer) {
                        return Ok(code);
                    }
                }
                Some(EngineEvent::TransferProgress { id: moved, done_bytes, .. }) if moved == id => bar.set_position(done_bytes),
                None => return Ok(ExitCode::FAILURE),
                _ => {}
            },
            _ = tokio::signal::ctrl_c() => {
                engine.cancel(id);
                // Let the cancel reach the other device.
                let _ = tokio::time::timeout(Duration::from_secs(3), async {
                    while let Some(e) = next_event(events).await {
                        if let EngineEvent::Transfer { transfer } = e
                            && transfer.id == id
                            && transfer.status.is_final()
                        {
                            return;
                        }
                    }
                })
                .await;
                bar.finish_and_clear();
                ui.say("You cancelled this transfer.");
                return Ok(ExitCode::FAILURE);
            }
        }
    }
}

/// Asks a yes or no question on the terminal. Defaults to no.
async fn confirm(question: String) -> bool {
    tokio::task::spawn_blocking(move || {
        use std::io::Write;
        print!("{question} [y/N] ");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).is_ok() && matches!(line.trim().to_lowercase().as_str(), "y" | "yes")
    })
    .await
    .unwrap_or(false)
}

async fn redeem(engine: &Engine, ui: &Ui, typed: &[String]) -> Result<Option<Device>> {
    let parts: Vec<String> = typed.iter().flat_map(|w| words::split_code(w)).collect();
    if parts.len() != words::CODE_WORDS {
        bail!("A code is four words, for example: nhako receive river lantern seven harbor");
    }
    ui.say("Looking for the device showing this code...");
    match engine.redeem_code(&parts).await {
        RedeemResult::Ok(device) => Ok(Some(device)),
        RedeemResult::Err(RedeemFailure::UnknownWord { index }) => bail!("\"{}\" isn't in the word list. Check the spelling.", parts[index]),
        RedeemResult::Err(RedeemFailure::TokenRejected) => {
            ui.say("Your server turned this device away, so the other device couldn't be reached. Check the token with: nhako server check");
            Ok(None)
        }
        RedeemResult::Err(_) => {
            ui.say("That code didn't match. Check the words and try again. Codes work once and last 10 minutes.");
            Ok(None)
        }
    }
}

async fn receive_code(engine: &Engine, mut events: broadcast::Receiver<EngineEvent>, ui: &Ui, typed: &[String], yes: bool, trust: bool) -> Result<ExitCode> {
    let Some(device) = redeem(engine, ui, typed).await? else {
        return Ok(ExitCode::FAILURE);
    };
    if trust {
        engine.set_trusted(&device.id, true);
    }
    let wait = tokio::time::sleep(Duration::from_secs(120));
    tokio::pin!(wait);
    // The sender may start sending before our "paired" event arrives, so handle both in one loop.
    loop {
        tokio::select! {
            e = next_event(&mut events) => match e {
                Some(EngineEvent::Paired { check, device: d, .. }) if d.id == device.id => {
                    ui.say(format!("Connected to {}. Check that both screens show {check}.", device.name));
                    if trust {
                        ui.say(format!("Saved {} as a trusted device.", device.name));
                    }
                    ui.say(format!("Waiting for {} to send...", device.name));
                }
                Some(EngineEvent::Incoming { request }) if request.device_id == device.id => {
                    let t = Transfer { files: request.files.clone(), total_bytes: request.total_bytes, text: request.text.clone(), ..placeholder() };
                    if !yes && !confirm(format!("{} wants to send you {}. Accept?", device.name, summary(&t))).await {
                        engine.decline(&request.id);
                        ui.say("Declined.");
                        return Ok(ExitCode::FAILURE);
                    }
                    let id = engine.accept(&request.id, false, false).context("the request went away")?;
                    return follow_transfer(engine, &mut events, ui, &id, &device.name).await;
                }
                Some(EngineEvent::Transfer { transfer }) if transfer.device_id == device.id && transfer.direction == TransferDirection::Receive => {
                    // Auto-accepted because this device is trusted with auto-accept on.
                    return follow_transfer(engine, &mut events, ui, &transfer.id, &device.name).await;
                }
                None => return Ok(ExitCode::FAILURE),
                _ => {}
            },
            _ = &mut wait => {
                ui.say(format!("{} didn't send anything. Stopped waiting.", device.name));
                return Ok(ExitCode::FAILURE);
            }
            _ = tokio::signal::ctrl_c() => {
                ui.say("Stopped.");
                return Ok(ExitCode::FAILURE);
            }
        }
    }
}

/// A blank transfer, only used to format request summaries.
fn placeholder() -> Transfer {
    Transfer {
        id: String::new(),
        direction: TransferDirection::Receive,
        device_id: String::new(),
        files: vec![],
        total_bytes: 0,
        done_bytes: 0,
        file_bytes: Default::default(),
        speed: 0.0,
        route: Route::Nearby,
        status: TransferStatus::Waiting,
        paused_by: None,
        cancelled_by: None,
        error: None,
        started_at: 0,
        moving_since: None,
        finished_at: None,
        waiting_long: false,
        waking: false,
        text: None,
    }
}

/// Waits for files from nearby and trusted devices until Ctrl+C.
async fn listen(engine: &Engine, mut events: broadcast::Receiver<EngineEvent>, ui: &Ui, yes: bool, trust: bool) -> Result<ExitCode> {
    let me = engine.me();
    ui.say(format!(
        "Waiting for files as {} ({}). Saving to {}.",
        me.name,
        me.fingerprint,
        me.save_dir.display()
    ));
    ui.say("Nearby and trusted devices can send to you. Press Ctrl+C to stop.");
    let mut active: Vec<String> = vec![];
    loop {
        tokio::select! {
            e = next_event(&mut events) => match e {
                Some(EngineEvent::Incoming { request }) => {
                    let name = engine.devices().into_iter().find(|d| d.id == request.device_id).map(|d| d.name).unwrap_or_else(|| "A device".into());
                    let t = Transfer { files: request.files.clone(), total_bytes: request.total_bytes, text: request.text.clone(), ..placeholder() };
                    if yes || confirm(format!("{name} wants to send you {}. Accept?", summary(&t))).await {
                        engine.accept(&request.id, trust, false);
                    } else {
                        engine.decline(&request.id);
                        ui.say(format!("Declined {name}."));
                    }
                }
                Some(EngineEvent::IncomingWithdrawn { .. }) => ui.say("The sender cancelled the request."),
                Some(EngineEvent::Transfer { transfer }) if transfer.direction == TransferDirection::Receive => {
                    let name = engine.devices().into_iter().find(|d| d.id == transfer.device_id).map(|d| d.name).unwrap_or_else(|| "a device".into());
                    if !active.contains(&transfer.id) && !transfer.status.is_final() {
                        active.push(transfer.id.clone());
                        ui.say(format!("Receiving {} from {name}...", summary(&transfer)));
                    }
                    match transfer.status {
                        TransferStatus::Done => ui.say(format!("Received {} from {name}.", summary(&transfer))),
                        TransferStatus::Failed => ui.say(format!("Transfer from {name} stopped. {}", error_text(transfer.error, &name))),
                        TransferStatus::Cancelled => ui.say(format!("Transfer from {name} was cancelled.")),
                        _ => {}
                    }
                    if transfer.status.is_final() || transfer.status == TransferStatus::Failed {
                        active.retain(|id| id != &transfer.id);
                    }
                }
                None => return Ok(ExitCode::FAILURE),
                _ => {}
            },
            _ = tokio::signal::ctrl_c() => {
                ui.say("Stopped.");
                return Ok(ExitCode::SUCCESS);
            }
        }
    }
}

async fn pair(engine: &Engine, mut events: broadcast::Receiver<EngineEvent>, ui: &Ui, typed: &[String]) -> Result<ExitCode> {
    let device = if typed.is_empty() {
        match show_code_and_wait(engine, &mut events, ui).await? {
            Some(d) => d,
            None => return Ok(ExitCode::FAILURE),
        }
    } else {
        let Some(device) = redeem(engine, ui, typed).await? else {
            return Ok(ExitCode::FAILURE);
        };
        let check = loop {
            match next_event(&mut events).await {
                Some(EngineEvent::Paired { check, .. }) => break check,
                None => return Ok(ExitCode::FAILURE),
                _ => {}
            }
        };
        ui.say(format!("Connected to {}. Check that both screens show {check}.", device.name));
        device
    };
    engine.set_trusted(&device.id, true);
    ui.say(format!(
        "Saved {} as a trusted device. You can now send with: nhako send FILE --to \"{}\"",
        device.name, device.name
    ));
    // Let the other side finish its half of the handshake before we close.
    tokio::time::sleep(Duration::from_millis(500)).await;
    Ok(ExitCode::SUCCESS)
}

async fn devices(engine: &Engine, ui: &Ui, wait: u64) -> Result<ExitCode> {
    if !ui.json {
        ui.say("Looking around...");
    }
    tokio::time::sleep(Duration::from_secs(wait)).await;
    let list = engine.devices();
    if ui.json {
        println!("{}", serde_json::to_string(&list)?);
        return Ok(ExitCode::SUCCESS);
    }
    if list.is_empty() {
        ui.say("No devices found. Make sure the other device has Nhako Beam open and is on the same Wi-Fi, or use a code.");
        return Ok(ExitCode::SUCCESS);
    }
    let width = list.iter().map(|d| d.name.chars().count()).max().unwrap_or(4).max(4);
    ui.say(format!("{:width$}  {:9}  {:10}  {:9}  ID", "NAME", "STATUS", "TRUST", "FINGERPR."));
    for d in list {
        let trust = match (d.trusted, d.auto_accept) {
            (true, true) => "auto",
            (true, false) => "trusted",
            _ => "",
        };
        ui.say(format!(
            "{:width$}  {:9}  {:10}  {:9}  {}",
            d.name,
            route_word(d.route),
            trust,
            d.fingerprint,
            &d.id[..12]
        ));
    }
    Ok(ExitCode::SUCCESS)
}

fn trust_cmd(engine: &Engine, ui: &Ui, action: TrustAction) -> Result<ExitCode> {
    match action {
        TrustAction::Remove { device } => {
            let d = find_device(engine, &device)?;
            if !d.trusted {
                bail!("{} isn't a trusted device.", d.name);
            }
            engine.set_trusted(&d.id, false);
            ui.say(format!("Removed {}. You'll need a code to connect again.", d.name));
        }
        TrustAction::AutoAccept { device, state } => {
            let d = find_device(engine, &device)?;
            if !d.trusted {
                bail!("{} isn't a trusted device. Pair first with `nhako pair`.", d.name);
            }
            let on = state == "on";
            engine.set_auto_accept(&d.id, on);
            ui.say(if on {
                format!("Files from {} will be accepted automatically.", d.name)
            } else {
                format!("You'll be asked before accepting files from {}.", d.name)
            });
        }
    }
    Ok(ExitCode::SUCCESS)
}
