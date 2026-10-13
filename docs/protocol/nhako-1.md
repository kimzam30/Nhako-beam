# nhako/1 protocol

Version 1. This is the wire protocol between two Nhako Beam devices. It runs on top of [iroh](https://iroh.computer) 1.x, which gives us an encrypted, authenticated QUIC connection between two device keys, plus hole punching and relay fallback. Everything below happens inside that connection.

The reference implementation is `core/src/wire.rs` (message types), `core/src/session.rs` (transfer sessions) and `core/src/pair.rs` with `core/src/codekeys.rs` (four-word codes).

## 1. Identity

- Every device has one long-lived Ed25519 key, stored in its config folder (`identity.key`). The public key is the iroh **EndpointId** and the device id in the app.
- The **fingerprint** shown in the app is the first 4 bytes of the public key in upper-case hex, split in two: `4F2A 91C3`.
- Device names are never published to the internet. They are only sent inside an encrypted connection (in `Hello`), so a device's name is only visible to devices that can already reach it.

## 2. Finding devices

| Where | How |
|---|---|
| Same network | mDNS with the service name `nhakobeam`, and the beacon (below, added in Phase 10). Both only announce the EndpointId and where it takes connections. To learn the name, the other device connects and sends `Intent::Ping`. |
| Across the internet | iroh address lookup: pkarr records on n0's server by default, or on your own lookup server (see Servers below). The record holds only the EndpointId and its home relay, not its IP addresses. Only trusted devices (and code peers) are looked up this way. |

### The beacon (added in Phase 10)

mDNS shares UDP port 5353 with the system's own mDNS service. Windows' DNS Client service keeps that port to itself for IPv4, so there mDNS only runs over IPv6, which other devices rarely hear. The beacon is Nhako Beam's own announcement on a port nothing else uses.

- **Where:** UDP port **47820**. Sent to the multicast group **239.255.78.66** (TTL 1) and to the subnet's broadcast address, on every IPv4 network adapter that is up, isn't loopback or a point-to-point tunnel, and has a same-network address. With no adapters listed, to the group and to 255.255.255.255 on the default route.

The message is 40 bytes, all big-endian:

| Bytes | Field |
|---|---|
| 0 to 3 | `NHKB` |
| 4 | version, `1` |
| 5 | flags: bit 0 is "ask" (please announce yourselves) |
| 6 to 37 | the EndpointId (its 32 bytes) |
| 38 to 39 | the UDP port its iroh endpoint takes connections on (IPv4) |

Receivers also read a higher version if it is at least 40 bytes (later versions may add fields at the end). They ignore anything shorter than 40 or longer than 512 bytes, with another magic, or with port 0.

- **When:** an ask when the app starts, rescans, or its adapters change, sent three times 2 s apart; then a plain announcement every 30 s. A device that hears an ask answers the sender directly (unicast), at most once per 2 s per address.
- **On hearing one:** the receiver adds `source IP : port` as an address for that EndpointId, then pings it as it does for mDNS. Nothing in the message is trusted: the QUIC handshake checks the EndpointId's key. A device announcing itself again within 20 s isn't pinged again. One not heard from for 100 s is pinged once more and loses its "nearby" mark if that fails. At most 256 devices are tracked.
- One device hearing the other is enough: the ping that follows shows each to the other. That matters on Linux with ufw, which lets mDNS in but drops other incoming traffic by default.

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

### Hello extension (added in Phase 8)

Right after a `ClientHello` or `ServerHello`, in the same frame, a device may add a `HelloExt`:

```rust
struct HelloExt { wake: Option<String> }
```

- `wake` is the device's wake token (a Firebase Cloud Messaging token), sent only to devices it trusts. A trusted device that later can't reach it asks the wake service to push an empty "wake up" to that token, then keeps dialling for up to 45 s (`core/src/wake.rs`, `wake/`).
- An empty extension isn't written at all, so a hello with nothing to add is byte for byte the version 1 hello.
- Builds from before Phase 8 decode the hello and ignore what follows it. A hello from one of them has nothing after it, which reads as an empty extension. Bytes after the hello that can't be read as a `HelloExt` (a later version adding fields) count as empty too. So `version` stays 1 and old and new builds talk to each other.

### Extension records (added in Phase 14)

After the wake token, the extension may carry a list of records. On the wire it's the Phase 8 `HelloExt` (just `wake`), then:

```rust
struct Record { kind: u16, body: Vec<u8> }   // body is the record's own postcard bytes
// written as Vec<Record>, only when there is at least one
```

| Kind | Body | Sent with | Meaning |
|---|---|---|---|
| 1 | `Folders { dirs: Vec<String>, of: Vec<u32> }` | `Offer` | The folder each file goes in: `of[i]` is 0 for none, otherwise `dirs[of[i] - 1]`, with `/` between folder names, starting with the sent folder's own name (`Trip`, `Trip/day 2`). Each folder is listed once, so an offer of thousands of files stays far under a frame. |
| 2 | `String` | `Offer` | This is text (a note, a link, a clipboard), not a file. The offer still lists exactly one file, the same text as UTF-8, named `Text.txt` or `Link.txt`. |
| 3 | `[u8; 32]` | `Ping` | A trust ask (below): the commitment to the asker's random number. |

- A Phase 8 to 13 build reads the hello, then the wake token, and ignores the rest: it saves a sent folder's files straight in its save folder, and text as a `.txt` file. Nothing breaks.
- A newer build reading an older hello finds no records. A record of a kind it doesn't know is skipped on its own; the others still count. So more kinds can be added later without a version bump.
- An empty extension is still left out entirely.

File names are the base name only. The receiver strips any path parts, control characters and names Windows can't store, and picks `name (1).ext` if the name is taken.

**Folders.** Each folder name is made safe the same way; empty, `.` and `..` parts are dropped, and a file deeper than 32 folders loses its folder. Each sent folder gets a new folder of its own in the save folder, made new when the request is accepted: `Trip`, or `Trip (1)` if anything has that name already, so arriving files never mix with what's there. The folders inside it are made as their first file arrives. A cancelled transfer clears away the folders made for it once its part files are gone (a folder with anything left in it stays). Empty folders aren't sent.

**Text.** The text must be at most 64 KiB and exactly as long as the one file says, or it's taken as a plain file. The receiver shows it in the request, keeping line breaks and tabs but dropping other control characters and the ones that reorder text (direction embeddings, overrides and isolates). Accepting answers `Accept { files: [Verified] }`: nothing is written to disk, and the sender, finding nothing left to send, sends `Done` straight away. A build without text answers `Have(0)` and the text arrives as a file, as before.

After `ServerHello`, the side holding the files waits for an `Answer`:

```rust
enum Answer { Accept { files: Vec<FileState> }, Decline(DeclineReason) }
enum DeclineReason { User, NoSpace, NotAllowed, Unknown }
enum FileState { Have(u64), Verified }
```

`files[i]` says how much of file `i` the receiver has: `Have(n)` means `n` bytes are on disk but not checked yet (0 for a new transfer), `Verified` means the file is complete and its hash matched. The two are kept apart on purpose: a file whose bytes have all arrived but haven't been checked, and an empty file, both still need a data stream so the receiver can check the hash.

While the sender waits for the person on the other side to decide, it may send `Ctrl::Withdraw` on the control stream. The receiver then removes the request (`incomingWithdrawn`) and closes with code 1.

### Trust asks (added in Phase 14)

"Trust this device" from Nearby: one device asks another on the same network to trust it, and to be trusted back, without a code. Both screens show a check value worked out from a random number from each side, so someone in the middle can't make them match except by a one in 4 billion guess (the way Bluetooth's numeric comparison works).

