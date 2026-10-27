//! Share via link: a small web page on the same network, so a device without Nhako Beam (a friend's
//! phone, a work laptop) can download files with its browser.
//!
//! - The address is `http://<this device>:<port>/<key>/`, where the key is 16 random letters and
//!   digits (80 bits). Anything else on the port gets a "not sharing" page.
//! - Each browser that opens it waits until you let it in on your screen (a cookie tells browsers
//!   apart). Nothing about the files, not even their names, reaches a browser before that.
//! - It's plain HTTP: anyone on the same Wi-Fi could read what's downloaded this way. The apps say
//!   so; between two copies of Nhako Beam everything is encrypted as usual.
//! - It stops when you press Stop, or after [`SHARE_FOR`] once nothing is downloading.
//!
//! The server is a deliberately small HTTP/1.1 one (GET and HEAD, one request per connection, tight
//! limits on the request's size and time), so it needs no web framework.

use std::{
    collections::HashMap,
    io::SeekFrom,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    time::{Duration, Instant},
};

use tokio::{
    io::{AsyncReadExt, AsyncSeekExt, AsyncWrite, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::watch,
};
use tracing::{debug, info};

use crate::{
    beacon,
    engine::{Engine, EngineError, FILES_MAX, Outgoing, TEXT_MAX, now_ms},
    files,
    types::{EngineEvent, FileItem, LinkShare, LinkVisitor, VisitorState},
};

/// How long a link works by default. Downloads still running then carry on, and the link stops after them.
pub const SHARE_FOR: Duration = Duration::from_secs(15 * 60);
/// Browsers waiting for you to let them in, at most.
const ASKING_MAX: usize = 8;
/// Browsers in all, at most (asking, let in and turned away).
const VISITORS_MAX: usize = 32;
/// Connections open at once, at most.
const CONNECTIONS_MAX: usize = 64;
/// The longest request head (the request line and its headers).
const HEAD_MAX: usize = 8 * 1024;
/// How long a browser has to send its request.
const HEAD_WAIT: Duration = Duration::from_secs(10);
/// How often the app hears about downloads moving.
const PROGRESS_EVERY: Duration = Duration::from_millis(250);
/// Letters for the key and the cookies: no look-alikes (0/o, 1/l/i).
const KEY_LETTERS: &[u8] = b"abcdefghjkmnpqrstuvwxyz23456789";
const KEY_LEN: usize = 16;
const COOKIE: &str = "nbv";

const PAGE: &str = include_str!("share_page.html");
const GONE_PAGE: &str = include_str!("share_gone.html");

struct ShareFile {
    path: PathBuf,
    item: FileItem,
    /// Its name inside the zip of everything (`Trip/beach.jpg`), unique.
    zip_name: String,
    modified: Option<std::time::SystemTime>,
}

struct Visitor {
    view: LinkVisitor,
    /// Downloads running now: bytes done and their sizes, by download.
    active: HashMap<u64, (u64, u64)>,
}

/// The link being shared, kept in the engine's state. Dropping it stops the server and every
/// download (they watch `_stop`).
pub(crate) struct Hosted {
    id: String,
    key: String,
    url: String,
    files: Vec<ShareFile>,
    text: Option<String>,
    expires_at: u64,
    visitors: Vec<Visitor>,
    /// Cookie value to visitor id.
    cookies: HashMap<String, String>,
    next_download: u64,
    last_emit: Option<Instant>,
    _stop: watch::Sender<()>,
}

impl Hosted {
    fn view(&self) -> LinkShare {
        LinkShare {
            id: self.id.clone(),
            url: self.url.clone(),
            files: self.files.iter().map(|f| f.item.clone()).collect(),
            total_bytes: self.files.iter().map(|f| f.item.size).sum(),
            text: self.text.clone(),
            expires_at: self.expires_at,
            visitors: self
                .visitors
                .iter()
                .map(|v| {
                    let mut view = v.view.clone();
                    view.active_bytes = v.active.values().map(|a| a.0).sum();
                    view.active_total = v.active.values().map(|a| a.1).sum();
                    view
                })
                .collect(),
        }
    }

    fn visitor(&mut self, id: &str) -> Option<&mut Visitor> {
        self.visitors.iter_mut().find(|v| v.view.id == id)
    }

    fn by_cookie(&self, cookie: Option<&str>) -> Option<&Visitor> {
        let id = self.cookies.get(cookie?)?;
        self.visitors.iter().find(|v| &v.view.id == id)
    }

    fn downloading(&self) -> bool {
        self.visitors.iter().any(|v| !v.active.is_empty())
    }

    fn closing(&self) -> bool {
        now_ms() >= self.expires_at
    }
}

fn random_key() -> String {
    (0..KEY_LEN).map(|_| KEY_LETTERS[rand::random_range(0..KEY_LETTERS.len())] as char).collect()
}

/// This device's address on the local network: the adapter with the default route when it has a
/// same-network address, otherwise the first one that does.
fn lan_address() -> Option<IpAddr> {
    let adapters = beacon::lan_adapters();
    let default: Vec<std::net::Ipv4Addr> = netdev::get_default_interface()
        .map(|i| i.ipv4.iter().map(|n| n.addr()).collect())
        .unwrap_or_default();
    adapters
        .iter()
        .find(|a| default.contains(&a.addr))
        .or_else(|| adapters.iter().next())
        .map(|a| IpAddr::V4(a.addr))
}

impl Engine {
    /// Starts sharing files (and or text) with a link. Replaces a link already being shared.
    /// Must be called inside the tokio runtime.
    pub fn share_link(&self, out: &[Outgoing], text: Option<&str>) -> Result<LinkShare, EngineError> {
        let text = text.map(str::trim).filter(|t| !t.is_empty()).map(String::from);
        if out.is_empty() && text.is_none() {
            return Err(EngineError::NoFiles);
        }
        if text.as_ref().is_some_and(|t| t.len() > TEXT_MAX) {
            return Err(EngineError::TextTooLong);
        }
        if out.len() > FILES_MAX {
            return Err(EngineError::File {
                path: out[0].path.clone(),
                reason: format!("more than {FILES_MAX} files"),
            });
        }
        let mut shared = Vec::new();
        let mut zip_names = std::collections::HashSet::new();
        for (i, f) in out.iter().enumerate() {
            let meta = files::open_to_send(&f.path).and_then(|file| file.metadata()).map_err(|e| EngineError::File {
                path: f.path.clone(),
                reason: e.to_string(),
            })?;
            let name = files::safe_name(&f.name);
            let folder = f.folder.as_deref().and_then(files::safe_folder).map(|p| p.join("/"));
            let zip_name = unique_zip_name(&mut zip_names, folder.as_deref(), &name);
            shared.push(ShareFile {
                path: f.path.clone(),
                item: FileItem {
                    id: format!("f{i}"),
                    mime: files::mime_for(&name),
                    name,
                    size: meta.len(),
                    folder,
                },
                zip_name,
                modified: meta.modified().ok(),
            });
        }
        let ip = self.inner.cfg.test_share_ip.or_else(lan_address).ok_or(EngineError::NoLan)?;
        let std_listener = std::net::TcpListener::bind(SocketAddr::new(ip, 0)).map_err(|e| EngineError::Bind(e.to_string()))?;
        std_listener.set_nonblocking(true).map_err(|e| EngineError::Bind(e.to_string()))?;
        let listener = TcpListener::from_std(std_listener).map_err(|e| EngineError::Bind(e.to_string()))?;
        let port = listener.local_addr().map_err(|e| EngineError::Bind(e.to_string()))?.port();
        let key = random_key();
        let (stop_tx, stop_rx) = watch::channel(());
        let hosted = Hosted {
            id: random_key(),
            url: format!("http://{ip}:{port}/{key}/"),
            key,
            files: shared,
            text,
            expires_at: now_ms() + self.inner.cfg.share_for.as_millis() as u64,
            visitors: Vec::new(),
            cookies: HashMap::new(),
            next_download: 0,
            last_emit: None,
            _stop: stop_tx,
        };
        let view = hosted.view();
        info!(files = view.files.len(), "sharing with a link on {ip}:{port}");
        self.st().share = Some(hosted);
        let e = self.clone();
        let id = view.id.clone();
        tokio::spawn(async move { e.serve_link(listener, id, stop_rx).await });
        self.emit(EngineEvent::LinkShare { share: Some(view.clone()) });
        Ok(view)
    }

    /// Stops sharing with a link. Downloads running stop too.
    pub fn stop_link(&self) {
        if self.st().share.take().is_some() {
            info!("stopped sharing with a link");
            self.emit(EngineEvent::LinkShare { share: None });
        }
    }

    /// Lets a browser in (`allow`), or turns it away.
    pub fn answer_visitor(&self, visitor_id: &str, allow: bool) {
        let view = {
            let mut st = self.st();
            let Some(h) = st.share.as_mut() else { return };
            let Some(v) = h.visitor(visitor_id) else { return };
            v.view.state = if allow { VisitorState::Allowed } else { VisitorState::Denied };
            h.view()
        };
        self.emit(EngineEvent::LinkShare { share: Some(view) });
    }

    /// The link being shared, if any.
    pub fn link_share(&self) -> Option<LinkShare> {
        self.st().share.as_ref().map(Hosted::view)
    }

    /// Emits the share, now or (`progress`) at most every [`PROGRESS_EVERY`].
    fn emit_share(&self, progress: bool) {
        let view = {
            let mut st = self.st();
            let Some(h) = st.share.as_mut() else { return };
            if progress && h.last_emit.is_some_and(|t| t.elapsed() < PROGRESS_EVERY) {
                return;
            }
            h.last_emit = Some(Instant::now());
            h.view()
        };
        self.emit(EngineEvent::LinkShare { share: Some(view) });
    }

    async fn serve_link(self, listener: TcpListener, id: String, mut stop: watch::Receiver<()>) {
        let open = std::sync::Arc::new(tokio::sync::Semaphore::new(CONNECTIONS_MAX));
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        loop {
            tokio::select! {
                _ = stop.changed() => break,
                _ = tick.tick() => {
                    let over = {
                        let st = self.st();
                        match st.share.as_ref() {
                            Some(h) if h.id == id => h.closing() && !h.downloading(),
                            _ => break,
                        }
                    };
                    if over {
                        self.stop_link();
                        break;
                    }
                }
                got = listener.accept() => {
                    let Ok((stream, from)) = got else { continue };
                    // Only devices on the same network: the link is never meant to cross the internet.
                    if !crate::net::is_lan(from.ip()) && !from.ip().is_loopback() {
                        continue;
                    }
                    let Ok(permit) = open.clone().try_acquire_owned() else { continue };
                    let e = self.clone();
                    let id = id.clone();
                    let mut stop = stop.clone();
                    tokio::spawn(async move {
                        tokio::select! {
                            _ = stop.changed() => {}
                            r = e.serve_one(stream, from, &id) => if let Err(err) = r { debug!("link share connection: {err}") },
                        }
                        drop(permit);
                    });
                }
            }
        }
        debug!("link share server stopped");
    }

    async fn serve_one(&self, mut stream: TcpStream, from: SocketAddr, id: &str) -> std::io::Result<()> {
        let head = match tokio::time::timeout(HEAD_WAIT, read_head(&mut stream)).await {
            Ok(Ok(Some(h))) => h,
            Ok(Ok(None)) | Err(_) => return Ok(()),
            Ok(Err(e)) => return Err(e),
        };
        let Some(req) = Request::parse(&head) else {
            return respond(&mut stream, 400, "text/plain; charset=utf-8", &[], b"Bad request", false).await;
        };
        let head_only = req.method == "HEAD";
        if req.method != "GET" && !head_only {
            return respond(
                &mut stream,
                405,
                "text/plain; charset=utf-8",
                &[("Allow", "GET, HEAD".into())],
                b"Only GET",
                false,
            )
            .await;
        }
        let key = self.st().share.as_ref().filter(|h| h.id == id).map(|h| h.key.clone());
        let Some(key) = key else {
            return respond(&mut stream, 404, "text/html; charset=utf-8", &[], GONE_PAGE.as_bytes(), head_only).await;
        };
        let path = req.target.split(['?', '#']).next().unwrap_or("");
        let parts: Vec<&str> = path.split('/').skip(1).collect();
        if parts.first() != Some(&key.as_str()) {
            return respond(&mut stream, 404, "text/html; charset=utf-8", &[], GONE_PAGE.as_bytes(), head_only).await;
        }
        let cookie = req.cookie(COOKIE);
        match &parts[1..] {
            [] => {
                // Without the slash, relative addresses on the page would miss the key.
                let to = format!("/{key}/");
                respond(&mut stream, 308, "text/plain; charset=utf-8", &[("Location", to)], b"", head_only).await
            }
            [""] => {
                let mut headers = vec![("Content-Security-Policy", PAGE_CSP.to_string())];
                if !head_only && let Some(set) = self.arrive(id, cookie.as_deref(), &req, from) {
                    headers.push(("Set-Cookie", format!("{COOKIE}={set}; Path=/{key}/; HttpOnly; SameSite=Strict")));
                }
                respond(&mut stream, 200, "text/html; charset=utf-8", &headers, PAGE.as_bytes(), head_only).await
            }
            ["state"] => {
                let body = self.state_json(id, cookie.as_deref());
                respond(&mut stream, 200, "application/json", &[], body.as_bytes(), head_only).await
            }
            ["list"] => match self.list_json(id, cookie.as_deref()) {
                Some(body) => respond(&mut stream, 200, "application/json", &[], body.as_bytes(), head_only).await,
                None => respond(&mut stream, 403, "text/plain; charset=utf-8", &[], b"Not let in", head_only).await,
            },
            ["f", index, ..] => {
                let Some(i) = index.parse::<usize>().ok() else {
                    return respond(&mut stream, 404, "text/plain; charset=utf-8", &[], b"Not found", head_only).await;
                };
                self.send_file(&mut stream, id, cookie.as_deref(), i, req.header("range"), head_only).await
            }
            ["all.zip"] => self.send_zip(&mut stream, id, cookie.as_deref(), head_only).await,
            _ => respond(&mut stream, 404, "text/plain; charset=utf-8", &[], b"Not found", head_only).await,
        }
    }

    /// A browser opened the page. Returns a new cookie when it's a new browser and there's room.
    fn arrive(&self, id: &str, cookie: Option<&str>, req: &Request, from: SocketAddr) -> Option<String> {
        let view = {
            let mut st = self.st();
            let h = st.share.as_mut().filter(|h| h.id == id)?;
            if h.by_cookie(cookie).is_some() || h.closing() {
                return None;
            }
            let asking = h.visitors.iter().filter(|v| v.view.state == VisitorState::Asking).count();
            if asking >= ASKING_MAX || h.visitors.len() >= VISITORS_MAX {
                return None;
            }
            let secret = format!("{}{}", random_key(), random_key());
            let visitor = LinkVisitor {
                id: random_key(),
                label: browser_label(req.header("user-agent").as_deref().unwrap_or("")),
                address: from.ip().to_string(),
                state: VisitorState::Asking,
                arrived_at: now_ms(),
                active_bytes: 0,
                active_total: 0,
                downloads: 0,
            };
            h.cookies.insert(secret.clone(), visitor.id.clone());
            h.visitors.push(Visitor {
                view: visitor,
                active: HashMap::new(),
            });
            (h.view(), secret)
        };
        self.emit(EngineEvent::LinkShare { share: Some(view.0) });
        Some(view.1)
    }

    fn state_json(&self, id: &str, cookie: Option<&str>) -> String {
        let me = self.me_info().name;
        let st = self.st();
        let state = match st.share.as_ref().filter(|h| h.id == id) {
            None => "ended",
            Some(h) => match h.by_cookie(cookie) {
                Some(v) => match v.view.state {
                    VisitorState::Asking if h.closing() => "ended",
                    VisitorState::Asking => "asking",
                    VisitorState::Allowed => "allowed",
                    VisitorState::Denied => "denied",
                },
                None if h.closing() => "ended",
                None => "full",
            },
        };
        serde_json::json!({ "state": state, "from": me }).to_string()
    }

    /// What's shared, for a browser that was let in.
    fn list_json(&self, id: &str, cookie: Option<&str>) -> Option<String> {
        let me = self.me_info().name;
        let st = self.st();
        let h = st.share.as_ref().filter(|h| h.id == id)?;
        if h.by_cookie(cookie)?.view.state != VisitorState::Allowed {
            return None;
        }
        let files: Vec<_> = h
            .files
            .iter()
            .enumerate()
            .map(|(i, f)| {
                serde_json::json!({
                    "name": f.item.name,
                    "folder": f.item.folder,
                    "size": f.item.size,
                    "href": format!("f/{i}/{}", pct_encode(&f.item.name)),
                })
            })
            .collect();
        let zip = (h.files.len() > 1 || h.files.iter().any(|f| f.item.folder.is_some())).then(|| zip_len(&zip_plan(&h.files)));
        Some(
            serde_json::json!({
                "from": me,
                "files": files,
                "totalBytes": h.files.iter().map(|f| f.item.size).sum::<u64>(),
                "zipBytes": zip,
                "text": h.text,
            })
            .to_string(),
        )
    }

    /// Checks the browser was let in and the link is still open for new downloads, then counts a
    /// download of `total` bytes for it. Returns the download's number and the files' paths.
    fn begin_download(&self, id: &str, cookie: Option<&str>, total: u64) -> Option<(String, u64)> {
        let mut st = self.st();
        let h = st.share.as_mut().filter(|h| h.id == id)?;
        if h.closing() {
            return None;
        }
        let visitor = h.by_cookie(cookie).filter(|v| v.view.state == VisitorState::Allowed)?.view.id.clone();
        let n = h.next_download;
        h.next_download += 1;
        h.visitor(&visitor)?.active.insert(n, (0, total));
        Some((visitor, n))
    }

    fn moved(&self, visitor: &str, n: u64, done: u64) {
        if let Some(v) = self.st().share.as_mut().and_then(|h| h.visitor(visitor))
            && let Some(a) = v.active.get_mut(&n)
        {
            a.0 = done;
        }
        self.emit_share(true);
    }

    fn end_download(&self, visitor: &str, n: u64, finished: bool) {
        if let Some(v) = self.st().share.as_mut().and_then(|h| h.visitor(visitor)) {
            v.active.remove(&n);
            if finished {
                v.view.downloads += 1;
            }
        }
        self.emit_share(false);
    }

    async fn send_file(&self, stream: &mut TcpStream, id: &str, cookie: Option<&str>, i: usize, range: Option<String>, head_only: bool) -> std::io::Result<()> {
        let file = {
            let st = self.st();
            st.share
                .as_ref()
                .filter(|h| h.id == id)
                .and_then(|h| h.files.get(i))
                .map(|f| (f.path.clone(), f.item.name.clone(), f.item.mime.clone(), f.item.size))
        };
        let Some((path, name, mime, size)) = file else {
            return respond(stream, 404, "text/plain; charset=utf-8", &[], b"Not found", head_only).await;
        };
        let (start, end) = match range.as_deref().map(|r| parse_range(r, size)) {
            None => (0, size),
            Some(Some(r)) => r,
            Some(None) => {
                return respond(
                    stream,
                    416,
                    "text/plain; charset=utf-8",
                    &[("Content-Range", format!("bytes */{size}"))],
                    b"",
                    head_only,
                )
                .await;
            }
        };
        let Some((visitor, n)) = self.begin_download(id, cookie, end - start) else {
            return respond(stream, 403, "text/plain; charset=utf-8", &[], b"Not let in", head_only).await;
        };
        let result = async {
            let mut f = tokio::fs::File::from_std(files::open_to_send(&path)?);
            f.seek(SeekFrom::Start(start)).await?;
            let mut headers = vec![
                ("Content-Disposition", attachment(&name)),
                ("Accept-Ranges", "bytes".to_string()),
                ("Content-Length", (end - start).to_string()),
            ];
            let status = if range.is_some() {
                headers.push(("Content-Range", format!("bytes {start}-{}/{size}", end.saturating_sub(1))));
                206
            } else {
                200
            };
            write_head(stream, status, &mime, &headers).await?;
            if head_only {
                return Ok(());
            }
            let mut done = 0;
            copy_exact(&mut f, stream, end - start, &mut |sent| {
                done += sent;
                self.moved(&visitor, n, done);
            })
            .await?;
            stream.flush().await
        }
        .await;
        self.end_download(&visitor, n, result.is_ok() && !head_only);
        result
    }

    async fn send_zip(&self, stream: &mut TcpStream, id: &str, cookie: Option<&str>, head_only: bool) -> std::io::Result<()> {
        let plan = {
            let st = self.st();
            st.share.as_ref().filter(|h| h.id == id).map(|h| zip_plan(&h.files))
        };
        let Some(plan) = plan.filter(|p| !p.is_empty()) else {
            return respond(stream, 404, "text/plain; charset=utf-8", &[], b"Not found", head_only).await;
        };
        let total = zip_len(&plan);
        let Some((visitor, n)) = self.begin_download(id, cookie, total) else {
            return respond(stream, 403, "text/plain; charset=utf-8", &[], b"Not let in", head_only).await;
        };
        let name = format!("{}.zip", zip_title(&plan));
        let result = async {
            write_head(
                stream,
                200,
                "application/zip",
                &[("Content-Disposition", attachment(&name)), ("Content-Length", total.to_string())],
            )
            .await?;
            if head_only {
                return Ok(());
            }
            let mut done = 0;
            write_zip(stream, &plan, &mut |sent| {
                done += sent;
                self.moved(&visitor, n, done);
            })
            .await?;
            stream.flush().await
        }
        .await;
        self.end_download(&visitor, n, result.is_ok() && !head_only);
        result
    }
}

/// The page's own rules: nothing from anywhere else, and its scripts may only talk to this server.
const PAGE_CSP: &str = "default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; connect-src 'self'; img-src data:; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

/// Reads the request head, up to the blank line. `None` when the browser closed or sent too much.
async fn read_head(stream: &mut TcpStream) -> std::io::Result<Option<String>> {
    let mut buf = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    loop {
        let n = stream.read(&mut chunk).await?;
        if n == 0 {
            return Ok(None);
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            buf.truncate(end);
            return Ok(String::from_utf8(buf).ok());
        }
        if buf.len() > HEAD_MAX {
            return Ok(None);
        }
    }
}

struct Request {
    method: String,
    target: String,
    headers: Vec<(String, String)>,
}

impl Request {
    fn parse(head: &str) -> Option<Self> {
        let mut lines = head.split("\r\n");
        let mut first = lines.next()?.split(' ');
        let (method, target, version) = (first.next()?, first.next()?, first.next()?);
        if !version.starts_with("HTTP/1.") || !target.starts_with('/') || first.next().is_some() {
            return None;
        }
        let headers = lines
            .filter_map(|l| l.split_once(':'))
            .map(|(k, v)| (k.trim().to_ascii_lowercase(), v.trim().to_string()))
            .take(64)
            .collect();
        Some(Self {
            method: method.to_string(),
            target: target.to_string(),
            headers,
        })
    }

    fn header(&self, name: &str) -> Option<String> {
        self.headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone())
    }

    fn cookie(&self, name: &str) -> Option<String> {
        self.headers
            .iter()
            .filter(|(k, _)| k == "cookie")
            .flat_map(|(_, v)| v.split(';'))
            .filter_map(|c| c.trim().split_once('='))
            .find(|(k, _)| *k == name)
            .map(|(_, v)| v.to_string())
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        206 => "Partial Content",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        416 => "Range Not Satisfiable",
        _ => "",
    }
}

