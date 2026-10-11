# Handoff: where Nhako Beam stands (start of Phase 8)

Last updated 2026-10-12, at the end of Phase 7. Read [CLAUDE.md](../CLAUDE.md) first for the working rules. The short version: work in phases, stop after each one, and wait for "ready boss". Keep everything free. Git is the owner's.

## Where the code is

- **Branch `dev` on GitHub** has everything:
  - `48cb0cb`: Phases 0 to 7.
  - `abe8915` ("beambrandkit"): the brand kit in `docs/brand/`.
- `main` still holds only the initial plan. `claude/youthful-euler-vxt9o5` is an unused cloud branch.
- Versions: 0.1.0 everywhere. Android: versionCode 1, minSdk 26, targetSdk 36, compileSdk 37.

## What's done

| Phase | What |
|---|---|
| 0 to 2 | Planning, design system and tokens, UX copy, every screen and state, motion spec; the browser prototype (Svelte) with a pretend engine and Playwright flows |
| 3 | The Rust engine on iroh 1.3 (identity, nearby discovery, pairing codes, the `nhako/1` protocol with pause, resume, retry), the `nhako` CLI, the protocol spec |
| 4 | Own servers: the Docker relay and lookup server in `relay/`, a relay token, Settings > Connection |
| 5 | The Linux desktop app (Tauri) on the real engine: tray, start at login, notifications, the .deb |
| 6 | Windows: the installer (.exe, unsigned, downloads WebView2 if missing), "Send with Nhako Beam" in Explorer and Files, the GitHub Build workflow |
| 7 | The Android app (Compose on the engine through `ffi/`), plus local build scripts |

### Phase 7 in detail

- **Screens:** every screen and state, matched to the prototype in light and dark, with full and reduced motion.
- **Share sheet target:** one file or many, of any kind.
- **Background transfers:** a foreground service (data sync) with a progress notification that has Pause or Resume and Cancel.
- **Notifications:** requests (with Accept and Decline) and "Got files" while the app is out of sight.
- **Received files:** saved to `Download/Nhako Beam` and added to the media index. Open and Share go through a FileProvider.
- **Permissions:**
  - Onboarding asks for Nearby devices and Notifications (Android 13+), and storage on Android 10 and older.
  - Camera is asked only when scanning.
- **No backups or phone-to-phone copies**, so a device identity key never ends up on two phones.
- **Tests:** 25 JVM tests (Robolectric screenshots of 11 states, the real engine through JNA, strings, the server form, formatting).
- **CI:** the Android job in `.github/workflows/build.yml` (checked with actionlint, not yet run on GitHub).
- **Local builds:** `scripts/build-android.sh` (tested, including `--fast`), `scripts/build-linux.sh` and `scripts/build-windows.ps1` (not run yet). Setup is in [building.md](building.md).

### Verified, and not

- **Verified:**
  - `cargo fmt` and `clippy` are clean on the whole workspace.
  - All Android tests pass. Android lint shows 0 errors.
  - The full debug APK is 64 MB (arm64, arm32, x86_64); `--fast` gives 34 MB (arm64).
  - The desktop app and engine tests passed in Phases 5 and 6.
- **Not verified yet:**
  - The APK on a real phone (no emulator was available).
  - Two real devices transferring through the Android app.
  - The two desktop build scripts as scripts.
  - The Android CI job on GitHub.
  - The owner hasn't reported back on installing the APK yet. Ask how it went.

## Phase 8: polish (not started, wait for "ready boss")

The owner's answers so far:

1. **Stay reachable while the app is closed, without an always-on notification ("like Blip").**
   - Today the app only receives new requests while it's open, because Android pauses background apps.
   - Without a persistent notification, the realistic way is a push message that wakes the app. That most likely means Firebase Cloud Messaging: free, but it needs a Firebase project, Google Play services on the phone, and a sender on a server (the lookup server, or a small service next to it).
   - **Bring options with trade-offs and any costs to the owner before building.** That includes privacy (what the push carries), phones without Play services, and the free tier's limits.
2. **New logo everywhere, plus a professional splash screen with animated micro interactions.**
   - Replace the app icon on Android (adaptive, monochrome and notification icons), Windows and Linux (`desktop/src-tauri/icons/`, made with `npx tauri icon` from an SVG) with the brand icon.
   - Ask which platforms get the splash screen and how long it may take. On Android, start from the system SplashScreen API, then an animated Compose intro that respects reduced motion.
3. **Smaller APK (the owner asked for a recommendation).** Recommended:
   - Build the engine with LTO, one codegen unit and stripped symbols.
   - Build one APK per phone type.
   - Measure before and after, and keep transfer speed.

Other open items worth raising when planning Phase 8:

- **Brand colors vs the app:**
  - The logo uses `#2F8FFF` blue, `#FFD23F` yellow and `#1A6FE0` dark blue; the app's accent is `#2457F5` (`design/tokens/tokens.json`).
  - Ask whether to retheme the apps to the brand. If yes, re-run `design/tokens/check-contrast.mjs` (yellow on white fails contrast for text).
- **LICENSE file:** the README links an MIT `LICENSE` that doesn't exist yet.
- **README:** the roadmap and "Repository Structure (Planned)" sections are out of date.
- **Not built yet:**
  - The About > licenses screen.
  - The desktop tray popover.
  - Action buttons on desktop notifications.
  - The LAN discovery on/off setting.
  - Transfer history.
  - The Android Quick Settings tile.
- **Android lint warnings:** 100 unused icon drawables (the generator copies the whole set) and targetSdk 36 vs the newest 37.
- **Later:** a Play Store release, "in a month or two" per the owner.

## The brand kit (`docs/brand/`)

| Folder | What's there |
|---|---|
| `logo/svg/` | `nhako-beam-icon.svg` (blue rounded square, yellow bolt with a dark blue offset), `-icon-flat` (no offset), `-icon-circle`, `-icon-mono-navy`, `-icon-mono-white`, `-bolt` (the bolt alone), `-wordmark-{light,dark}`, `-logo-horizontal-{light,dark}`, `-logo-stacked-{light,dark}` |
| `logo/png/` | The same in PNG, with the icon at 16 to 1024 px, and `favicon.ico` |
| `posters/` | Hero, features and "no cloud" posters at 16x9, 1x1, 4x5 and 9x16; a GitHub social preview |
| `video/` | The launch video at 16x9, 1x1, 4x5 and 9x16, each with a cover image |

For icons, `-icon-mono-white` and `-bolt` suit the Android monochrome and notification icons; the full-color icon suits launchers and the desktop.

## Setting up a local machine

Everything is in [building.md](building.md). In short: Rust (1.91+), Node 22, JDK 17 or newer, the Android SDK with NDK `29.0.14206865`, `platforms;android-37.0` and `build-tools;37.0.0`, the three Rust Android targets, and `cargo-ndk`. For the Linux app, the GTK and WebKit packages.

Notes from the cloud machine that don't belong in the repo:

- When Maven Central rate limited it (HTTP 429), Gradle used Google's mirror, set up in `~/.gradle/init.d/`. Robolectric took a mirror through `nhako.robolectricRepo=<url>` in `~/.gradle/gradle.properties` (the app's build reads it). A normal machine needs neither.
- JDK 21 needs the `--add-exports` flags for Robolectric on Android 16; they're already in `android/app/build.gradle.kts`.
