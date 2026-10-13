//! Four-word codes: showing one (host) and typing one (joiner). See docs/protocol/nhako-1.md, section 7.

use std::time::Duration;

use iroh::{EndpointId, endpoint::Connection};
use n0_future::StreamExt;
use tokio::sync::oneshot;
use tracing::{debug, info, warn};

use crate::{
    codekeys::{CodeKeys, WINDOW_SECS, candidate_windows, macs_equal},
    engine::{Engine, now_ms},
    net,
    types::{Device, EngineEvent, PairVia, PairingCode, RedeemFailure, RedeemResult, Route, ServerProblem},
    wire::{self, DeviceInfo, PairDone, PairJoin, PairWelcome},
    words,
};

/// Bad handshakes allowed before a shown code is burned.
const MAX_BAD_ATTEMPTS: u32 = 3;

pub(crate) struct HostedCode {
    pub code: PairingCode,
    stop: Option<oneshot::Sender<()>>,
}

impl Engine {
    /// Shows a new code (replacing any current one). The code lives 10 minutes and works once.
    pub fn create_code(&self) -> PairingCode {
        self.cancel_code();
        let words = words::random_words();
        let now = now_ms();
        let window = now / 1000 / WINDOW_SECS;
        let code = PairingCode {
            words: words.clone(),
            expires_at: now + WINDOW_SECS * 1000,
            uri: format!("nhakobeam://pair/{}", words.join("-")),
        };
        let (stop_tx, stop_rx) = oneshot::channel();
        let task = tokio::spawn(self.clone().host_code(code.clone(), window, stop_rx));
        if let Some(old) = self.inner.code_task.lock().expect("code task lock").replace(task) {
            old.abort();
        }
        self.st().code = Some(HostedCode {
            code: code.clone(),
            stop: Some(stop_tx),
        });
        self.emit(EngineEvent::Code { code: Some(code.clone()) });
        code
    }

    pub fn cancel_code(&self) {
        let hosted = self.st().code.take();
        if let Some(mut h) = hosted {
            if let Some(stop) = h.stop.take() {
                let _ = stop.send(());
            }
            self.emit(EngineEvent::Code { code: None });
        }
    }

    pub fn current_code(&self) -> Option<PairingCode> {
        self.st().code.as_ref().map(|h| h.code.clone())
    }

    /// Clears the shown code if it is still `code`.
    fn code_finished(&self, code: &PairingCode) {
        let cleared = {
            let mut st = self.st();
            if st.code.as_ref().is_some_and(|h| &h.code == code) {
                st.code = None;
                true
            } else {
                false
            }
        };
        if cleared {
            self.emit(EngineEvent::Code { code: None });
        }
    }

    async fn host_code(self, code: PairingCode, window: u64, mut stop: oneshot::Receiver<()>) {
        let kdf = self.inner.cfg.kdf;
        let words = code.words.clone();
        let Ok(keys) = tokio::task::spawn_blocking(move || CodeKeys::derive(&words, window, kdf)).await else {
            return;
        };
        let bound = net::bind(net::EndpointOpts {
            secret: keys.rendezvous.clone(),
            alpns: vec![wire::PAIR_ALPN.to_vec()],
            network: &self.inner.network,
            relay_only: self.inner.cfg.relay_only,
            mdns: self.inner.cfg.mdns,
            memory: self.inner.cfg.memory_lookup.as_ref(),
        })
        .await;
        let ep = match bound {
            Ok(b) => b.endpoint,
            Err(err) => {
                warn!(%err, "couldn't start the code endpoint");
                return self.code_finished(&code);
            }
        };
        debug!(rendezvous = %keys.rendezvous_id().fmt_short(), "showing code");
        let left = Duration::from_millis(code.expires_at.saturating_sub(now_ms()));
        let expiry = tokio::time::sleep(left);
        tokio::pin!(expiry);
        let mut bad = 0;
        loop {
            tokio::select! {
                _ = &mut stop => break,
                _ = &mut expiry => break,
                incoming = ep.accept() => {
                    let Some(incoming) = incoming else { break };
                    let Ok(accepting) = incoming.accept() else { continue };
                    let Ok(conn) = accepting.await else { continue };
                    match self.host_handshake(&keys, conn).await {
                        Ok(()) => break,
                        Err(Handshake::Bad) => {
                            bad += 1;
                            if bad >= MAX_BAD_ATTEMPTS {
                                warn!("code burned after {bad} bad attempts");
                                break;
                            }
                        }
                        Err(Handshake::Other(err)) => debug!(%err, "pairing attempt dropped"),
                    }
                }
            }
        }
        ep.close().await;
        self.code_finished(&code);
    }