async fn write_head(stream: &mut (impl AsyncWrite + Unpin), status: u16, kind: &str, headers: &[(&str, String)]) -> std::io::Result<()> {
    let mut head = format!(
        "HTTP/1.1 {status} {}\r\nContent-Type: {kind}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nX-Frame-Options: DENY\r\nConnection: close\r\n",
        reason(status)
    );
    for (k, v) in headers {
        head.push_str(k);
        head.push_str(": ");
        head.push_str(v);
        head.push_str("\r\n");
    }
    head.push_str("\r\n");
    stream.write_all(head.as_bytes()).await
}

async fn respond(stream: &mut TcpStream, status: u16, kind: &str, headers: &[(&str, String)], body: &[u8], head_only: bool) -> std::io::Result<()> {
    let mut all = headers.to_vec();
    all.push(("Content-Length", body.len().to_string()));
    write_head(stream, status, kind, &all).await?;
    if !head_only {
        stream.write_all(body).await?;
    }
    stream.flush().await
}

/// `bytes=N-`, `bytes=N-M` or `bytes=-N` (the last N), as `start..end`. `None` when it can't be
/// served (other forms, several ranges, or outside the file).
fn parse_range(r: &str, size: u64) -> Option<(u64, u64)> {
    let spec = r.trim().strip_prefix("bytes=")?;
    if spec.contains(',') {
        return None;
    }
    let (a, b) = spec.split_once('-')?;
    let (start, end) = if a.is_empty() {
        let last: u64 = b.parse().ok()?;
        (size.saturating_sub(last), size)
    } else {
        let start: u64 = a.parse().ok()?;
        let end = if b.is_empty() {
            size
        } else {
            b.parse::<u64>().ok()?.saturating_add(1).min(size)
        };
        (start, end)
    };
    (start < end && start < size).then_some((start, end))
}

