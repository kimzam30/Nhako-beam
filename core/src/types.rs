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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileItem {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub mime: String,
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingRequest {
    pub id: String,
    pub device_id: String,
    pub files: Vec<FileItem>,
    pub total_bytes: u64,
    pub received_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingCode {
    pub words: [String; 3],
    /// Epoch milliseconds. Codes live 10 minutes.
    pub expires_at: u64,
    /// Encoded in the QR code.
    pub uri: String,
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
        };
        let v = serde_json::to_value(&t).unwrap();
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