```
asker:  ClientHello { intent: Ping }  +  record 3 = commit(nA)       commit(n) = BLAKE3.derive_key("nhakobeam 2026-10 trust commit", n)
asked:  ServerHello, then TrustMsg::Nonce(nB)      (or Answer(Busy))
asker:  TrustMsg::Reveal(nA)                       asked checks commit(nA)
both:   check = BLAKE3.derive_key("nhakobeam 2026-10 trust check", nA || nB || asker_id || asked_id)[0..4], shown as 5D19 E04A
asked:  TrustMsg::Answer(Yes | No)                 once the person there decides (or up to 2 minutes)

enum TrustMsg { Nonce([u8; 32]), Reveal([u8; 32]), Answer(TrustAnswer) }
enum TrustAnswer { Yes, No, Busy }
```

- The asker commits to its number before it sees the other's, and the asked device sends its number before it sees the asker's. Neither can choose its number to steer the check value.
- On `Yes` both save the other as trusted and show the same "paired" result as a code (`via: nearby`). On `No` nothing changes.
- The asked device only takes asks from a device on the same network (a same-network connection, or seen nearby), at most 3 waiting at once, one per device, and 3 a minute per device; otherwise it answers `Busy`.
- If the asker cancels or goes away, its connection closes and the ask goes away on the other device (`withdrawn`).
- A build from before Phase 14 answers the ping and closes without a `Nonce`; the asker shows "needs a newer Nhako Beam".

