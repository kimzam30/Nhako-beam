# Nhako Beam desktop app

Svelte 5 + TypeScript + Vite, inside a Tauri 2 window. The same UI runs two ways:

- **The desktop app** (`src-tauri/`), for Windows and Linux: the real engine from `core/`, native file and folder pickers, files dragged in from the file manager, "Send with Nhako Beam" in Explorer and Files, a tray icon, start at login, and notifications.
- **The browser prototype** (`npm run dev`): a pretend engine with a scenarios panel, for designing and testing every screen without real devices.

The app picks the real engine when it runs inside Tauri, and the pretend one in a browser.

## Run the desktop app (Linux)

You need Rust 1.91 or newer, Node 20 or newer, and the GTK and WebKit libraries Tauri uses:

```bash
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev build-essential file
cd desktop
npm install
npm run tauri dev        # opens the app, reloads when you edit the UI
```

### Build the .deb

```bash
npx tauri build          # target/release/bundle/deb/Nhako Beam_0.1.0_amd64.deb
sudo apt install "../target/release/bundle/deb/Nhako Beam_0.1.0_amd64.deb"
```

It installs `nhako-beam` with a menu entry and icon. Settings and the device key live in `~/.config/io.github.kimzam30.nhakobeam/`, separate from the command line tool's, so both can run on one computer as two devices.

## Windows

The installer is `Nhako Beam_0.1.0_x64-setup.exe`. It installs for your Windows account only, so it doesn't ask for an administrator.

- **Get it:** run the "Build" workflow on GitHub (Actions > Build > Run workflow) and download `nhako-beam-windows-setup` from the run. Or build it on a Windows PC with Rust, Node and `npx tauri build`.
- **It isn't signed yet,** so Windows SmartScreen says "Windows protected your PC". Choose More info, then Run anyway.
- **WebView2** (the browser engine the window uses) comes with Windows 10 and 11. On a PC without it, setup downloads it from Microsoft, so it needs internet then.
- **First start:** Windows Firewall asks whether Nhako Beam may use the network. Allow it on private networks, or devices on your Wi-Fi can't find it.
- **Explorer:** right-click files, then Send with Nhako Beam. On Windows 11 it's under Show more options. The app adds this on its first start; Settings > On this computer turns it off, and uninstalling removes it.
- Settings and the device key live in `%APPDATA%\io.github.kimzam30.nhakobeam\`.

Building the Windows installer on Linux also works for a quick check (`rustup target add x86_64-pc-windows-gnu`, `sudo apt install gcc-mingw-w64-x86-64 nsis`, then `npx tauri build --target x86_64-pc-windows-gnu`), but Tauri calls that experimental: release builds come from the workflow.

## What the desktop app adds

| | |
|---|---|
| Tray icon | Open Nhako Beam, or Quit. On GNOME, the tray needs the AppIndicator extension (Ubuntu has it on by default). |
| Closing the window | Keeps running in the tray so devices can still send files. Settings > On this computer turns this off. The first time, a notification says it is still running. |
| Start when you log in | Settings > On this computer. Starts in the tray with no window (`nhako-beam --hidden`). |
| Notifications | A file request, or files finished arriving, while the window is closed or in the background. |
| Opening the app twice | Brings the open window to the front. |
| Files | Choose with the system dialog, or drag them in from the file manager onto a device. Folders aren't sent yet. |
| Send with Nhako Beam | Right-click files in Explorer (Windows) or Files (Linux, under Scripts). The app opens with them ready and asks which device. On Linux, turn it on in Settings > On this computer. Under the hood: `nhako-beam --send FILE...`, and a running app takes the files over. |
| Received files | Open, or Show in folder, from the finished transfer. |

## Run the browser prototype

```bash
cd desktop
npm install
npm run dev          # http://localhost:5173 (also on your network, so you can open it on a phone)
```

To try it on your Android phone: run `npm run dev` on a computer on the same Wi-Fi, then open the "Network" address Vite prints. Touch, swipes and vibration (haptics) work in Chrome on Android.

### Scenarios panel

The flask button ("Scenarios", or a small tab on the right edge on phones) only exists in the prototype. It triggers every state in `docs/design/screens-and-states.md` without real devices: files to send, incoming requests, connection drops, failures, disk full, codes, offline, your own server turning the token down, more devices, platform switch (Desktop or Android behaviour), and a live log of which haptic would fire.

## Checks

```bash
npm run check      # svelte-check + TypeScript
npm test           # unit tests (physics, formatting, pretend engine)
npm run e2e        # Playwright flows on the prototype, desktop and phone sizes, with video
npm run build      # production build of the UI
npm run tokens     # regenerate design tokens after editing design/tokens/tokens.json
cargo test -p nhako-desktop   # the Tauri side
```

### The real app test

`npm run test:app` drives the built desktop app on Linux through WebDriver, with the `nhako` command line tool as the other device on the same machine. It goes through onboarding, pairs with a code, receives a file after Accept, sends a dropped file, checks Settings, turns start at login on and off, and sends a file with "Send with Nhako Beam" from Files. Each run uses fresh temporary folders. Screenshots go to `test-results/app/`.

```bash
cargo install tauri-driver --locked
sudo apt install webkit2gtk-driver xvfb dbus-x11
npx tauri build --no-bundle && cargo build --release -p nhako-cli
xvfb-run -a dbus-run-session -- npm run test:app
```

Where devices can't find each other on the local network (some containers and VMs), point both at your own server from `relay/`: `NHAKO_TEST_RELAY=http://127.0.0.1:3340 NHAKO_TEST_LOOKUP=http://127.0.0.1:3341 NHAKO_RELAY_TOKEN=...`. That also tests Settings > Connection.

