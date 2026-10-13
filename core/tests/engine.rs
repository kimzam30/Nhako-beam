//! Two (or three) real engines talking nhako/1 over QUIC on localhost.
//! No relay, no mDNS: addresses come from a shared in-memory table.

use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use nhako_core::{
    Engine, EngineConfig, EngineError, EngineEvent, IncomingRequest, MemoryLookup, Network, Outgoing, PairVia, RedeemFailure, RedeemResult, Side, Transfer,
    TransferDirection, TransferError, TransferStatus, TrustOutcome, WhoCanSend, codekeys::KdfCost, device_id, engine_limits, files,
};
use tokio::sync::broadcast;

const WAIT: Duration = Duration::from_secs(30);

struct Node {
    engine: Engine,
    events: broadcast::Receiver<EngineEvent>,
    dir: PathBuf,
    /// How long `wait` waits. Benchmarks raise it.
    patience: Duration,
    /// Transfers as events have described them so far.
    seen: std::collections::HashMap<String, Transfer>,
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
        // Never GitHub from a test.
        cfg.update_url = None;
        cfg.save_dir = Some(dir.join("save"));
        cfg.network = Some(Network::LocalOnly);
        cfg.mdns = false;
        cfg.beacon = None;
        cfg.kdf = KdfCost::TEST;
        cfg.reconnect_for = Duration::from_secs(10);
        cfg.code_lookup_for = Duration::from_secs(3);
        cfg.memory_lookup = Some(mem.clone());
        cfg.test_rate_limit = rate;
        cfg.wake_url = None;
        tweak(&mut cfg);
        let engine = Engine::start(cfg).await.unwrap();
        engine.rename_self(name);
        // Most tests send between devices that never paired; only_your_devices_can_send_by_default
        // covers the default.
        engine.set_who_can_send(WhoCanSend::EveryoneNearby).unwrap();
        let events = engine.subscribe();
        Self {
            engine,
            events,
            dir,
            patience: WAIT,
            seen: Default::default(),
        }
    }

    fn id(&self) -> String {
        device_id(&self.engine.id())
    }

    fn save(&self) -> PathBuf {
        self.dir.join("save")
    }

    /// Waits for an event matching `f`, skipping others.
    async fn wait<T>(&mut self, what: &str, mut f: impl FnMut(&EngineEvent) -> Option<T>) -> T {
        let patience = self.patience;
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
        tokio::time::timeout(patience, fut)
            .await
            .unwrap_or_else(|_| panic!("timed out waiting for {what}"))
    }

    /// Waits until a transfer matches `f`, as the app would see it: progress events merged into
    /// the last full one.
    async fn wait_transfer(&mut self, what: &str, f: impl Fn(&Transfer) -> bool) -> Transfer {
        let engine = self.engine.clone();
        let mut seen = std::mem::take(&mut self.seen);
        let found = self
            .wait(what, |e| {
                let t = match e {
                    EngineEvent::Transfer { transfer } => {
                        seen.insert(transfer.id.clone(), transfer.clone());
                        seen.get(&transfer.id)
                    }
                    EngineEvent::TransferProgress {
                        id,
                        done_bytes,
                        speed,
                        moving_since,
                        file_bytes,
                    } => {
                        if !seen.contains_key(id) {
                            seen.insert(id.clone(), engine.transfer(id)?);
                        }
                        let t = seen.get_mut(id)?;
                        t.done_bytes = *done_bytes;
                        t.speed = *speed;
                        t.moving_since = *moving_since;
                        t.file_bytes.extend(file_bytes.iter().map(|(k, v)| (k.clone(), *v)));
                        Some(&*t)
                    }
                    _ => None,
                };
                t.filter(|t| f(t)).cloned()
            })
            .await;
        self.seen = seen;
        found
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

/// Everything under `dir`, folders included.
fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut out = vec![];
    for e in std::fs::read_dir(dir).map(|d| d.flatten().collect::<Vec<_>>()).unwrap_or_default() {
        out.push(e.path());
        if e.path().is_dir() {
            out.extend(walk(&e.path()));
        }
    }
    out
}

/// What `walk_folder` finds, ready to send.
fn folder_to_send(root: &Path) -> Vec<Outgoing> {
    files::walk_folder(root, 1000)
        .unwrap()
        .into_iter()
        .map(|(path, folder)| Outgoing {
            name: path.file_name().unwrap().to_string_lossy().into_owned(),
            path,
            folder: Some(folder),
        })
        .collect()
}

impl Node {
    async fn wait_request(&mut self) -> IncomingRequest {
        self.wait("incoming request", |e| match e {
            EngineEvent::Incoming { request } => Some(request.clone()),
            _ => None,
        })
        .await
    }
}

