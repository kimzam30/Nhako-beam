//! Nhako Beam core: the transfer engine shared by the command line tool, the desktop app
//! (`desktop/src-tauri`) and the Android app (UniFFI, Phase 7).
//!
//! - [`Engine`] is the whole API. It mirrors the app's contract in
//!   `desktop/src/lib/engine/types.ts`, and its [`EngineEvent`]s serialize to the same JSON.
//! - The wire protocol is documented in `docs/protocol/nhako-1.md`.

pub mod codekeys;
mod engine;
pub mod files;
pub mod net;
mod pair;
pub mod server;
mod session;
pub mod store;
pub mod types;
pub mod wire;
pub mod words;

pub use engine::{Engine, EngineConfig, EngineError, Me, device_id, parse_device_id};
pub use iroh::{RelayUrl, address_lookup::MemoryLookup};
pub use net::Network;
pub use server::{OwnNetwork, SettingError, check_server};
pub use types::*;
