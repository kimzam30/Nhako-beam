//! The UI contract, mirrored from `desktop/src/lib/engine/types.ts`.
//!
//! These types serialize to exactly the JSON the app expects (camelCase fields, `type` tags), so
//! the Tauri shell in Phase 5 can forward events without mapping. Keep both files in step.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DeviceKind {
    Laptop,
    Desktop,
    Phone,
    Tablet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OsPlatform {
    Windows,
    Linux,
    Android,
    Macos,
    Ios,
    Unknown,
}

impl OsPlatform {
    pub fn current() -> Self {
        match std::env::consts::OS {
            "windows" => Self::Windows,
            "linux" => Self::Linux,
            "android" => Self::Android,
            "macos" => Self::Macos,
            "ios" => Self::Ios,
            _ => Self::Unknown,
        }
    }
}

/// How bytes travel: same network, direct over the internet, or through a relay.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Route {
    Nearby,
    Direct,
    Relayed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub name: String,
    pub kind: DeviceKind,
    pub os: OsPlatform,
    pub trusted: bool,
    pub auto_accept: bool,
    pub reachable: bool,
    pub route: Option<Route>,
    pub fingerprint: String,
    /// Not reachable right now, but can be woken up to receive (a phone Android put to sleep).
    pub wakeable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileItem {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub mime: String,
    /// The folder it's in, inside a folder that was sent (`Photos/2024`, `/` between folders).
    /// Missing for files sent on their own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransferDirection {
    Send,
    Receive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransferStatus {
    Waiting,
    Connecting,
    Transferring,
    Paused,
    Reconnecting,
    Done,
    Failed,
    Cancelled,
    Declined,
}

impl TransferStatus {
    pub fn is_final(self) -> bool {
        matches!(self, Self::Done | Self::Cancelled | Self::Declined)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TransferError {
    PeerGone,
    Network,
    FileChanged,
    Integrity,
    RelayUnreachable,
    Permission,
    DiskFull,
    DiskFullPeer,
    /// The receiver only takes files from its own devices (Who can send to me).
    NotAllowed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    Me,
    Peer,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Transfer {
    pub id: String,
    pub direction: TransferDirection,
    pub device_id: String,
    pub files: Vec<FileItem>,
    pub total_bytes: u64,
    /// Bytes confirmed by the receiver.
    pub done_bytes: u64,
    /// Per file confirmed bytes, keyed by file id.
    pub file_bytes: BTreeMap<String, u64>,
    /// Smoothed bytes per second (3 second window). 0 when not moving.
    pub speed: f64,
    pub route: Route,
    pub status: TransferStatus,
    pub paused_by: Option<Side>,
    pub cancelled_by: Option<Side>,
    pub error: Option<TransferError>,
    /// Epoch milliseconds.
    pub started_at: u64,
    pub moving_since: Option<u64>,
    pub finished_at: Option<u64>,
    pub waiting_long: bool,
    /// Sender: the other device was asleep and has been asked to wake up.
    pub waking: bool,
    /// A text transfer (a note, a link, a clipboard): the text. Its one file is the same text, for
    /// devices on a version without text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// A transfer that ended, as History lists it. Kept after a restart, and after the device is removed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    /// The transfer's id.
    pub id: String,
    pub direction: TransferDirection,
    pub device_id: String,
    /// The other device's name when the transfer ended.
    pub device_name: String,
    pub device_kind: DeviceKind,
    pub files: Vec<FileItem>,
    pub total_bytes: u64,
    pub done_bytes: u64,
    pub route: Route,
    /// Done, failed, cancelled or declined.
    pub status: TransferStatus,
    pub error: Option<TransferError>,
    pub cancelled_by: Option<Side>,
    /// Epoch milliseconds.
    pub started_at: u64,
    pub finished_at: u64,
    /// Bytes per second while bytes were moving (pauses left out). 0 when nothing moved.
    pub average_speed: f64,
    /// A text transfer: the text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingRequest {
    pub id: String,
    pub device_id: String,
    pub files: Vec<FileItem>,
    pub total_bytes: u64,
    pub received_at: u64,
    /// Text rather than files: the text, to read before accepting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

/// Another device on the same network asking to be one of your devices, and you one of its own
/// (Trust this device, from Nearby). Both screens show `check`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustRequest {
    pub device_id: String,
    pub check: String,
    pub received_at: u64,
}

/// How a trust ask ended without both devices trusting each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TrustOutcome {
    /// The other device said no (asker), or you did (asked).
    Declined,
    /// The asker gave up or went away before you answered (asked).
    Withdrawn,
    /// The other device has a version without trust asks (asker).
    Outdated,
    /// The other device couldn't be reached, or didn't answer in time (asker).
    Unreachable,
    /// The other device is answering someone else's ask (asker).
    Busy,
}

/// A newer Nhako Beam on GitHub (Settings > About > Check for new versions).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    /// `0.3.0`.
    pub version: String,
    /// Its release page, where the downloads are.
    pub url: String,
}

/// Files (and or text) shared with a link, for a device without the app (Share via link).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkShare {
    pub id: String,
    /// `http://192.168.1.20:41234/k3jq.../`, also in the QR code.
    pub url: String,
    pub files: Vec<FileItem>,
    pub total_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Epoch milliseconds. Downloads running then carry on, and the link stops after them.
    pub expires_at: u64,
    /// Browsers that opened the link, in the order they came.
    pub visitors: Vec<LinkVisitor>,
}

/// A browser that opened a shared link.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkVisitor {
    pub id: String,
    /// "Chrome on Android", from what the browser says about itself.
    pub label: String,
    /// Its address on the network, `192.168.1.23`.
    pub address: String,
    pub state: VisitorState,
    pub arrived_at: u64,
    /// Downloads running now: bytes so far, and their sizes. Both 0 when nothing is running.
    pub active_bytes: u64,
    pub active_total: u64,
    /// Downloads finished (a file, or the zip of everything).
    pub downloads: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VisitorState {
    /// Waiting for you to let it in.
    Asking,
    Allowed,
    Denied,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingCode {
    /// Four words (`words::CODE_WORDS`).
    pub words: [String; crate::words::CODE_WORDS],
    /// Epoch milliseconds. Codes live 10 minutes.
    pub expires_at: u64,
    /// Encoded in the QR code.
    pub uri: String,
}

/// Settings > Receiving > Who can send to me. Devices you trust, and devices you've just
/// connected with a code, can always send.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WhoCanSend {
    /// Requests from anyone else are refused, and the sender is told why.
    #[default]
    YourDevices,
    /// Anyone on the same network can ask (one request at a time per device).
    EveryoneNearby,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiscoveryState {
    Searching,
    Idle,
    Blocked,
    Offline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PairVia {
    Shown,
    Entered,
    /// Trust this device, from Nearby: one device asked, the other agreed.
    Nearby,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "reason")]
pub enum RedeemFailure {
    UnknownWord {
        index: usize,
    },
    NoMatch,
    Expired,
    /// Your own relay turned this device away, so the other device couldn't be reached.
    TokenRejected,
}

/// `{ ok: true, device }` or `{ ok: false, reason, index? }` in JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedeemResult {
    Ok(Device),
    Err(RedeemFailure),
}

impl Serialize for RedeemResult {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let v = match self {
            Self::Ok(device) => serde_json::json!({ "ok": true, "device": device }),
            Self::Err(f) => {
                let mut v = serde_json::to_value(f).map_err(serde::ser::Error::custom)?;
                v["ok"] = false.into();
                v
            }
        };
        v.serialize(s)
    }
}