/// A folder keeps its shape on the other side, in a new folder of its own when the name is taken,
/// and a cancelled one leaves nothing behind.
#[tokio::test(flavor = "multi_thread")]
async fn sends_a_folder_and_keeps_its_shape() {
    let mem = MemoryLookup::new();
    let a = Node::new(&mem, "Laptop", None).await;
    let mut b = Node::new(&mem, "Phone", None).await;
    let root = a.dir.join("out/Trip");
    let inside = [
        make_file(&root, "a.jpg", 200_000),
        make_file(&root.join("day 2"), "b.jpg", 50_000),
        make_file(&root.join("day 2/raw"), "c.raw", 0),
    ];
    let loose = make_file(&a.dir.join("out"), "notes.txt", 10);
    let mut out = folder_to_send(&root);
    out.push(Outgoing {
        path: loose.clone(),
        name: "notes.txt".into(),
        folder: None,
    });
    let id = a.engine.send_files(&b.id(), &out).unwrap();
    let req = b.wait_request().await;
    let folders: Vec<Option<&str>> = req.files.iter().map(|f| f.folder.as_deref()).collect();
    assert_eq!(folders, [Some("Trip"), Some("Trip/day 2"), Some("Trip/day 2/raw"), None]);
    b.engine.accept(&req.id, false, false);
    b.wait_transfer("folder received", |t| t.id == id && t.status == TransferStatus::Done).await;
    same_bytes(&inside[0], &b.save().join("Trip/a.jpg"));
    same_bytes(&inside[1], &b.save().join("Trip/day 2/b.jpg"));
    same_bytes(&inside[2], &b.save().join("Trip/day 2/raw/c.raw"));
    same_bytes(&loose, &b.save().join("notes.txt"));
    no_part_files(&b.save().join("Trip"));
    no_part_files(&b.save().join("Trip/day 2"));
    assert_eq!(b.engine.saved_files(&id).len(), 4);

    // The same folder again: it gets a folder of its own, and nothing mixes with the first.
    std::fs::write(b.save().join("Trip/mine.txt"), b"mine").unwrap();
    let id2 = a.engine.send_files(&b.id(), &out[..3]).unwrap();
    let req = b.wait_request().await;
    b.engine.accept(&req.id, false, false);
    b.wait_transfer("second folder", |t| t.id == id2 && t.status == TransferStatus::Done).await;
    same_bytes(&inside[1], &b.save().join("Trip (1)/day 2/b.jpg"));
    assert!(!b.save().join("Trip (1)/mine.txt").exists());
    assert!(b.save().join("Trip/mine.txt").exists());

    // Cancelled part way: the folder made for it goes too.
    let slow = Node::new(&mem, "Slow laptop", Some(2_000_000)).await;
    let big = slow.dir.join("out/Big");
    make_file(&big.join("inner"), "huge.bin", 8_000_000);
    make_file(&big.join("inner"), "z-after.bin", 1000);
    let id3 = slow.engine.send_files(&b.id(), &folder_to_send(&big)).unwrap();
    let req = b.wait_request().await;
    b.engine.accept(&req.id, false, false);
    b.wait_transfer("bytes moving", |t| t.id == id3 && t.done_bytes > 300_000).await;
    assert!(b.save().join("Big/inner").exists());
    b.engine.cancel(&id3);
    b.wait_transfer("cancelled", |t| t.id == id3 && t.status == TransferStatus::Cancelled).await;
    tokio::time::sleep(Duration::from_millis(300)).await;
    let left: Vec<_> = walk(&b.save().join("Big"));
    assert!(!b.save().join("Big").exists(), "the cancelled folder was cleared away: {left:?}");

    a.stop().await;
    b.stop().await;
    slow.stop().await;
}

/// Text travels in the request: the card can show it, and nothing is written to disk.
#[tokio::test(flavor = "multi_thread")]
async fn text_comes_in_the_request_and_never_touches_the_disk() {
    let mem = MemoryLookup::new();
    let mut a = Node::new(&mem, "Laptop", None).await;
    let mut b = Node::new(&mem, "Phone", None).await;
    let id = a.engine.send_text(&b.id(), "Meet at 6\r\nhttps://example.com/x\u{202E}").unwrap();
    let req = b.wait_request().await;
    assert_eq!(req.text.as_deref(), Some("Meet at 6\nhttps://example.com/x"));
    assert_eq!(req.files.len(), 1, "older builds get it as a file");
    b.engine.accept(&req.id, false, false);
    let got = b.wait_transfer("text received", |t| t.id == id && t.status == TransferStatus::Done).await;
    assert_eq!(got.text.as_deref(), Some("Meet at 6\nhttps://example.com/x"));
    assert_eq!(got.done_bytes, got.total_bytes);
    let sent = a.wait_transfer("text sent", |t| t.id == id && t.status == TransferStatus::Done).await;
    assert!(sent.text.is_some());
    let saved: Vec<_> = std::fs::read_dir(b.save()).map(|d| d.flatten().collect()).unwrap_or_default();
    assert!(saved.is_empty(), "nothing saved: {saved:?}");
    assert!(b.engine.saved_files(&id).is_empty());
    assert_eq!(b.engine.history()[0].text.as_deref(), Some("Meet at 6\nhttps://example.com/x"));
    tokio::time::sleep(Duration::from_millis(200)).await;
    let left: Vec<_> = std::fs::read_dir(a.dir.join("config/outgoing"))
        .map(|d| d.flatten().collect())
        .unwrap_or_default();
    assert!(left.is_empty(), "the sender's copy is gone: {left:?}");

    assert!(matches!(a.engine.send_text(&b.id(), "   "), Err(EngineError::NoFiles)));
    let long = "x".repeat(engine_limits::TEXT_MAX + 1);
    assert!(matches!(a.engine.send_text(&b.id(), &long), Err(EngineError::TextTooLong)));
    a.stop().await;
    b.stop().await;
}

