# nhako-core

The Nhako Beam transfer engine. One library for the command line tool, the desktop app (`desktop/src-tauri`) and Android (UniFFI, Phase 7).

It runs on [iroh](https://iroh.computer) 1.x: encrypted QUIC between device keys, hole punching, and relay fallback. On top of that it adds the `nhako/1` protocol (see [docs/protocol/nhako-1.md](../docs/protocol/nhako-1.md)).

## The API

`Engine` mirrors the app's contract in `desktop/src/lib/engine/types.ts`, and its events serialize to the exact JSON the app expects.

```rust
let engine = Engine::start(EngineConfig::new(config_dir)).await?;
let mut events = engine.subscribe();          // EngineEvent stream
let id = engine.send(&device_id, &paths)?;     // to a nearby, trusted or code-paired device
let code = engine.create_code();               // three words, 10 minutes, one use
let result = engine.redeem_code(&words).await; // RedeemResult
engine.accept(&request_id, trust, auto_accept);
engine.pause(&id); engine.resume(&id); engine.cancel(&id); engine.retry(&id);
engine.set_connection(Connection::Own(own))?; // saved; used from the next start
let check = check_server(&own).await;         // ServerCheck { relay, lookup }
let snap = engine.snapshot();                 // devices, transfers, requests, code: for a window that (re)loads
let paths = engine.saved_files(&id);          // where a received transfer's files ended up
let problem = engine.server_problem();        // your own server: None, Unreachable or TokenRejected
```

On your own server the engine checks the relay at start and on every rescan, every 5 minutes (every 30 s while something is wrong) and when entering a code fails. Changes arrive as a `server` event, and a code that failed because the relay turned the token down comes back as `tokenRejected` instead of `noMatch`.

One difference from the TypeScript contract: `send` takes file paths, because the engine reads the files itself. The desktop app maps the window's `FileItem`s to paths.

## Layout

| File | What |
|---|---|
| `src/engine.rs` | State, public API, incoming connections, discovery, reconnecting |
| `src/session.rs` | One transfer over one connection: sender pump, receiver writer, pause, resume, verify |
| `src/pair.rs` | Three-word codes: showing and entering |
| `src/codekeys.rs` | Argon2id and BLAKE3 key derivation for codes |
| `src/wire.rs` | Wire messages and framing |
| `src/net.rs` | Endpoint setup (public, your own server, or same network only), route detection, ping |
| `src/server.rs` | Your own server: reading the setting, `NO_PROXY`, checking the relay and lookup server |
| `src/store.rs` | Device key, name, save folder, trusted devices on disk |
| `src/files.rs` | Safe file names, unique paths, free space |
| `src/types.rs` | The UI contract types |
| `src/words.txt` | The 256 code words (also read by the app) |

## Tests

```bash
cargo test -p nhako-core
```

Unit tests cover the contract JSON, wire round trips, key derivation, file name safety and the store. The integration tests in `tests/engine.rs` start real engines that talk over QUIC on localhost: send and verify, decline, withdraw, pause and resume from both sides, cancel, reconnect after a drop, retry from both sides, changed source file, damaged part file, code pairing, wrong codes, auto-accept, and restart.

`tests/own_server.rs` runs a real relay with a token and a real lookup server inside the test (the same software as `relay/`), then pairs by code and sends a file with direct paths turned off, checks that a wrong token can't reach anyone, and that the server check tells a wrong token, a missing server and a typo apart.
