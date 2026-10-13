//! Trust this device, from Nearby: a device asks another on the same network to trust it, and to
//! be trusted back. Both screens show the same check value, and the person at the other device
//! agrees or not. On a match both save the other as trusted, as pairing with a code does.
//! See docs/protocol/nhako-1.md, section 4 (Trust asks).

use std::time::{Duration, Instant};

use iroh::{
    EndpointId,
    endpoint::{Connection, SendStream},
};
use tokio::sync::oneshot;
use tracing::{debug, info};

use crate::{
    codekeys,
    engine::{Engine, EngineError, FrameRx, device_id, now_ms, parse_device_id},
    net,
    types::{EngineEvent, PairVia, TrustOutcome, TrustRequest},
    wire::{self, HelloExt, Intent, TrustAnswer, TrustMsg, WireError},
};

/// How long an ask waits for the person at the other device.
pub const TRUST_WAIT: Duration = Duration::from_secs(120);
/// Asks waiting here at once, each from a different device.
const ASKS_WAITING: usize = 3;
/// Asks one device may make here in a minute (answered or not).
const ASKS_PER_MINUTE: usize = 3;

/// An ask from another device, waiting for an answer here.
pub(crate) struct Asked {
    pub request: TrustRequest,
    pub decide: Option<oneshot::Sender<bool>>,
}

impl Engine {
    /// Trust asks waiting for an answer here, oldest first.
    pub fn trust_asks(&self) -> Vec<TrustRequest> {
        let mut out: Vec<TrustRequest> = self.st().trust_asks.values().map(|a| a.request.clone()).collect();
        out.sort_by_key(|r| r.received_at);
        out
    }

    /// Asks a device on the same network to trust this one, and to be trusted back. Returns at
    /// once. Then comes `TrustWaiting` with the check value both screens show, and in the end
    /// `Paired` (via `nearby`) on both devices, or `TrustEnded` saying why not.
    pub fn ask_trust(&self, device: &str) -> Result<(), EngineError> {
        let peer = parse_device_id(device).ok_or(EngineError::UnknownDevice)?;
        if peer == self.id() {
            return Err(EngineError::UnknownDevice);
        }
        {
            let st = self.st();
            let d = st.devices.get(&peer).ok_or(EngineError::UnknownDevice)?;
            if !(d.reachable && d.nearby) {
                return Err(EngineError::NotNearby);
            }
            if st.trust_asking.contains_key(&peer) {
                return Ok(());
            }
        }
        let (stop_tx, stop_rx) = oneshot::channel::<()>();
        self.st().trust_asking.insert(peer, stop_tx);
        let e = self.clone();
        self.spawn_bg(async move {
            let outcome = tokio::select! {
                r = e.ask(peer) => Some(r),
                // `cancel_trust`, or the engine stopping.
                _ = stop_rx => None,
            };
            e.st().trust_asking.remove(&peer);
            match outcome {
                Some(Ok(check)) => e.trusted_both(peer, check),
                Some(Err(outcome)) => {
                    info!(peer = %peer.fmt_short(), ?outcome, "trust ask ended");
                    e.emit(EngineEvent::TrustEnded {
                        device_id: device_id(&peer),
                        outcome,
                    });
                }
                None => {}
            }
        });
        Ok(())
    }

    /// Stops this device's ask to `device`. The other device's ask goes away.
    pub fn cancel_trust(&self, device: &str) {
        if let Some(peer) = parse_device_id(device) {
            self.st().trust_asking.remove(&peer);
        }
    }

    /// Answers another device's ask: `true` trusts it, and tells it to trust this one.
    pub fn answer_trust(&self, device: &str, yes: bool) {
        let Some(peer) = parse_device_id(device) else { return };
        let decide = self.st().trust_asks.get_mut(&peer).and_then(|a| a.decide.take());
        if let Some(d) = decide {
            let _ = d.send(yes);
        }
    }

    /// The asker's side: dial, swap random numbers, show the check, wait for the answer.
    async fn ask(&self, peer: EndpointId) -> Result<String, TrustOutcome> {
        let mine: [u8; 32] = rand::random();
        let extra = HelloExt {
            trust: Some(codekeys::trust_commit(&mine)),
            ..HelloExt::default()
        };
        let (conn, mut send, mut frames) = self.open(peer, Intent::Ping, &extra).await.map_err(|err| {
            debug!(%err, "couldn't reach it to ask");
            TrustOutcome::Unreachable
        })?;
        let theirs = match next_msg(&mut frames, Duration::from_secs(10)).await {
            Ok(TrustMsg::Nonce(n)) => n,
            Ok(TrustMsg::Answer(TrustAnswer::Busy)) => return Err(TrustOutcome::Busy),
            // A build without trust asks answers a ping and closes.
            Err(Some(WireError::Closed)) => return Err(TrustOutcome::Outdated),
            _ => return Err(TrustOutcome::Unreachable),
        };
        if wire::write_frame(&mut send, &TrustMsg::Reveal(mine)).await.is_err() {
            return Err(TrustOutcome::Unreachable);
        }
        let check = codekeys::trust_check(&mine, &theirs, &self.id(), &peer);
        self.emit(EngineEvent::TrustWaiting {
            device_id: device_id(&peer),
            check: check.clone(),
        });
        let answer = next_msg(&mut frames, TRUST_WAIT + Duration::from_secs(10)).await;
        let _ = send.finish();
        conn.close(wire::close::DONE, b"");
        match answer {
            Ok(TrustMsg::Answer(TrustAnswer::Yes)) => Ok(check),
            Ok(TrustMsg::Answer(TrustAnswer::No)) => Err(TrustOutcome::Declined),
            _ => Err(TrustOutcome::Unreachable),
        }
    }