### Who may offer files

The receiver answers `Decline(NotAllowed)` and closes with code 4 unless the sender is one of these:

- a trusted device,
- a device seen on the same network through mDNS or the beacon,
- a device paired with a code during this run of the app.

So someone who only knows your EndpointId can't push requests at you from across the internet.

**Who can send to me (added in Phase 11).** A device that gets past the check above may still be refused at the `Offer`. The receiver's setting decides:

| Setting | Offers from trusted devices and ones paired with a code this run | Offers from anyone else |
|---|---|---|
| Your devices (the default, also for settings files from before Phase 11) | shown as a request | answered `Decline(NotAllowed)`, no request is shown; the sender shows `notAllowed` ("only takes files from its own devices") |
| Everyone nearby | shown as a request | shown, but at most one waiting request per device, 3 offers a minute per device (refused ones count), and 5 waiting requests from such devices in total. Over a limit: `Decline(Unknown)` |

`Ping` is never refused this way, so the device still shows up nearby and can be sent to.

**Names.** Each side cleans the other's `DeviceInfo.name` before showing or saving it: control and invisible characters (direction overrides and isolates, zero-width characters, tag characters and the like) are removed, spaces are collapsed, and it's cut to 32 characters. An empty result shows as "Unnamed device". File names in an offer lose the same invisible characters, so `invoice{U+202E}fdp.exe` saves as `invoicefdp.exe` and shows its real extension.

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

- Files' data goes one after another, never two at once. The receiver writes each checked file to disk for good (fsync) and moves it to its name in the background, up to 8 at a time, while the next file arrives, so `Verdict`s can come back out of order. A file only gets `Verdict { ok: true }` once it's safely on disk.
- The receiver writes into `<name>.nhakopart` in the save folder. The part file is created new when that file's data first arrives (the first file's when the request is accepted, so a folder that can't be written to is caught straight away). A name already taken by a file, folder or link is skipped, and on Linux and Android it's never opened through a link. `offset` must not be past the part file's length (that is a protocol error). If it is shorter, the receiver cuts the part file back to `offset`, which happens when a resume races with bytes still in flight.
- If a stream arrives for a file the receiver has already verified (a resume raced with the check), the receiver reads and drops it and sends `Verdict { ok: true }` again.
- Both sides hash the whole file. A side starting at `offset > 0` first hashes the bytes before the offset from its own copy. Hasher state is kept in memory across pauses in the same session, so a pause near the end of a big file doesn't re-read it. Each side also saves its hasher state every 16 MiB of the file being moved, for the life of the transfer, so after a dropped connection it hashes forward from the nearest saved state at or before `offset` (at most 16 MiB, plus what was in flight) instead of from the start. Neither changes what goes over the wire.
- If the hashes match, the receiver moves the part file to the real name without replacing anything (a hard link then removing the part file; where the disk has no hard links, it checks first and renames), and sends `Verdict { ok: true }`. On Windows the saved file also gets the "downloaded from another computer" mark (a `Zone.Identifier` stream with `ZoneId=3`), so SmartScreen and Office Protected View check it. If they don't, it deletes the part file and sends `Verdict { ok: false }`. The transfer then stops with the `integrity` error, and **Try again** resends that file from the start.
- If the sender's file changes size or modified time after the offer, the sender closes with code 5 and reason `fileChanged`, and both sides show that error.

