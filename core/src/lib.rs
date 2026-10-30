//! Nhako Beam core: the transfer engine shared by the command line tool, the desktop app
//! (`desktop/src-tauri`) and the Android app (UniFFI, Phase 7).
//!
//! - [`Engine`] is the whole API. It mirrors the app's contract in
//!   `desktop/src/lib/engine/types.ts`, and its [`EngineEvent`]s serialize to the same JSON.
//! - The wire protocol is documented in `docs/protocol/nhako-1.md`.

pub mod beacon;
pub mod codekeys;
mod diag;
mod engine;
pub mod files;
pub mod firewall;
mod history;
pub mod logfile;
pub mod net;
mod pair;
pub mod server;
mod session;
mod share;
pub mod store;
pub mod text;
mod trust;
pub mod types;
pub mod update;
pub mod wake;
pub mod wire;
pub mod words;

pub use diag::diagnostics_without_engine;
pub use engine::{Engine, EngineConfig, EngineError, Me, Outgoing, device_id, parse_device_id};
/// How much one transfer can hold.
pub mod engine_limits {
    pub use crate::engine::{FILES_MAX, TEXT_MAX};
}
pub use iroh::{RelayUrl, address_lookup::MemoryLookup};
pub use net::Network;
pub use server::{OwnNetwork, SettingError, check_server};
pub use types::*;
