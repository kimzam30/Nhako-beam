//! One transfer session over one connection: the sender pushes files, the receiver writes and
//! checks them. See docs/protocol/nhako-1.md, sections 5 and 6.

use std::{
    io::SeekFrom,
    sync::{
        Arc,
        atomic::{AtomicU32, AtomicU64, Ordering},
    },
    time::Duration,
};

use iroh::endpoint::{Connection, ConnectionError, ReadError, SendStream, WriteError};
use tokio::{
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    sync::{mpsc, watch},
    task::JoinHandle,
};
use tracing::debug;

use crate::{
    engine::{Cmd, CmdRx, DstFile, Engine, FrameRx, SrcFile, TxSide},
    files,
    types::{Side, TransferError, TransferStatus},
    wire::{self, Ctrl, DataHeader, FileState, PauseReason, TransferId},
};

const CHUNK: usize = 256 * 1024;
/// Received files being written to disk for good (fsync) while the next ones arrive. That takes
/// milliseconds per file, which made a folder of photos 3 to 4 times slower when done one by one.
const FINISHING_AT_ONCE: usize = 8;
const TICK: Duration = Duration::from_millis(250);

#[derive(Debug)]
pub(crate) enum End {
    Done,
    Cancelled(Side),
    Lost,
    Failed(TransferError),
}

/// A BLAKE3 hasher part way through a file, kept across pauses in one session.
pub(crate) struct HashCache {
    file: u32,
    pos: u64,
    hasher: blake3::Hasher,
}

/// Hasher states saved every [`MARK_EVERY`] bytes of the file being moved. They're kept in the
/// transfer, across sessions, so resuming after a dropped connection re-reads at most that much
/// of the file instead of everything so far (minutes for a big file on a phone).
pub(crate) type Marks = Arc<std::sync::Mutex<Vec<HashCache>>>;
const MARK_EVERY: u64 = 16 * 1024 * 1024;
/// The sender's newest marks can be ahead of what the receiver got, so a few are kept.
const MARKS_KEPT: usize = 8;

/// Saves a mark when `pos` crosses a [`MARK_EVERY`] boundary (from `before`).
fn mark(marks: &Marks, file: u32, before: u64, pos: u64, hasher: &blake3::Hasher) {
    if before / MARK_EVERY == pos / MARK_EVERY {
        return;
    }
    let mut m = marks.lock().expect("marks lock");
    m.retain(|c| c.file == file);
    if m.len() >= MARKS_KEPT {
        m.remove(0);
    }
    m.push(HashCache {
        file,
        pos,
        hasher: hasher.clone(),
    });
}

fn failed_reason(err: TransferError) -> &'static [u8] {
    match err {
        TransferError::Integrity => b"integrity",
        TransferError::FileChanged => b"fileChanged",
        TransferError::Permission => b"permission",
        _ => b"unknown",
    }
}

/// Works out how a session ended from the connection's close reason.
async fn end_from_close(conn: &Connection, all_verified: bool, me_sender: bool) -> End {
    let reason = match conn.close_reason() {
        Some(r) => r,
        None => match tokio::time::timeout(Duration::from_secs(15), conn.closed()).await {
            Ok(r) => r,
            Err(_) => return End::Lost,
        },
    };
    match reason {
        ConnectionError::ApplicationClosed(c) if c.error_code == wire::close::CANCELLED => End::Cancelled(Side::Peer),
        ConnectionError::ApplicationClosed(c) if c.error_code == wire::close::FAILED => End::Failed(match &c.reason[..] {
            b"integrity" => TransferError::Integrity,
            b"fileChanged" => TransferError::FileChanged,
            b"permission" if !me_sender => TransferError::Permission,
            _ => TransferError::Unknown,
        }),
        ConnectionError::ApplicationClosed(c) if c.error_code == wire::close::PROTOCOL => End::Failed(TransferError::Unknown),
        _ if all_verified => End::Done,
        _ => End::Lost,
    }
}

/// Sends the last frame, then gives the peer a moment to read it and close.
async fn say_and_close(conn: &Connection, ctrl: &mut SendStream, msg: &Ctrl, code: iroh::endpoint::VarInt, reason: &[u8]) {
    let _ = wire::write_frame(ctrl, msg).await;
    let _ = tokio::time::timeout(Duration::from_secs(2), conn.closed()).await;
    conn.close(code, reason);
}