/// One HTTP request the way a browser makes it: returns the status, the head and the body.
async fn http_get(url: &str, path: &str, cookie: Option<&str>, extra: &str) -> (u16, String, Vec<u8>) {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let host = url.trim_start_matches("http://").split('/').next().unwrap().to_string();
    let mut s = tokio::net::TcpStream::connect(&host).await.unwrap();
    let cookie = cookie.map(|c| format!("Cookie: {c}\r\n")).unwrap_or_default();
    let req =
        format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nUser-Agent: Mozilla/5.0 (Linux; Android 14) Chrome/129.0 Mobile Safari/537.36\r\n{cookie}{extra}\r\n");
    s.write_all(req.as_bytes()).await.unwrap();
    let mut all = Vec::new();
    s.read_to_end(&mut all).await.unwrap();
    let end = all.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
    let head = String::from_utf8(all[..end].to_vec()).unwrap();
    let status = head[9..12].parse().unwrap();
    (status, head, all[end + 4..].to_vec())
}

fn set_cookie(head: &str) -> Option<String> {
    head.lines()
        .find_map(|l| l.strip_prefix("Set-Cookie: "))
        .map(|c| c.split(';').next().unwrap().to_string())
}

/// Share via link: a browser waits until it's let in, sees nothing before, then downloads a
/// file (also part of one), and the zip of everything. One turned away stays out, and Stop ends it.
#[tokio::test(flavor = "multi_thread")]
async fn share_via_link_lets_in_only_who_you_allow() {
    let mem = MemoryLookup::new();
    let mut a = Node::with(&mem, "Laptop", None, |c| c.test_share_ip = Some([127, 0, 0, 1].into())).await;
    let photo = make_file(&a.dir, "beach.jpg", 300_000);
    let root = a.dir.join("send");
    std::fs::create_dir_all(root.join("Trip")).unwrap();
    let note = make_file(&root.join("Trip"), "note.txt", 10);
    let out = [
        Outgoing {
            path: photo.clone(),
            name: "beach.jpg".into(),
            folder: None,
        },
        Outgoing {
            path: note.clone(),
            name: "note.txt".into(),
            folder: Some("Trip".into()),
        },
    ];
    let share = a.engine.share_link(&out, Some("Wi-Fi password is on the fridge")).unwrap();
    assert_eq!(share.total_bytes, 300_010);
    let path = share.url.splitn(4, '/').nth(3).map(|p| format!("/{p}")).unwrap();
    let key = path.trim_matches('/').to_string();
    assert_eq!(key.len(), 16);

    // A wrong key, or no key, shows nothing.
    let (status, _, body) = http_get(&share.url, "/aaaaaaaaaaaaaaaa/", None, "").await;
    assert_eq!(status, 404);
    assert!(String::from_utf8_lossy(&body).contains("Nothing is shared here now"));
    assert_eq!(http_get(&share.url, "/", None, "").await.0, 404);
    assert!(a.engine.link_share().unwrap().visitors.is_empty());

    // A browser opens it, gets a cookie, and asks.
    let (status, head, _) = http_get(&share.url, &path, None, "").await;
    assert_eq!(status, 200);
    assert!(head.contains("Content-Security-Policy"));
    let cookie = set_cookie(&head).expect("a cookie for the browser");
    let visitor = a
        .wait("a browser asks", |e| match e {
            EngineEvent::LinkShare { share: Some(s) } => s.visitors.first().cloned(),
            _ => None,
        })
        .await;
    assert_eq!(visitor.label, "Chrome on Android");
    assert_eq!(visitor.address, "127.0.0.1");
    let (_, _, state) = http_get(&share.url, &format!("{path}state"), Some(&cookie), "").await;
    assert!(String::from_utf8_lossy(&state).contains(r#""state":"asking""#));
    // Nothing before it's let in: not the list, not a file, not the zip.
    assert_eq!(http_get(&share.url, &format!("{path}list"), Some(&cookie), "").await.0, 403);
    assert_eq!(http_get(&share.url, &format!("{path}f/0/beach.jpg"), Some(&cookie), "").await.0, 403);
    assert_eq!(http_get(&share.url, &format!("{path}all.zip"), Some(&cookie), "").await.0, 403);

    a.engine.answer_visitor(&visitor.id, true);
    let (_, _, list) = http_get(&share.url, &format!("{path}list"), Some(&cookie), "").await;
    let list: serde_json::Value = serde_json::from_slice(&list).unwrap();
    assert_eq!(list["from"], "Laptop");
    assert_eq!(list["text"], "Wi-Fi password is on the fridge");
    assert_eq!(list["files"][1]["folder"], "Trip");
    let (status, head, body) = http_get(&share.url, &format!("{path}f/0/beach.jpg"), Some(&cookie), "").await;
    assert_eq!(status, 200);
    assert!(head.contains("filename*=UTF-8''beach.jpg"), "{head}");
    assert_eq!(body, std::fs::read(&photo).unwrap());
    let (status, head, part) = http_get(&share.url, &format!("{path}f/0/beach.jpg"), Some(&cookie), "Range: bytes=100000-\r\n").await;
    assert_eq!(status, 206);
    assert!(head.contains("Content-Range: bytes 100000-299999/300000"), "{head}");
    assert_eq!(part, std::fs::read(&photo).unwrap()[100_000..]);
    let (status, head, zip) = http_get(&share.url, &format!("{path}all.zip"), Some(&cookie), "").await;
    assert_eq!(status, 200);
    assert_eq!(head.lines().find_map(|l| l.strip_prefix("Content-Length: ")).unwrap(), zip.len().to_string());
    assert_eq!(list["zipBytes"], zip.len() as u64);
    let zip = zip::ZipArchive::new(std::io::Cursor::new(zip)).unwrap();
    assert_eq!(
        zip.file_names().collect::<std::collections::BTreeSet<_>>(),
        ["beach.jpg", "Trip/note.txt"].into()
    );
    let done = a.engine.link_share().unwrap().visitors[0].clone();
    assert_eq!(done.downloads, 3);
    assert_eq!((done.active_bytes, done.active_total), (0, 0));

    // A second browser is turned away, and stays out.
    let (_, head, _) = http_get(&share.url, &path, None, "").await;
    let other = set_cookie(&head).unwrap();
    let second = a.engine.link_share().unwrap().visitors[1].id.clone();
    a.engine.answer_visitor(&second, false);
    let (_, _, state) = http_get(&share.url, &format!("{path}state"), Some(&other), "").await;
    assert!(String::from_utf8_lossy(&state).contains(r#""state":"denied""#));
    assert_eq!(http_get(&share.url, &format!("{path}f/1/note.txt"), Some(&other), "").await.0, 403);
    // A made-up cookie is a stranger, not the first browser.
    assert_eq!(http_get(&share.url, &format!("{path}f/1/note.txt"), Some("nbv=guess"), "").await.0, 403);

    a.engine.stop_link();
    assert!(a.engine.link_share().is_none());
    assert!(a.engine.snapshot().link_share.is_none());
    let gone = tokio::net::TcpStream::connect(share.url.trim_start_matches("http://").split('/').next().unwrap()).await;
    if let Ok(mut s) = gone {
        // The port may still take the connection for a moment; nothing is served on it.
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let _ = s.write_all(format!("GET {path} HTTP/1.1\r\n\r\n").as_bytes()).await;
        let mut buf = Vec::new();
        let _ = tokio::time::timeout(Duration::from_secs(2), s.read_to_end(&mut buf)).await;
        assert!(!String::from_utf8_lossy(&buf).contains("200 OK"));
    }
    assert!(matches!(a.engine.share_link(&[], None), Err(EngineError::NoFiles)));
    a.stop().await;
}

/// A link ends by itself once its time is up and nothing is downloading.
#[tokio::test(flavor = "multi_thread")]
async fn a_shared_link_ends_by_itself() {
    let mem = MemoryLookup::new();
    let mut a = Node::with(&mem, "Laptop", None, |c| {
        c.test_share_ip = Some([127, 0, 0, 1].into());
        c.share_for = Duration::from_secs(1);
    })
    .await;
    a.engine.share_link(&[], Some("hello")).unwrap();
    a.wait("the link to end", |e| matches!(e, EngineEvent::LinkShare { share: None }).then_some(()))
        .await;
    assert!(a.engine.link_share().is_none());
    a.stop().await;
}

/// The daily look for a new version: found once, kept in the snapshot, gone when turned off.
#[tokio::test(flavor = "multi_thread")]
async fn finds_a_newer_version_and_forgets_it_when_turned_off() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    // A stand-in for GitHub's list of releases, counting how often it's asked.
    let server = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = server.local_addr().unwrap().port();
    let asked = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count = asked.clone();
    tokio::spawn(async move {
        loop {
            let Ok((mut s, _)) = server.accept().await else { return };
            count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let mut buf = [0u8; 2048];
            let _ = s.read(&mut buf).await;
            let body = r#"[{"tag_name":"v99.0.0","html_url":"https://github.com/kimzam30/Nhako-beam/releases/tag/v99.0.0","prerelease":true}]"#;
            let res = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = s.write_all(res.as_bytes()).await;
        }
    });
    let mem = MemoryLookup::new();
    let url = format!("http://127.0.0.1:{port}/releases");
    let mut a = Node::with(&mem, "Laptop", None, |c| c.update_url = Some(url.parse().unwrap())).await;
    let found = a
        .wait("a newer version", |e| match e {
            EngineEvent::Update { update: Some(u) } => Some(u.clone()),
            _ => None,
        })
        .await;
    assert_eq!(found.version, "99.0.0");
    assert_eq!(a.engine.snapshot().update, Some(found));
    assert_eq!(a.engine.update_check(), Some(true));
    // Not again the same day, even after a restart.
    let dir = a.dir.clone();
    a.engine.stop().await;
    let b = Node::with(&mem, "Laptop", None, |c| {
        c.config_dir = dir.join("config");
        c.update_url = Some(url.parse().unwrap());
    })
    .await;
    assert!(b.engine.snapshot().update.is_some(), "kept from the last look");
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(asked.load(std::sync::atomic::Ordering::Relaxed), 1);
    b.engine.set_update_check(false).unwrap();
    assert_eq!(b.engine.update_check(), Some(false));
    assert!(b.engine.snapshot().update.is_none());
    b.stop().await;
    // A build that gets updates from a store doesn't look, and has no setting.
    let c = Node::with(&mem, "Phone", None, |c| c.update_url = None).await;
    assert_eq!(c.engine.update_check(), None);
    c.stop().await;
}

/// Trust this device, from Nearby: both screens show the same check, and nothing changes until
/// the other device agrees.
#[tokio::test(flavor = "multi_thread")]
async fn trust_from_nearby_needs_both_to_agree() {
    let mem = MemoryLookup::new();
    let mut a = Node::new(&mem, "Laptop", None).await;
    let mut b = Node::new(&mem, "Phone", None).await;
    // They meet: a sends something, so each has seen the other on the network.
    let id = a.engine.send_text(&b.id(), "hi").unwrap();
    let req = b.wait_request().await;
    b.engine.accept(&req.id, false, false);
    a.wait_transfer("met", |t| t.id == id && t.status == TransferStatus::Done).await;

    a.engine.ask_trust(&b.id()).unwrap();
    let asked = b
        .wait("the ask", |e| match e {
            EngineEvent::TrustAsk { request } => Some(request.clone()),
            _ => None,
        })
        .await;
    let check = a
        .wait("the check", |e| match e {
            EngineEvent::TrustWaiting { check, .. } => Some(check.clone()),
            _ => None,
        })
        .await;
    assert_eq!(asked.check, check, "both screens show the same");
    assert_eq!(asked.device_id, a.id());
    assert_eq!(b.engine.snapshot().trust_asks.len(), 1);
    b.engine.answer_trust(&a.id(), true);
    let paired = |want: String| {
        move |e: &EngineEvent| match e {
            EngineEvent::Paired { device, via, check } if device.id == want => Some((*via, check.clone(), device.trusted)),
            _ => None,
        }
    };
    let (via, c, trusted) = a.wait("paired on the asker", paired(b.id())).await;
    assert_eq!((via, c.as_str(), trusted), (PairVia::Nearby, check.as_str(), true));
    let (via, c, trusted) = b.wait("paired on the asked", paired(a.id())).await;
    assert_eq!((via, c.as_str(), trusted), (PairVia::Nearby, check.as_str(), true));
    assert!(b.engine.snapshot().trust_asks.is_empty());

    // Saying no changes nothing.
    a.engine.set_trusted(&b.id(), false);
    b.engine.set_trusted(&a.id(), false);
    a.engine.ask_trust(&b.id()).unwrap();
    b.wait("second ask", |e| matches!(e, EngineEvent::TrustAsk { .. }).then_some(())).await;
    b.engine.answer_trust(&a.id(), false);
    let outcome = a
        .wait("told no", |e| match e {
            EngineEvent::TrustEnded { outcome, .. } => Some(*outcome),
            _ => None,
        })
        .await;
    assert_eq!(outcome, TrustOutcome::Declined);
    assert!(!b.engine.devices().iter().any(|d| d.id == a.id() && d.trusted));
    assert!(!a.engine.devices().iter().any(|d| d.id == b.id() && d.trusted));

    // Cancelled on the asker: the ask goes away on the other device too.
    a.engine.ask_trust(&b.id()).unwrap();
    b.wait("third ask", |e| matches!(e, EngineEvent::TrustAsk { .. }).then_some(())).await;
    a.engine.cancel_trust(&b.id());
    let outcome = b
        .wait("withdrawn", |e| match e {
            EngineEvent::TrustEnded { outcome, .. } => Some(*outcome),
            _ => None,
        })
        .await;
    assert_eq!(outcome, TrustOutcome::Withdrawn);
    assert!(b.engine.trust_asks().is_empty());

    // Only a device that's here can be asked.
    let stranger = device_id(&iroh_base_id());
    assert!(a.engine.ask_trust(&stranger).is_err());
    a.stop().await;
    b.stop().await;
}

fn iroh_base_id() -> iroh::EndpointId {
    iroh::SecretKey::from_bytes(&[3; 32]).public()
}

/// A request of many files: part files are made one at a time as each file starts, and the app
/// gets small progress updates instead of the whole file list every time a file finishes.
#[tokio::test(flavor = "multi_thread")]
async fn many_files_arrive_without_flooding_the_folder_or_the_app() {
    let mem = MemoryLookup::new();
    let a = Node::new(&mem, "A", Some(4_000_000)).await;
    let mut b = Node::new(&mem, "B", None).await;
    let src = a.dir.join("out");
    let mut paths = vec![make_file(&src, "first.mov", 3_000_000)];
    paths.extend((0..59).map(|i| make_file(&src, &format!("IMG_{i:03}.jpg"), 20_000)));
    let mut app = b.engine.subscribe();
    let id = a.engine.send(&b.id(), &paths).unwrap();
    let req = b.wait_incoming().await;
    b.engine.accept(&req, false, false);
    b.wait_transfer("first file moving", |t| t.id == id && t.done_bytes > 300_000).await;
    let parts = std::fs::read_dir(b.save())
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().ends_with(".nhakopart"))
        .count();
    assert_eq!(parts, 1, "only the file that's moving has a part file");

    b.wait_transfer("done", |t| t.id == id && t.status == TransferStatus::Done).await;
    for p in &paths {
        same_bytes(p, &b.save().join(p.file_name().unwrap()));
    }
    no_part_files(&b.save());
    let (mut full, mut progress) = (0, 0);
    while let Ok(e) = app.try_recv() {
        match e {
            EngineEvent::Transfer { transfer } if transfer.id == id => full += 1,
            EngineEvent::TransferProgress { id: moved, file_bytes, .. } if moved == id => {
                progress += 1;
                assert!(file_bytes.len() < 30, "progress lists only files that moved, got {}", file_bytes.len());
            }
            _ => {}
        }
    }
    assert!(progress > 0, "progress updates were sent");
    assert!(full <= 6, "the whole transfer was sent {full} times, once per file or worse");
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
    assert_eq!(code.words.len(), 4);
    assert_eq!(code.uri, format!("nhakobeam://pair/{}", code.words.join("-")));
    a.wait("code shown", |e| matches!(e, EngineEvent::Code { code: Some(_) }).then_some(())).await;

    // Typos are caught before any network work.
    let mut typo = code.words.clone();
    typo[1] = "notaword".into();
    let r = b.engine.redeem_code(&typo).await;
    assert_eq!(r, RedeemResult::Err(RedeemFailure::UnknownWord { index: 1 }));
    // Three words (a code from an older build) never match.
    let r = b.engine.redeem_code(&code.words[..3]).await;
    assert_eq!(r, RedeemResult::Err(RedeemFailure::NoMatch));

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

/// Who can send to me: by default only trusted devices and ones connected with a code. Others
/// are refused straight away and told why; with Everyone nearby, each may ask once at a time.
#[tokio::test(flavor = "multi_thread")]
async fn only_your_devices_can_send_by_default() {
    let mem = MemoryLookup::new();
    let mut a = Node::new(&mem, "Stranger's phone", None).await;
    let mut b = Node::new(&mem, "Kim's laptop", None).await;
    b.engine.set_who_can_send(WhoCanSend::YourDevices).unwrap();
    let f = make_file(&a.dir.join("out"), "hello.txt", 2000);

    // A device B has never paired with is turned away, and A shows why.
    let id = a.engine.send(&b.id(), std::slice::from_ref(&f)).unwrap();
    let t = a.wait_transfer("refused", |t| t.id == id && t.status.is_final()).await;
    assert_eq!((t.status, t.error), (TransferStatus::Declined, Some(TransferError::NotAllowed)));
    assert!(b.engine.requests().is_empty(), "no request card for a refused device");

    // Everyone nearby: A may ask, but only once at a time.
    b.engine.set_who_can_send(WhoCanSend::EveryoneNearby).unwrap();
    let first = a.engine.send(&b.id(), std::slice::from_ref(&f)).unwrap();
    let req = b.wait_incoming().await;
    assert_eq!(req, first);
    let second = a.engine.send(&b.id(), std::slice::from_ref(&f)).unwrap();
    let t = a.wait_transfer("second refused", |t| t.id == second && t.status.is_final()).await;
    assert_eq!((t.status, t.error), (TransferStatus::Declined, None));
    assert_eq!(b.engine.requests().len(), 1, "the second ask didn't add a card");
    b.engine.decline(&req);
    a.wait_transfer("first declined", |t| t.id == first && t.status == TransferStatus::Declined)
        .await;

    // Three asks a minute, refused ones included: the third shows, the fourth doesn't.
    let third = a.engine.send(&b.id(), std::slice::from_ref(&f)).unwrap();
    let req = b.wait_incoming().await;
    assert_eq!(req, third);
    b.engine.decline(&req);
    a.wait_transfer("third declined", |t| t.id == third && t.status == TransferStatus::Declined)
        .await;
    let fourth = a.engine.send(&b.id(), std::slice::from_ref(&f)).unwrap();
    let t = a.wait_transfer("fourth refused", |t| t.id == fourth && t.status.is_final()).await;
    assert_eq!((t.status, t.error), (TransferStatus::Declined, None));
    assert!(b.engine.requests().is_empty());

    // Back to Your devices: after pairing with a code, A can send.
    b.engine.set_who_can_send(WhoCanSend::YourDevices).unwrap();
    let code = b.engine.create_code();
    assert!(matches!(a.engine.redeem_code(&code.words).await, RedeemResult::Ok(_)));
    let ok = a.engine.send(&b.id(), std::slice::from_ref(&f)).unwrap();
    let req = b.wait_incoming().await;
    assert_eq!(req, ok);
    b.engine.accept(&req, false, false);
    b.wait_transfer("done", |t| t.id == ok && t.status == TransferStatus::Done).await;
    same_bytes(&f, &b.save().join("hello.txt"));

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

    // Never GitHub from a test.

    cfg.update_url = None;
    cfg.network = Some(Network::LocalOnly);
    cfg.mdns = false;
    cfg.beacon = None;
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
    // Never GitHub from a test.
    cfg.update_url = None;
    cfg.network = Some(Network::LocalOnly);
    cfg.mdns = false;
    cfg.beacon = None;
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
    // Never GitHub from a test.
    cfg.update_url = None;
    cfg.network = Some(Network::LocalOnly);
    cfg.memory_lookup = Some(mem.clone());
    // mDNS and the beacon allowed for this run, but the setting turns them off: nothing is
    // announced or looked for.
    cfg.mdns = true;
    let a2 = Engine::start(cfg).await.unwrap();
    assert!(!a2.lan_discovery());
    assert_ne!(a2.snapshot().discovery, nhako_core::DiscoveryState::Blocked, "off by choice isn't blocked");
    let diag = a2.diagnostics("test");
    assert!(diag.contains("mDNS: off") && diag.contains("Beacon: off"), "{diag}");
    a2.stop().await;
    let _ = std::fs::remove_dir_all(&a.dir);
}

/// Phase 10: two devices find each other through the beacon alone, with no mDNS and no shared
/// address table, the way a Windows PC finds a phone while Windows keeps the mDNS port to itself.
#[tokio::test(flavor = "multi_thread")]
async fn beacon_alone_finds_nearby_devices() {
    let free_port = || std::net::UdpSocket::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let (pa, pb) = (free_port(), free_port());
    let mem = MemoryLookup::new();
    let beacon = |listen: u16, to: u16| {
        move |cfg: &mut EngineConfig| {
            cfg.memory_lookup = None;
            cfg.beacon = Some(nhako_core::beacon::Config {
                port: listen,
                send_to: Some(vec![std::net::SocketAddr::from(([127, 0, 0, 1], to))]),
            });
        }
    };
    let mut a = Node::with(&mem, "A", None, beacon(pa, pb)).await;
    let mut b = Node::with(&mem, "B", None, beacon(pb, pa)).await;
    let (a_id, b_id) = (a.id(), b.id());
    let nearby = |id: String| {
        move |e: &EngineEvent| match e {
            EngineEvent::Device { device } if device.id == id && device.reachable && device.route == Some(nhako_core::Route::Nearby) => Some(()),
            _ => None,
        }
    };
    a.wait("B found by the beacon", nearby(b_id.clone())).await;
    b.wait("A found by the beacon", nearby(a_id.clone())).await;

    // And the address it gave works for a transfer.
    let f = make_file(&a.dir.join("out"), "notes.txt", 200_000);
    let id = a.engine.send(&b_id, std::slice::from_ref(&f)).unwrap();
    let req = b.wait_incoming().await;
    b.engine.accept(&req, false, false);
    b.wait_transfer("done", |t| t.id == id && t.status == TransferStatus::Done).await;
    same_bytes(&f, &b.save().join("notes.txt"));

    let diag = a.engine.diagnostics("test");
    assert!(diag.contains(&format!("Beacon: listening on UDP {pa}")), "{diag}");
    assert!(!diag.contains("Heard: 0 "), "{diag}");
    a.stop().await;
    b.stop().await;
}

// ---------------------------------------------------------------- benchmarks
//
// Run by hand, in release, one at a time (`scripts/bench.sh` runs them all):
// `cargo test --release -p nhako-core --test engine bench_ -- --ignored --nocapture --test-threads=1`
// Each prints BENCH lines. Both engines run in this process on localhost, so the CPU time is the
// two together, and "UI events" is what the receiving app would have to read (as JSON).

/// Wall time, CPU time, and the events the receiving app gets.
struct Meter {
    start: std::time::Instant,
    cpu: f64,
    events: std::sync::Arc<std::sync::atomic::AtomicU64>,
    json: std::sync::Arc<std::sync::atomic::AtomicU64>,
    task: tokio::task::JoinHandle<()>,
}

/// CPU seconds this process has used so far.
#[cfg(unix)]
fn cpu_secs() -> f64 {
    // SAFETY: getrusage only writes into the struct it's given.
    let mut u: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut u) };
    let t = |v: libc::timeval| v.tv_sec as f64 + v.tv_usec as f64 / 1e6;
    t(u.ru_utime) + t(u.ru_stime)
}

/// Not measured off Unix (libc is a Unix-only dependency): the benchmarks print "CPU NaN s".
#[cfg(not(unix))]
fn cpu_secs() -> f64 {
    f64::NAN
}

impl Meter {
    fn start(receiver: &Engine) -> Self {
        use std::sync::{
            Arc,
            atomic::{AtomicU64, Ordering},
        };
        let (events, json) = (Arc::new(AtomicU64::new(0)), Arc::new(AtomicU64::new(0)));
        let mut rx = receiver.subscribe();
        let (n, b) = (events.clone(), json.clone());
        let task = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(e) => {
                        n.fetch_add(1, Ordering::Relaxed);
                        b.fetch_add(serde_json::to_vec(&e).unwrap().len() as u64, Ordering::Relaxed);
                    }
                    Err(broadcast::error::RecvError::Lagged(k)) => {
                        n.fetch_add(k, Ordering::Relaxed);
                    }
                    Err(_) => return,
                }
            }
        });
        Self {
            start: std::time::Instant::now(),
            cpu: cpu_secs(),
            events,
            json,
            task,
        }
    }

    fn report(self, name: &str, files: usize, bytes: u64, extra: &str) {
        use std::sync::atomic::Ordering;
        let secs = self.start.elapsed().as_secs_f64();
        let cpu = cpu_secs() - self.cpu;
        self.task.abort();
        let (n, json) = (self.events.load(Ordering::Relaxed), self.json.load(Ordering::Relaxed));
        println!(
            "BENCH {name}: {files} files, {:.0} MB in {secs:.2} s = {:.0} MB/s, {:.0} files/s, CPU {cpu:.2} s, UI events {n} ({:.1} MB JSON){extra}",
            bytes as f64 / 1e6,
            bytes as f64 / secs / 1e6,
            files as f64 / secs,
            json as f64 / 1e6
        );
    }
}