/// `Content-Disposition` for a download: a plain fallback name, and the real one for browsers
/// that read UTF-8 (all current ones).
fn attachment(name: &str) -> String {
    let plain: String = name
        .chars()
        .map(|c| if c.is_ascii_graphic() && c != '"' && c != '\\' || c == ' ' { c } else { '_' })
        .collect();
    format!("attachment; filename=\"{plain}\"; filename*=UTF-8''{}", pct_encode(name))
}

/// Percent-encodes everything but letters, digits and `-._~`.
fn pct_encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// Copies exactly `len` bytes, telling `progress` as they go. A file that got shorter is an error.
async fn copy_exact(from: &mut tokio::fs::File, to: &mut (impl AsyncWrite + Unpin), len: u64, progress: &mut impl FnMut(u64)) -> std::io::Result<u32> {
    let mut crc = crc32fast::Hasher::new();
    let mut left = len;
    let mut buf = vec![0u8; 64 * 1024];
    while left > 0 {
        let want = buf.len().min(usize::try_from(left).unwrap_or(usize::MAX));
        let n = from.read(&mut buf[..want]).await?;
        if n == 0 {
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "the file got shorter"));
        }
        crc.update(&buf[..n]);
        to.write_all(&buf[..n]).await?;
        left -= n as u64;
        progress(n as u64);
    }
    Ok(crc.finalize())
}