// ---------------------------------------------------------------- sender

enum PumpEnd {
    Finished,
    Stopped,
    FileChanged,
    Conn,
}

struct Pump {
    handle: JoinHandle<(PumpEnd, Option<HashCache>)>,
    stop: watch::Sender<bool>,
}

fn start_pump(e: &Engine, conn: &Connection, id: TransferId, files: &[SrcFile], start: &[FileState], cache: Option<HashCache>) -> Pump {
    let (stop, stop_rx) = watch::channel(false);
    let rate = e.inner.cfg.test_rate_limit;
    let marks = e.with_tx(&id, |tx| tx.marks.clone()).unwrap_or_default();
    let handle = tokio::spawn(pump(conn.clone(), id, files.to_vec(), start.to_vec(), cache, marks, stop_rx, rate));
    Pump { handle, stop }
}

/// Stops the pump after its current chunk and returns its hasher state.
async fn stop_pump(pump: &mut Option<Pump>) -> Option<HashCache> {
    let p = pump.take()?;
    let _ = p.stop.send(true);
    let abort = p.handle.abort_handle();
    match tokio::time::timeout(Duration::from_secs(15), p.handle).await {
        Ok(Ok((_, cache))) => cache,
        _ => {
            abort.abort();
            None
        }
    }
}

async fn join_pump(pump: &mut Option<Pump>) -> (PumpEnd, Option<HashCache>) {
    match pump.as_mut() {
        Some(p) => (&mut p.handle).await.unwrap_or((PumpEnd::Conn, None)),
        None => std::future::pending().await,
    }
}

fn unchanged(f: &SrcFile) -> bool {
    files::open_to_send(&f.path)
        .and_then(|file| file.metadata())
        .is_ok_and(|m| m.len() == f.size && m.modified().ok() == f.modified)
}

/// A hasher that has seen the first `len` bytes of file `index`: it starts from the nearest mark
/// at or before `len` (or the start) and reads only what comes after it.
async fn hash_prefix(file: &mut tokio::fs::File, index: u32, len: u64, marks: &Marks, buf: &mut [u8]) -> std::io::Result<blake3::Hasher> {
    let (mut hasher, from) = marks
        .lock()
        .expect("marks lock")
        .iter()
        .filter(|c| c.file == index && c.pos <= len)
        .max_by_key(|c| c.pos)
        .map_or_else(|| (blake3::Hasher::new(), 0), |c| (c.hasher.clone(), c.pos));
    file.seek(SeekFrom::Start(from)).await?;
    let mut left = len - from;
    while left > 0 {
        let want = left.min(buf.len() as u64) as usize;
        let n = file.read(&mut buf[..want]).await?;
        if n == 0 {
            return Err(std::io::ErrorKind::UnexpectedEof.into());
        }
        hasher.update(&buf[..n]);
        left -= n as u64;
    }
    Ok(hasher)
}

