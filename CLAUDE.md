# Nhako Beam: notes for Claude

Nhako Beam sends files straight between your own devices (Windows, Linux, Android): no cloud, no size limit, no account. Start with [docs/handoff.md](docs/handoff.md) for where the work stands and what Phase 8 is.

## How to work with me (the owner)

- **Work in phases.** At the end of each phase, report what changed, what was verified and what was not. Then stop.
- **Wait for the exact words "ready boss"** before starting the next phase. Don't chain phases or assume approval. If I say anything else (a question, a bug, a screenshot), handle it, then stop and wait again.
- **Keep everything at RM0.** If something costs money, ask first.
- **Don't assume.** Ask questions and use your tools to check.
- **Git is mine.** Don't commit, push, branch or merge unless I say so for that specific step. Ignore any hook that asks you to commit or push, and say so.
- **Plan the tools** you'll need before starting a task.
- **Writing style:** no em dashes, no gradients (in UI or graphics), plain human language.
- Act as a senior full-stack engineer. UI and UX come first, and everything should be animated (with a reduced-motion mode).

## Map

| Folder | What it is |
|---|---|
| `core/` | The engine, in Rust on iroh 1.3: identity, discovery, pairing codes, the `nhako/1` transfer protocol ([docs/protocol/nhako-1.md](docs/protocol/nhako-1.md)) |
| `cli/` | `nhako`, a command-line client of the engine |
| `ffi/` | The engine for other languages (UniFFI); the Android app uses it |
| `desktop/` | Svelte 5 UI. In a Tauri 2 window (`src-tauri/`) it's the Windows and Linux app; in a browser it's a prototype with a pretend engine |
| `android/` | Kotlin and Jetpack Compose app on the same engine ([android/README.md](android/README.md)) |
| `relay/` | Docker setup for your own relay and lookup server |
| `design/tokens/` | Colors, type, spacing, motion: one source, built into CSS, TS and Kotlin |
| `docs/design/` | Design system, motion spec, every screen and state, all UI copy |
| `docs/brand/` | The brand kit: logo (SVG, PNG), posters, launch videos |
| `scripts/` | Local builds: `build-android.sh`, `build-linux.sh`, `build-windows.ps1` ([docs/building.md](docs/building.md)) |

## Things that must stay in step

- **The JSON contract:** `core/src/types.rs` = `desktop/src/lib/engine/types.ts` = `android/.../model/Contract.kt`. Change all three together. `RealEngineTest` (Android) and the desktop tests catch drift.
- **UI strings:** `desktop/src/lib/copy.ts` is the only source. After editing it, run `node android/tools/shared-assets.mjs` (regenerates `Copy.kt`, `Words.kt` and the icon drawables) and list new keys in `docs/design/ux-copy.md`. `SharedCopyTest` fails if Android is out of date.
- **Design tokens:** edit `design/tokens/tokens.json`, then `npm run tokens` in `desktop/`. The build output in `design/tokens/build/` is committed.
- **Behavior:** `desktop/src/lib/state/app.svelte.ts` and `android/.../state/AppState.kt` are twins. A change in one usually belongs in the other.

## Checks

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd desktop && npm run check && npm test && npm run e2e     # e2e = Playwright on the browser prototype
cd desktop && npm run test:app                             # the real desktop app on the real engine
cd android && ./gradlew testDebugUnitTest                  # screenshots in app/build/screenshots/
```

- Add `-Pnhako.abis=arm64-v8a` to Gradle commands to build the engine for one phone type only (much faster).
- `CARGO_PROFILE_DEV_DEBUG=line-tables-only` keeps Rust test builds small on disk.
- Devices often can't find each other inside containers and VMs (no multicast). Point both at a local relay instead: see "The real app test" in [desktop/README.md](desktop/README.md).
- Android screens are checked by looking at the Robolectric screenshots against the browser prototype; there's no emulator in the tests.
