# Plan: the new mascot logo on Android and Windows

Written 2026-10-19. The new logo is already in the code (commit `95a16b4`, on `main` and pushed). What's left is building fresh apps from it and installing them over the old ones. No code changes are needed, and nothing here costs money.

## Where each icon comes from

| Device | Icon files | State |
|---|---|---|
| Linux | `desktop/src-tauri/icons/*.png` | New logo. Rebuilt and reinstalled on this computer (Phase A) |
| Windows | `desktop/src-tauri/icons/icon.ico` (installer, app, Start menu, tray) | New logo in the code. The installed app is the old build (`out/` .exe from Oct 12) |
| Android | `android/app/src/main/res/drawable/ic_launcher_foreground.xml`, `ic_launcher_monochrome.xml`, background `#3F86F0` in `values/themes.xml` | New logo in the code. The phone has the old Oct 12 APK |
| Android status bar | `drawable/ic_notify.xml` | Unchanged on purpose: Android needs a plain white shape there, so it stays the bolt outline |

## Phase B: Android (built on this computer)

Built here, not on GitHub, so the APK is signed with this computer's debug key, the same key as the APK already on the phone. That means it installs as an update: your pairings, trusted devices and history stay.

1. Build: `scripts/build-android.sh --fast` (arm64 only, about 5 minutes; the Galaxy S24+ is arm64).
2. Check the launcher icon in the Robolectric screenshots or the emulator before it goes to the phone.
3. Install, either way:
   - **No cable:** send `out/nhako-beam-arm64-v8a.apk` to the phone (Nhako Beam itself, Telegram, Drive) and tap it. Choose **Update**.
   - **Cable:** USB debugging on, then `adb install -r out/nhako-beam-arm64-v8a.apk`.
4. If the home screen still shows the old icon: long-press it, remove it from the home screen and add it again from the app drawer. Some launchers (One UI included) cache icons until then, or until a restart.

Avoid the GitHub-built APK for this: it's signed with a throwaway key, so the phone would refuse it ("App not installed") unless you uninstall first, and uninstalling resets the device key and every pairing.

## Phase C: Windows

The .exe can't be built on this Linux computer. Two free ways:

- **A. GitHub Build workflow (recommended, nothing to set up on the PC):** Actions > Build > Run workflow, on `main`. Download `nhako-beam-windows-setup` from the run's Artifacts. The repo is public, so it's free. Windows apps don't have a signing key to match, so installing over the old one keeps your settings and pairings.
- **B. On the Windows PC:** `git pull`, then `powershell -ExecutionPolicy Bypass -File scripts\build-windows.ps1`. The installer lands in `out\`.

Then:

1. Quit Nhako Beam from the tray (right-click > Quit), so the installer can replace it.
2. Run the new `Nhako Beam_0.1.0_x64-setup.exe`. SmartScreen asks first (unsigned): More info, then Run anyway.
3. If the taskbar, Start menu or desktop shortcut still shows the old icon, Windows is using its icon cache. Run `ie4uinit.exe -show` in a terminal, or sign out and back in. Unpin and re-pin a taskbar shortcut if it still looks old.

## What I'll check, and what only you can

- Me: the build logs, the icon files inside each package, the Android screenshot.
- You: how the icon looks on the actual phone launcher and the Windows taskbar, since I can't see those screens.

## Not in this plan

- Bumping the version. Same 0.1.0 is fine for reinstalling on your own devices. For a public release, run `node scripts/bump-version.mjs 0.1.1` first ([releasing.md](releasing.md)).
