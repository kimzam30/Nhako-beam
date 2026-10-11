# nhako/1 protocol

Version 1. This is the wire protocol between two Nhako Beam devices. It runs on top of [iroh](https://iroh.computer) 1.x, which gives us an encrypted, authenticated QUIC connection between two device keys, plus hole punching and relay fallback. Everything below happens inside that connection.

The reference implementation is `core/src/wire.rs` (message types), `core/src/session.rs` (transfer sessions) and `core/src/code.rs` (three-word codes).

## 1. Identity

- Every device has one long-lived Ed25519 key, stored in its config folder (`identity.key`). The public key is the iroh **EndpointId** and the device id in the app.
- The **fingerprint** shown in the app is the first 4 bytes of the public key in upper-case hex, split in two: `4F2A 91C3`.
- Device names are never published to the internet. They are only sent inside an encrypted connection (in `Hello`), so a device's name is only visible to devices that can already reach it.

## 2. Finding devices

| Where | How |
|---|---|
| Same network | mDNS with the service name `nhakobeam`. This only announces the EndpointId and its local addresses. To learn the name, the other device connects and sends `Intent::Ping`. |
| Across the internet | iroh address lookup: pkarr records on n0's server by default, or on your own lookup server (see Servers below). The record holds only the EndpointId and its home relay, not its IP addresses. Only trusted devices (and code peers) are looked up this way. |

## 3. Framing

Two ALPNs:

- `nhako/1` for everything between real device keys.
- `nhako-pair/1` only for the short-lived code endpoint (section 7).

**Control frames** are a 4-byte big-endian length followed by a [postcard](https://docs.rs/postcard) 1.x encoding of the Rust type. The longest allowed frame is 4 MiB. Anything longer is a protocol error.

**Data** is raw bytes on QUIC unidirectional streams (section 6).

Connection close codes:

| Code | Meaning |
|---|---|
| 0 | Finished normally |
| 1 | Cancelled |
| 2 | Declined |
| 3 | Protocol error |
| 4 | Not allowed (unknown device asking from the internet) |
| 5 | Failed. The close reason says why: `integrity`, `fileChanged`, `permission` or `unknown` |

## 4. Opening a connection

The device that dials opens one bidirectional stream, the **control stream**, and sends `ClientHello`. The other side answers with `ServerHello`.

```rust
struct DeviceInfo { name: String, kind: DeviceKind, os: OsPlatform }

struct ClientHello { version: u16 /* = 1 */, info: DeviceInfo, intent: Intent }
struct ServerHello { version: u16, info: DeviceInfo }

enum Intent {
    Ping,                                        // just swap DeviceInfo, then close with 0
    Offer(Offer),                                // "I want to send you these files"
    Resume { id: TransferId },                   // sender reconnecting to a known transfer
    Pull { id: TransferId, files: Vec<FileState> }, // receiver asking the sender to continue
}

struct Offer { id: TransferId /* 16 random bytes */, files: Vec<FileMeta> }
struct FileMeta { name: String, size: u64 }
```

File names are the base name only. The receiver strips any path parts, control characters and names Windows can't store, and picks `name (1).ext` if the name is taken.

After `ServerHello`, the side holding the files waits for an `Answer`:

```rust
enum Answer { Accept { files: Vec<FileState> }, Decline(DeclineReason) }
enum DeclineReason { User, NoSpace, NotAllowed, Unknown }
enum FileState { Have(u64), Verified }
```

`files[i]` says how much of file `i` the receiver has: `Have(n)` means `n` bytes are on disk but not checked yet (0 for a new transfer), `Verified` means the file is complete and its hash matched. The two are kept apart on purpose: a file whose bytes have all arrived but haven't been checked, and an empty file, both still need a data stream so the receiver can check the hash.

While the sender waits for the person on the other side to decide, it may send `Ctrl::Withdraw` on the control stream. The receiver then removes the request (`incomingWithdrawn`) and closes with code 1.

### Who may offer files

The receiver answers `Decline(NotAllowed)` and closes with code 4 unless the sender is one of these:

- a trusted device,
- a device seen on the same network through mDNS,
- a device paired with a code during this run of the app.

So someone who only knows your EndpointId can't push requests at you from across the internet.

## 5. During a transfer

Both sides send `Ctrl` frames on the control stream:

```rust
enum Ctrl {
    Progress { file: u32, bytes: u64 }, // receiver -> sender, about every 250 ms while moving
    Verdict { file: u32, ok: bool },     // receiver -> sender, after checking a file's hash
    Pause { reason: PauseReason },      // either way
    Resume,                             // sender -> receiver: "send me Want"
    Want { files: Vec<FileState> },     // receiver -> sender: start or restart the data
    Cancel,                             // either way, then close with 1
    Withdraw,                           // sender -> receiver, only before Answer
    Done,                               // sender -> receiver, after every file has Verdict ok
}
enum PauseReason { User, DiskFull }
```

Rules:

- The sender only pushes data after an `Accept` or a `Want`, for every file that isn't `Verified`, starting at its `Have` offset.
- **Pause** (from either side) stops the current data stream. The sender finishes the chunk it is on, then resets the stream. Nothing is lost: everything the receiver has written stays on disk.
- **Resume** always ends with the receiver sending `Want` with its current file states. If the receiver resumes, it just sends `Want`. If the sender resumes, it sends `Resume` and the receiver answers with `Want`. Either side may resume a pause, whoever paused it.
- **Disk full** on the receiver: it sends `Pause { reason: DiskFull }`. The sender shows "{device} ran out of space". The receiver resumes once there's room.
- **Done**: when the last file has `Verdict ok`, the sender sends `Done`, the receiver marks the transfer finished and closes the connection with code 0.

## 6. Data streams

For each file still needed, the sender opens one unidirectional stream:

```rust
struct DataHeader { id: TransferId, file: u32, offset: u64, len: u64 }  // framed like a control frame
// then exactly `len` raw bytes (the file from `offset` to the end)
// then 32 bytes: the BLAKE3 hash of the WHOLE file
```

- Files go one after another, never two at once, so the first file finishes first.
- The receiver writes into `<name>.nhakopart` in the save folder. `offset` must not be past the part file's length (that is a protocol error). If it is shorter, the receiver cuts the part file back to `offset`, which happens when a resume races with bytes still in flight.
- If a stream arrives for a file the receiver has already verified (a resume raced with the check), the receiver reads and drops it and sends `Verdict { ok: true }` again.
- Both sides hash the whole file. A side starting at `offset > 0` first hashes the bytes before the offset from its own copy. Hasher state is kept in memory across pauses in the same session, so a pause near the end of a big file doesn't re-read it.
- If the hashes match, the receiver renames the part file to the real name and sends `Verdict { ok: true }`. If they don't, it deletes the part file and sends `Verdict { ok: false }`. The transfer then stops with the `integrity` error, and **Try again** resends that file from the start.
- If the sender's file changes size or modified time after the offer, the sender closes with code 5 and reason `fileChanged`, and both sides show that error.

## 7. Three-word codes

A code is three words from `core/src/words.txt` (exactly 256 words, so 16.7 million codes). It lives 10 minutes and works once.

**Turning words into keys.** Both sides compute

```
window = floor(unix_seconds / 600)
K      = Argon2id(password = "w1 w2 w3", salt = "nhakobeam-code-v1:" + window,
                  memory = 64 MiB, iterations = 3, lanes = 1, output = 32 bytes)
rendezvous_key = BLAKE3.derive_key("nhakobeam 2026-10 pair rendezvous", K)   // an iroh secret key
confirm_key    = BLAKE3.derive_key("nhakobeam 2026-10 pair confirm", K)
```

Argon2id is slow on purpose (64 MB of memory and about 0.13 s per try on a desktop CPU, more on a phone), and the salt changes every 10 minutes. That makes it impractical to scan millions of codes or to work them out in advance. The person entering a code tries the windows that overlap the last 11 minutes and the next 2 minutes, to allow for clock differences. That is usually two windows, sometimes three.

**Host (shows the code).** It starts a second, temporary iroh endpoint whose secret key is `rendezvous_key`, with ALPN `nhako-pair/1`. The endpoint announces itself on mDNS and (when relays are on) on the address lookup service. It waits for a connection until the code expires.

**Joiner (types the code).** It connects from its normal endpoint to the rendezvous EndpointId. It keeps trying for up to 20 seconds, because the host may still be starting and its address can take a few seconds to appear in lookups. If nothing answers, the result is `noMatch`. The TLS handshake already proves the host holds `rendezvous_key`, so it knows the code. Then:

```rust
struct PairJoin    { version: u16, info: DeviceInfo, device: EndpointId, mac: [u8; 32] }
struct PairWelcome { info: DeviceInfo, device: EndpointId, mac: [u8; 32] }
struct PairDone    { ok: bool }

join mac    = BLAKE3.keyed(confirm_key, "join" || rendezvous_id || joiner_id)
welcome mac = BLAKE3.keyed(confirm_key, "welcome" || rendezvous_id || joiner_id || host_id)
check       = BLAKE3.derive_key("nhakobeam 2026-10 pair check", confirm_key || host_id || joiner_id)[0..4]
```

- The host checks that the connection's authenticated remote id equals `PairJoin.device` and that the mac is right. After 3 bad attempts the code is burned.
- The joiner checks the welcome mac, then sends `PairDone { ok: true }`.
- Both sides now know each other's real EndpointId, and the code endpoint shuts down. Both show `check` in hex (`5D19 E04A`) under "Check that both screens show". If someone in the middle knew the code, the two screens would show different values.
- Afterwards, files move over normal `nhako/1` connections between the real device keys.

## 8. Reconnecting

QUIC keep-alive runs every 2 seconds, and a connection with no traffic for 12 seconds counts as lost. iroh moves between Wi-Fi, mobile data and the relay by itself. Only a fully lost connection reaches this protocol:

- Both sides set the transfer to `reconnecting`.
- The sender redials every 2 seconds for up to 60 seconds with `Intent::Resume`. The receiver answers `Accept` with its current offsets.
- After 60 seconds both sides mark the transfer `failed` (`peerGone`). **Try again** on the sender redials with `Resume`; on the receiver it dials the sender with `Pull`.

## 9. What is not in version 1

- Folders (files only; a folder picker sends the files inside it).
- Resuming after an app restart. Part files stay on disk, but the transfer list is in memory only.
- Several files in parallel. One at a time is simpler and already fills the link.

## Servers

None of this protocol changes with the servers. They only help two devices find and reach each other.

| Setting | Relay | Address lookup |
|---|---|---|
| Public (default) | n0's free relays | n0's pkarr server, plus n0's DNS |
| Your own | Your relay (`relay/` in this repo). With a token, it lets in only devices that send it (`Authorization: Bearer`). | Your pkarr server if set, otherwise n0's |
| Same network only (CLI `--local-only`) | none | none, mDNS only |

- Devices only reach each other over the internet when they share the same servers. A device on public servers can't look up a record that lives only on your lookup server, and can't get through a relay that needs a token it doesn't have. Same-network discovery and transfers work either way.
- An `http://` relay has no QUIC address discovery (that needs TLS), so hole punching relies on the relay alone. That's fine at home; an internet-facing relay should use `https://`.
- The lookup server limits publishing per IP address. It does not limit lookups, so guessing codes is still held back by the Argon2id cost and the 10 minute life of a code, not by the server.
