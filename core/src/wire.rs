//! nhako/1 wire messages and framing. See docs/protocol/nhako-1.md.

use iroh::{
    EndpointAddr, EndpointId,
    endpoint::{RecvStream, SendStream, VarInt},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::types::{DeviceKind, OsPlatform};

pub const ALPN: &[u8] = b"nhako/1";
pub const PAIR_ALPN: &[u8] = b"nhako-pair/1";
pub const VERSION: u16 = 1;

/// Longest control frame we accept.
pub const MAX_FRAME: usize = 4 * 1024 * 1024;

pub mod close {
    use super::VarInt;
    pub const DONE: VarInt = VarInt::from_u32(0);
    pub const CANCELLED: VarInt = VarInt::from_u32(1);
    pub const DECLINED: VarInt = VarInt::from_u32(2);
    pub const PROTOCOL: VarInt = VarInt::from_u32(3);
    pub const NOT_ALLOWED: VarInt = VarInt::from_u32(4);
    /// Stopped after an error both sides know about. The close reason says which:
    /// `integrity`, `fileChanged`, `permission` or `unknown`.
    pub const FAILED: VarInt = VarInt::from_u32(5);
}

/// Stream reset code used when the sender stops a data stream for a pause.
pub const RESET_PAUSED: VarInt = VarInt::from_u32(10);

pub type TransferId = [u8; 16];

pub fn transfer_id_str(id: &TransferId) -> String {
    data_encoding::HEXLOWER.encode(id)
}

pub fn parse_transfer_id(s: &str) -> Option<TransferId> {
    data_encoding::HEXLOWER.decode(s.as_bytes()).ok()?.try_into().ok()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub name: String,
    pub kind: DeviceKind,
    pub os: OsPlatform,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientHello {
    pub version: u16,
    pub info: DeviceInfo,
    pub intent: Intent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerHello {
    pub version: u16,
    pub info: DeviceInfo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Intent {
    Ping,
    Offer(Offer),
    Resume { id: TransferId },
    Pull { id: TransferId, files: Vec<FileState> },
}

/// How much of one file the receiver has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileState {
    /// This many bytes are on disk (not yet checked).
    Have(u64),
    /// Complete and its hash matched. The sender skips it.
    Verified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Offer {
    pub id: TransferId,
    pub files: Vec<FileMeta>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileMeta {
    pub name: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Answer {
    Accept { files: Vec<FileState> },
    Decline(DeclineReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeclineReason {
    User,
    NoSpace,
    NotAllowed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ctrl {
    Progress { file: u32, bytes: u64 },
    Verdict { file: u32, ok: bool },
    Pause { reason: PauseReason },
    Resume,
    Want { files: Vec<FileState> },
    Cancel,
    Withdraw,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PauseReason {
    User,
    DiskFull,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataHeader {
    pub id: TransferId,
    pub file: u32,
    pub offset: u64,
    pub len: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairJoin {
    pub version: u16,
    pub info: DeviceInfo,
    pub device: EndpointId,
    /// Where the joiner's real endpoint can be reached right now (relay and addresses).
    pub addr: EndpointAddr,
    pub mac: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairWelcome {
    pub info: DeviceInfo,
    pub device: EndpointId,
    pub addr: EndpointAddr,
    pub mac: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairDone {
    pub ok: bool,
}

#[derive(Debug, thiserror::Error)]
pub enum WireError {
    #[error("stream closed")]
    Closed,
    #[error("frame too long ({0} bytes)")]
    TooLong(usize),
    #[error("bad frame: {0}")]
    Decode(#[from] postcard::Error),
    #[error("read failed: {0}")]
    Read(String),
    #[error("write failed: {0}")]
    Write(String),
}

pub async fn write_frame<T: Serialize>(send: &mut SendStream, msg: &T) -> Result<(), WireError> {
    let body = postcard::to_stdvec(msg)?;
    if body.len() > MAX_FRAME {
        return Err(WireError::TooLong(body.len()));
    }
    let mut buf = Vec::with_capacity(4 + body.len());
    buf.extend_from_slice(&(body.len() as u32).to_be_bytes());
    buf.extend_from_slice(&body);
    send.write_all(&buf).await.map_err(|e| WireError::Write(e.to_string()))
}

/// Reads one frame. Returns `Closed` if the stream ended cleanly before a new frame started.
pub async fn read_frame<T: DeserializeOwned>(recv: &mut RecvStream) -> Result<T, WireError> {
    let mut len = [0u8; 4];
    match recv.read_exact(&mut len).await {
        Ok(()) => {}
        Err(iroh::endpoint::ReadExactError::FinishedEarly(0)) => return Err(WireError::Closed),
        Err(e) => return Err(WireError::Read(e.to_string())),
    }
    let len = u32::from_be_bytes(len) as usize;
    if len > MAX_FRAME {
        return Err(WireError::TooLong(len));
    }
    let mut body = vec![0u8; len];
    recv.read_exact(&mut body).await.map_err(|e| WireError::Read(e.to_string()))?;
    Ok(postcard::from_bytes(&body)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_round_trip() {
        let hello = ClientHello {
            version: VERSION,
            info: DeviceInfo {
                name: "Hakim's laptop".into(),
                kind: DeviceKind::Laptop,
                os: OsPlatform::Linux,
            },
            intent: Intent::Offer(Offer {
                id: [7; 16],
                files: vec![FileMeta {
                    name: "a.jpg".into(),
                    size: 42,
                }],
            }),
        };
        let bytes = postcard::to_stdvec(&hello).unwrap();
        assert_eq!(postcard::from_bytes::<ClientHello>(&bytes).unwrap(), hello);
        for c in [
            Ctrl::Progress { file: 1, bytes: u64::MAX },
            Ctrl::Want {
                files: vec![FileState::Have(0), FileState::Verified],
            },
            Ctrl::Pause { reason: PauseReason::DiskFull },
        ] {
            let bytes = postcard::to_stdvec(&c).unwrap();
            assert_eq!(postcard::from_bytes::<Ctrl>(&bytes).unwrap(), c);
        }
    }

    #[test]
    fn transfer_ids_print_as_hex() {
        let id = [0xab; 16];
        let s = transfer_id_str(&id);
        assert_eq!(s.len(), 32);
        assert_eq!(parse_transfer_id(&s), Some(id));
        assert_eq!(parse_transfer_id("nope"), None);
    }
}
