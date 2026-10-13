//! nhako/1 wire messages and framing. See docs/protocol/nhako-1.md.

use iroh::{
    EndpointAddr, EndpointId,
    endpoint::{RecvStream, SendStream, VarInt},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::types::{DeviceKind, OsPlatform};

pub const ALPN: &[u8] = b"nhako/1";
/// 2: four-word codes (Phase 11). A build on 1 can't pair with one on 2, and says "no match".
pub const PAIR_ALPN: &[u8] = b"nhako-pair/2";
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

/// Fields added after version 1 shipped, written straight after a `ClientHello` or `ServerHello`
/// in the same frame. Older builds decode the hello and ignore the bytes after it; a hello from an
/// older build has nothing after it, which reads as the default. docs/protocol/nhako-1.md, section 4.
///
/// On the wire it's the wake token (Phase 8), then a list of records (Phase 14), each a kind and
/// its own postcard bytes, so a build that doesn't know a kind skips just that one.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HelloExt {
    /// Where a wake-up push reaches this device (its Firebase Cloud Messaging token), for a phone
    /// that Android has put to sleep. Only sent to trusted devices.
    pub wake: Option<String>,
    /// With an `Offer`: the folder each file goes in (Phase 14). Builds without it save every file
    /// straight in the save folder.
    pub folders: Option<Folders>,
    /// With an `Offer`: this is text, not a file (Phase 14). The offer still lists one file, the
    /// text as UTF-8, so builds without it save it as a `.txt` file.
    pub text: Option<String>,
    /// With a `Ping`: asks the other device to trust this one, and this one it (Phase 14). It's
    /// the commitment to the asker's random number (`codekeys::trust_commit`). See [`TrustMsg`].
    pub trust: Option<[u8; 32]>,
}

/// Which folder each offered file goes in, relative to where received files are saved, with `/`
/// between folder names. `of[i]` is file `i`'s: 0 for none, otherwise `dirs[of[i] - 1]`. A folder
/// is listed once however many files it holds, so an offer of thousands stays well under a frame.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Folders {
    pub dirs: Vec<String>,
    pub of: Vec<u32>,
}

impl Folders {
    /// The folder of file `i`, if it has one.
    pub fn of(&self, i: usize) -> Option<&str> {
        let k = *self.of.get(i)? as usize;
        self.dirs.get(k.checked_sub(1)?).map(String::as_str)
    }
}

/// Frames after the `ServerHello` of a trust ask. The asked device sends `Nonce` (or `Answer(Busy)`),
/// the asker `Reveal`, then the asked device `Answer` once the person there decides. A build
/// without trust asks sends nothing and closes the stream, which reads as "needs a newer version".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustMsg {
    /// The asked device's random number.
    Nonce([u8; 32]),
    /// The asker's random number, matching the commitment in its hello.
    Reveal([u8; 32]),
    Answer(TrustAnswer),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustAnswer {
    Yes,
    No,
    /// Already answering an ask, or asked too often.
    Busy,
}

#[derive(Serialize, Deserialize)]
struct ExtV1 {
    wake: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct Record {
    kind: u16,
    body: Vec<u8>,
}

mod record {
    pub const FOLDERS: u16 = 1;
    pub const TEXT: u16 = 2;
    pub const TRUST: u16 = 3;
}

impl HelloExt {
    fn records(&self) -> Result<Vec<Record>, postcard::Error> {
        let mut out = vec![];
        if let Some(f) = &self.folders {
            out.push(Record {
                kind: record::FOLDERS,
                body: postcard::to_stdvec(f)?,
            });
        }
        if let Some(t) = &self.text {
            out.push(Record {
                kind: record::TEXT,
                body: postcard::to_stdvec(t)?,
            });
        }
        if let Some(c) = &self.trust {
            out.push(Record {
                kind: record::TRUST,
                body: postcard::to_stdvec(c)?,
            });
        }
        Ok(out)
    }

    fn encode(&self) -> Result<Vec<u8>, postcard::Error> {
        let mut out = postcard::to_stdvec(&ExtV1 { wake: self.wake.clone() })?;
        let records = self.records()?;
        if !records.is_empty() {
            out.extend(postcard::to_stdvec(&records)?);
        }
        Ok(out)
    }

