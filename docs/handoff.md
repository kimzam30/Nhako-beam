# Handoff: where Nhako Beam stands (end of Phase 9)

Last updated 2026-10-12, at the end of Phase 9. Read [CLAUDE.md](../CLAUDE.md) first for the working rules. The short version: work in phases, stop after each one, and wait for "ready boss". Keep everything free. Git is the owner's.

## Where the code is

- **Branch `dev` on GitHub:**
  - `48cb0cb`: Phases 0 to 7.
  - `abe8915` ("beambrandkit"): the brand kit in `docs/brand/`.
  - `d6264f7`: these handoff notes for Phase 8.
- **Phases 8 and 9 are not committed yet.** They sit in the working tree on `dev` until the owner commits them.
- `main` still holds only the initial plan. `claude/youthful-euler-vxt9o5` is an unused cloud branch.
- Versions: 0.1.0 everywhere. Android: versionCode 1, minSdk 26, targetSdk 37, compileSdk 37.

## What's done

| Phase | What |
|---|---|
| 0 to 2 | Planning, design system and tokens, UX copy, every screen and state, motion spec; the browser prototype (Svelte) with a pretend engine and Playwright flows |
| 3 | The Rust engine on iroh 1.3 (identity, nearby discovery, pairing codes, the `nhako/1` protocol with pause, resume, retry), the `nhako` CLI, the protocol spec |
| 4 | Own servers: the Docker relay and lookup server in `relay/`, a relay token, Settings > Connection |
| 5 | The Linux desktop app (Tauri) on the real engine: tray, start at login, notifications, the .deb |
| 6 | Windows: the installer (.exe, unsigned, downloads WebView2 if missing), "Send with Nhako Beam" in Explorer and Files, the GitHub Build workflow |
| 7 | The Android app (Compose on the engine through `ffi/`), plus local build scripts |
| 8 | Brand, launch intro, smaller APKs, push wake-up, history, licenses, tray quick view, notification buttons, nearby switch, Quick Settings tile |
| 9 | Sounds for every interaction, on both apps, plus the fixes from the first real-device test |

### Phase 8 in detail

- **Brand:**
  - The logo is the app icon everywhere: Android (adaptive, monochrome, notification) and desktop (`desktop/src-tauri/icons/`).
  - The accent is the logo's dark blue `#1A6FE0`. The logo's `#2F8FFF` is kept for the icon, splash and beam glow, because white text on it fails contrast. `design/tokens/check-contrast.mjs` passes all 64 pairs.
  - The brand colors are a token group (`brand` in `tokens.json`), built into CSS, TS and Kotlin.
- **Launch intro** (Android and desktop, about 0.8 s): the bolt presses in, springs back with a ring pulse, then the app fades in. Reduced motion: a 0.2 s fade. On Android it follows the system SplashScreen and only plays on a fresh launch. Spec: motion-spec 4.14.
- **Smaller APK:**
  - The release profile is fat LTO, one codegen unit, stripped (`Cargo.toml`). On arm64, `libnhako_ffi.so` went from 16.8 MB to 14.5 MB, and speed from about 345 to 365 MB/s (the ignored `throughput` test in `core/tests/engine.rs`).
  - `opt-level = "s"` was measured and rejected: 2 MB smaller, but 10% slower.
  - One APK per phone type (`splits.abi` in `android/app/build.gradle.kts`).
  - The build scripts, VS Code tasks and CI now make shrunk release APKs (R8, rules in `android/app/proguard-rules.pro`), signed with the computer's debug key until there's a real release key. The app id no longer ends in `.debug`.
  - Result for arm64: 34 MB in Phase 7, 17.3 MB now (arm32 12.4 MB, x86_64 19.3 MB). The engine is 14.6 MB of that, stored uncompressed so Android can load it in place.
- **Push wake-up (built, switched off):**
  - A phone shares a push token only with devices it trusts (a hello extension, compatible with Phase 7 builds both ways; see the protocol doc).
  - To reach a sleeping phone, the sender calls the wake Worker (`wake/`, Cloudflare), which sends an empty high-priority Firebase push. The phone stays ready for 90 s, and the sender keeps trying for 45 s.
  - It needs a Firebase project and a Cloudflare account, both free. The owner said "not now", so the apps build without it. Setup: [wake/README.md](../wake/README.md).