/// A browser's name and system, from what it says about itself: "Chrome on Android".
pub(crate) fn browser_label(ua: &str) -> String {
    let browser = [
        ("SamsungBrowser", "Samsung Internet"),
        ("Edg", "Edge"),
        ("OPR", "Opera"),
        ("Firefox", "Firefox"),
        ("FxiOS", "Firefox"),
        ("CriOS", "Chrome"),
        ("Chrome", "Chrome"),
        ("Safari", "Safari"),
    ]
    .iter()
    .find(|(k, _)| ua.contains(k))
    .map(|(_, v)| *v);
    let system = [
        ("iPhone", "iPhone"),
        ("iPad", "iPad"),
        ("Android", "Android"),
        ("CrOS", "ChromeOS"),
        ("Windows", "Windows"),
        ("Mac OS X", "Mac"),
        ("Linux", "Linux"),
    ]
    .iter()
    .find(|(k, _)| ua.contains(k))
    .map(|(_, v)| *v);
    match (browser, system) {
        (Some(b), Some(s)) => format!("{b} on {s}"),
        (Some(b), None) => b.to_string(),
        (None, Some(s)) => format!("A browser on {s}"),
        (None, None) => "A browser".to_string(),
    }
}

// The zip of everything: stored (no compression, so it streams at disk speed and its length is
// known up front), with sizes and checksums after each file (data descriptors), and Zip64 where
// a file or the archive passes 4 GB.