/// How many files a benchmark sends: `default`, or `NHAKO_BENCH_FILES` when set.
fn bench_files(default: usize) -> usize {
    std::env::var("NHAKO_BENCH_FILES").ok().and_then(|v| v.parse().ok()).unwrap_or(default)
}

/// Sends `paths` from A to B, accepted, and waits until both sides are done. Returns when B's
/// transfer first showed (how long accepting took) and the transfer id.
async fn send_and_wait(a: &mut Node, b: &mut Node, paths: &[PathBuf]) -> (Duration, String) {
    (a.patience, b.patience) = (Duration::from_secs(900), Duration::from_secs(900));
    let id = a.engine.send(&b.id(), paths).unwrap();
    let req = b.wait_incoming().await;
    let accepted = std::time::Instant::now();
    b.engine.accept(&req, false, false);
    b.wait_transfer("receiving", |t| t.id == id).await;
    let to_start = accepted.elapsed();
    b.wait_transfer("done", |t| t.id == id && t.status == TransferStatus::Done).await;
    a.wait_transfer("done", |t| t.id == id && t.status == TransferStatus::Done).await;
    (to_start, id)
}

/// One big file: raw speed (Phase 8 measured 345 to 365 MB/s here).
#[tokio::test(flavor = "multi_thread")]
#[ignore = "benchmark: run by hand"]
async fn bench_one_big_file() {
    let mem = MemoryLookup::new();
    let mut a = Node::new(&mem, "A", None).await;
    let mut b = Node::new(&mem, "B", None).await;
    let size = 512 * 1024 * 1024;
    let f = make_file(&a.dir.join("out"), "big.bin", size);
    let m = Meter::start(&b.engine);
    send_and_wait(&mut a, &mut b, &[f]).await;
    m.report("one big file", 1, size as u64, "");
    a.stop().await;
    b.stop().await;
}