- **New features on both apps:**
  - History (`history.json`, newest 500), with Open and Show in folder.
  - Settings > About > Open source licenses, made by `scripts/licenses.mjs`.
  - Settings > "Find nearby devices" on or off (the engine restarts).
  - Devices that can be woken show as asleep; a transfer to one shows "Waking".
- **Desktop only:** a tray quick view (popover window), and Accept/Decline and Open/Show in folder buttons on notifications (notify-rust, Linux and Windows).
- **Android only:** a "Receive" Quick Settings tile. The unused icons are gone (the generator only copies icons the Kotlin code names).
- **Repo:** `LICENSE-MIT` and `LICENSE-APACHE`, a new README, VS Code tasks (`.vscode/tasks.json`) for building and installing the APK.

### Verified, and not

- **Verified:**
  - `cargo fmt` and `clippy --workspace -D warnings` are clean.
  - The engine: 27 unit and 16 integration tests pass, including history, the nearby switch, waking a sleeping phone, and the wake token going only to trusted devices.
  - Desktop: 7 Rust tests, 51 unit tests, svelte-check with 0 errors, and 61 Playwright flows (5 skipped, the usual platform-only ones).
  - The wake Worker: 4 tests.
  - Android: all 33 JVM tests pass, with screenshots of the intro, history, licenses, and the asleep and waking phone.
  - **On the emulator** (Android 15, x86_64), the shrunk release APK: onboarding, pairing with a code against the `nhako` CLI on this computer (same check value on both sides), receiving a 20 MB file with an identical checksum, History, licenses, and the nearby switch. No crashes.
  - **On real devices** (the owner, after Phase 8): the Linux .deb on Zorin OS 18.1 and the arm64 APK on a Galaxy S24+ found each other on the same Wi-Fi ("Nearby", fast), files went both ways, a 790 MB file paused and resumed, and a transfer kept going with the phone's screen off.
  - Two desktop bugs they found, fixed with a test each (`found on real devices after phase 8` in `flows.spec.ts`): Settings couldn't be scrolled (the tray popover's `body { overflow: hidden }` reached the main window too, since both share one stylesheet), and a long file name pushed the request card's content past its edge.
  - On mobile data the phone showed "No devices nearby". As designed: only trusted devices are reached over the internet, and these two had never been saved as trusted. Not tested again yet after trusting.
  - The emulator found two bugs, both fixed: sheets stayed under the on-screen keyboard (the code words sheet couldn't be used), and short waits read "About a few seconds left".
- **Not verified yet:**
  - Sending from the phone, the share sheet, the Quick Settings tile, and background transfers with the screen off (none tried on the emulator).
  - Push wake-up end to end (no Firebase project).
  - The real desktop app test (`npm run test:app` needs `tauri-driver` and `webkit2gtk-driver`).
  - The tray quick view, notification buttons and window background color in the real desktop app.
  - The Windows build script as a script, and the CI jobs on GitHub. (`scripts/build-linux.sh` has now run end to end.)

### Phase 9 in detail

- **Sounds, on by default** ([design/sounds/README.md](../design/sounds/README.md)):
  - 22 sounds made from scratch by `design/sounds/build.mjs`: a game console menu feel (soft clicks, woody marimba notes, small bell chimes), nothing copied. The owner asked for something "inspired by Nintendo Switch OS audio"; these are our own.
  - Every tap and switch, sheets and screens, a device showing up, requests, Accept and Decline, sending, pause and resume, done, failed, cancelled, codes matching, errors, the swipe line on a request card, picking up and snapping dragged files, and the launch intro.
  - One sound per press: a tap gives way to a control's own sound, and a repeat within 400 ms plays once. Devices showing up make one sound, not one each.
  - Settings > Appearance > Sounds turns them off.
  - **Desktop:** played by the Rust side (`sound.rs`, rodio), because WebKit keeps a page silent until its first click: the launch sound and requests that arrive while the app sits in the tray would be lost otherwise. Notifications are silent (`SuppressSound` on Linux; Windows toasts have no sound set); the app plays the sound.
  - **Android:** SoundPool (`platform/Sounds.kt`, rules in `state/SoundPlayer.kt`), quiet on silent or vibrate and while the app is out of sight. Notifications have the sounds through new channels: `requests.chime` (incoming) and `received` (done); the old `requests` channel is deleted.
  - Size: about 0.7 MB more in each app (WAVs, which Android stores uncompressed). The desktop UI bundle leaves them out (the Rust side has them).
- **Fixes from the real-device test:** Settings scrolls again, long file names stay in the request card, `apt install --reinstall` in the instructions (same version numbers made apt skip the update).
- **Linux builds now need `libasound2-dev`** (rodio). The build script checks for it, CI installs it, and the .deb depends on `libasound2t64 | libasound2`.

### Phase 9: verified, and not

- **Verified:**
  - fmt and clippy clean; all Rust tests pass (desktop: 8, plus `speakers`, run once by hand: it opened this laptop's speakers through PipeWire and played the done chime).
  - Desktop: 57 unit tests (6 new for the sound rules), 68 Playwright flows (taps, screens, switches, the Sounds switch staying off after a reload, and request to accept to done).
  - Android: 39 JVM tests (6 new: the sound rules, the state playing the same sounds as desktop, a tap on Settings playing only "open", and the notification channels having the new sounds).
  - On the emulator (release APK): Android logged a playback from the app for every tap and for the launch intro on a cold start, none on silent mode, and the Sounds switch shows in Settings.
  - The sounds checked by numbers (no clipping, clean starts and ends, each at its loudness target), and a listening page published for the owner.
- **Not verified yet:**
  - Hearing them on the owner's phone and laptop, and whether they like them.
  - The notification sounds on a real phone (the channels are right in the test; not heard).
  - Windows: rodio there is untested, and so are silent toasts.

### After Phase 9: the owner's second real-device test

- **Sounds:** the tap was too loud on both apps. It's rounder and 7 dB quieter, the other small sounds 5 dB, and Settings > Appearance > Sounds is now Loud, Soft (40%, the default) or Off, on both apps. An earlier on/off choice carries over (on becomes Soft).
- **"Changed while sending" for files shared from a file manager (Android):** the engine re-opened `/proc/self/fd/<n>` by path, which Android's storage layer refuses for files from providers like Files. It now reads through the descriptor Android opened (`core/src/files.rs` `open_to_send`); apps that share a stream rather than a file get a cache copy first (`RealEngine.send`). Reproduced on the emulator with the system file picker, then fixed: the same photo arrives with the same checksum. Tests: `reads_a_file_android_opened_even_when_its_path_is_off_limits`, `sends_a_file_given_as_an_open_descriptor`.
- **Drag and drop on desktop** was built but never worked in the real app: WebKit fires the page's own drag events too, without files, and their drop ended the drag before Tauri's drop with the paths arrived. The desktop app now uses only Tauri's events. Checked in a real Tauri window on XWayland with a scripted drag from a GTK window (the file was staged and the send sheet opened). Home now says files can be dropped.
- **A removed trusted device that's offline stayed on screen until a restart:** the engine now reports it gone (a device in reach stays, untrusted). Engine test `removing_a_trusted_device_that_is_away_takes_it_off_the_list`; the browser prototype and the Android fake do the same, with a Playwright flow.
- Seen once, not again: a "Send with a code" transfer from the emulator to the CLI's `receive <words>` sat at 0%. Two retries of the same flow worked.

## Next (Phase 10 is not planned yet; ask the owner)

- **The owner's verdict on the sounds**, then adjust recipes in `build.mjs` (each is a few lines).
- **"Trust this device" from Nearby, and a clearer empty state off Wi-Fi** (offered after the mobile data test; not decided).

- **Real devices:** the basics, big files with pause and resume, and the screen off all work. Still to try: across the internet after pairing with a code and saving as trusted.
- **Push wake-up:** when the owner wants it, the Firebase and Cloudflare setup in [wake/README.md](../wake/README.md), then a test with the app closed.
- **Play Store** "in a month or two" per the owner: a release signing key, an app bundle (.aab), the store listing, a privacy policy, and the Data safety form.
- **Desktop signing:** the Windows installer is unsigned (SmartScreen warns).
- **Speed on the emulator** was only 1.4 MB/s, through the emulator's own network layer and a debug build of the CLI. The engine benchmark gives about 365 MB/s, so this is very likely the emulator, but check it between two real devices.
- **This computer has an Android emulator** (`Medium_Phone_API_35`, KVM works): `emulator -avd Medium_Phone_API_35 -read-only -no-window` leaves its saved state untouched.

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