const U32_MAX: u64 = 0xFFFF_FFFF;

struct ZipEntry {
    path: PathBuf,
    name: String,
    size: u64,
    offset: u64,
    dos: (u16, u16),
    /// The modified time in Unix seconds, as an extended timestamp (extra field 0x5455), so
    /// extractors show it in local time; the DOS time has no time zone.
    unix: Option<u32>,
}

/// Bytes of the extended timestamp field: id, size, flags and the time.
const UT_LEN: u64 = 9;

impl ZipEntry {
    fn zip64(&self) -> bool {
        self.size >= U32_MAX || self.offset >= U32_MAX
    }
    fn ut_len(&self) -> u64 {
        if self.unix.is_some() { UT_LEN } else { 0 }
    }
    fn local_len(&self) -> u64 {
        30 + self.name.len() as u64 + self.ut_len() + self.size + if self.size >= U32_MAX { 24 } else { 16 }
    }
    fn central_len(&self) -> u64 {
        46 + self.name.len() as u64 + if self.zip64() { 28 } else { 0 } + self.ut_len()
    }
    fn ut_field(&self) -> Vec<u8> {
        let Some(t) = self.unix else { return vec![] };
        let mut f = Vec::with_capacity(UT_LEN as usize);
        f.extend_from_slice(&0x5455u16.to_le_bytes());
        f.extend_from_slice(&5u16.to_le_bytes());
        // Flags: the modified time is there.
        f.push(1);
        f.extend_from_slice(&t.to_le_bytes());
        f
    }
}