    async fn host_handshake(&self, keys: &CodeKeys, conn: Connection) -> Result<(), Handshake> {
        let joiner = conn.remote_id();
        let (mut send, mut recv) = conn.accept_bi().await.map_err(Handshake::other)?;
        let join: PairJoin = tokio::time::timeout(Duration::from_secs(10), wire::read_frame(&mut recv))
            .await
            .map_err(Handshake::other)?
            .map_err(Handshake::other)?;
        if join.version != wire::VERSION || join.device != joiner || !macs_equal(&join.mac, &keys.join_mac(&joiner)) {
            conn.close(wire::close::PROTOCOL, b"bad join");
            return Err(Handshake::Bad);
        }
        let host = self.id();
        let addr = self.endpoint().addr();
        wire::write_frame(
            &mut send,
            &PairWelcome {
                info: self.me_info(),
                device: host,
                addr,
                mac: keys.welcome_mac(&joiner, &host),
            },
        )
        .await
        .map_err(Handshake::other)?;
        let done: PairDone = tokio::time::timeout(Duration::from_secs(10), wire::read_frame(&mut recv))
            .await
            .map_err(Handshake::other)?
            .map_err(Handshake::other)?;
        if !done.ok {
            conn.close(wire::close::PROTOCOL, b"rejected");
            return Err(Handshake::Bad);
        }
        // Record the pairing before closing, so the joiner can send to us the moment it sees the close.
        self.learn_addr(joiner, join.addr);
        self.mark_paired(joiner, join.info, net::route_of(&conn));
        conn.close(wire::close::DONE, b"");
        self.announce_paired(joiner, PairVia::Shown, keys.check(&host, &joiner));
        Ok(())
    }

    /// Enters a code shown on another device.
    pub async fn redeem_code(&self, typed: &[String]) -> RedeemResult {
        let words: Vec<String> = typed.iter().map(|w| w.trim().to_ascii_lowercase()).collect();
        if let Some(index) = words.iter().position(|w| !words::is_word(w)) {
            return RedeemResult::Err(RedeemFailure::UnknownWord { index });
        }
        let Ok(words): Result<[String; words::CODE_WORDS], _> = words.try_into() else {
            return RedeemResult::Err(RedeemFailure::NoMatch);
        };
        let own = self.current_code().map(|c| c.words);
        if own.as_ref() == Some(&words) {
            return RedeemResult::Err(RedeemFailure::NoMatch);
        }
        // Checked alongside, so a failure can say "your server turned you away" instead of "no match".
        let server = {
            let e = self.clone();
            tokio::spawn(async move { e.check_own_server().await })
        };
        let kdf = self.inner.cfg.kdf;
        let deadline = tokio::time::Instant::now() + self.inner.cfg.code_lookup_for;
        let mut attempts = n0_future::FuturesUnordered::new();
        for window in candidate_windows(now_ms() / 1000) {
            let w = words.clone();
            let Ok(keys) = tokio::task::spawn_blocking(move || CodeKeys::derive(&w, window, kdf)).await else {
                continue;
            };
            attempts.push(self.join_until(keys, deadline));
        }
        let all = async {
            while let Some(r) = attempts.next().await {
                if let Ok(device) = r {
                    return Some(device);
                }
            }
            None
        };
        match tokio::time::timeout_at(deadline + Duration::from_secs(15), all).await {
            Ok(Some(device)) => RedeemResult::Ok(device),
            _ => match server.await {
                Ok(Some(ServerProblem::TokenRejected)) => RedeemResult::Err(RedeemFailure::TokenRejected),
                _ => RedeemResult::Err(RedeemFailure::NoMatch),
            },
        }
    }