/// A folder of photos: 5,000 files of 200 KB.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "benchmark: run by hand"]
async fn bench_many_small_files() {
    let mem = MemoryLookup::new();
    let mut a = Node::new(&mem, "A", None).await;
    let mut b = Node::new(&mem, "B", None).await;
    let (count, size) = (bench_files(5_000), 200 * 1024);
    let src = a.dir.join("out");
    let paths: Vec<PathBuf> = (0..count).map(|i| make_file(&src, &format!("IMG_{i:05}.jpg"), size)).collect();
    let m = Meter::start(&b.engine);
    let (to_start, _) = send_and_wait(&mut a, &mut b, &paths).await;
    m.report(
        "many small files",
        count,
        (count * size) as u64,
        &format!(", accept to start {:.2} s", to_start.as_secs_f64()),
    );
    a.stop().await;
    b.stop().await;
}

/// A huge offer of tiny files: how long accepting takes before anything moves.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "benchmark: run by hand"]
async fn bench_big_offer() {
    let mem = MemoryLookup::new();
    let mut a = Node::new(&mem, "A", None).await;
    let mut b = Node::new(&mem, "B", None).await;
    let (count, size) = (bench_files(20_000), 100);
    let src = a.dir.join("out");
    let paths: Vec<PathBuf> = (0..count).map(|i| make_file(&src, &format!("note-{i:05}.txt"), size)).collect();
    let m = Meter::start(&b.engine);
    let (to_start, _) = send_and_wait(&mut a, &mut b, &paths).await;
    m.report(
        "big offer",
        count,
        (count * size) as u64,
        &format!(", accept to start {:.2} s", to_start.as_secs_f64()),
    );
    a.stop().await;
    b.stop().await;
}