fn unique_zip_name(taken: &mut std::collections::HashSet<String>, folder: Option<&str>, name: &str) -> String {
    let join = |n: &str| folder.map(|f| format!("{f}/{n}")).unwrap_or_else(|| n.to_string());
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) if !s.is_empty() => (s.to_string(), format!(".{e}")),
        _ => (name.to_string(), String::new()),
    };
    let mut candidate = join(name);
    let mut n = 1;
    while !taken.insert(candidate.to_lowercase()) {
        candidate = join(&format!("{stem} ({n}){ext}"));
        n += 1;
    }
    candidate
}

fn zip_plan(files: &[ShareFile]) -> Vec<ZipEntry> {
    let mut offset = 0;
    files
        .iter()
        .map(|f| {
            let e = ZipEntry {
                path: f.path.clone(),
                name: f.zip_name.clone(),
                size: f.item.size,
                offset,
                dos: dos_time(f.modified),
                unix: f
                    .modified
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .and_then(|d| u32::try_from(d.as_secs()).ok()),
            };
            offset += e.local_len();
            e
        })
        .collect()
}

/// The zip's name: the folder's when everything is in one folder, otherwise "Nhako Beam".
fn zip_title(plan: &[ZipEntry]) -> String {
    let top = |e: &ZipEntry| e.name.split_once('/').map(|(t, _)| t.to_string());
    match plan.first().and_then(top) {
        Some(t) if plan.iter().all(|e| top(e).as_deref() == Some(&t)) => t,
        _ => "Nhako Beam".to_string(),
    }
}

fn zip_needs64(plan: &[ZipEntry], cd_offset: u64, cd_len: u64) -> bool {
    plan.len() >= 0xFFFF || cd_offset >= U32_MAX || cd_len >= U32_MAX || plan.iter().any(ZipEntry::zip64)
}

fn zip_len(plan: &[ZipEntry]) -> u64 {
    let cd_offset: u64 = plan.iter().map(ZipEntry::local_len).sum();
    let cd_len: u64 = plan.iter().map(ZipEntry::central_len).sum();
    cd_offset + cd_len + if zip_needs64(plan, cd_offset, cd_len) { 56 + 20 } else { 0 } + 22
}

/// MS-DOS date and time (UTC; zip has no time zone), or 1980-01-01 when unknown.
fn dos_time(t: Option<std::time::SystemTime>) -> (u16, u16) {
    let secs = t.and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    // Civil date from days since 1970 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    if !(1980..=2107).contains(&year) {
        return (0, (1 << 5) | 1);
    }
    let time = (((rem / 3600) << 11) | ((rem % 3600 / 60) << 5) | ((rem % 60) / 2)) as u16;
    let date = (((year - 1980) << 9) | (month << 5) | day) as u16;
    (time, date)
}