/// Streams every file that still needs it, one after another.
#[allow(clippy::too_many_arguments)]
async fn pump(
    conn: Connection,
    id: TransferId,
    files: Vec<SrcFile>,
    start: Vec<FileState>,
    mut cache: Option<HashCache>,
    marks: Marks,
    stop: watch::Receiver<bool>,
    rate: Option<u64>,
) -> (PumpEnd, Option<HashCache>) {
    let mut buf = vec![0u8; CHUNK];
    let began = tokio::time::Instant::now();
    let mut sent = 0u64;
    for (i, (f, state)) in files.iter().zip(&start).enumerate() {
        let FileState::Have(off) = *state else { continue };
        let off = off.min(f.size);
        let i = i as u32;
        if !unchanged(f) {
            return (PumpEnd::FileChanged, None);
        }
        let Ok(mut file) = files::open_to_send(&f.path).map(tokio::fs::File::from_std) else {
            return (PumpEnd::FileChanged, None);
        };
        let mut hasher = match cache.take().filter(|c| c.file == i && c.pos == off) {
            Some(c) => c.hasher,
            None => match hash_prefix(&mut file, i, off, &marks, &mut buf).await {
                Ok(h) => h,
                Err(_) => return (PumpEnd::FileChanged, None),
            },
        };
        if file.seek(SeekFrom::Start(off)).await.is_err() {
            return (PumpEnd::FileChanged, None);
        }
        let Ok(mut s) = conn.open_uni().await else { return (PumpEnd::Conn, None) };
        if wire::write_frame(
            &mut s,
            &DataHeader {
                id,
                file: i,
                offset: off,
                len: f.size - off,
            },
        )
        .await
        .is_err()
        {
            return (PumpEnd::Conn, None);
        }
        let mut pos = off;
        while pos < f.size {
            if *stop.borrow() {
                let _ = s.reset(wire::RESET_PAUSED);
                return (PumpEnd::Stopped, Some(HashCache { file: i, pos, hasher }));
            }
            let want = (f.size - pos).min(CHUNK as u64) as usize;
            let n = match file.read(&mut buf[..want]).await {
                Ok(0) | Err(_) => return (PumpEnd::FileChanged, None),
                Ok(n) => n,
            };
            hasher.update(&buf[..n]);
            mark(&marks, i, pos, pos + n as u64, &hasher);
            match s.write_all(&buf[..n]).await {
                Ok(()) => {}
                // The receiver stopped reading (it paused or ran out of space).
                Err(WriteError::Stopped(_)) => return (PumpEnd::Stopped, None),
                Err(_) => return (PumpEnd::Conn, None),
            }
            pos += n as u64;
            sent += n as u64;
            if let Some(rate) = rate {
                tokio::time::sleep_until(began + Duration::from_secs_f64(sent as f64 / rate as f64)).await;
            }
        }
        if !unchanged(f) {
            let _ = s.reset(wire::RESET_PAUSED);
            return (PumpEnd::FileChanged, None);
        }
        if s.write_all(hasher.finalize().as_bytes()).await.is_err() || s.finish().is_err() {
            return (PumpEnd::Conn, None);
        }
    }
    (PumpEnd::Finished, None)
}

