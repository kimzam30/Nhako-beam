# Handoff: where Nhako Beam stands (end of Phase 19)

Last updated 2026-10-30, at the end of Phase 19. Read [CLAUDE.md](../CLAUDE.md) first for the working rules. The short version: work in phases, stop after each one, and wait for "ready boss". Keep everything free. Git is the owner's.

## Where the code is

- **Branch `dev` on GitHub:**
  - `48cb0cb`: Phases 0 to 7.
  - `abe8915` ("beambrandkit"): the brand kit in `docs/brand/`.
  - `d6264f7`: these handoff notes for Phase 8.
  - `f2843f3` ("initialrelease"): Phases 8 and 9.
- **Branch `main`:** `009d45d` ("phase10-phase14:securityfix"): Phases 10 to 14, `2139efc` (the owner's fix so the engine tests compile on Windows), and `95a16b4`, the lightning mascot logo.
- **Branch `dev`:** the same up to `2139efc`, then `484b4ba` ("fdroid/flatpak"): Phase 15, and `c2c6145`, the same logo change as `95a16b4` made again on top.
- **`78db661`** (2026-10-28): `dev` merged into `main`, so `main` has Phase 15 too and both branches hold the same files.
- **`9c40fb8`, Phases 16 and 17:** one commit on top of the merge. `main` and `dev` both point at it, pushed to GitHub on 2026-10-28.
- **`7f6ebe7` ("windowsnearby/fix"), Phases 18 and 19:** on `dev`, pushed by the owner on 2026-10-30. Two small fixes after it are not committed yet: the accessibility test race and the firewall check needing the rule switched on (below).
- The plan for Phases 18 to 23 (cleanup, Windows discovery, always on, the voxel mascot and 3D UI, the nightly build) is in `~/.claude/plans/check-the-code-repo-frolicking-storm.md` on the owner's machine.
- The audit of 2026-10-27 that set Phases 16 and 17 is in `~/.claude/plans/scan-whole-repo-and-clever-lighthouse.md` on the owner's machine.
- The audit after Phase 9 (security, performance, how it compares, the motion plan) is in `~/.claude/plans/eager-kindling-ember.md` on the owner's machine. Its order: 10 Windows discovery, 11 security, 12 performance, 13 the bolt motion pass. The owner picked Phase 14 from its "later" list.
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
| 10 | Nearby discovery that works on Windows (the beacon), a log file, and Settings > About > Copy diagnostics |
| 11 | Security: four-word codes, Who can send to me (your devices by default) with request limits, clean names, the Windows download mark and asking before opening programs, safer part files, desktop paths only from you, no `file://` shares on Android, dependency audits |
| 12 | Performance: a benchmark script, small progress updates instead of the whole file list, part files made as each file starts, background disk syncs, resume save points, fewer device checks in the background, long file lists a page at a time |
| 14 | Folders (sent as folders, kept in shape), text, links and the clipboard, "Trust this device" from Nearby (both devices agree and check the same code), and a clearer screen off Wi-Fi |
| 15 | Accessibility (axe on every screen in both themes, keyboard focus kept in sheets, 320 px reflow, token contrast, Android names and touch sizes, 200% font), release ready at RM0 (update check, winget, SignPath, Flathub, F-Droid build, .aab), folders from the file manager, Share via link |
| 16 | The fix pass from the 2026-10-27 audit: stuck request cards, offer limits that held under a burst, memory and connection caps, the code host listening for Stop mid-handshake, events never dropped on Android, listeners that doubled on desktop, Pause and Cancel sounds on Android, and the protocol, copy and building docs brought in line with the code |
| 17 | Cleanup: unused copy keys, functions, dependencies and the `select` sound removed; the prototype's dev panel out of the desktop bundle |
| 19 | Windows nearby discovery: a firewall rule at install (one admin prompt), devices answer beacons from devices they can't reach, a relayed first contact gets 3 s to go same-network, Tailscale and link-local left out of the beacon, the Fix it banner, diagnostics for the firewall; the Android .aab build fixed |
| 18 | Cleanup: dead code, a finished plan doc and an unused icon copy removed; README, CLAUDE.md, `core/README.md` and `cli/README.md` brought in line with the code |
| 13 | The bolt motion pass on both apps: the logo bolt as a living icon (strike, check, crack, sleep), sparks, arcs to found devices, a bolt riding the beam and the progress ring, rolling percent, rim flash, icon nudges, field charge line, swipe charge meter, QR draw-in, toast time bar, and a Motion lab in the prototype |

### Phase 19 in detail

**What the owner's PC showed** (through the "beam testing windows" Claude session, read only):
- Windows 11 Home 25H2. Wired Ethernet `192.168.1.64/24` on the **Public** network profile, Tailscale `100.117.78.26`. svchost holds UDP 5353 for IPv4, as found in Phase 10.
- Before installing: no firewall rule for Nhako Beam, and the app wasn't installed.
- The CI build of `dev` (`9c40fb8`) installed to `C:\APPS\Nhako Beam`. The owner allowed Windows' prompt on Private and Public. Windows then made two "Nhako Beam" Allow rules for both profiles, and **the PC found the laptop under Nearby by itself**, even with the laptop's ufw on. The two trusted each other from Nearby.
- So the cause was the firewall: without a rule that covers Public, nothing comes in on a network Windows marks Public, while codes still work through the relay. Earlier installs most likely had a Private-only or Block rule from that prompt.

**What changed:**
- **Installer** (`desktop/src-tauri/windows/hooks.nsh`): one admin prompt after install, only if our rule ("Nhako Beam (nearby devices)") is missing and the install isn't silent (winget). It removes every rule for the exe first (Windows' own prompt rules, where a Block would win), then allows it in on every profile. Declining still installs. Uninstall removes the rule (another prompt, only if it's there).
- **`core/src/firewall.rs`:** the rule, the same commands for the app's Fix it (PowerShell `Start-Process -Verb RunAs`), whether the rule is there (netsh's exit code, since its text is in the system's language), and each network's profile (`Get-NetConnectionProfile`).
- **Engine:** devices answer beacons from devices they can't reach yet, not only asks. Windows lets a unicast answer to its own broadcast in for a few seconds even with no rule, then the PC connects out itself. A connection from a device the engine doesn't know gets up to 3 s to move from the relay to a same-network path before it's turned away (`LAN_PATH_WAIT`), and turned-away connections are logged at info and counted. The beacon leaves out link-local and 100.64/10 adapters (it was sending on Tailscale's /32 on the PC), and joins its group again every 5 minutes.
- **Copy diagnostics:** a Windows Firewall section (the rule there or missing, each network's profile), group join errors, connections turned away.
- **Desktop app:** on Windows without the rule, Home shows "Windows Firewall is keeping nearby devices out. Codes still work." with **Fix it** (`shell::fix_firewall`). The prototype's scenario panel has it under Discovery. Copy: ux-copy.md section 18. Android has no firewall, so nothing changes there.
- **Android build:** the Phase 15 `.aab` step had never run on GitHub and failed: with one APK per phone type and shrinking, Gradle left three shrunk resource files and `bundleRelease` couldn't choose. ABI splits are now off for bundle builds (Play splits by itself).

### Phase 19: verified, and not

- **Verified:**
  - Rust: fmt and clippy clean; 57 engine unit tests (3 new: the rule's commands, reading network profiles, which adapters get the beacon), 27 engine and 4 own-server integration tests, 10 desktop tests.
  - Desktop: svelte-check 0 errors, 127 unit tests, 144 Playwright flows (8 skipped), new: the firewall banner and Fix it on both layouts.
  - Android: 67 JVM tests; `bundleRelease` with two phone types builds a 46 MB `.aab` (the case that failed on GitHub).
  - On the owner's PC with the `dev` build: with an Allow rule on Public, Nearby works by itself.
- **Not verified:**
  - **GitHub Build on `7f6ebe7`** (run 41028703520): Windows passed (the Windows-only code and the NSIS hook compile, the installer builds), Android passed (the `.aab` now builds there too). Linux failed only in `a11y.spec.ts` "every sheet" on the phone layout: the prototype's pretend camera "finds" a code and pairs about 2.5 s after the sheet opens, and on GitHub's slower runner the axe scan outlasted that, so a Paired sheet covered the next button. An old race (Phase 15), not Phase 19. The test now types the words on the phone layout; 72 of 72 accessibility runs pass here (3 repeats).
  - **The new installer on the PC:** one admin prompt; afterwards a single rule "Nhako Beam (nearby devices)", Inbound, Allow, profiles Domain, Private and Public, and Windows' two old "Nhako Beam" prompt rules gone. The beacon goes out on Ethernet only (Tailscale left out). The laptop sees the PC (KIMA5) nearby.
  - **The fallback, on the PC:** the owner switched our rule off by hand from 09:35 to 09:40 UTC without restarting the app. A device that had never met the PC ("Fallback test", the `nhako` CLI from `7f6ebe7` on the laptop with ufw off) showed under Nearby on the PC. So with no rule at all, nearby discovery still works between Phase 19 builds.
- **Fixed after the test (not committed):** the firewall check (`firewall::rule_present`) asked only whether the rule exists, so a rule switched off wouldn't bring up Fix it. It now asks PowerShell for the rule with `Enabled` True (the same word in every language).
- **Not verified:**
  - The Fix it banner and its admin prompt on the real PC (the rule was there, so the banner never showed), and the new `rule_present` compiled on Windows (next push).
  - Which of the two fallback pieces did it (the answer to the PC's broadcast, or the 3 s wait while the connection moves off the relay): the PC doesn't log untrusted devices it finds. Both are in.
  - An older build on the other side of the fallback: the answering is new in Phase 19, so a Phase 18 or older laptop or phone may still not show up on a PC with no rule. The installer's rule covers that.

### Phase 18 in detail

From a scan of the whole repo on 2026-10-30 (every pub fn, Tauri command, TS and Svelte export, Kotlin declaration, icon, copy key and markdown link).

- **Removed:** `docs/logo-update-plan.md` (done, nothing linked to it); `desktop/src-tauri/icons/app-icon.png` (an unreferenced copy of `docs/brand/logo/png/nhako-beam-icon-1024.png`); `focusCard()` in `IncomingCard.svelte` and `focusFirst()` in `CodeInput.svelte` (never called); six bundled icons no screen uses (`power`, `arrow-counter-clockwise`, `dots-three`, `textbox`, `pencil-simple`, `scroll`); the FFI's `wake_available()` (only a test called it; Settings uses `wakeReady`).
- **Docs fixed:** `core/README.md` (four-word codes, the twelve modules missing from its layout table), `cli/README.md` (folders are supported), `README.md` (features and status through Phase 17, Node 20 or newer, start at login is a choice), `CLAUDE.md` (no more "Phase 8", `packaging/`, `fastlane/`, `bench.sh` and `winget.mjs` in the map), `building.md` (the old Phase 8 note), the `shared-assets.mjs` comment (`SharedCopyTest`).
- **Kept on purpose:** `.vscode/tasks.json` (building.md documents it), `.claude/launch.json` (the prototype preview), the brand kit's PNG sizes and videos, `fastlane/`, `design/*/build/`. `packaging/fdroid` and `packaging/flatpak` say 0.2.0 while the rest says 0.1.0; they're set by hand at release time ([releasing.md](releasing.md)).
- **Disk, not in git:** `target/` is 53 GB (`cargo clean`), `out/` holds 37 MB of builds from Oct 12, `desktop/test-results` 98 MB.

### Phase 18: verified, and not

- **Verified:** fmt and clippy clean; Rust 54 engine unit, 27 engine and 4 own-server integration, 10 desktop tests. Desktop: svelte-check 0 errors, 127 unit tests, 142 Playwright flows (8 skipped, as before). Android: 67 JVM tests (the FFI bindings regenerated by Gradle without `wake_available`).
- **Seen once:** `reconnects_after_a_drop_and_keeps_progress` failed while the Playwright run loaded the machine (resumed from 1.3 MB, the test wants 1.5 MB). It passed 5 times out of 5 alone and in the full run after. A timing margin in the test, not a code change from this phase; worth widening if it comes back.
- **Noticed for Phase 19:** this laptop's beacon also goes out on `docker0` and `virbr0`, the same kind of virtual adapter that Windows' Hyper-V and WSL add.
- **Not verified:** nothing here runs differently on a device, so no real-device or Windows try.

### Phase 8 in detail

- **Brand:**
  - The logo is the app icon everywhere: Android (adaptive, monochrome, notification) and desktop (`desktop/src-tauri/icons/`).
  - The accent is the logo's dark blue `#1A6FE0`. The logo's `#2F8FFF` is kept for the icon, splash and beam glow, because white text on it fails contrast. `design/tokens/check-contrast.mjs` passes all 64 pairs.
  - The brand colors are a token group (`brand` in `tokens.json`), built into CSS, TS and Kotlin.
- **Launch intro** (Android and desktop, about 0.8 s): the bolt presses in, springs back with a ring pulse, then the app fades in. Reduced motion: a 0.2 s fade. On Android it follows the system SplashScreen and only plays on a fresh launch. Spec: motion-spec 4.14.
- **Smaller APK:**
  - The release profile is fat LTO, one codegen unit, stripped (`Cargo.toml`). On arm64, `libnhako_ffi.so` went from 16.8 MB to 14.5 MB, and speed from about 345 to 365 MB/s (now `bench_one_big_file`, run with `scripts/bench.sh`).
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

### Phase 10 in detail

- **Why Windows found nothing nearby (found with the owner on their PC):** Windows' DNS Client service holds UDP port 5353 for IPv4 by itself. The mDNS library (`swarm-discovery`) drops the failed IPv4 socket without a word (`socket_v4(None).ok()`) and runs on IPv6 only, which the phone and laptop never heard. The app didn't show "blocked" because mDNS had "started". The firewall was fine (Private network, app allowed).
- **The beacon** (`core/src/beacon.rs`, spec in `docs/protocol/nhako-1.md` section 2): a 40-byte "I'm here" on UDP 47820, to the group 239.255.78.66 and the broadcast address of every LAN adapter. It runs next to mDNS. Hearing one adds the address and pings the device, as mDNS does. Asks are answered directly.
- **Fewer pings:** mDNS re-reported every device on each announcement, and each one started a connection. A device seen again within 20 s isn't checked again (both mDNS and the beacon).
- **Discovery state:** "blocked" now means neither mDNS nor the beacon could start. When mDNS can't get the IPv4 port, the log says so and the diagnostics show it.
- **Log file:** `<config folder>/logs/nhako-beam.log`, 1 MB plus one older file (`core/src/logfile.rs`), on the desktop app and Android.
- **Copy diagnostics** (Settings > About, both apps; `nhako diagnostics` on the CLI): network, nearby discovery (mDNS, beacon adapters, packets sent and heard), devices by short id, and the last 48 KB of the log. No device names, file names or internet addresses.
- **CLI:** `--no-mdns` is now `--no-nearby` (the old name still works).

### Phase 10: verified, and not

- **Verified:**
  - cargo fmt and clippy clean. Engine: 33 unit and 23 integration tests, including `beacon_alone_finds_nearby_devices` (two engines with no mDNS and no shared address table find each other through the beacon, then send a file).
  - Desktop: svelte-check 0 errors, 59 unit tests, 72 Playwright flows (6 skipped, the usual platform-only ones), including copy diagnostics on desktop and phone sizes.
  - Android: 39 JVM tests, with the new UniFFI binding.
  - On this laptop's real Wi-Fi: `nhako diagnostics` sends the beacon on all three adapters (18 packets, none failed).
- **Found:** this laptop runs ufw (Zorin's default: incoming dropped, mDNS let in), so it can't hear other devices' beacons. That's fine in practice: one side hearing the other is enough, since the ping that follows shows each to the other.
- **Not verified:**
  - On Windows: not compiled here (no Windows target on this machine), not run. The real test is the owner's PC finding the phone and the laptop.
  - Phone and laptop with the new build, both ways.
  - A firewall "Fix it" button: dropped from this phase, since the firewall wasn't the cause.

### Phase 11 in detail

Audit items S1 to S5, S7, S8 and S11 (`~/.claude/plans/eager-kindling-ember.md`). S6 (wrapping the device key), S9 (wake service limits) and S10 (signing) were not in this phase.

- **S1, codes are four words** (owner's choice over a PAKE, which without our own server would leave part of the code public). 4.3 billion codes; salt `nhakobeam-code-v2`, ALPN `nhako-pair/2`. **Old and new builds can't pair with each other**; three words get "no match". Both apps show the words as two rows of two and type them in two rows of two slots (four across on a wide desktop sheet).
- **S2, Who can send to me** (Settings > Receiving, both apps; `nhako who-can-send yours|everyone`). Default **Your devices** (owner's choice), also for settings saved before Phase 11: trusted devices and devices paired with a code this run. Anyone else gets `Decline(NotAllowed)` with no request card, and the sender's card says "{device} only takes files from its own devices" with **Pair with a code** and Try again. With **Everyone nearby**: one waiting request per device, 3 asks a minute per device (refused ones count), 5 waiting from strangers in total. Pings are never refused, so devices still show up.
- **S3, names:** other devices' names lose control and invisible characters (direction overrides, zero-width, tags), spaces collapse, 32 characters at most (`core/src/text.rs`). File names lose the same invisible characters. Request cards from devices that aren't trusted show **Not one of your devices**.
- **S4, opening programs:** received files on Windows get the "downloaded from another computer" mark (`Zone.Identifier`, zone 3). Opening a single received program or script (exe, msi, bat, ps1, lnk, sh, AppImage, apk and more; `isRunnable` in `format.ts` and `Format.kt`) asks first, from the transfer card and History.
- **S5, Android:** shares and picks only take `content://` addresses; `file://` ones are skipped.
- **S7, part files:** created new only (a planted file or link is skipped), opened without following links on Linux and Android, and finished with a hard link so nothing is ever replaced (rename after a check where the disk has no hard links).
- **S8, desktop paths:** the file and folder dialogs now open from Rust. Drops and "Send with Nhako Beam" paths are recorded on the Rust side, and `files_from_paths` and `set_save_dir` only take those. The page lost the `dialog:allow-open` permission and the `@tauri-apps/plugin-dialog` npm package. The real-app test sets `NHAKO_TEST_ANY_PATH=1`, because WebDriver can't drag real files.
- **S11, dependency audits** (cargo-audit 0.22.2 and osv-scanner 2.6.0 installed for this, both free; npm audit): npm 0, Android 0 (322 resolved libraries). Rust: no vulnerabilities, 4 warnings, all deep in other libraries: `glib` 0.18 unsound iterator (RUSTSEC-2024-0429, from Tauri's Linux GTK stack, fixed only in glib 0.20, which Tauri doesn't use yet), and unmaintained `paste` (iroh), `proc-macro-error` (GTK) and `atomic-polyfill`. Nothing to do on our side until Tauri and iroh move.

### Phase 11: verified, and not

- **Verified:**
  - Rust: fmt and clippy clean; 36 engine unit tests, 20 engine and 4 own-server integration tests, 8 desktop tests. New: `only_your_devices_can_send_by_default` (refused with the reason, Everyone nearby one at a time and 3 a minute, then allowed after a code), four-word pairing (three words never match), name cleaning, a planted link never written through, finishing never replaces a file, older settings get the new default.
  - Desktop: svelte-check 0 errors, 62 unit tests, 78 Playwright flows (6 skipped), new: the refused send with Pair with a code, the Settings switch, the Not one of your devices label and asking before opening `Setup.exe`.
  - Android: 43 JVM tests, new screenshots 21 to 23 (label, refused send, ask before opening) and the four-word code screens checked by eye.
- **Not verified:**
  - Windows: the download mark and the Rust-side dialogs and drops aren't compiled or run here. Real drag and drop on Linux isn't either (only the test's fake drop).
  - Real devices with the new builds. All of them need updating together, because of the four-word codes.

### Phase 12 in detail

Measured first with `scripts/bench.sh` (the ignored `bench_` tests in `core/tests/engine.rs`: two engines on localhost, release build, capped at 3 GB of memory with systemd-run). This computer, ext4 on NVMe:

| Benchmark | Before | After |
|---|---|---|
| One 512 MB file | 221 to 354 MB/s (noisy) | 241 to 360 MB/s (unchanged) |
| 5,000 files of 200 KB | ran out of memory (2,000 files: 56 MB/s, 335 MB of updates to the app) | 228 MB/s, 1,114 files/s, 2.5 MB of updates |
| 20,000 files of 100 bytes | ran out of memory (2,000 files: 0.15 s to accept, 427 files/s) | 0.04 s to accept, 2,064 files/s |
| Connection dropped at 85% of 1 GB | 0.9 s spent re-reading what both sides had | none measurable |

- **Progress updates** (the cause of the out-of-memory runs, which also took the Claude app down once): every finished file sent the app the whole transfer with its whole file list, so N files cost N x N. Now there's a `transferProgress` event (`types.rs`, `types.ts`, `Contract.kt`) with only bytes, speed and the files that moved; full `transfer` events are for status changes. Both apps merge it (`app.svelte.ts`, `AppState.kt`); a window that falls behind still gets a snapshot. Per-file progress also no longer walks every file (`Tx::set_file`).
- **Part files** are made when each file's data starts (`DstFile::claim`), not all at accept time; the first one still is, so an unwritable folder is caught straight away. No more thousands of empty `.nhakopart` files in Downloads before anything moves.
- **Disk syncs:** each received file's fsync and move to its name run in the background, up to 8 at a time, while the next file arrives. It was 3 to 4 times slower one by one. A file still only counts as received once it's on disk, so `Verdict`s can arrive out of order (protocol doc section 6).
- **Resume save points:** both sides keep hasher states every 16 MiB for the life of the transfer (`session::Marks`), so a reconnect re-reads at most 16 MiB instead of everything so far. No wire change.
- **Battery:** `Engine::set_foreground` (desktop: whether the main window or quick view is on screen, checked every 5 s in `forward`; Android: `MainActivity` onStart/onStop through `ffi` `set_foreground`). In the background, trusted devices and devices seen nearby are checked every 5 minutes instead of every 60 and 20 seconds, and free space every minute; coming back checks trusted devices at once. Devices just checked because they were seen nearby are skipped by the minute check.
- **Long lists:** the Files sheet and History's file list show 100 files, then **Show 100 more** (`transfer.files.more`), on both apps (Android's History list used to stop at 50 silently). Counts group thousands: "1,240 photos". The prototype's scenario panel has "A folder of 1,240 photos".
- **Not done from the audit:** sending several small files at once (P3). The time went into disk syncs, not the network, so it wasn't needed. P5 (repeated mDNS announcements) was already handled by `RECHECK_NEARBY`.

### Phase 12: verified, and not

- **Verified:**
  - Rust: fmt and clippy clean; 38 engine unit tests, 21 engine and 4 own-server integration tests, 8 desktop tests. New: `many_files_arrive_without_flooding_the_folder_or_the_app` (one part file at a time, small progress updates, at most a few full ones), `resumes_hashing_from_the_nearest_mark`, `progress_is_camel_case_and_small`.
  - Desktop: svelte-check 0 errors, 63 unit tests, 80 Playwright flows (6 skipped), new: a folder of 1,240 photos lists 100, then 200.
  - Android: 44 JVM tests, new screenshot 24 (a folder of 1,240 photos, with a progress update merged in), checked by eye.
- **Not verified:**
  - The real desktop app test (`npm run test:app`): `tauri-driver` and `webkit2gtk-driver` aren't installed on this computer.
  - The background check timing (`set_foreground`) has no automated test; it's 60 s and 5 minute timers.
  - Phones: how much faster small files and resumes are on real phone storage. The benchmark ran on a laptop's NVMe disk.
  - The radar and beam animations' CPU use while the window is in the tray (P7) wasn't profiled.

### Phase 16 in detail

From the audit of 2026-10-27 (the whole repo against the docs). Every finding was checked again on `dev` before fixing.

- **Engine:**
  - A waiting request loses its card whenever its sender's stream ends or says anything out of turn, not only on `Withdraw` (before, a stray `Done` left a card that couldn't be accepted).
  - The duplicate check, the "Everyone nearby" limits and adding the request happen in one hold of the state lock, so offers arriving at once can't all slip under the one-per-device and five-waiting limits (`admit_offer`, `bad_offer`).
  - At most 4 frames are read ahead per stream (was 64, each up to 4 MiB), and at most 256 incoming connections are handled at once; past that they're refused (`FRAMES_AHEAD`, `CONNECTIONS_MAX`). The copied frame reader is gone; `wire::read_raw` is the one.
  - The code host keeps watching for Stop and the code running out while someone is mid-handshake, and burns the code after 10 attempts that never finish (`MAX_DROPPED_ATTEMPTS`), as it does after 3 wrong ones.
- **FFI (Android):** events reach Kotlin in order on a thread of their own (`nhako-events`), never on a runtime thread, so Kotlin may block or call back into the engine there. `RealEngine` uses that to wait instead of dropping an event when the app falls behind.
- **Desktop app:**
  - `Engine.start()` now returns a promise (contract change in `types.ts`, mock and Tauri), so a failed start shows its reason as a toast, as on Android.
  - The Tauri engine lets go of its listeners when a start fails or is stopped halfway, so a retry never hears every event twice (`tauri.test.ts`).
  - Reset onboarding no longer adds the window listeners and "Send with Nhako Beam" a second time, and stops listening to the old engine.
  - The phone layout (and its pretend QR scanner) never turns on in the desktop app, even on a touch screen.
  - Copy code and Copy diagnostics go through the Rust clipboard in the app, and say "Copied" only when it worked.
  - An unknown server problem shows as it came instead of breaking the Connection screen (as Android does).
  - The pairing animation stops when its sheet closes; "Pair and send" sends staged text too; a long press on an orb that goes away doesn't open its menu; the code countdown stops ticking once the code runs out.
  - Calls that could fail with no one listening now log or toast.
  - The Rust side reads dropped and picked folders off the async threads, and forgets staged files once they're sent or shared.
- **Android:**
  - Pause, Resume, Cancel and Try again play their own sounds, as on desktop.
  - After Android stops the transfer service (its 6-hour daily limit, or nothing to show), the next transfer starts it again.
  - A swiped answer goes even when the card leaves mid-flight.
- **Docs:** the real app test steps are back, in [building.md](building.md) (they went with `desktop/README.md` in `f2843f3`; CLAUDE.md points there now). The protocol doc has `nhako-pair/2`, the `addr` field in the pairing messages, same-network connections in "Who may offer files", the 100,000-file limit and the 300 s background checks. `ux-copy.md` matches `copy.ts`: the 22 keys drafted in Phase 1 and never built moved to their own list at the end, and two values were corrected. The README's Development link works again.
- **Phase 15's own code** (`share.rs`, `update.rs`, the file manager menus) was read for the same kinds of bugs; none found. `share_link` opens each file once on the calling thread, which is slow for a huge folder but not wrong.

### Phase 17 in detail

- **Removed:** 7 copy keys no screen used (`onboarding.skip`, `home.badge.trusted`, `home.orb.busy`, `send.summary`, `send.retry`, `history.detail.text`, `common.done`); `Engine::online`; the FFI's `set_save_dir` (Android never called it); the desktop's `quit` command, `springAnimate` and `hasIcon`; Android's `wakeAvailable` property and five unused FakeEngine hooks; the `select` sound (21 sounds now).
- **Dependencies removed:** npm `@tauri-apps/plugin-notification` and `@tauri-apps/plugin-opener` (only the Rust side uses those plugins); `tracing` in `ffi/`, tokio's `signal` feature and a second `rand` in `core/`; Gradle `compose-ui-tooling`, `compose-ui-tooling-preview`, `lifecycle-process` and `kotlinx-coroutines-test`. Licenses regenerated.
- **The browser prototype's scenario panel and Motion lab** load on demand, so the desktop app's bundle no longer carries them (a separate 15 KB chunk).
- **Left as they are, on purpose:**
  - The engine host in `desktop/src-tauri/src/engine.rs` and in `ffi/` (start, stop, restart on a setting): sharing it would mean a new layer in `core` for two callers that differ in threads and errors.
  - The plain-language error words in those two places: they differ on purpose (Android can't name a file shared as a handle, and phones have no cable).
  - Splitting `core/src/engine.rs` (about 2,600 lines) and `Sheets.svelte` (14 sheets): pure moves with no change in behavior, best done when nothing else is in flight.
  - Contract values the engine never sends (`TransferError::Network` and `RelayUnreachable`, `RedeemFailure::Expired`): harmless, and removing them changes all three contract files. `DiscoveryState::Offline` is used by the desktop app itself.

### Phases 16 and 17: verified, and not

- **Verified:**
  - Rust: fmt and clippy clean; 54 engine unit tests (2 new: `a_request_whose_sender_speaks_out_of_turn_loses_its_card`, `offers_arriving_at_once_still_get_one_card_per_device`), 27 engine and 4 own-server integration tests, 10 desktop tests.
  - Desktop: svelte-check 0 errors, 127 unit tests (3 new in `tauri.test.ts`: one pair of listeners, none left after a failed start, a stop while starting wins), 142 Playwright flows (8 skipped, as in Phase 15).
  - Android: 67 JVM tests, new `pauseAndCancelOnATransferPlayTheirOwnSoundsAsOnDesktop`. `SharedCopyTest` passes with the regenerated `Copy.kt`.
- **Not verified:**
  - No automated test for: Reset onboarding not doubling listeners, the code host stopping mid-handshake, the 10-attempt burn, the service starting again after Android stops it, Kotlin calling back into the engine from an event, the connection cap.
  - The real desktop app (`npm run test:app`, still no `tauri-driver`): the clipboard for Copy code, folders read off the async threads, and a failed start's toast.
  - Real devices and Windows: nothing new tried.
  - The flows print Svelte warnings (`derived_inert`, and `assignment_value_stale` at `Home.svelte:238`); they were there before these phases and nothing fails.

### Phase 15 in detail

The owner chose three of the four options: an accessibility pass, release ready at RM0, and folders from the file manager plus Share via link. Decisions they made: a browser opening a shared link **waits until you let it in**; the apps **check GitHub for a new version once a day, on by default, with a switch**; prepare **winget and SignPath, Flathub, F-Droid, and an .aab** for Play later.

- **Share via link** ([share-via-link.md](protocol/share-via-link.md), `core/src/share.rs`): a small HTTP server on this device's network address, a random key in the address, a cookie per browser, and nothing about the files until you let it in. Each file downloads on its own (with resume), or everything as a stored zip (Zip64 past 4 GB, local times). Ends after 15 minutes once downloads finish, or on Stop. The page itself is `core/src/share_page.html`, light and dark, with reduced motion. Contract: `LinkShare`, `LinkVisitor`, `VisitorState`, the `linkShare` event, `Snapshot.linkShare`, in all three files. Desktop and Android: "Share via link" in Send to, the link sheet (QR, address, who opened it, Let in or No, progress), a "Sharing a link" banner, and a notification with Let in and No when the app is hidden. CLI: `nhako share`.
- **Update check** (`core/src/update.rs`): GitHub's release list at most once a day (it includes pre-releases, which every 0.x release is). Settings > About has the switch and a "Nhako Beam 0.3.0 is out" row; Home a quiet banner with Not now. Off in builds that update through a store: the Flatpak (it sees `FLATPAK_ID`) and F-Droid (`setFromStore`), and the CLI.
- **Folders from the file manager:** Explorer gets a second entry for folders (and the uninstaller removes it); Linux adds Nemo and Dolphin beside Files. Existing entries get the new ones on the next start.
- **Accessibility, desktop:** `desktop/tests/e2e/a11y.spec.ts` runs axe (WCAG 2.2 A and AA) on onboarding, Home (tray, transfer, banner, request), every sheet, Settings, History and Licenses, in light and dark, on both layouts; keyboard checks (focus stays inside a sheet, Escape returns focus, Home works with Tab alone, a request answered by keyboard); and a 320 px reflow check. `src/lib/contrast.test.ts` checks every text color against its surfaces from the tokens. Fixes: sheets keep focus inside, the transfer bar has a name, a transfer card's title is announced when its state changes, Settings no longer runs 11 px wider than a 320 px screen.
- **Accessibility, Android:** `AccessibilityTest.kt` checks that everything tappable has a name and a 48 dp touch area, on the main screens and sheets, and at 200% font (screenshots 37 to 40). Google's Accessibility Test Framework doesn't run under Robolectric (it passes a broken button), so the test reads the semantics tree itself and a canary proves it catches problems. Fixes: sheet backdrops are no longer a nameless full-screen button, the device name field has a label, and at large font the code buttons, the device name field, Who can send and the Send to rows stack instead of cutting text off.
- **Release:** `-Pnhako.fdroid` builds Android without Firebase (Push.kt moved to `src/push/`, a stub in `src/nopush/`, its manifest entry with it), unsigned and without the update check; the Build workflow builds it and fails if Firebase code appears. The workflow also makes an `.aab` (run artifact), signs the Windows installer with SignPath once the owner sets it up, and the Release workflow writes winget manifests (`scripts/winget.mjs`). Flatpak: `packaging/flatpak/` (manifest on GNOME 51, offline sources, `build-local.sh`), `packaging/linux/` (AppStream, desktop entry), `packaging/screenshots/`. F-Droid: `fastlane/metadata/android/` and a draft recipe in `packaging/fdroid/`. In a Flatpak the file manager menu and start at login are hidden. Steps only the owner can do are in [releasing.md](releasing.md), "Stores and signing".
- **Copy:** 34 new keys and 3 changed (ux-copy.md section 17, with the web page's words). **Screens:** 3.12, 8.5, 8.6. **Motion:** 4.17. **New dependencies:** `crc32fast` (already in the tree), `zip` (tests only), `@axe-core/playwright` (tests only); licenses regenerated.

### Phase 15: verified, and not

- **Verified:**
  - Rust: fmt and clippy clean; 52 engine unit tests, 27 engine and 4 own-server integration tests, 10 desktop tests. New: a browser is let in or turned away, sees nothing before, downloads a file, part of one (Range) and the zip (read back by a real zip reader); a link ends by itself; finding a newer version once a day, kept across a restart, gone when turned off, absent in store builds; ranges, cookies, browser names, zip names and DOS times; the Linux menu entries and their quoting. A one-off Zip64 run past 4 GB (ignored in normal runs) read back in the test, `unzip -t`, Python and 7-Zip.
  - On this computer's real network: `nhako share` served a folder and a file at its Wi-Fi address; the page, letting in, the list, a file and the zip (same bytes, local times) worked from curl and a browser, in light and dark.
  - Desktop: svelte-check 0 errors, 124 unit tests (54 new contrast checks), 142 Playwright flows (8 skipped: 6 as before, and the store screenshot script on both layouts). New: 4 Phase 15 flows and 11 accessibility checks, each on desktop and phone layouts. axe found one problem (the transfer bar had no name) and the 320 px check one (Settings 11 px too wide); both fixed. Screens checked by eye: the link sheet, the banners, and the store screenshots.
  - Android: 66 JVM tests: the real engine sharing a link (or saying it needs Wi-Fi) with no update check in a store build, screenshots 35 to 40 (link sheet, update banner, 200% font), and the accessibility checks with a canary proving they catch a nameless button. The F-Droid build compiled, with no Firebase code or push service in it, where the normal build has both.
  - Flatpak: built from source and offline on GNOME 51 with the generated source lists, installed for this user; inside it all 134 libraries resolve and the tray library, desktop entry and AppStream file are in place. AppStream and the desktop entry pass `appstreamcli validate` and `desktop-file-validate`. The winget script ran on a dummy installer and its YAML parses; the workflow files parse.
- **Not verified:**
  - On real devices: Share via link from a phone, or to a phone's browser; letting a browser in from a notification; Explorer, Nemo and Dolphin menus on folders; TalkBack, NVDA or Orca reading the apps (only automated checks); the update check against GitHub itself (tests use a stand-in).
  - The Flatpak's window was not opened (the .deb was running and both use one single-instance name), so the tray, notifications and file dialogs inside the sandbox are untried.
  - The new workflow steps (F-Droid check, .aab, SignPath, winget) have not run on GitHub yet. SignPath can't run until the owner is accepted.
  - The F-Droid recipe and the Flathub submission are drafts until their reviewers see them.

### Phase 14 in detail

The owner chose all three: folders, text and the clipboard, and nearby trust with the off Wi-Fi screen. Decisions they made: trust needs **both devices to agree**; text from your own devices with "Accept files automatically" goes **straight to the clipboard**, anything else shows a card first; a folder whose name is taken arrives as **`Trip (1)`**, never merged.

- **The protocol** ([nhako-1.md](protocol/nhako-1.md), section 4): the hello extension now carries records (kind and bytes; an unknown kind is skipped), so builds from Phases 8 to 13 still talk to new ones. Folders: record 1, each file's folder. Text: record 2; the offer still lists one `Text.txt`, so an older build saves it as a file. Trust asks: record 3 on a ping, then a commit, reveal and answer, so the check code can't be steered by someone in the middle.
- **The engine:** `send_files` with `Outgoing { path, name, folder }`, `send_text`, `ask_trust` / `answer_trust` / `cancel_trust` (`core/src/trust.rs`), `files::walk_folder`, `safe_folder`, `reserve_folder`, `prune_empty`. Received text is never written to disk. The contract gained `FileItem.folder`, `text` on transfers, requests and History, `TrustRequest`, the `trustAsk`, `trustWaiting` and `trustEnded` events, `PairVia::nearby` and `Snapshot.trustAsks`, in all three files.
- **Desktop:** Home has Folder, Text and Clipboard under Choose files; folders can be dropped or picked (native dialog); Ctrl+V on Home starts a text. A device's own sheet opens with a right-click or long press on its orb. Received text copies with the Rust side (`tauri-plugin-clipboard-manager`, a new dependency, free), so it works with the window in the tray, with a notification. Open and Show in folder open the arrived folder.
- **Android:** the same buttons; the system folder picker; text and links shared from other apps (`EXTRA_TEXT`); long press on an orb. Files in a picked folder are opened one at a time as each is sent (`FileOpener` across the FFI), so a big folder never holds thousands of handles. Off Wi-Fi comes from the system's network callback. Notifications for text requests, copied text and trust asks.
- **CLI:** `nhako send FOLDER` and `nhako send --text "..."`; received text is printed.
- **Copy:** 59 new keys (ux-copy.md section 16), three changed. **Screens:** 2.12, 2.13, 3.9 to 3.11, 5.7 to 5.9. **Motion:** 4.16.
- **Worth knowing:** file and folder names that start with a dot lose it on arrival (`.gitignore` arrives as `gitignore`), the rule from before for single files. Empty folders aren't sent. Text is at most 64 KB.

### Phase 14: verified, and not

- **Verified:**
  - Rust: fmt and clippy clean; 43 engine unit tests, 24 engine and 4 own-server integration tests, 9 desktop tests. New: `sends_a_folder_and_keeps_its_shape` (folders, a taken name gets `Trip (1)`, a cancelled folder is cleared away), `text_comes_in_the_request_and_never_touches_the_disk`, `trust_from_nearby_needs_both_to_agree` (the same check on both, yes, no, and a cancelled ask), and unit tests for the records (an old build reads past them, an unknown kind is skipped), folder names, the folder walk and text cleaning.
  - Desktop: svelte-check 0 errors, 70 unit tests, 110 Playwright flows (6 skipped). 11 new flows on both the desktop and phone layouts, run 3 times each: a folder sent and received, writing and sending text, too long text, a link request, auto-copy from your own device, paste on Home, trust from Nearby (right-click, and a real long press on the phone layout), a no and an outdated device, being asked, and off Wi-Fi. Every new screen checked by eye in light, dark and phone width.
  - Android: 54 JVM tests, new: folder and text summaries, the contract's new calls on the real engine, and screenshots 27 to 34 (link request, folder request, Send text, a device's sheet, trust wait, trust ask, paired from Nearby, off Wi-Fi), checked by eye.
- **Not verified:**
  - Nothing on real devices yet: the Android folder picker and its one-file-at-a-time opening, text shared from the phone's browser, the clipboard on Windows and on Linux under Wayland, trust between a real phone and laptop, and the phone on mobile data afterwards.
  - The real desktop app (still no `tauri-driver`): the folder dialog, dropping a folder, the clipboard plugin, and the notifications for copied text and trust asks. They build and their Rust tests pass.
  - Old and new builds side by side for real: covered by the wire format tests, not by running a Phase 13 build against this one.
  - "Send with Nhako Beam" from the file manager on a folder (Explorer and Files register it for files only, as before).

### Phase 13 in detail

- **The spec** is [motion-spec.md](design/motion-spec.md), section 4.15, with every moment and its reduced-motion version. Tokens: `motion.bolt` (spark count, reach, size and time, flash, arc, morph).
- **Shared parts:** `desktop/src/lib/motion/bolt.ts` (shape, morph, arc path, `spark()`, `flash()`, `strike()`) and `components/Bolt.svelte`; `android/.../ui/components/Bolt.kt` (`Bolt`, `drawBolt`, `Modifier.sparks`, `arcPoints`). The geometry is tested to match on both (`bolt.test.ts`, `BoltTest.kt`).
- **The Motion lab** (prototype only): Scenarios > Open the motion lab, or `localhost:5173/#motion`. Every effect with a replay button, and a System / Full / Reduced switch.
- **Small model change:** a toast now carries how long it stays (`ms`), on both apps, so its bar can drain.
- **Not done:** the file stack folding into a bolt on its way along the beam (the drop already shrinks into the orb, and the orb pulses); "each letter pops" in the code input (the browser's text field can't animate single letters; the words lock with a zap instead).

### Phase 13: verified, and not

- **Verified:**
  - Desktop: svelte-check 0 errors, 65 unit tests, 88 Playwright flows (6 skipped), new: arcs, sparks, the riding bolt and the done check; a failed transfer's cracked bolt and History's sleeping bolt; reduced motion has no sparks or arcs but keeps the states; the Motion lab. Frames frozen mid-animation (sparks, strike, arcs, swipe meter, QR draw-in, check code scan) were checked by eye in light and dark.
  - Android: 47 JVM tests, new `BoltTest` and screenshots 25 and 26 (the strike with sparks out, then bolt, check, crack and sleep). Screenshots 06, 07, 10 and 18 now show the badge, the riding bolts, the bolt between paired devices and the sleeping bolt; checked by eye.
  - Rust wasn't touched in this phase.
- **Not verified:**
  - On a real phone and the real desktop app: how the sparks and arcs feel at full speed, and their frame time. The 60 fps budget wasn't measured.
  - The tray popover's drop runs on the window's focus event; not tried in the real Linux or Windows tray.
  - Android haptics weren't changed; they already land on the same beats as the sounds.

## Next (ask the owner)

- **Git (the owner's):** commit and push the two small fixes after `7f6ebe7` (`a11y.spec.ts`, `core/src/firewall.rs`), then the Build workflow should pass on all three.
- **Phase 20, always on like Blip:** next, on the owner's "ready boss".
- **The owner's try of Phase 15:** Share via link from the laptop and from the phone to a device without the app (a browser on the same Wi-Fi): letting it in, one file, Download everything, and the notification with the app hidden. Right-click a folder in Explorer, Files, Nemo or Dolphin. On a phone with TalkBack, and with the system font at its largest.
- **The owner's store steps** ([releasing.md](releasing.md), "Stores and signing"): SignPath application (can take weeks), winget after the first published release, the Flathub and F-Droid submissions. Each is free; Play waits for the owner's decision on its fee.
- **The Flatpak is installed on this computer** from the local build (`flatpak list --user`). It wasn't started, because the .deb was running and both use the same single-instance name. Remove it with `flatpak uninstall --user io.github.kimzam30.nhakobeam`, or try it with the .deb closed.
- **The owner's try of Phase 14 on real devices:** a folder both ways (Android picker, desktop drop), a link shared from the phone's browser, Clipboard, and Trust this device between the phone and a laptop, then the phone on mobile data finding the laptop.
- **The owner's look at the Motion lab** (`npm run dev` in `desktop/`, then `localhost:5173/#motion`) and the apps, then adjust: spark size and count, arc speed and the rest are tokens.
- **The owner's Windows test of Phases 10 to 14**, with Copy diagnostics if something is missing. All devices need the new build (four-word codes), and they'll need pairing again only if they weren't trusted already.

- **The owner's verdict on the sounds**, then adjust recipes in `build.mjs` (each is a few lines).

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