    /// Keeps trying one candidate address until `deadline`: the host may still be starting, and
    /// its address can take a few seconds to show up in lookups.
    async fn join_until(&self, keys: CodeKeys, deadline: tokio::time::Instant) -> Result<Device, String> {
        loop {
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            match tokio::time::timeout(left, self.endpoint().connect(keys.rendezvous_id(), wire::PAIR_ALPN)).await {
                Ok(Ok(conn)) => return self.try_join(&keys, conn).await,
                Ok(Err(err)) => debug!(%err, "code host not reachable yet"),
                Err(_) => return Err("timed out".into()),
            }
            if tokio::time::Instant::now() + Duration::from_millis(500) >= deadline {
                return Err("timed out".into());
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    async fn try_join(&self, keys: &CodeKeys, conn: Connection) -> Result<Device, String> {
        let me = self.id();
        let (mut send, mut recv) = conn.open_bi().await.map_err(|e| e.to_string())?;
        let addr = self.endpoint().addr();
        wire::write_frame(
            &mut send,
            &PairJoin {
                version: wire::VERSION,
                info: self.me_info(),
                device: me,
                addr,
                mac: keys.join_mac(&me),
            },
        )
        .await
        .map_err(|e| e.to_string())?;
        let welcome: PairWelcome = tokio::time::timeout(Duration::from_secs(10), wire::read_frame(&mut recv))
            .await
            .map_err(|_| "timed out".to_string())?
            .map_err(|e| e.to_string())?;
        if !macs_equal(&welcome.mac, &keys.welcome_mac(&me, &welcome.device)) || welcome.device == me {
            let _ = wire::write_frame(&mut send, &PairDone { ok: false }).await;
            conn.close(wire::close::PROTOCOL, b"bad welcome");
            return Err("bad welcome".into());
        }
        // Record the pairing before confirming it: the host may start sending as soon as it reads PairDone.
        self.learn_addr(welcome.device, welcome.addr);
        self.mark_paired(welcome.device, welcome.info, net::route_of(&conn));
        wire::write_frame(&mut send, &PairDone { ok: true }).await.map_err(|e| e.to_string())?;
        let _ = send.finish();
        self.announce_paired(welcome.device, PairVia::Entered, keys.check(&welcome.device, &me));
        // Keep the connection until the host has read PairDone and closes it.
        let _ = tokio::time::timeout(Duration::from_secs(3), conn.closed()).await;
        self.device_snapshot(&welcome.device).ok_or_else(|| "device vanished".into())
    }

    /// Remembers where a freshly paired device can be reached. Only for the id it claims.
    fn learn_addr(&self, id: EndpointId, addr: iroh::EndpointAddr) {
        if addr.id == id && !addr.is_empty() {
            self.inner.learned.add_endpoint_info(addr);
        }
    }

    /// Allows the device to send to us from now on (for this run of the app).
    fn mark_paired(&self, id: EndpointId, info: DeviceInfo, route: Route) {
        self.st().devices.entry(id).or_default().paired = true;
        self.note_peer(id, info, route);
    }

    fn announce_paired(&self, id: EndpointId, via: PairVia, check: String) {
        info!(peer = %id.fmt_short(), ?via, "paired with a code");
        if let Some(device) = self.device_snapshot(&id) {
            self.emit(EngineEvent::Device { device: device.clone() });
            self.emit(EngineEvent::Paired { device, via, check });
        }
    }
}

enum Handshake {
    Bad,
    Other(String),
}

impl Handshake {
    fn other(e: impl std::fmt::Display) -> Self {
        Self::Other(e.to_string())
    }
}
