# nhako-ffi

The engine from `core/` for apps that aren't written in Rust. It's a thin layer made with [UniFFI](https://mozilla.github.io/uniffi-rs/): one `NhakoEngine` object with its own async runtime, whose methods take and return JSON in the same shapes as `core/src/types.rs`, and an `EventListener` the app implements to hear about devices, requests and transfers.

The Android app uses it (see `android/README.md`). Its Gradle build:

1. builds `libnhako_ffi.so` for each phone type with `cargo ndk`, and
2. generates the Kotlin bindings with the `uniffi-bindgen` program in this crate, from a host build of the library (release builds are stripped of the metadata UniFFI reads):

```bash
cargo build -p nhako-ffi --lib
cargo run -p nhako-ffi --bin uniffi-bindgen -- generate --library target/debug/libnhako_ffi.so --language kotlin --out-dir <folder>
```

Files shared from other Android apps arrive as open file handles rather than paths, so `send` takes each file's path (`/proc/self/fd/N` for those) together with the name to show.