async fn write_zip(out: &mut (impl AsyncWrite + Unpin), plan: &[ZipEntry], progress: &mut impl FnMut(u64)) -> std::io::Result<()> {
    let mut crcs = Vec::with_capacity(plan.len());
    for e in plan {
        let mut h = Vec::with_capacity(30 + e.name.len());
        h.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        h.extend_from_slice(&(if e.zip64() { 45u16 } else { 20 }).to_le_bytes());
        // Sizes after the data (bit 3), names in UTF-8 (bit 11).
        h.extend_from_slice(&0x0808u16.to_le_bytes());
        h.extend_from_slice(&0u16.to_le_bytes());
        h.extend_from_slice(&e.dos.0.to_le_bytes());
        h.extend_from_slice(&e.dos.1.to_le_bytes());
        h.extend_from_slice(&[0; 12]);
        h.extend_from_slice(&(e.name.len() as u16).to_le_bytes());
        h.extend_from_slice(&(e.ut_len() as u16).to_le_bytes());
        h.extend_from_slice(e.name.as_bytes());
        h.extend_from_slice(&e.ut_field());
        out.write_all(&h).await?;
        progress(h.len() as u64);
        let mut f = tokio::fs::File::from_std(files::open_to_send(&e.path)?);
        let crc = copy_exact(&mut f, out, e.size, progress).await?;
        let mut d = Vec::with_capacity(24);
        d.extend_from_slice(&0x0807_4b50u32.to_le_bytes());
        d.extend_from_slice(&crc.to_le_bytes());
        if e.size >= U32_MAX {
            d.extend_from_slice(&e.size.to_le_bytes());
            d.extend_from_slice(&e.size.to_le_bytes());
        } else {
            d.extend_from_slice(&(e.size as u32).to_le_bytes());
            d.extend_from_slice(&(e.size as u32).to_le_bytes());
        }
        out.write_all(&d).await?;
        progress(d.len() as u64);
        crcs.push(crc);
    }
    let cd_offset: u64 = plan.iter().map(ZipEntry::local_len).sum();
    let mut cd = Vec::new();
    for (e, crc) in plan.iter().zip(crcs) {
        let big = e.zip64();
        let clamp = |v: u64| if big { u32::MAX } else { v as u32 };
        cd.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        // Made by: Unix (3), spec 4.5; needed: 2.0, or 4.5 for Zip64.
        cd.extend_from_slice(&((3u16 << 8) | 45).to_le_bytes());
        cd.extend_from_slice(&(if big { 45u16 } else { 20 }).to_le_bytes());
        cd.extend_from_slice(&0x0808u16.to_le_bytes());
        cd.extend_from_slice(&0u16.to_le_bytes());
        cd.extend_from_slice(&e.dos.0.to_le_bytes());
        cd.extend_from_slice(&e.dos.1.to_le_bytes());
        cd.extend_from_slice(&crc.to_le_bytes());
        cd.extend_from_slice(&clamp(e.size).to_le_bytes());
        cd.extend_from_slice(&clamp(e.size).to_le_bytes());
        cd.extend_from_slice(&(e.name.len() as u16).to_le_bytes());
        cd.extend_from_slice(&((if big { 28u16 } else { 0 }) + e.ut_len() as u16).to_le_bytes());
        // Comment length, disk number, internal attributes.
        cd.extend_from_slice(&[0; 6]);
        // External attributes: a regular file, rw-r--r--.
        cd.extend_from_slice(&((0o100_644u32) << 16).to_le_bytes());
        cd.extend_from_slice(&clamp(e.offset).to_le_bytes());
        cd.extend_from_slice(e.name.as_bytes());
        if big {
            cd.extend_from_slice(&1u16.to_le_bytes());
            cd.extend_from_slice(&24u16.to_le_bytes());
            cd.extend_from_slice(&e.size.to_le_bytes());
            cd.extend_from_slice(&e.size.to_le_bytes());
            cd.extend_from_slice(&e.offset.to_le_bytes());
        }
        cd.extend_from_slice(&e.ut_field());
    }
    let cd_len = cd.len() as u64;
    let count = plan.len() as u64;
    if zip_needs64(plan, cd_offset, cd_len) {
        let end64 = cd_offset + cd_len;
        cd.extend_from_slice(&0x0606_4b50u32.to_le_bytes());
        cd.extend_from_slice(&44u64.to_le_bytes());
        cd.extend_from_slice(&((3u16 << 8) | 45).to_le_bytes());
        cd.extend_from_slice(&45u16.to_le_bytes());
        cd.extend_from_slice(&[0; 8]);
        cd.extend_from_slice(&count.to_le_bytes());
        cd.extend_from_slice(&count.to_le_bytes());
        cd.extend_from_slice(&cd_len.to_le_bytes());
        cd.extend_from_slice(&cd_offset.to_le_bytes());
        cd.extend_from_slice(&0x0706_4b50u32.to_le_bytes());
        cd.extend_from_slice(&0u32.to_le_bytes());
        cd.extend_from_slice(&end64.to_le_bytes());
        cd.extend_from_slice(&1u32.to_le_bytes());
    }
    let c16 = count.min(0xFFFF) as u16;
    cd.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
    cd.extend_from_slice(&[0; 4]);
    cd.extend_from_slice(&c16.to_le_bytes());
    cd.extend_from_slice(&c16.to_le_bytes());
    cd.extend_from_slice(&(cd_len.min(U32_MAX) as u32).to_le_bytes());
    cd.extend_from_slice(&(cd_offset.min(U32_MAX) as u32).to_le_bytes());
    cd.extend_from_slice(&0u16.to_le_bytes());
    out.write_all(&cd).await?;
    progress(cd.len() as u64);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges() {
        assert_eq!(parse_range("bytes=0-", 10), Some((0, 10)));
        assert_eq!(parse_range("bytes=4-6", 10), Some((4, 7)));
        assert_eq!(parse_range("bytes=4-60", 10), Some((4, 10)));
        assert_eq!(parse_range("bytes=-3", 10), Some((7, 10)));
        assert_eq!(parse_range("bytes=10-", 10), None);
        assert_eq!(parse_range("bytes=0-1,4-5", 10), None);
        assert_eq!(parse_range("items=0-1", 10), None);
    }

    #[test]
    fn browsers_by_name() {
        let android = "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/129.0 Mobile Safari/537.36";
        assert_eq!(browser_label(android), "Chrome on Android");
        let iphone = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1";
        assert_eq!(browser_label(iphone), "Safari on iPhone");
        let edge = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/129.0 Safari/537.36 Edg/129.0";
        assert_eq!(browser_label(edge), "Edge on Windows");
        assert_eq!(browser_label("curl/8.5"), "A browser");
    }

    #[test]
    fn requests_and_cookies() {
        let r = Request::parse("GET /abc/f/0/a.jpg HTTP/1.1\r\nHost: x\r\nCookie: a=1; nbv=secret\r\nRange: bytes=5-").unwrap();
        assert_eq!(r.method, "GET");
        assert_eq!(r.target, "/abc/f/0/a.jpg");
        assert_eq!(r.cookie("nbv").as_deref(), Some("secret"));
        assert_eq!(r.header("range").as_deref(), Some("bytes=5-"));
        assert!(Request::parse("GET http://evil/ HTTP/1.1").is_none());
        assert!(Request::parse("GET / SPDY").is_none());
    }

    #[test]
    fn names_in_the_zip_stay_apart() {
        let mut taken = std::collections::HashSet::new();
        assert_eq!(unique_zip_name(&mut taken, None, "a.jpg"), "a.jpg");
        assert_eq!(unique_zip_name(&mut taken, None, "A.jpg"), "A (1).jpg");
        assert_eq!(unique_zip_name(&mut taken, Some("Trip"), "a.jpg"), "Trip/a.jpg");
        assert_eq!(unique_zip_name(&mut taken, None, "notes"), "notes");
        assert_eq!(unique_zip_name(&mut taken, None, "notes"), "notes (1)");
    }

    #[test]
    fn dos_times() {
        // 2024-02-29 13:45:30 UTC.
        let t = std::time::UNIX_EPOCH + Duration::from_secs(1_709_214_330);
        let (time, date) = dos_time(Some(t));
        assert_eq!(date >> 9, 44);
        assert_eq!((date >> 5) & 0xF, 2);
        assert_eq!(date & 0x1F, 29);
        assert_eq!(time >> 11, 13);
        assert_eq!((time >> 5) & 0x3F, 45);
        assert_eq!((time & 0x1F) * 2, 30);
        assert_eq!(dos_time(None), (0, 33));
    }

    #[test]
    fn attachment_names() {
        assert_eq!(attachment("a b.jpg"), "attachment; filename=\"a b.jpg\"; filename*=UTF-8''a%20b.jpg");
        assert_eq!(
            attachment("café\".txt"),
            "attachment; filename=\"caf__.txt\"; filename*=UTF-8''caf%C3%A9%22.txt"
        );
    }

    /// The zip reads back in a real zip reader, its length is what was promised, and a big entry
    /// switches to Zip64 in the plan.
    #[tokio::test]
    async fn zip_of_everything() {
        let dir = std::env::temp_dir().join(format!("nhako-zip-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mut taken = std::collections::HashSet::new();
        let mut shared = Vec::new();
        for (name, folder, body) in [
            ("a.txt", None, &b"hello"[..]),
            ("b.bin", Some("Trip/day 1"), &[7u8; 70_000][..]),
            ("é.txt", Some("Trip"), b""),
        ] {
            let path = dir.join(name);
            std::fs::write(&path, body).unwrap();
            shared.push(ShareFile {
                zip_name: unique_zip_name(&mut taken, folder, name),
                path,
                item: FileItem {
                    id: String::new(),
                    name: name.into(),
                    size: body.len() as u64,
                    mime: String::new(),
                    folder: folder.map(String::from),
                },
                modified: Some(std::time::SystemTime::now()),
            });
        }
        let plan = zip_plan(&shared);
        let mut out = Vec::new();
        let mut moved = 0;
        write_zip(&mut out, &plan, &mut |n| moved += n).await.unwrap();
        assert_eq!(out.len() as u64, zip_len(&plan));
        assert_eq!(moved, zip_len(&plan));
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(out)).unwrap();
        let names: Vec<String> = zip.file_names().map(String::from).collect();
        assert_eq!(names, ["a.txt", "Trip/day 1/b.bin", "Trip/é.txt"]);
        let mut body = Vec::new();
        std::io::Read::read_to_end(&mut zip.by_name("Trip/day 1/b.bin").unwrap(), &mut body).unwrap();
        assert_eq!(body, vec![7u8; 70_000]);
        assert_eq!(zip_title(&plan), "Nhako Beam");
        assert_eq!(zip_title(&plan[1..]), "Trip");

        let big = ZipEntry {
            path: PathBuf::new(),
            name: "big".into(),
            size: 5 << 30,
            offset: 0,
            dos: (0, 0),
            unix: None,
        };
        assert!(big.zip64());
        assert_eq!(zip_len(&[big]), 30 + 3 + (5 << 30) + 24 + 46 + 3 + 28 + 56 + 20 + 22);
        let _ = std::fs::remove_dir_all(dir);
    }

    /// A file past 4 GB goes through Zip64 and reads back. Slow and needs about 5 GB of disk:
    /// `cargo test -p nhako-core --lib zip64 -- --ignored`.
    #[tokio::test]
    #[ignore]
    async fn zip64_reads_back() {
        let dir = std::env::temp_dir().join(format!("nhako-zip64-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let big = dir.join("big.bin");
        // Sparse: takes no disk until read.
        std::fs::File::create(&big).unwrap().set_len((4 << 30) + 123).unwrap();
        let small = dir.join("small.txt");
        std::fs::write(&small, "after the big one").unwrap();
        let mut taken = std::collections::HashSet::new();
        let shared: Vec<ShareFile> = [(&big, "big.bin", (4u64 << 30) + 123), (&small, "small.txt", 17)]
            .into_iter()
            .map(|(path, name, size)| ShareFile {
                path: path.clone(),
                zip_name: unique_zip_name(&mut taken, None, name),
                item: FileItem {
                    id: String::new(),
                    name: name.into(),
                    size,
                    mime: String::new(),
                    folder: None,
                },
                modified: None,
            })
            .collect();
        let plan = zip_plan(&shared);
        assert!(plan[0].zip64() && plan[1].zip64(), "the second starts past 4 GB");
        let out_path = dir.join("all.zip");
        let mut out = tokio::io::BufWriter::new(tokio::fs::File::create(&out_path).await.unwrap());
        write_zip(&mut out, &plan, &mut |_| {}).await.unwrap();
        out.flush().await.unwrap();
        drop(out);
        assert_eq!(std::fs::metadata(&out_path).unwrap().len(), zip_len(&plan));
        let mut zip = zip::ZipArchive::new(std::fs::File::open(&out_path).unwrap()).unwrap();
        assert_eq!(zip.by_name("big.bin").unwrap().size(), (4 << 30) + 123);
        let mut body = String::new();
        std::io::Read::read_to_string(&mut zip.by_name("small.txt").unwrap(), &mut body).unwrap();
        assert_eq!(body, "after the big one");
        if std::env::var_os("NHAKO_KEEP_ZIP").is_none() {
            let _ = std::fs::remove_dir_all(dir);
        } else {
            eprintln!("kept {}", out_path.display());
        }
    }
}