/// The connection drops at 80% of a 1 GB file. Measures how long the rest takes against how long
/// moving it should take, which shows the time spent re-reading what both sides already had.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "benchmark: run by hand"]
async fn bench_resume_after_drop() {
    let mem = MemoryLookup::new();
    let a = Node::new(&mem, "A", None).await;
    let mut b = Node::new(&mem, "B", None).await;
    let size = 1024 * 1024 * 1024;
    let f = make_file(&a.dir.join("out"), "big.bin", size);
    let started = std::time::Instant::now();
    let id = a.engine.send(&b.id(), &[f]).unwrap();
    let req = b.wait_incoming().await;
    b.engine.accept(&req, false, false);
    let at = b.wait_transfer("80%", |t| t.id == id && t.done_bytes >= size as u64 * 8 / 10).await.done_bytes;
    let speed = at as f64 / started.elapsed().as_secs_f64();
    let m = Meter::start(&b.engine);
    b.engine.test_drop_connection(&id);
    b.wait_transfer("done", |t| t.id == id && t.status == TransferStatus::Done).await;
    let rest = size as u64 - at;
    let should = rest as f64 / speed;
    let took = m.start.elapsed().as_secs_f64();
    m.report(
        "resume after drop",
        1,
        rest,
        &format!(
            ", dropped at {:.0}%, the rest at full speed would take {should:.2} s, extra {:.2} s",
            at as f64 * 100.0 / size as f64,
            took - should
        ),
    );
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
    // Never GitHub from a test.
    cfg.update_url = None;
    cfg.save_dir = Some(phone_dir.join("save"));
    cfg.network = Some(Network::LocalOnly);
    cfg.mdns = false;
    cfg.beacon = None;
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