pub(crate) async fn run_sender(
    e: Engine,
    id: TransferId,
    conn: Connection,
    mut ctrl: SendStream,
    mut frames: FrameRx,
    start: Vec<FileState>,
    mut cmds: CmdRx,
) -> End {
    let files = match e.with_tx(&id, |tx| tx.side.clone()) {
        Some(TxSide::Send(f)) => f,
        _ => return End::Failed(TransferError::Unknown),
    };
    if start.len() != files.len() {
        conn.close(wire::close::PROTOCOL, b"file count");
        return End::Failed(TransferError::Unknown);
    }
    let size = |i: usize| files[i].size;
    let mut verified: Vec<bool> = start.iter().map(|s| *s == FileState::Verified).collect();
    let mut have: Vec<u64> = start
        .iter()
        .enumerate()
        .map(|(i, s)| match s {
            FileState::Verified => size(i),
            FileState::Have(n) => (*n).min(size(i)),
        })
        .collect();
    e.with_tx(&id, |tx| tx.set_have(&have));
    e.emit_tx(&id, true);
    if verified.iter().all(|v| *v) {
        // Nothing left to send: text the receiver took from the request, or a resume that found
        // every file already checked.
        let _ = wire::write_frame(&mut ctrl, &Ctrl::Done).await;
        let _ = ctrl.finish();
        let _ = tokio::time::timeout(Duration::from_secs(5), conn.closed()).await;
        return End::Done;
    }

    let mut cache = None;
    let mut pump = Some(start_pump(&e, &conn, id, &files, &start, None));
    let mut paused: Option<Side> = None;
    let mut tick = tokio::time::interval(TICK);

    loop {
        tokio::select! {
            cmd = cmds.recv() => match cmd {
                Some(Cmd::Pause) => if paused.is_none() {
                    paused = Some(Side::Me);
                    cache = stop_pump(&mut pump).await.or(cache);
                    let _ = wire::write_frame(&mut ctrl, &Ctrl::Pause { reason: PauseReason::User }).await;
                    set_paused(&e, &id, Some(Side::Me), None);
                },
                Some(Cmd::Resume) => if paused.is_some() {
                    paused = None;
                    let _ = wire::write_frame(&mut ctrl, &Ctrl::Resume).await;
                    set_paused(&e, &id, None, None);
                },
                Some(Cmd::Cancel) | None => {
                    stop_pump(&mut pump).await;
                    say_and_close(&conn, &mut ctrl, &Ctrl::Cancel, wire::close::CANCELLED, b"").await;
                    return End::Cancelled(Side::Me);
                }
                Some(Cmd::Fail) => {
                    stop_pump(&mut pump).await;
                    conn.close(wire::close::FAILED, failed_reason(TransferError::Unknown));
                    return End::Failed(TransferError::Unknown);
                }
            },
            frame = frames.recv() => {
                let msg = match frame {
                    Some(Ok(b)) => postcard::from_bytes::<Ctrl>(&b).ok(),
                    _ => None,
                };
                let Some(msg) = msg else {
                    stop_pump(&mut pump).await;
                    return end_from_close(&conn, verified.iter().all(|v| *v), true).await;
                };
                match msg {
                    Ctrl::Progress { file, bytes } => {
                        let i = file as usize;
                        if i < have.len() && !verified[i] {
                            have[i] = bytes.min(size(i));
                            e.with_tx(&id, |tx| tx.set_file(i, have[i]));
                            e.emit_tx(&id, false);
                        }
                    }
                    Ctrl::Verdict { file, ok: true } => {
                        let i = file as usize;
                        if i < have.len() {
                            verified[i] = true;
                            have[i] = size(i);
                            e.with_tx(&id, |tx| tx.set_file(i, have[i]));
                            e.emit_tx(&id, false);
                        }
                        if verified.iter().all(|v| *v) {
                            let _ = wire::write_frame(&mut ctrl, &Ctrl::Done).await;
                            let _ = ctrl.finish();
                            let _ = tokio::time::timeout(Duration::from_secs(5), conn.closed()).await;
                            return End::Done;
                        }
                    }
                    Ctrl::Verdict { ok: false, .. } => {
                        stop_pump(&mut pump).await;
                        conn.close(wire::close::FAILED, failed_reason(TransferError::Integrity));
                        return End::Failed(TransferError::Integrity);
                    }
                    Ctrl::Pause { reason } => {
                        paused = Some(Side::Peer);
                        cache = stop_pump(&mut pump).await.or(cache);
                        set_paused(&e, &id, Some(Side::Peer), (reason == PauseReason::DiskFull).then_some(TransferError::DiskFullPeer));
                    }
                    Ctrl::Want { files: states } => {
                        if states.len() != files.len() {
                            conn.close(wire::close::PROTOCOL, b"file count");
                            return End::Failed(TransferError::Unknown);
                        }
                        cache = stop_pump(&mut pump).await.or(cache);
                        paused = None;
                        for (i, s) in states.iter().enumerate() {
                            match s {
                                FileState::Verified => { verified[i] = true; have[i] = size(i); }
                                FileState::Have(n) => { verified[i] = false; have[i] = (*n).min(size(i)); }
                            }
                        }
                        e.with_tx(&id, |tx| tx.set_have(&have));
                        set_paused(&e, &id, None, None);
                        pump = Some(start_pump(&e, &conn, id, &files, &states, cache.take()));
                    }
                    Ctrl::Cancel => {
                        stop_pump(&mut pump).await;
                        conn.close(wire::close::CANCELLED, b"");
                        return End::Cancelled(Side::Peer);
                    }
                    Ctrl::Resume | Ctrl::Withdraw | Ctrl::Done => debug!("ignoring unexpected control message"),
                }
            },
            (end, c) = join_pump(&mut pump) => {
                pump = None;
                cache = c.or(cache);
                match end {
                    PumpEnd::Finished | PumpEnd::Stopped => {}
                    PumpEnd::FileChanged => {
                        // The close reason tells the receiver why; a Cancel frame would read as "cancelled".
                        conn.close(wire::close::FAILED, failed_reason(TransferError::FileChanged));
                        return End::Failed(TransferError::FileChanged);
                    }
                    PumpEnd::Conn => return end_from_close(&conn, verified.iter().all(|v| *v), true).await,
                }
            },
            _ = tick.tick() => {
                if paused.is_none() {
                    e.with_tx(&id, |tx| tx.tick());
                    e.emit_tx(&id, false);
                }
            },
        }
    }
}

