//! Two (or three) real engines talking nhako/1 over QUIC on localhost.
//! No relay, no mDNS: addresses come from a shared in-memory table.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use nhako_core::{
    Engine, EngineConfig, EngineEvent, MemoryLookup, Network, PairVia, RedeemFailure, RedeemResult, Side, Transfer, TransferDirection, TransferError,
    TransferStatus, codekeys::KdfCost, device_id,
};
use tokio::sync::broadcast;

const WAIT: Duration = Duration::from_secs(30);

struct Node {
    engine: Engine,
    events: broadcast::Receiver<EngineEvent>,
    dir: PathBuf,
}

impl Node {
    async fn new(mem: &MemoryLookup, name: &str, rate: Option<u64>) -> Self {
        Self::with(mem, name, rate, |_| {}).await
    }

    async fn with(mem: &MemoryLookup, name: &str, rate: Option<u64>, tweak: impl FnOnce(&mut EngineConfig)) -> Self {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
            .with_test_writer()
            .try_init();
        let dir = std::env::temp_dir().join(format!("nhako-it-{name}-{}", rand::random::<u32>()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut cfg = EngineConfig::new(dir.join("config"));
        cfg.save_dir = Some(dir.join("save"));
        cfg.network = Some(Network::LocalOnly);
        cfg.mdns = false;
        cfg.kdf = KdfCost::TEST;
        cfg.reconnect_for = Duration::from_secs(10);
        cfg.code_lookup_for = Duration::from_secs(3);
        cfg.memory_lookup = Some(mem.clone());
        cfg.test_rate_limit = rate;
        cfg.wake_url = None;
        tweak(&mut cfg);
        let engine = Engine::start(cfg).await.unwrap();
        engine.rename_self(name);
        let events = engine.subscribe();
        Self { engine, events, dir }
    }

    fn id(&self) -> String {
        device_id(&self.engine.id())
    }

    fn save(&self) -> PathBuf {
        self.dir.join("save")
    }

    /// Waits for an event matching `f`, skipping others.
    async fn wait<T>(&mut self, what: &str, mut f: impl FnMut(&EngineEvent) -> Option<T>) -> T {
        let fut = async {
            loop {
                match self.events.recv().await {
                    Ok(e) => {
                        if let Some(t) = f(&e) {
                            return t;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(e) => panic!("events closed: {e}"),
                }
            }
        };
        tokio::time::timeout(WAIT, fut).await.unwrap_or_else(|_| panic!("timed out waiting for {what}"))
    }

    async fn wait_transfer(&mut self, what: &str, f: impl Fn(&Transfer) -> bool) -> Transfer {
        self.wait(what, |e| match e {
            EngineEvent::Transfer { transfer } if f(transfer) => Some(transfer.clone()),
            _ => None,
        })
        .await
    }

    async fn wait_incoming(&mut self) -> String {
        self.wait("incoming request", |e| match e {
            EngineEvent::Incoming { request } => Some(request.id.clone()),
            _ => None,
        })
        .await
    }

    async fn stop(self) {
        self.engine.stop().await;
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn status(s: TransferStatus) -> impl Fn(&Transfer) -> bool {
    move |t| t.status == s
}

/// A file of `len` bytes with a recognisable pattern.
fn make_file(dir: &Path, name: &str, len: usize) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join(name);
    let data: Vec<u8> = (0..len).map(|i| (i % 251) as u8 ^ (i / 4096) as u8).collect();
    std::fs::write(&path, data).unwrap();
    path
}

fn same_bytes(a: &Path, b: &Path) {
    let (x, y) = (std::fs::read(a).unwrap(), std::fs::read(b).unwrap());
    assert_eq!(x.len(), y.len(), "{} and {} differ in length", a.display(), b.display());
    assert!(x == y, "{} and {} differ", a.display(), b.display());
}

fn no_part_files(dir: &Path) {
    let parts: Vec<_> = std::fs::read_dir(dir)
        .map(|d| d.flatten().filter(|e| e.file_name().to_string_lossy().ends_with(".nhakopart")).collect())
        .unwrap_or_default();
    assert!(parts.is_empty(), "left part files behind: {parts:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn send_accept_and_verify() {
    let mem = MemoryLookup::new();
    let mut a = Node::new(&mem, "Aisyah's laptop", None).await;
    let mut b = Node::new(&mem, "Studio PC", None).await;
    let src = a.dir.join("out");
    let files = [
        make_file(&src, "photo.jpg", 3_000_000),
        make_file(&src, "notes.pdf", 12_345),
        make_file(&src, "empty.txt", 0),
    ];
    let id = a.engine.send(&b.id(), &files).unwrap();

    let req = b.wait_incoming().await;
    assert_eq!(req, id, "request id and transfer id are the same");
    a.wait_transfer("waiting", status(TransferStatus::Waiting)).await;
    // A window that loads now still sees the request card.
    let snap = b.engine.snapshot();
    assert_eq!(snap.requests.len(), 1);
    assert_eq!(snap.requests[0].id, req);
    assert_eq!(snap.requests[0].files.len(), 3);
    assert_eq!(snap.server_problem, None);
    let got = b.engine.accept(&req, false, false);
    assert_eq!(got.as_deref(), Some(id.as_str()));

    let sent = a.wait_transfer("sender done", status(TransferStatus::Done)).await;
    let recv = b.wait_transfer("receiver done", status(TransferStatus::Done)).await;
    assert_eq!(sent.done_bytes, sent.total_bytes);
    assert_eq!(recv.done_bytes, 3_000_000 + 12_345);
    assert_eq!(recv.direction, TransferDirection::Receive);
    assert_eq!(recv.device_id, a.id());
    assert_eq!(sent.route, nhako_core::Route::Nearby);
    for f in &files {
        same_bytes(f, &b.save().join(f.file_name().unwrap()));
    }
    no_part_files(&b.save());
    assert!(b.engine.requests().is_empty());
    let mut saved = b.engine.saved_files(&id);
    saved.sort();
    let mut want: Vec<_> = files.iter().map(|f| b.save().join(f.file_name().unwrap())).collect();
    want.sort();
    assert_eq!(saved, want, "Open and Show in folder point at the real files");
    assert!(a.engine.saved_files(&id).is_empty(), "nothing was saved on the sending side");

    // The receiver learned the sender's name and kind from the hello.
    let dev = b.engine.devices().into_iter().find(|d| d.id == a.id()).unwrap();
    assert_eq!(dev.name, "Aisyah's laptop");
    assert!(!dev.trusted);

    // Sending the same names again picks new names instead of overwriting.
    let id2 = a.engine.send(&b.id(), &files[..1]).unwrap();
    let req = b.wait_incoming().await;
    b.engine.accept(&req, false, false);
    b.wait_transfer("second done", |t| t.id == id2 && t.status == TransferStatus::Done).await;
    same_bytes(&files[0], &b.save().join("photo (1).jpg"));
    assert_eq!(b.engine.saved_files(&id2), vec![b.save().join("photo (1).jpg")]);

    a.stop().await;
    b.stop().await;
}

/// Android shares files as open handles: the engine reads `/proc/self/fd/N` and uses the name given.
#[cfg(any(target_os = "linux", target_os = "android"))]
#[tokio::test(flavor = "multi_thread")]
async fn sends_an_open_file_handle_under_its_real_name() {
    use std::os::fd::AsRawFd;
    let mem = MemoryLookup::new();
    let a = Node::new(&mem, "Phone", None).await;
    let mut b = Node::new(&mem, "Laptop", None).await;
    let f = make_file(&a.dir.join("out"), "content-12345", 700_000);
    let handle = std::fs::File::open(&f).unwrap();
    let fd_path = std::path::PathBuf::from(format!("/proc/self/fd/{}", handle.as_raw_fd()));
    let id = a.engine.send_named(&b.id(), &[(fd_path, "Holiday photo.jpg".into())]).unwrap();
    let req = b.wait_incoming().await;
    b.engine.accept(&req, false, false);
    b.wait_transfer("received", |t| t.id == id && t.status == TransferStatus::Done).await;
    same_bytes(&f, &b.save().join("Holiday photo.jpg"));
    drop(handle);
    a.stop().await;
    b.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn decline_and_withdraw() {
    let mem = MemoryLookup::new();
    let mut a = Node::new(&mem, "A", None).await;
    let mut b = Node::new(&mem, "B", None).await;
    let f = make_file(&a.dir.join("out"), "a.bin", 1000);

    a.engine.send(&b.id(), std::slice::from_ref(&f)).unwrap();
    let req = b.wait_incoming().await;
    b.engine.decline(&req);
    a.wait_transfer("declined", status(TransferStatus::Declined)).await;

    let id = a.engine.send(&b.id(), &[f]).unwrap();
    let req = b.wait_incoming().await;
    a.wait_transfer("waiting", status(TransferStatus::Waiting)).await;
    a.engine.cancel(&id);
    let withdrawn = b
        .wait("withdrawn", |e| match e {
            EngineEvent::IncomingWithdrawn { request_id } => Some(request_id.clone()),
            _ => None,
        })
        .await;
    assert_eq!(withdrawn, req);
    let t = a.wait_transfer("cancelled", status(TransferStatus::Cancelled)).await;
    assert_eq!(t.cancelled_by, Some(Side::Me));
    assert!(b.engine.accept(&req, false, false).is_none(), "a withdrawn request can't be accepted");

    a.stop().await;
    b.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn pause_and_resume_from_both_sides() {
    let mem = MemoryLookup::new();
    let mut a = Node::new(&mem, "A", Some(4_000_000)).await;
    let mut b = Node::new(&mem, "B", None).await;
    let f = make_file(&a.dir.join("out"), "video.mp4", 12_000_000);
    let id = a.engine.send(&b.id(), std::slice::from_ref(&f)).unwrap();
    let req = b.wait_incoming().await;
    b.engine.accept(&req, false, false);

    // Receiver pauses.
    b.wait_transfer("bytes moving", |t| t.done_bytes > 500_000).await;
    b.engine.pause(&id);
    let t = a.wait_transfer("peer paused", status(TransferStatus::Paused)).await;
    assert_eq!(t.paused_by, Some(Side::Peer));
    let t = b.wait_transfer("me paused", status(TransferStatus::Paused)).await;
    assert_eq!(t.paused_by, Some(Side::Me));
    assert_eq!(t.speed, 0.0);
    let at_pause = t.done_bytes;
    tokio::time::sleep(Duration::from_millis(800)).await;
    let still = b.engine.transfer(&id).unwrap();
    assert!(
        still.done_bytes <= at_pause + 2 * 256 * 1024,
        "kept moving while paused: {} -> {}",
        at_pause,
        still.done_bytes
    );

    // Sender resumes a pause made by the receiver (either side may resume).
    a.engine.resume(&id);
    b.wait_transfer("moving again", |t| t.status == TransferStatus::Transferring && t.done_bytes > at_pause)
        .await;

    // Sender pauses, receiver resumes.
    a.engine.pause(&id);
    let t = b.wait_transfer("sender paused", status(TransferStatus::Paused)).await;
    assert_eq!(t.paused_by, Some(Side::Peer));
    b.engine.resume(&id);

    a.wait_transfer("sender done", status(TransferStatus::Done)).await;
    b.wait_transfer("receiver done", status(TransferStatus::Done)).await;
    same_bytes(&f, &b.save().join("video.mp4"));
    no_part_files(&b.save());

    a.stop().await;
    b.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn cancel_mid_transfer_cleans_up() {
    let mem = MemoryLookup::new();
    let mut a = Node::new(&mem, "A", Some(4_000_000)).await;
    let mut b = Node::new(&mem, "B", None).await;
    let f = make_file(&a.dir.join("out"), "big.iso", 10_000_000);
    let id = a.engine.send(&b.id(), &[f]).unwrap();
    let req = b.wait_incoming().await;
    b.engine.accept(&req, false, false);
    b.wait_transfer("bytes moving", |t| t.done_bytes > 300_000).await;
    b.engine.cancel(&id);
    let t = a.wait_transfer("sender sees cancel", status(TransferStatus::Cancelled)).await;
    assert_eq!(t.cancelled_by, Some(Side::Peer));
    let t = b.wait_transfer("receiver cancelled", status(TransferStatus::Cancelled)).await;
    assert_eq!(t.cancelled_by, Some(Side::Me));
    tokio::time::sleep(Duration::from_millis(300)).await;
    no_part_files(&b.save());
    assert!(!b.save().join("big.iso").exists());

    a.stop().await;
    b.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn reconnects_after_a_drop_and_keeps_progress() {
    let mem = MemoryLookup::new();
    let mut a = Node::new(&mem, "A", Some(5_000_000)).await;
    let mut b = Node::new(&mem, "B", None).await;
    let f = make_file(&a.dir.join("out"), "archive.zip", 10_000_000);
    let id = a.engine.send(&b.id(), std::slice::from_ref(&f)).unwrap();
    let req = b.wait_incoming().await;
    b.engine.accept(&req, false, false);
    b.wait_transfer("bytes moving", |t| t.done_bytes > 2_000_000).await;

    b.engine.test_drop_connection(&id);
    a.wait_transfer("sender reconnecting", status(TransferStatus::Reconnecting)).await;
    b.wait_transfer("receiver reconnecting", status(TransferStatus::Reconnecting)).await;
    // The sender comes back with Resume and starts from what the receiver already had.
    let back = a.wait_transfer("sender back", status(TransferStatus::Transferring)).await;
    assert!(back.done_bytes >= 1_500_000, "restarted from {} instead of keeping progress", back.done_bytes);

    a.wait_transfer("sender done", status(TransferStatus::Done)).await;
    b.wait_transfer("receiver done", status(TransferStatus::Done)).await;
    same_bytes(&f, &b.save().join("archive.zip"));

    a.stop().await;
    b.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn retry_after_failure_from_either_side() {
    let mem = MemoryLookup::new();
    let mut a = Node::new(&mem, "A", Some(5_000_000)).await;
    let mut b = Node::new(&mem, "B", None).await;
    let f = make_file(&a.dir.join("out"), "movie.mkv", 10_000_000);
    let id = a.engine.send(&b.id(), std::slice::from_ref(&f)).unwrap();
    let req = b.wait_incoming().await;
    b.engine.accept(&req, false, false);
    b.wait_transfer("bytes moving", |t| t.done_bytes > 1_000_000).await;

    // Fail, then the receiver retries (Pull).
    a.engine.test_fail_connection(&id);
    let t = b.wait_transfer("receiver failed", status(TransferStatus::Failed)).await;
    assert_eq!(t.error, Some(TransferError::Unknown));
    a.wait_transfer("sender failed", status(TransferStatus::Failed)).await;
    b.engine.retry(&id);
    let back = b
        .wait_transfer("receiver moving again", |t| t.status == TransferStatus::Transferring && t.done_bytes > 0)
        .await;
    assert!(back.done_bytes >= 1_000_000, "receiver retry started over at {}", back.done_bytes);
    b.wait_transfer("more bytes", |t| t.done_bytes > back.done_bytes + 1_000_000).await;

    // Fail again, then the sender retries (Resume).
    b.engine.test_fail_connection(&id);
    a.wait_transfer("sender failed again", status(TransferStatus::Failed)).await;
    b.wait_transfer("receiver failed again", status(TransferStatus::Failed)).await;
    a.engine.retry(&id);
    a.wait_transfer("sender done", status(TransferStatus::Done)).await;
    b.wait_transfer("receiver done", status(TransferStatus::Done)).await;
    same_bytes(&f, &b.save().join("movie.mkv"));

    a.stop().await;
    b.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn changed_source_file_stops_the_transfer() {
    let mem = MemoryLookup::new();
    let mut a = Node::new(&mem, "A", Some(3_000_000)).await;
    let mut b = Node::new(&mem, "B", None).await;
    let f = make_file(&a.dir.join("out"), "draft.docx", 9_000_000);
    let id = a.engine.send(&b.id(), std::slice::from_ref(&f)).unwrap();
    let req = b.wait_incoming().await;
    b.engine.accept(&req, false, false);
    b.wait_transfer("bytes moving", |t| t.done_bytes > 500_000).await;
    a.engine.pause(&id);
    a.wait_transfer("paused", status(TransferStatus::Paused)).await;
    std::fs::write(&f, b"edited while sending").unwrap();
    a.engine.resume(&id);
    let t = a.wait_transfer("sender failed", status(TransferStatus::Failed)).await;
    assert_eq!(t.error, Some(TransferError::FileChanged));
    let t = b.wait_transfer("receiver failed", status(TransferStatus::Failed)).await;
    assert_eq!(t.error, Some(TransferError::FileChanged));

    a.stop().await;
    b.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn code_pairing_then_trusted_auto_accept() {
    let mem = MemoryLookup::new();
    let mut a = Node::new(&mem, "Hakim's laptop", None).await;
    let mut b = Node::new(&mem, "Work phone", None).await;

    let code = a.engine.create_code();
    assert_eq!(code.uri, format!("nhakobeam://pair/{}", code.words.join("-")));
    a.wait("code shown", |e| matches!(e, EngineEvent::Code { code: Some(_) }).then_some(())).await;

    // Typos are caught before any network work.
    let r = b.engine.redeem_code(&[code.words[0].clone(), "notaword".into(), code.words[2].clone()]).await;
    assert_eq!(r, RedeemResult::Err(RedeemFailure::UnknownWord { index: 1 }));

    let r = b.engine.redeem_code(&code.words).await;
    let RedeemResult::Ok(dev) = r else { panic!("redeem failed: {r:?}") };
    assert_eq!(dev.id, a.id());
    assert_eq!(dev.name, "Hakim's laptop");

    let (dev_a, via_a, check_a) = a
        .wait("paired on host", |e| match e {
            EngineEvent::Paired { device, via, check } => Some((device.clone(), *via, check.clone())),
            _ => None,
        })
        .await;
    let (_, via_b, check_b) = b
        .wait("paired on joiner", |e| match e {
            EngineEvent::Paired { device, via, check } => Some((device.clone(), *via, check.clone())),
            _ => None,
        })
        .await;
    assert_eq!(dev_a.id, b.id());
    assert_eq!((via_a, via_b), (PairVia::Shown, PairVia::Entered));
    assert_eq!(check_a, check_b, "both screens must show the same check");
    assert_eq!(check_a.len(), 9);

    // The code is used up.
    a.wait("code gone", |e| matches!(e, EngineEvent::Code { code: None }).then_some(())).await;
    assert!(a.engine.current_code().is_none());
    let again = b.engine.redeem_code(&code.words).await;
    assert_eq!(again, RedeemResult::Err(RedeemFailure::NoMatch));

    // Save each other as trusted; B accepts from A automatically.
    a.engine.set_trusted(&b.id(), true);
    b.engine.set_trusted(&a.id(), true);
    b.engine.set_auto_accept(&a.id(), true);
    let trusted = b.engine.devices().into_iter().find(|d| d.id == a.id()).unwrap();
    assert!(trusted.trusted && trusted.auto_accept);

    let f = make_file(&a.dir.join("out"), "auto.txt", 5000);
    a.engine.send(&b.id(), std::slice::from_ref(&f)).unwrap();
    b.wait_transfer("auto accepted and done", status(TransferStatus::Done)).await;
    same_bytes(&f, &b.save().join("auto.txt"));

    a.stop().await;
    b.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn wrong_code_finds_nothing() {
    let mem = MemoryLookup::new();
    let a = Node::new(&mem, "A", None).await;
    let b = Node::new(&mem, "B", None).await;
    let code = a.engine.create_code();
    let mut wrong = code.words.clone();
    wrong[2] = if wrong[2] == "zebra" { "acorn".into() } else { "zebra".into() };
    assert_eq!(b.engine.redeem_code(&wrong).await, RedeemResult::Err(RedeemFailure::NoMatch));
    assert!(a.engine.current_code().is_some(), "a wrong guess elsewhere doesn't burn the real code");
    a.stop().await;
    b.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn trusted_state_survives_restart() {
    let mem = MemoryLookup::new();
    let a = Node::new(&mem, "A", None).await;
    let b = Node::new(&mem, "B", None).await;
    let code = a.engine.create_code();
    assert!(matches!(b.engine.redeem_code(&code.words).await, RedeemResult::Ok(_)));
    b.engine.set_trusted(&a.id(), true);
    let b_id = b.id();
    let a_id = a.id();
    let cfg_dir = b.dir.clone();
    b.engine.stop().await;

    let mut cfg = EngineConfig::new(cfg_dir.join("config"));
    cfg.network = Some(Network::LocalOnly);
    cfg.mdns = false;
    cfg.memory_lookup = Some(mem.clone());
    let b2 = Engine::start(cfg).await.unwrap();
    assert_eq!(device_id(&b2.id()), b_id, "same identity after restart");
    let d = b2.devices().into_iter().find(|d| d.id == a_id).expect("trusted device remembered");
    assert!(d.trusted);
    assert_eq!(b2.me().name, "B");
    b2.stop().await;
    a.stop().await;
    let _ = std::fs::remove_dir_all(cfg_dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn damaged_part_file_is_caught_and_resent() {
    let mem = MemoryLookup::new();
    let mut a = Node::new(&mem, "A", Some(4_000_000)).await;
    let mut b = Node::new(&mem, "B", None).await;
    let f = make_file(&a.dir.join("out"), "scan.tiff", 9_000_000);
    let id = a.engine.send(&b.id(), std::slice::from_ref(&f)).unwrap();
    let req = b.wait_incoming().await;
    b.engine.accept(&req, false, false);
    b.wait_transfer("bytes moving", |t| t.done_bytes > 2_000_000).await;
    a.engine.pause(&id);
    b.wait_transfer("paused", status(TransferStatus::Paused)).await;

    // Damage a byte the receiver already wrote, then drop the connection. The next session
    // re-hashes the part file from disk, so the final check must fail.
    let part = b.save().join("scan.tiff.nhakopart");
    let mut bytes = std::fs::read(&part).unwrap();
    bytes[100] ^= 0xff;
    std::fs::write(&part, bytes).unwrap();
    b.engine.test_drop_connection(&id);

    let t = a.wait_transfer("sender sees integrity error", status(TransferStatus::Failed)).await;
    assert_eq!(t.error, Some(TransferError::Integrity));
    let t = b.wait_transfer("receiver integrity error", status(TransferStatus::Failed)).await;
    assert_eq!(t.error, Some(TransferError::Integrity));
    assert!(!part.exists(), "a damaged part file is thrown away");

    // Try again resends that file from the start.
    a.engine.retry(&id);
    a.wait_transfer("sender done", status(TransferStatus::Done)).await;
    b.wait_transfer("receiver done", status(TransferStatus::Done)).await;
    same_bytes(&f, &b.save().join("scan.tiff"));

    a.stop().await;
    b.stop().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn history_lists_finished_transfers_and_survives_restart() {
    let mem = MemoryLookup::new();
    let mut a = Node::new(&mem, "Aisyah's laptop", None).await;
    let mut b = Node::new(&mem, "Studio PC", None).await;
    let f = make_file(&a.dir.join("out"), "song.flac", 400_000);

    // One that arrives, then one that's declined.
    let done_id = a.engine.send(&b.id(), std::slice::from_ref(&f)).unwrap();
    let req = b.wait_incoming().await;
    b.engine.accept(&req, false, false);
    let entry = b
        .wait("history entry", |e| match e {
            EngineEvent::History { entry } if entry.id == done_id => Some(entry.clone()),
            _ => None,
        })
        .await;
    assert_eq!(entry.status, TransferStatus::Done);
    assert_eq!(entry.direction, TransferDirection::Receive);
    assert_eq!(entry.device_name, "Aisyah's laptop");
    assert_eq!(entry.done_bytes, 400_000);
    assert!(entry.average_speed > 0.0, "bytes moved, so there's a speed");
    assert!(entry.finished_at >= entry.started_at);
    a.wait_transfer("sender done", |t| t.id == done_id && t.status == TransferStatus::Done).await;

    let declined_id = a.engine.send(&b.id(), std::slice::from_ref(&f)).unwrap();
    let req = b.wait_incoming().await;
    b.engine.decline(&req);
    a.wait_transfer("declined", |t| t.id == declined_id && t.status == TransferStatus::Declined)
        .await;

    let sent: Vec<_> = a.engine.history().into_iter().map(|e| (e.id, e.status)).collect();
    assert_eq!(
        sent,
        [(declined_id, TransferStatus::Declined), (done_id.clone(), TransferStatus::Done)],
        "newest first"
    );
    assert_eq!(a.engine.snapshot().history.len(), 2);

    // After a restart, History is still there and still opens the received file.
    let cfg_dir = b.dir.join("config");
    b.engine.stop().await;
    let mut cfg = EngineConfig::new(&cfg_dir);
    cfg.network = Some(Network::LocalOnly);
    cfg.mdns = false;
    cfg.memory_lookup = Some(mem.clone());
    let b2 = Engine::start(cfg).await.unwrap();
    assert_eq!(b2.history().len(), 1);
    assert_eq!(b2.saved_files(&done_id), vec![b.save().join("song.flac")]);
    b2.clear_history();
    assert!(b2.history().is_empty());
    assert_eq!(b2.saved_files(&done_id), Vec::<PathBuf>::new());
    b2.stop().await;
    a.stop().await;
    let _ = std::fs::remove_dir_all(&b.dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn lan_discovery_setting_is_saved() {
    let mem = MemoryLookup::new();
    let a = Node::new(&mem, "A", None).await;
    assert!(a.engine.lan_discovery(), "on by default");
    a.engine.set_lan_discovery(false).unwrap();
    let cfg_dir = a.dir.join("config");
    a.engine.stop().await;
    let mut cfg = EngineConfig::new(&cfg_dir);
    cfg.network = Some(Network::LocalOnly);
    cfg.memory_lookup = Some(mem.clone());
    // mDNS allowed for this run, but the setting turns it off: nothing is announced or looked for.
    cfg.mdns = true;
    let a2 = Engine::start(cfg).await.unwrap();
    assert!(!a2.lan_discovery());
    assert_ne!(a2.snapshot().discovery, nhako_core::DiscoveryState::Blocked, "off by choice isn't blocked");
    a2.stop().await;
    let _ = std::fs::remove_dir_all(&a.dir);
}

/// Throughput on localhost, for comparing build settings (Phase 8: smaller APK without losing speed).
/// `cargo test --release -p nhako-core --test engine throughput -- --ignored --nocapture`
#[tokio::test(flavor = "multi_thread")]
#[ignore = "benchmark: run by hand"]
async fn throughput() {
    let mem = MemoryLookup::new();
    let a = Node::new(&mem, "A", None).await;
    let mut b = Node::new(&mem, "B", None).await;
    let size = 512 * 1024 * 1024;
    let f = make_file(&a.dir.join("out"), "big.bin", size);
    let start = std::time::Instant::now();
    let id = a.engine.send(&b.id(), &[f]).unwrap();
    let req = b.wait_incoming().await;
    b.engine.accept(&req, false, false);
    b.wait_transfer("done", |t| t.id == id && t.status == TransferStatus::Done).await;
    let secs = start.elapsed().as_secs_f64();
    println!("THROUGHPUT {:.0} MB/s ({size} bytes in {secs:.2} s)", size as f64 / secs / 1e6);
    a.stop().await;
    b.stop().await;
}

/// Pairs two nodes with a code and saves each as trusted on the other.
async fn pair_and_trust(a: &mut Node, b: &mut Node) {
    let code = a.engine.create_code();
    let r = b.engine.redeem_code(&code.words).await;
    assert!(matches!(r, RedeemResult::Ok(_)), "pairing failed: {r:?}");
    a.engine.set_trusted(&b.id(), true);
    b.engine.set_trusted(&a.id(), true);
}

const WAKE_TOKEN: &str = "c3VwZXItc2VjcmV0:APA91bHn-test_token-0123456789abcdefghijklmnopqrstuvwxyz";

/// A pretend wake service: answers 200 and passes on each token it was asked to wake.
async fn fake_wake_service() -> (url::Url, tokio::sync::mpsc::UnboundedReceiver<String>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = url::Url::parse(&format!("http://127.0.0.1:{}/wake", listener.local_addr().unwrap().port())).unwrap();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(async move {
        while let Ok((mut sock, _)) = listener.accept().await {
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut got = Vec::new();
                let mut buf = [0u8; 4096];
                loop {
                    let n = sock.read(&mut buf).await.unwrap_or(0);
                    if n == 0 {
                        return;
                    }
                    got.extend_from_slice(&buf[..n]);
                    let Some(end) = got.windows(4).position(|w| w == b"\r\n\r\n") else { continue };
                    let head = String::from_utf8_lossy(&got[..end]).to_ascii_lowercase();
                    let len: usize = head
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length:"))
                        .and_then(|v| v.trim().parse().ok())
                        .unwrap_or(0);
                    if got.len() < end + 4 + len {
                        continue;
                    }
                    let body: serde_json::Value = serde_json::from_slice(&got[end + 4..end + 4 + len]).unwrap();
                    let _ = tx.send(body["token"].as_str().unwrap_or_default().to_string());
                    let _ = sock.write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 0\r\nconnection: close\r\n\r\n").await;
                    return;
                }
            });
        }
    });
    (url, rx)
}

#[tokio::test(flavor = "multi_thread")]
async fn wakes_a_sleeping_phone_to_receive() {
    let mem = MemoryLookup::new();
    let (url, mut woken) = fake_wake_service().await;
    let mut phone = Node::new(&mem, "Aisyah's phone", None).await;
    let mut laptop = Node::with(&mem, "Studio PC", None, |cfg| {
        cfg.wake_url = Some(url.clone());
        cfg.wake_for = Duration::from_secs(30);
    })
    .await;
    pair_and_trust(&mut phone, &mut laptop).await;
    phone.engine.set_auto_accept(&laptop.id(), true);
    phone.engine.set_wake_token(Some(WAKE_TOKEN.into()));

    // The laptop learns the token from the phone's hello, and only because it's trusted.
    laptop.engine.rescan();
    let state = laptop.dir.join("config/state.json");
    let learned = async {
        while !std::fs::read_to_string(&state).unwrap_or_default().contains(WAKE_TOKEN) {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    };
    tokio::time::timeout(WAIT, learned).await.expect("the laptop never learned the wake token");

    // The phone goes to sleep: its engine stops, as when Android pauses the app.
    let phone_dir = phone.dir.clone();
    let phone_id = phone.id();
    phone.engine.stop().await;
    laptop.engine.rescan();
    laptop
        .wait("phone shown as asleep but wakeable", |e| match e {
            EngineEvent::Device { device } if device.id == phone_id && !device.reachable && device.wakeable => Some(()),
            _ => None,
        })
        .await;

    // Sending wakes it: the wake service gets the token, the phone starts again (as the push
    // would start it), and the transfer goes through.
    let f = make_file(&laptop.dir.join("out"), "while asleep.jpg", 250_000);
    let id = laptop.engine.send(&phone_id, std::slice::from_ref(&f)).unwrap();
    laptop.wait_transfer("waking", |t| t.id == id && t.waking).await;
    let token = tokio::time::timeout(WAIT, woken.recv()).await.unwrap().unwrap();
    assert_eq!(token, WAKE_TOKEN);
    let mut cfg = EngineConfig::new(phone_dir.join("config"));
    cfg.save_dir = Some(phone_dir.join("save"));
    cfg.network = Some(Network::LocalOnly);
    cfg.mdns = false;
    cfg.kdf = KdfCost::TEST;
    cfg.memory_lookup = Some(mem.clone());
    cfg.wake_url = None;
    let phone2 = Engine::start(cfg).await.unwrap();
    let done = laptop
        .wait_transfer("sent after waking", |t| t.id == id && t.status == TransferStatus::Done)
        .await;
    assert!(!done.waking);
    same_bytes(&f, &phone_dir.join("save/while asleep.jpg"));
    phone2.stop().await;
    laptop.stop().await;
    let _ = std::fs::remove_dir_all(phone_dir);
}

#[tokio::test(flavor = "multi_thread")]
async fn wake_token_goes_only_to_trusted_devices() {
    let mem = MemoryLookup::new();
    let mut phone = Node::new(&mem, "Phone", None).await;
    let mut friend = Node::with(&mem, "Friend", None, |cfg| cfg.wake_url = url::Url::parse("https://wake.example.com/wake").ok()).await;
    phone.engine.set_wake_token(Some(WAKE_TOKEN.into()));
    // Pair, but the phone doesn't save the friend as trusted.
    let code = phone.engine.create_code();
    assert!(matches!(friend.engine.redeem_code(&code.words).await, RedeemResult::Ok(_)));
    friend.engine.set_trusted(&phone.id(), true);
    let f = make_file(&friend.dir.join("out"), "hi.txt", 10);
    let id = friend.engine.send(&phone.id(), &[f]).unwrap();
    let req = phone.wait_incoming().await;
    phone.engine.accept(&req, false, false);
    friend.wait_transfer("done", |t| t.id == id && t.status == TransferStatus::Done).await;
    let saved = std::fs::read_to_string(friend.dir.join("config/state.json")).unwrap();
    assert!(!saved.contains(WAKE_TOKEN), "an untrusted device must not learn the wake token");
    phone.stop().await;
    friend.stop().await;
}

#[tokio::test]
async fn removing_a_trusted_device_that_is_away_takes_it_off_the_list() {
    let mem = MemoryLookup::new();
    let mut laptop = Node::new(&mem, "Laptop", None).await;
    let mut phone = Node::new(&mem, "Phone", None).await;
    let mut tablet = Node::new(&mem, "Tablet", None).await;
    pair_and_trust(&mut laptop, &mut phone).await;
    pair_and_trust(&mut laptop, &mut tablet).await;
    let (phone_id, tablet_id) = (phone.id(), tablet.id());

    // The phone goes away; the laptop notices on its next check.
    phone.stop().await;
    laptop.engine.rescan();
    laptop
        .wait("the phone shown as away", |e| match e {
            EngineEvent::Device { device } if device.id == phone_id && !device.reachable => Some(()),
            _ => None,
        })
        .await;
    laptop.engine.set_trusted(&phone_id, false);
    laptop
        .wait("the phone taken off the list", |e| match e {
            EngineEvent::DeviceLost { device_id } if *device_id == phone_id => Some(()),
            _ => None,
        })
        .await;

    // The tablet is still in reach: it stays, just no longer trusted.
    laptop.engine.set_trusted(&tablet_id, false);
    laptop
        .wait("the tablet shown as untrusted", |e| match e {
            EngineEvent::Device { device } if device.id == tablet_id && !device.trusted && device.reachable => Some(()),
            _ => None,
        })
        .await;
    laptop.stop().await;
    tablet.stop().await;
}

/// How the Android app hands over shared files: as `/proc/self/fd/<n>` of a file it opened, whose
/// own path the engine may not be allowed to open again.
#[cfg(unix)]
#[tokio::test]
async fn sends_a_file_given_as_an_open_descriptor() {
    use std::os::fd::AsRawFd;
    let mem = MemoryLookup::new();
    let mut phone = Node::new(&mem, "Phone", None).await;
    let mut laptop = Node::new(&mem, "Laptop", None).await;
    pair_and_trust(&mut phone, &mut laptop).await;
    let f = make_file(&phone.dir.join("out"), "IMG_2041.jpg", 3 * 1024 * 1024 + 17);
    let original = std::fs::read(&f).unwrap();
    let held = std::fs::File::open(&f).unwrap();
    std::fs::remove_file(&f).unwrap();
    let via_fd = PathBuf::from(format!("/proc/self/fd/{}", held.as_raw_fd()));
    let id = phone.engine.send(&laptop.id(), &[via_fd]).unwrap();
    let req = laptop.wait_incoming().await;
    laptop.engine.accept(&req, false, false);
    phone.wait_transfer("sent", |t| t.id == id && t.status == TransferStatus::Done).await;
    laptop.wait_transfer("received", status(TransferStatus::Done)).await;
    // (Named after the path here; the Android app passes the real name alongside, through ffi.)
    let saved: Vec<_> = std::fs::read_dir(laptop.save()).unwrap().map(|e| e.unwrap().path()).collect();
    assert_eq!(saved.len(), 1);
    assert_eq!(std::fs::read(&saved[0]).unwrap(), original);
    drop(held);
    phone.stop().await;
    laptop.stop().await;
}