### The Build workflow

`.github/workflows/build.yml` runs on GitHub when you start it (Actions > Build > Run workflow): formatting, lints and every test on Linux and Windows, then the .deb and the Windows installer (and the Android .apk), which you download from the run. It's free on a public repository. To build on your own computer instead, use `scripts/build-linux.sh` or `scripts\build-windows.ps1` ([docs/building.md](../docs/building.md)).

## Where things live

| Path | What |
|---|---|
| `src/lib/engine/types.ts` | **The engine contract.** Implemented by the pretend engine and by the Rust core. |
| `src/lib/engine/tauri.ts` | The real engine: each method is a Tauri command, engine events arrive as `nhako://event`. |
| `src/lib/engine/mock.ts` | The pretend engine, with realistic timings and `sim*` test hooks. |
| `src/lib/shell.ts` | Desktop app only: file and folder pickers, dropped files, opening received files, start at login, keep running. |
| `src-tauri/src/engine.rs` | The engine bridge: starts the engine, one command per contract method, forwards events, maps file ids to paths. |
| `src-tauri/src/shell.rs` | Tray icon, close to tray, quitting cleanly, notifications, files handed over by `--send`. |
| `src-tauri/src/filemenu.rs` | "Send with Nhako Beam": the Explorer registry entry, the Files script, reading `--send`. |
| `src-tauri/tauri.conf.json` | Window and security policy. `tauri.linux.conf.json` and `tauri.windows.conf.json` hold the .deb and installer settings. |
| `src-tauri/windows/hooks.nsh` | Uninstall clean-up on Windows (the Explorer entry and start at login). |
| `src-tauri/icons/icon.svg` | The app icon source. Regenerate the sizes with `npx tauri icon src-tauri/icons/app-icon.png -o src-tauri/icons`. |
| `src/lib/motion/physics.ts` | Spring, rubber band, magnet and swipe math (pure, unit tested, portable to Kotlin). |
| `src/lib/motion/springs.ts` | Interruptible, velocity-preserving springs on top of Motion. |
| `src/lib/motion/transitions.ts` | Svelte transitions driven by the token springs, reduced-motion aware. |
| `src/lib/copy.ts` | Every UI string (mirrors `docs/design/ux-copy.md`). |
| `src/lib/engine/server.ts` | Your own server: address and token rules, the same as `core/src/server.rs`. The pretend server: a host with "offline" in it (or port 9) doesn't answer; no token or the token "wrong" is turned down. |
| `src/lib/engine/words.ts` | Code word helpers. The list itself is `core/src/words.txt`, shared with the Rust core. |
| `src/components/` | Orb, radar, beam, swipe card, sheets, code display and input, etc. |
| `src/screens/` | Onboarding, Home, Settings. |
| `tests/e2e/` | Playwright flows on the prototype. |
| `tests/app/` | The real app test. |