    /// The asked side, from a ping carrying a trust ask: only from devices on the same network,
    /// a few at a time.
    pub(crate) async fn answer_trust_ask(
        &self,
        conn: Connection,
        mut send: SendStream,
        mut frames: FrameRx,
        peer: EndpointId,
        commit: [u8; 32],
    ) -> Result<(), String> {
        let nearby = net::is_lan_conn(&conn) || self.st().devices.get(&peer).is_some_and(|d| d.nearby);
        if !nearby || self.too_many_asks(&peer) {
            let _ = wire::write_frame(&mut send, &TrustMsg::Answer(TrustAnswer::Busy)).await;
            let _ = send.finish();
            let _ = tokio::time::timeout(Duration::from_secs(3), conn.closed()).await;
            return Ok(());
        }
        let mine: [u8; 32] = rand::random();
        wire::write_frame(&mut send, &TrustMsg::Nonce(mine)).await.map_err(|e| e.to_string())?;
        let theirs = match next_msg(&mut frames, Duration::from_secs(10)).await {
            Ok(TrustMsg::Reveal(n)) if codekeys::macs_equal(&codekeys::trust_commit(&n), &commit) => n,
            _ => {
                conn.close(wire::close::PROTOCOL, b"trust");
                return Err("a trust ask didn't reveal what it committed to".into());
            }
        };
        let check = codekeys::trust_check(&theirs, &mine, &peer, &self.id());
        let (decide, decided) = oneshot::channel();
        let request = TrustRequest {
            device_id: device_id(&peer),
            check: check.clone(),
            received_at: now_ms(),
        };
        self.st().trust_asks.insert(
            peer,
            Asked {
                request: request.clone(),
                decide: Some(decide),
            },
        );
        self.emit(EngineEvent::TrustAsk { request });
        let answer = tokio::select! {
            a = decided => a.ok(),
            // The asker cancelled or went away.
            _ = frames.recv() => None,
            _ = tokio::time::sleep(TRUST_WAIT) => None,
        };
        self.st().trust_asks.remove(&peer);
        let ended = |outcome| {
            self.emit(EngineEvent::TrustEnded {
                device_id: device_id(&peer),
                outcome,
            })
        };
        match answer {
            Some(true) => {
                if wire::write_frame(&mut send, &TrustMsg::Answer(TrustAnswer::Yes)).await.is_err() {
                    ended(TrustOutcome::Withdrawn);
                    return Ok(());
                }
                self.trusted_both(peer, check);
            }
            Some(false) => {
                let _ = wire::write_frame(&mut send, &TrustMsg::Answer(TrustAnswer::No)).await;
                ended(TrustOutcome::Declined);
            }
            None => {
                ended(TrustOutcome::Withdrawn);
                conn.close(wire::close::CANCELLED, b"");
                return Ok(());
            }
        }
        let _ = send.finish();
        let _ = tokio::time::timeout(Duration::from_secs(5), conn.closed()).await;
        Ok(())
    }

    /// Already answering this device or several others, or asked too often by it.
    fn too_many_asks(&self, peer: &EndpointId) -> bool {
        let mut st = self.st();
        if st.trust_asks.len() >= ASKS_WAITING || st.trust_asks.contains_key(peer) {
            return true;
        }
        let now = Instant::now();
        let times = st.trust_ask_times.entry(*peer).or_default();
        times.retain(|t| now.duration_since(*t) < Duration::from_secs(60));
        times.push(now);
        times.len() > ASKS_PER_MINUTE
    }

    /// Both sides agreed: save the other device as trusted and say so, as pairing does.
    fn trusted_both(&self, peer: EndpointId, check: String) {
        self.set_trusted(&device_id(&peer), true);
        if let Some(device) = self.device_snapshot(&peer) {
            info!(peer = %peer.fmt_short(), "trusted each other");
            self.emit(EngineEvent::Paired {
                device,
                via: PairVia::Nearby,
                check,
            });
        }
    }
}

/// The next trust frame. `Err(Some(Closed))` when the other side finished without one.
async fn next_msg(frames: &mut FrameRx, timeout: Duration) -> Result<TrustMsg, Option<WireError>> {
    match tokio::time::timeout(timeout, frames.recv()).await {
        Ok(Some(Ok(bytes))) => postcard::from_bytes(&bytes).map_err(|e| Some(e.into())),
        Ok(Some(Err(e))) => Err(Some(e)),
        Ok(None) => Err(Some(WireError::Closed)),
        Err(_) => Err(None),
    }
}