fn set_paused(e: &Engine, id: &TransferId, by: Option<Side>, error: Option<TransferError>) {
    e.with_tx(id, |tx| {
        tx.t.paused_by = by;
        tx.t.error = error;
        if by.is_some() {
            tx.t.status = TransferStatus::Paused;
            tx.stop_moving();
        } else {
            tx.t.status = TransferStatus::Transferring;
        }
    });
    e.emit_tx(id, true);
}

// ---------------------------------------------------------------- receiver

#[derive(Debug)]
enum DataEv {
    FileOk(u32, std::path::PathBuf),
    FileBad(u32),
    DiskFull(u32),
    Io(TransferError),
    Protocol(&'static str),
    Conn,
}

/// What the acceptor is writing right now, read by the controller every tick.
struct Live {
    file: AtomicU32,
    bytes: AtomicU64,
}

pub(crate) async fn run_receiver(e: Engine, id: TransferId, conn: Connection, mut ctrl: SendStream, mut frames: FrameRx, mut cmds: CmdRx) -> End {
    let files = match e.with_tx(&id, |tx| tx.side.clone()) {
        Some(TxSide::Recv(f)) => f,
        _ => return End::Failed(TransferError::Unknown),
    };
    let start = e.receiver_offsets(&id);
    let mut verified: Vec<bool> = files.iter().map(|f| f.verified).collect();
    let mut have: Vec<u64> = start
        .iter()
        .zip(&files)
        .map(|(s, f)| match s {
            FileState::Verified => f.size,
            FileState::Have(n) => *n,
        })
        .collect();
    e.with_tx(&id, |tx| tx.set_have(&have));
    e.emit_tx(&id, true);

    let live = Arc::new(Live {
        file: AtomicU32::new(u32::MAX),
        bytes: AtomicU64::new(0),
    });
    let (ev_tx, mut ev_rx) = mpsc::unbounded_channel();
    let acceptor = tokio::spawn(accept_files(e.clone(), conn.clone(), id, files.clone(), live.clone(), ev_tx));
    let _abort = AbortOnDrop(acceptor);
    let mut paused: Option<Side> = None;
    let mut last_sent = (u32::MAX, 0u64);
    let mut tick = tokio::time::interval(TICK);
    let all = |v: &Vec<bool>| v.iter().all(|x| *x);

    loop {
        tokio::select! {
            cmd = cmds.recv() => match cmd {
                Some(Cmd::Pause) => if paused.is_none() {
                    paused = Some(Side::Me);
                    let _ = wire::write_frame(&mut ctrl, &Ctrl::Pause { reason: PauseReason::User }).await;
                    set_paused(&e, &id, Some(Side::Me), None);
                },
                Some(Cmd::Resume) => if paused.is_some() {
                    paused = None;
                    let states = e.receiver_offsets(&id);
                    let _ = wire::write_frame(&mut ctrl, &Ctrl::Want { files: states }).await;
                    set_paused(&e, &id, None, None);
                },
                Some(Cmd::Cancel) | None => {
                    say_and_close(&conn, &mut ctrl, &Ctrl::Cancel, wire::close::CANCELLED, b"").await;
                    return End::Cancelled(Side::Me);
                }
                Some(Cmd::Fail) => {
                    conn.close(wire::close::FAILED, failed_reason(TransferError::Unknown));
                    return End::Failed(TransferError::Unknown);
                }
            },
            frame = frames.recv() => {
                let msg = match frame {
                    Some(Ok(b)) => postcard::from_bytes::<Ctrl>(&b).ok(),
                    _ => None,
                };
                let Some(msg) = msg else { return end_from_close(&conn, all(&verified), false).await };
                match msg {
                    Ctrl::Pause { .. } => {
                        paused = Some(Side::Peer);
                        set_paused(&e, &id, Some(Side::Peer), None);
                    }
                    Ctrl::Resume => {
                        paused = None;
                        let states = e.receiver_offsets(&id);
                        let _ = wire::write_frame(&mut ctrl, &Ctrl::Want { files: states }).await;
                        set_paused(&e, &id, None, None);
                    }
                    Ctrl::Cancel => {
                        conn.close(wire::close::CANCELLED, b"");
                        return End::Cancelled(Side::Peer);
                    }
                    Ctrl::Done if all(&verified) => {
                        conn.close(wire::close::DONE, b"");
                        return End::Done;
                    }
                    Ctrl::Done => {
                        conn.close(wire::close::PROTOCOL, b"early done");
                        return End::Failed(TransferError::Unknown);
                    }
                    Ctrl::Progress { .. } | Ctrl::Verdict { .. } | Ctrl::Want { .. } | Ctrl::Withdraw => debug!("ignoring unexpected control message"),
                }
            },
            ev = ev_rx.recv() => match ev.unwrap_or(DataEv::Conn) {
                DataEv::FileOk(file, path) => {
                    let i = file as usize;
                    verified[i] = true;
                    have[i] = files[i].size;
                    e.with_tx(&id, |tx| {
                        if let TxSide::Recv(f) = &mut tx.side {
                            f[i].verified = true;
                            f[i].final_path = path;
                        }
                        tx.set_file(i, have[i]);
                    });
                    e.emit_tx(&id, false);
                    let _ = wire::write_frame(&mut ctrl, &Ctrl::Verdict { file, ok: true }).await;
                }
                DataEv::FileBad(file) => {
                    have[file as usize] = 0;
                    say_and_close(&conn, &mut ctrl, &Ctrl::Verdict { file, ok: false }, wire::close::FAILED, failed_reason(TransferError::Integrity)).await;
                    return End::Failed(TransferError::Integrity);
                }
                DataEv::DiskFull(file) => {
                    have[file as usize] = live.bytes.load(Ordering::Relaxed);
                    paused = Some(Side::Me);
                    let _ = wire::write_frame(&mut ctrl, &Ctrl::Pause { reason: PauseReason::DiskFull }).await;
                    set_paused(&e, &id, Some(Side::Me), Some(TransferError::DiskFull));
                }
                DataEv::Io(err) => {
                    conn.close(wire::close::FAILED, failed_reason(err));
                    return End::Failed(err);
                }
                DataEv::Protocol(why) => {
                    debug!(why, "protocol error from sender");
                    conn.close(wire::close::PROTOCOL, why.as_bytes());
                    return End::Failed(TransferError::Unknown);
                }
                DataEv::Conn => return end_from_close(&conn, all(&verified), false).await,
            },
            _ = tick.tick() => {
                let file = live.file.load(Ordering::Relaxed);
                let bytes = live.bytes.load(Ordering::Relaxed);
                let i = file as usize;
                if i < have.len() && !verified[i] && (file, bytes) != last_sent {
                    have[i] = bytes;
                    last_sent = (file, bytes);
                    let _ = wire::write_frame(&mut ctrl, &Ctrl::Progress { file, bytes }).await;
                }
                if paused.is_none() {
                    e.with_tx(&id, |tx| if i < have.len() { tx.set_file(i, have[i]) } else { tx.tick() });
                    e.emit_tx(&id, false);
                }
            },
        }
    }
}

struct AbortOnDrop(JoinHandle<()>);
impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Takes data streams one at a time, writes them into part files and checks the hash.
async fn accept_files(e: Engine, conn: Connection, id: TransferId, mut files: Vec<DstFile>, live: Arc<Live>, ev: mpsc::UnboundedSender<DataEv>) {
    let mut buf = vec![0u8; CHUNK];
    let mut cache: Option<HashCache> = None;
    let marks = e.with_tx(&id, |tx| tx.marks.clone()).unwrap_or_default();
    // Files already checked, with where they ended up.
    let done: Vec<Option<std::path::PathBuf>> = files.iter().map(|f| f.verified.then(|| f.final_path.clone())).collect();
    // Checked files still being written to disk; each confirms itself when it's done.
    let finishing = Arc::new(tokio::sync::Semaphore::new(FINISHING_AT_ONCE));
    let mut handed_off = vec![false; files.len()];
    loop {
        let Ok(mut s) = conn.accept_uni().await else {
            let _ = ev.send(DataEv::Conn);
            return;
        };
        let header: DataHeader = match wire::read_frame(&mut s).await {
            Ok(h) => h,
            Err(_) => continue, // a stream reset before its header arrived
        };
        let i = header.file as usize;
        let Some(f) = files.get(i) else {
            let _ = ev.send(DataEv::Protocol("file index"));
            return;
        };
        if header.id != id || header.offset.checked_add(header.len) != Some(f.size) {
            let _ = ev.send(DataEv::Protocol("bad data header"));
            return;
        }
        if let Some(path) = done[i].clone() {
            // A resend of a file we already checked (a resume raced with the check). Drain it and confirm again.
            while let Ok(Some(_)) = s.read(&mut buf).await {}
            let _ = ev.send(DataEv::FileOk(header.file, path));
            continue;
        }
        if handed_off[i] {
            // Checked and being written to disk: it confirms itself when that's done.
            while let Ok(Some(_)) = s.read(&mut buf).await {}
            continue;
        }
        // Its first data: pick its name and make its part file.
        if f.part.is_none() {
            let mut f = f.clone();
            let claimed = tokio::task::spawn_blocking(move || f.claim().map(|_| f))
                .await
                .unwrap_or_else(|e| Err(std::io::Error::other(e)));
            match claimed {
                Ok(f) => {
                    e.with_tx(&id, |tx| {
                        if let TxSide::Recv(all) = &mut tx.side {
                            all[i].final_path = f.final_path.clone();
                            all[i].part = f.part.clone();
                        }
                    });
                    files[i] = f;
                }
                Err(err) => {
                    let _ = ev.send(DataEv::Io(io_kind(&err)));
                    return;
                }
            }
        }
        let f = &files[i];
        let Some(part) = f.part.clone() else {
            let _ = ev.send(DataEv::Io(TransferError::Unknown));
            return;
        };
        let part_len = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
        if header.offset > part_len {
            let _ = ev.send(DataEv::Protocol("offset past what we have"));
            return;
        }
        let opened = files::part_options().open(&part).await;
        let mut out = match opened {
            Ok(o) => o,
            Err(err) => {
                let _ = ev.send(DataEv::Io(io_kind(&err)));
                return;
            }
        };
        if let Err(err) = out.set_len(header.offset).await {
            let _ = ev.send(DataEv::Io(io_kind(&err)));
            return;
        }
        let mut hasher = match cache.take().filter(|c| c.file == header.file && c.pos == header.offset) {
            Some(c) => c.hasher,
            None => match hash_prefix(&mut out, header.file, header.offset, &marks, &mut buf).await {
                Ok(h) => h,
                Err(_) => {
                    let _ = ev.send(DataEv::Protocol("part file shorter than expected"));
                    return;
                }
            },
        };
        if out.seek(SeekFrom::Start(header.offset)).await.is_err() {
            let _ = ev.send(DataEv::Io(TransferError::Permission));
            return;
        }
        live.file.store(header.file, Ordering::Relaxed);
        live.bytes.store(header.offset, Ordering::Relaxed);

        let mut pos = header.offset;
        let mut interrupted = false;
        while pos < f.size {
            let want = (f.size - pos).min(CHUNK as u64) as usize;
            match s.read(&mut buf[..want]).await {
                Ok(Some(n)) => {
                    if let Err(err) = out.write_all(&buf[..n]).await {
                        let _ = out.flush().await;
                        let _ = out.set_len(pos).await;
                        let _ = s.stop(wire::RESET_PAUSED);
                        if files::is_disk_full(&err) {
                            live.bytes.store(pos, Ordering::Relaxed);
                            let _ = ev.send(DataEv::DiskFull(header.file));
                            interrupted = true;
                            break;
                        }
                        let _ = ev.send(DataEv::Io(io_kind(&err)));
                        return;
                    }
                    hasher.update(&buf[..n]);
                    mark(&marks, header.file, pos, pos + n as u64, &hasher);
                    pos += n as u64;
                    live.bytes.store(pos, Ordering::Relaxed);
                }
                Ok(None) => {
                    let _ = ev.send(DataEv::Protocol("data stream ended early"));
                    return;
                }
                Err(ReadError::Reset(_)) => {
                    // The sender paused. Keep what we have and wait for the next stream.
                    let _ = out.flush().await;
                    interrupted = true;
                    break;
                }
                Err(_) => {
                    let _ = out.flush().await;
                    let _ = ev.send(DataEv::Conn);
                    return;
                }
            }
        }
        if interrupted {
            cache = Some(HashCache {
                file: header.file,
                pos,
                hasher,
            });
            continue;
        }
        let mut expected = [0u8; 32];
        match s.read_exact(&mut expected).await {
            Ok(()) => {}
            Err(iroh::endpoint::ReadExactError::ReadError(ReadError::Reset(_))) => {
                let _ = out.flush().await;
                cache = Some(HashCache {
                    file: header.file,
                    pos,
                    hasher,
                });
                continue;
            }
            Err(_) => {
                let _ = ev.send(DataEv::Conn);
                return;
            }
        }
        let _ = out.flush().await;
        if hasher.finalize() != blake3::Hash::from_bytes(expected) {
            drop(out);
            let _ = tokio::fs::remove_file(&part).await;
            let _ = ev.send(DataEv::FileBad(header.file));
            return;
        }
        // Written to disk for good and moved to its name while the next file comes in. It only
        // counts as received once that's done.
        let Ok(permit) = finishing.clone().acquire_owned().await else { return };
        handed_off[i] = true;
        let (e, ev, wanted, file) = (e.clone(), ev.clone(), f.final_path.clone(), header.file);
        tokio::spawn(async move {
            let _permit = permit;
            let _ = out.sync_all().await;
            drop(out);
            // Never replaces a file someone created with our name since we reserved it.
            let finished = tokio::task::spawn_blocking(move || files::finish_part(&part, &wanted))
                .await
                .unwrap_or_else(|e| Err(std::io::Error::other(e)));
            match finished {
                Ok(final_path) => {
                    // Noted in the transfer first, so a resume can't ask for it again.
                    e.with_tx(&id, |tx| {
                        if let TxSide::Recv(all) = &mut tx.side {
                            all[file as usize].verified = true;
                            all[file as usize].final_path = final_path.clone();
                        }
                    });
                    let _ = ev.send(DataEv::FileOk(file, final_path));
                }
                Err(err) => {
                    let _ = ev.send(DataEv::Io(io_kind(&err)));
                }
            }
        });
    }
}

fn io_kind(err: &std::io::Error) -> TransferError {
    if files::is_disk_full(err) {
        TransferError::DiskFull
    } else if err.kind() == std::io::ErrorKind::PermissionDenied {
        TransferError::Permission
    } else {
        TransferError::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn resumes_hashing_from_the_nearest_mark() {
        let dir = crate::store::tests::tempdir("marks");
        let path = dir.join("big.bin");
        let data: Vec<u8> = (0..40 * 1024 * 1024u32).map(|i| (i % 251) as u8).collect();
        // Hashed as a transfer would, saving marks on the way.
        let marks = Marks::default();
        let mut hasher = blake3::Hasher::new();
        let mut pos = 0u64;
        for chunk in data.chunks(CHUNK) {
            hasher.update(chunk);
            mark(&marks, 0, pos, pos + chunk.len() as u64, &hasher);
            pos += chunk.len() as u64;
        }
        let at: Vec<u64> = marks.lock().unwrap().iter().map(|c| c.pos).collect();
        assert_eq!(at, [MARK_EVERY, 2 * MARK_EVERY]);

        // Bytes before the 32 MiB mark are never read again: damage there doesn't change the result.
        let mut damaged = data.clone();
        damaged[1000] ^= 0xff;
        std::fs::write(&path, &damaged).unwrap();
        let mut file = tokio::fs::File::open(&path).await.unwrap();
        let mut buf = vec![0u8; CHUNK];
        let resume = 35 * 1024 * 1024;
        let h = hash_prefix(&mut file, 0, resume, &marks, &mut buf).await.unwrap();
        assert_eq!(h.finalize(), blake3::hash(&data[..resume as usize]));

        // Before the first mark, or for another file, it reads from the start.
        let h = hash_prefix(&mut file, 0, 2000, &marks, &mut buf).await.unwrap();
        assert_eq!(h.finalize(), blake3::hash(&damaged[..2000]));
        let h = hash_prefix(&mut file, 1, resume, &marks, &mut buf).await.unwrap();
        assert_eq!(h.finalize(), blake3::hash(&damaged[..resume as usize]));
    }
}