/// Which servers help devices find and reach each other over the internet.
/// Same-network discovery works the same either way.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", tag = "mode")]
pub enum Connection {
    /// n0's free public relays and address lookup.
    #[default]
    Public,
    /// Your own server, for example the Docker setup in `relay/`.
    Own(OwnServer),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnServer {
    /// Relay address, for example `http://192.168.1.20:3340` or `https://relay.example.com`.
    pub relay: String,
    /// Sent to the relay when it only lets in devices that know the token.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// Address lookup server (pkarr). When unset, n0's public lookup is used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lookup: Option<String>,
}

/// Result of checking one server.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum CheckResult {
    /// Answered, in this many milliseconds.
    Ok { ms: u32 },
    /// The address is fine but nothing usable answered.
    Unreachable { reason: String },
    /// The relay answered and turned the token down (or wanted one).
    TokenRejected,
    /// The address itself can't be used.
    Invalid { reason: String },
}

/// Something wrong with your own server, found while running on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ServerProblem {
    /// Nothing answered at the relay address.
    Unreachable,
    /// The relay answered and turned this device's token down.
    TokenRejected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerCheck {
    pub relay: CheckResult,
    /// `None` when no lookup server is set.
    pub lookup: Option<CheckResult>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum EngineEvent {
    Discovery {
        state: DiscoveryState,
    },
    Device {
        device: Device,
    },
    #[serde(rename_all = "camelCase")]
    DeviceLost {
        device_id: String,
    },
    Incoming {
        request: IncomingRequest,
    },
    #[serde(rename_all = "camelCase")]
    IncomingWithdrawn {
        request_id: String,
    },
    Transfer {
        transfer: Transfer,
    },
    /// A transfer moved: only what changed since the last `transfer` or `transferProgress` event
    /// for it, merged into the transfer the app has. `fileBytes` lists only the files whose count
    /// changed, so a transfer of thousands of files doesn't resend its whole list every update.
    #[serde(rename_all = "camelCase")]
    TransferProgress {
        id: String,
        done_bytes: u64,
        speed: f64,
        moving_since: Option<u64>,
        file_bytes: BTreeMap<String, u64>,
    },
    /// `check` is the same short value on both screens.
    Paired {
        device: Device,
        via: PairVia,
        check: String,
    },
    /// The code this device is showing changed (new, expiring, or gone).
    Code {
        code: Option<PairingCode>,
    },
    FreeSpace {
        bytes: u64,
    },
    /// Your own server's state changed. `null` when it's fine (or public servers are in use).
    Server {
        problem: Option<ServerProblem>,
    },
    /// A transfer ended and was added to History (or replaced its own entry after a retry).
    History {
        entry: HistoryEntry,
    },
    /// A device nearby asks to trust this one, and this one it. Answer with `answer_trust`.
    TrustAsk {
        request: TrustRequest,
    },
    /// On the device that asked: the other device has the ask, and both screens show `check`.
    #[serde(rename_all = "camelCase")]
    TrustWaiting {
        device_id: String,
        check: String,
    },
    /// A trust ask ended without a match: on the device that asked, or the one that was asked.
    /// A match ends with `Paired` (via `nearby`) on both instead.
    #[serde(rename_all = "camelCase")]
    TrustEnded {
        device_id: String,
        outcome: TrustOutcome,
    },
    /// The link being shared changed: started, a browser came or moved, or it stopped (`null`).
    LinkShare {
        share: Option<LinkShare>,
    },
    /// A newer version is out (or, `null`, none is).
    Update {
        update: Option<UpdateInfo>,
    },
}

/// Everything the app needs to draw itself, for when its window (re)loads after the engine started.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub discovery: DiscoveryState,
    pub devices: Vec<Device>,
    pub transfers: Vec<Transfer>,
    pub requests: Vec<IncomingRequest>,
    pub code: Option<PairingCode>,
    pub free_space: Option<u64>,
    pub server_problem: Option<ServerProblem>,
    /// Newest first.
    pub history: Vec<HistoryEntry>,
    /// Trust asks waiting for an answer here.
    pub trust_asks: Vec<TrustRequest>,
    /// The link being shared, if any.
    pub link_share: Option<LinkShare>,
    /// A newer version, when the last check found one.
    pub update: Option<UpdateInfo>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connection_matches_the_ts_contract() {
        assert_eq!(serde_json::to_string(&Connection::Public).unwrap(), r#"{"mode":"public"}"#);
        let own = Connection::Own(OwnServer {
            relay: "http://10.0.0.2:3340".into(),
            token: Some("t0k".into()),
            lookup: None,
        });
        let json = serde_json::to_string(&own).unwrap();
        assert_eq!(json, r#"{"mode":"own","relay":"http://10.0.0.2:3340","token":"t0k"}"#);
        assert_eq!(serde_json::from_str::<Connection>(&json).unwrap(), own);
        let check = ServerCheck {
            relay: CheckResult::TokenRejected,
            lookup: Some(CheckResult::Ok { ms: 12 }),
        };
        assert_eq!(
            serde_json::to_string(&check).unwrap(),
            r#"{"relay":{"status":"tokenRejected"},"lookup":{"status":"ok","ms":12}}"#
        );
    }

    #[test]
    fn events_match_the_ts_contract() {
        let e = EngineEvent::DeviceLost { device_id: "abc".into() };
        assert_eq!(serde_json::to_string(&e).unwrap(), r#"{"type":"deviceLost","deviceId":"abc"}"#);

        let e = EngineEvent::Code { code: None };
        assert_eq!(serde_json::to_string(&e).unwrap(), r#"{"type":"code","code":null}"#);

        let e = EngineEvent::IncomingWithdrawn { request_id: "r".into() };
        assert_eq!(serde_json::to_string(&e).unwrap(), r#"{"type":"incomingWithdrawn","requestId":"r"}"#);

        let r = RedeemResult::Err(RedeemFailure::UnknownWord { index: 2 });
        let v: serde_json::Value = serde_json::to_value(&r).unwrap();
        assert_eq!(v, serde_json::json!({ "ok": false, "reason": "unknownWord", "index": 2 }));
        let r = RedeemResult::Err(RedeemFailure::NoMatch);
        assert_eq!(serde_json::to_value(&r).unwrap(), serde_json::json!({ "ok": false, "reason": "noMatch" }));
        let r = RedeemResult::Err(RedeemFailure::TokenRejected);
        assert_eq!(serde_json::to_value(&r).unwrap(), serde_json::json!({ "ok": false, "reason": "tokenRejected" }));

        let e = EngineEvent::Server {
            problem: Some(ServerProblem::TokenRejected),
        };
        assert_eq!(serde_json::to_string(&e).unwrap(), r#"{"type":"server","problem":"tokenRejected"}"#);
        let e = EngineEvent::Server { problem: None };
        assert_eq!(serde_json::to_string(&e).unwrap(), r#"{"type":"server","problem":null}"#);

        let e = EngineEvent::TrustAsk {
            request: TrustRequest {
                device_id: "d".into(),
                check: "5D19 E04A".into(),
                received_at: 1,
            },
        };
        assert_eq!(
            serde_json::to_string(&e).unwrap(),
            r#"{"type":"trustAsk","request":{"deviceId":"d","check":"5D19 E04A","receivedAt":1}}"#
        );
        let e = EngineEvent::TrustEnded {
            device_id: "d".into(),
            outcome: TrustOutcome::Outdated,
        };
        assert_eq!(
            serde_json::to_string(&e).unwrap(),
            r#"{"type":"trustEnded","deviceId":"d","outcome":"outdated"}"#
        );
        assert_eq!(serde_json::to_string(&PairVia::Nearby).unwrap(), r#""nearby""#);

        let e = EngineEvent::LinkShare {
            share: Some(LinkShare {
                id: "s".into(),
                url: "http://192.168.1.20:4000/k/".into(),
                files: vec![],
                total_bytes: 0,
                text: Some("hi".into()),
                expires_at: 2,
                visitors: vec![LinkVisitor {
                    id: "v".into(),
                    label: "Chrome on Android".into(),
                    address: "192.168.1.23".into(),
                    state: VisitorState::Asking,
                    arrived_at: 1,
                    active_bytes: 0,
                    active_total: 0,
                    downloads: 0,
                }],
            }),
        };
        assert_eq!(
            serde_json::to_string(&e).unwrap(),
            r#"{"type":"linkShare","share":{"id":"s","url":"http://192.168.1.20:4000/k/","files":[],"totalBytes":0,"text":"hi","expiresAt":2,"visitors":[{"id":"v","label":"Chrome on Android","address":"192.168.1.23","state":"asking","arrivedAt":1,"activeBytes":0,"activeTotal":0,"downloads":0}]}}"#
        );
        assert_eq!(
            serde_json::to_string(&EngineEvent::LinkShare { share: None }).unwrap(),
            r#"{"type":"linkShare","share":null}"#
        );
        let e = EngineEvent::Update {
            update: Some(UpdateInfo {
                version: "0.3.0".into(),
                url: "https://github.com/x/releases/tag/v0.3.0".into(),
            }),
        };
        assert_eq!(
            serde_json::to_string(&e).unwrap(),
            r#"{"type":"update","update":{"version":"0.3.0","url":"https://github.com/x/releases/tag/v0.3.0"}}"#
        );

        let f = FileItem {
            id: "f0".into(),
            name: "a.jpg".into(),
            size: 1,
            mime: "image/jpeg".into(),
            folder: Some("Photos/2024".into()),
        };
        assert_eq!(
            serde_json::to_string(&f).unwrap(),
            r#"{"id":"f0","name":"a.jpg","size":1,"mime":"image/jpeg","folder":"Photos/2024"}"#
        );
        let old: FileItem = serde_json::from_str(r#"{"id":"f0","name":"a.jpg","size":1,"mime":"image/jpeg"}"#).unwrap();
        assert_eq!(old.folder, None, "History from before Phase 14 still reads");
    }

    #[test]
    fn progress_is_camel_case_and_small() {
        let e = EngineEvent::TransferProgress {
            id: "t".into(),
            done_bytes: 5,
            speed: 2.0,
            moving_since: Some(1),
            file_bytes: BTreeMap::from([("f3".into(), 5)]),
        };
        assert_eq!(
            serde_json::to_string(&e).unwrap(),
            r#"{"type":"transferProgress","id":"t","doneBytes":5,"speed":2.0,"movingSince":1,"fileBytes":{"f3":5}}"#
        );
    }

    #[test]
    fn transfer_fields_are_camel_case() {
        let t = Transfer {
            id: "t".into(),
            direction: TransferDirection::Send,
            device_id: "d".into(),
            files: vec![],
            total_bytes: 1,
            done_bytes: 0,
            file_bytes: BTreeMap::new(),
            speed: 0.0,
            route: Route::Nearby,
            status: TransferStatus::Waiting,
            paused_by: Some(Side::Peer),
            cancelled_by: None,
            error: Some(TransferError::DiskFullPeer),
            started_at: 1,
            moving_since: None,
            finished_at: None,
            waiting_long: false,
            waking: false,
            text: None,
        };
        let v = serde_json::to_value(&t).unwrap();
        assert!(v.get("text").is_none(), "left out when there's none");
        for key in [
            "deviceId",
            "totalBytes",
            "doneBytes",
            "fileBytes",
            "pausedBy",
            "cancelledBy",
            "startedAt",
            "movingSince",
            "finishedAt",
            "waitingLong",
        ] {
            assert!(v.get(key).is_some(), "missing {key}");
        }
        assert_eq!(v["pausedBy"], "peer");
        assert_eq!(v["error"], "diskFullPeer");
        assert_eq!(v["status"], "waiting");
    }
}