## 7. Four-word codes

A code is four words from `core/src/words.txt` (exactly 256 words, so 4.3 billion codes). It lives 10 minutes and works once. Until Phase 11 codes were three words (`nhakobeam-code-v1`, ALPN `nhako-pair/1`); a build from before then can't pair with a newer one and gets "no match".

**Turning words into keys.** Both sides compute

```
window = floor(unix_seconds / 600)
K      = Argon2id(password = "w1 w2 w3 w4", salt = "nhakobeam-code-v2:" + window,
                  memory = 64 MiB, iterations = 3, lanes = 1, output = 32 bytes)
rendezvous_key = BLAKE3.derive_key("nhakobeam 2026-10 pair rendezvous", K)   // an iroh secret key
confirm_key    = BLAKE3.derive_key("nhakobeam 2026-10 pair confirm", K)
```

Argon2id is slow on purpose (64 MB of memory and about 0.13 s per try on a desktop CPU, more on a phone), and the salt changes every 10 minutes. That makes it impractical to scan millions of codes or to work them out in advance. Someone on the same network sees the rendezvous EndpointId (it's announced on mDNS) and could test guesses against it offline. With four words that's about 2 billion tries on average, roughly 78,000 CPU-core-hours, and it has to finish before the real person types the code, since a code works once. That's why codes got a fourth word in Phase 11. A PAKE (as Magic Wormhole uses) was considered: without a server of our own, part of the code would still have to be public for the devices to find each other, which leaves a 1 in 22,000 chance per code, so it isn't stronger here. The person entering a code tries the windows that overlap the last 11 minutes and the next 2 minutes, to allow for clock differences. That is usually two windows, sometimes three.

**Host (shows the code).** It starts a second, temporary iroh endpoint whose secret key is `rendezvous_key`, with ALPN `nhako-pair/2`. The endpoint announces itself on mDNS and (when relays are on) on the address lookup service. It waits for a connection until the code expires.

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

- Empty folders (a folder is sent as the files inside it, keeping their places since Phase 14).
- Resuming after an app restart. Part files stay on disk, but the transfer list is in memory only.
- Several files in parallel. One at a time is simpler and already fills the link.

## Servers

None of this protocol changes with the servers. They only help two devices find and reach each other.

| Setting | Relay | Address lookup |
|---|---|---|
| Public (default) | n0's free relays | n0's pkarr server, plus n0's DNS |
| Your own | Your relay (`relay/` in this repo). With a token, it lets in only devices that send it (`Authorization: Bearer`). | Your pkarr server if set, otherwise n0's |
| Same network only (CLI `--local-only`) | none | none, mDNS and the beacon only |

- Devices only reach each other over the internet when they share the same servers. A device on public servers can't look up a record that lives only on your lookup server, and can't get through a relay that needs a token it doesn't have. Same-network discovery and transfers work either way.
- An `http://` relay has no QUIC address discovery (that needs TLS), so hole punching relies on the relay alone. That's fine at home; an internet-facing relay should use `https://`.
- The lookup server limits publishing per IP address. It does not limit lookups, so guessing codes is still held back by the Argon2id cost and the 10 minute life of a code, not by the server.