    /// Reads what it can: a part that's missing or can't be read counts as empty.
    fn decode(bytes: &[u8]) -> Self {
        let Ok((v1, rest)) = postcard::take_from_bytes::<ExtV1>(bytes) else {
            return Self::default();
        };
        let mut ext = Self {
            wake: v1.wake,
            ..Self::default()
        };
        for r in postcard::from_bytes::<Vec<Record>>(rest).unwrap_or_default() {
            match r.kind {
                record::FOLDERS => ext.folders = postcard::from_bytes(&r.body).ok(),
                record::TEXT => ext.text = postcard::from_bytes(&r.body).ok(),
                record::TRUST => ext.trust = postcard::from_bytes(&r.body).ok(),
                _ => {}
            }
        }
        ext
    }
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
    write_body(send, postcard::to_stdvec(msg)?).await
}

/// A hello followed by its extension (left out entirely when it's empty).
pub async fn write_hello<T: Serialize>(send: &mut SendStream, hello: &T, ext: &HelloExt) -> Result<(), WireError> {
    write_body(send, encode_hello(hello, ext)?).await
}

pub fn encode_hello<T: Serialize>(hello: &T, ext: &HelloExt) -> Result<Vec<u8>, WireError> {
    let mut body = postcard::to_stdvec(hello)?;
    if *ext != HelloExt::default() {
        body.extend(ext.encode()?);
    }
    Ok(body)
}

/// A hello and whatever extension came after it. An extension that can't be read counts as empty,
/// so a newer peer adding fields never breaks the hello itself.
pub fn decode_hello<T: DeserializeOwned>(bytes: &[u8]) -> Result<(T, HelloExt), WireError> {
    let (hello, rest) = postcard::take_from_bytes::<T>(bytes)?;
    Ok((hello, HelloExt::decode(rest)))
}

async fn write_body(send: &mut SendStream, body: Vec<u8>) -> Result<(), WireError> {
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
    Ok(postcard::from_bytes(&read_raw(recv).await?)?)
}

/// Reads a hello frame with its extension.
pub async fn read_hello<T: DeserializeOwned>(recv: &mut RecvStream) -> Result<(T, HelloExt), WireError> {
    decode_hello(&read_raw(recv).await?)
}

async fn read_raw(recv: &mut RecvStream) -> Result<Vec<u8>, WireError> {
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
    Ok(body)
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
    fn hello_extension_works_both_ways_with_version_1() {
        let hello = ServerHello {
            version: VERSION,
            info: DeviceInfo {
                name: "Phone".into(),
                kind: DeviceKind::Phone,
                os: OsPlatform::Android,
            },
        };
        let ext = HelloExt {
            wake: Some("fcm-token".into()),
            ..HelloExt::default()
        };
        let bytes = encode_hello(&hello, &ext).unwrap();
        // A version 1 build reads the hello and stops.
        assert_eq!(postcard::from_bytes::<ServerHello>(&bytes).unwrap(), hello);
        assert_eq!(decode_hello::<ServerHello>(&bytes).unwrap(), (hello.clone(), ext));
        // A hello from a version 1 build has no extension.
        let old = postcard::to_stdvec(&hello).unwrap();
        assert_eq!(decode_hello::<ServerHello>(&old).unwrap(), (hello.clone(), HelloExt::default()));
        assert_eq!(encode_hello(&hello, &HelloExt::default()).unwrap(), old, "an empty extension isn't written");
        // Bytes a later version might add that this one can't read: the hello still counts.
        let mut later = old.clone();
        later.extend([0xff, 0xff, 0xff]);
        assert_eq!(decode_hello::<ServerHello>(&later).unwrap().0, hello);
    }

    #[test]
    fn records_reach_new_builds_and_old_ones_ignore_them() {
        let hello = ClientHello {
            version: VERSION,
            info: DeviceInfo {
                name: "Laptop".into(),
                kind: DeviceKind::Laptop,
                os: OsPlatform::Linux,
            },
            intent: Intent::Ping,
        };
        let ext = HelloExt {
            wake: None,
            folders: Some(Folders {
                dirs: vec!["Photos".into(), "Photos/2024".into()],
                of: vec![0, 1, 2],
            }),
            text: Some("https://example.com".into()),
            trust: Some([9; 32]),
        };
        let bytes = encode_hello(&hello, &ext).unwrap();
        assert_eq!(decode_hello::<ClientHello>(&bytes).unwrap(), (hello.clone(), ext.clone()));
        // A Phase 8 to 13 build reads the hello, then its extension (just the wake token), and stops.
        let (old_hello, rest) = postcard::take_from_bytes::<ClientHello>(&bytes).unwrap();
        assert_eq!(old_hello, hello);
        assert_eq!(postcard::from_bytes::<ExtV1>(rest).unwrap().wake, None);
        // A Phase 8 to 13 hello with a wake token still reads here.
        let mut old = postcard::to_stdvec(&hello).unwrap();
        old.extend(postcard::to_stdvec(&ExtV1 { wake: Some("t".into()) }).unwrap());
        assert_eq!(decode_hello::<ClientHello>(&old).unwrap().1.wake.as_deref(), Some("t"));
        // A record of a kind this build doesn't know is skipped; the rest still count.
        let mut later = postcard::to_stdvec(&hello).unwrap();
        later.extend(postcard::to_stdvec(&ExtV1 { wake: None }).unwrap());
        later.extend(
            postcard::to_stdvec(&vec![
                Record { kind: 99, body: vec![1, 2, 3] },
                Record {
                    kind: record::TEXT,
                    body: postcard::to_stdvec("hi").unwrap(),
                },
            ])
            .unwrap(),
        );
        assert_eq!(decode_hello::<ClientHello>(&later).unwrap().1.text.as_deref(), Some("hi"));
        assert_eq!(ext.folders.as_ref().unwrap().of(0), None);
        assert_eq!(ext.folders.as_ref().unwrap().of(2), Some("Photos/2024"));
        assert_eq!(ext.folders.as_ref().unwrap().of(7), None);
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
