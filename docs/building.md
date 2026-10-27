# Building Nhako Beam

You can build every app two ways, and both give the same files:

- **On your own computer**, with the scripts in `scripts/`. Everything lands in `out/`.
- **On GitHub**, with the Build workflow (Actions > Build > Run workflow). The files are under "Artifacts" on the run's page for 14 days. It's free on a public repository.

| App | On your computer | From the Build workflow |
|---|---|---|
| Android (.apk, one per kind of phone) | `scripts/build-android.sh` (Linux or macOS) | `nhako-beam-android-apk` |
| Linux (.deb) | `scripts/build-linux.sh` (Debian or Ubuntu) | `nhako-beam-linux-deb` |
| Windows (.exe) | `scripts\build-windows.ps1` (Windows) | `nhako-beam-windows-setup` |

All three need **Rust** ([rustup.rs](https://rustup.rs)), because the engine in `core/` is Rust. Each script checks what it needs first and tells you what's missing.

## Android

### One-time setup (Debian or Ubuntu)

1. **Java 17 or newer:** `sudo apt install openjdk-21-jdk` (on Debian 12, use `openjdk-17-jdk`).
2. **The Android SDK command-line tools.** You don't need Android Studio. On [developer.android.com/studio](https://developer.android.com/studio), scroll to "Command line tools only" and download the Linux zip. Then:

   ```bash
   mkdir -p ~/Android/Sdk/cmdline-tools
   unzip commandlinetools-linux-*.zip -d ~/Android/Sdk/cmdline-tools
   mv ~/Android/Sdk/cmdline-tools/cmdline-tools ~/Android/Sdk/cmdline-tools/latest
   export ANDROID_HOME=~/Android/Sdk          # also add this line to ~/.bashrc
   $ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager --licenses
   $ANDROID_HOME/cmdline-tools/latest/bin/sdkmanager "platform-tools" "platforms;android-37.0" "build-tools;37.0.0" "ndk;29.0.14206865"
   ```

3. **Rust for phones:**

   ```bash
   rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
   cargo install cargo-ndk
   ```

All of this is free. The SDK and NDK take about 3 GB of disk.

### Build

```bash
scripts/build-android.sh           # runs the tests, then one .apk for each kind of phone
scripts/build-android.sh --fast    # arm64 only, no tests: fine for almost every phone in use today
```

The first build takes a while (it compiles the engine once per phone type). Later builds only redo what changed. The full build also saves a screenshot of every screen in `android/app/build/screenshots/`.

### Install the .apk on your phone

- **Which file:** almost every phone today needs `out/nhako-beam-arm64-v8a.apk`. Older or
  cheaper phones may need `armeabi-v7a`; `x86_64` is for emulators and a few Chromebooks. If the
  wrong one is installed, Android says the app isn't compatible.
- **Without a cable:** copy the .apk to the phone (Google Drive, Telegram to yourself, a USB stick) and tap it. Android asks once to allow installs from that app; allow it.
- **With a cable:** turn on USB debugging (Settings > About phone > tap Build number 7 times, then Settings > System > Developer options), plug in, and run `$ANDROID_HOME/platform-tools/adb install -r out/nhako-beam-arm64-v8a.apk`.

Things to know:

- It's a **shrunk release build** (R8 removes the code the app never uses, about half the size of a debug build). It's signed with the release key if you've set up `android/keystore.properties` ([releasing.md](releasing.md)), otherwise with your computer's own debug key.
- Before Phase 8 the script made debug builds, whose app id ends in `.debug`. Android treats that as a different app, so uninstall the old "Nhako Beam" if you see two.
- Without the release key, each computer signs with its own debug key, and so does the Build workflow. If Android says **"App not installed"** when you update, the new .apk was signed with a different key: uninstall Nhako Beam first, then install. That also resets the app's device key, so pair your devices again.
- Google Play Protect may warn about an app it hasn't seen. Choose "Install anyway".

### From VS Code

Open the `Nhako-beam` folder in VS Code, then **Terminal > Run Task...** (or Ctrl+Shift+B for the
first one):

- **Build Android APK (arm64, quick):** the .apk for almost every phone, in a few minutes after the
  first build.
- **Build Android APK (all phones, with tests):** the full check, as the GitHub workflow does.
- **Install APK on the phone (USB):** with the phone plugged in and USB debugging on.

The tasks run the same scripts as above (`.vscode/tasks.json`), so the setup steps are the same.
You can also type the commands in VS Code's terminal (Terminal > New Terminal).

### Push wake-up (optional)

So your other devices can wake the phone while Nhako Beam is closed: set up the free wake service
in [wake/README.md](../wake/README.md), put your `google-services.json` in `android/app/`, and set
`NHAKO_WAKE_URL` before building. Without them everything else works the same.

## Linux (.deb)

### One-time setup

```bash
sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev libasound2-dev libssl-dev build-essential file pkg-config
```

You also need Node 20 or newer ([nodejs.org](https://nodejs.org)).

### Build

```bash
scripts/build-linux.sh             # the .deb
scripts/build-linux.sh --check     # first the same checks as the Build workflow: format, lints, every test
sudo apt install --reinstall ./out/*.deb
```

`--reinstall` matters when you rebuild the same version: without it apt says "already the newest version" and keeps the old one. Versions only change with a release ([releasing.md](releasing.md)).

## Windows (.exe)

### One-time setup

1. Rust from [rustup.rs](https://rustup.rs). It offers to install the Visual Studio C++ build tools; say yes.
2. Node 20 or newer from [nodejs.org](https://nodejs.org).

### Build

In PowerShell, from the repository folder:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\build-windows.ps1
```

The installer lands in `out\`. It isn't signed yet, so Windows SmartScreen asks first: More info, then Run anyway.

## The real app test (Linux)

`npm run test:app` in `desktop/` drives the built desktop app through WebDriver, with the `nhako` command line tool as the other device on the same machine. It goes through onboarding, pairs with a code, receives a file after Accept, sends a file, checks Settings, turns start at login on and off, and sends a file with "Send with Nhako Beam" from Files. Each run uses fresh temporary folders, so your own settings are never touched. Screenshots go to `desktop/test-results/app/`.

```bash
cargo install tauri-driver --locked
sudo apt install webkit2gtk-driver xvfb dbus-x11
cd desktop && npm run build && npx tauri build --no-bundle && cargo build --release -p nhako-cli
xvfb-run -a dbus-run-session -- npm run test:app
```

Where devices can't find each other on the local network (some containers and VMs, no multicast), point both at your own server from `relay/`: `NHAKO_TEST_RELAY=http://127.0.0.1:3340 NHAKO_TEST_LOOKUP=http://127.0.0.1:3341 NHAKO_RELAY_TOKEN=...`. That also tests Settings > Connection.

## The Build workflow on GitHub

`.github/workflows/build.yml` runs three jobs when you start it:

- **Linux:** format, lints and every test (engine, command line, desktop app, UI, browser flows), then the .deb.
- **Windows:** lints and tests, then the installer.
- **Android:** the unit and screenshot tests, then the .apk for every phone. The screenshots are uploaded too (`android-screenshots`). With the release key in the repository secrets, the .apk is signed with it; otherwise with the runner's throwaway debug key.

To publish a release (a version tag, the same checks and builds, then a draft release on GitHub), see [releasing.md](releasing.md).
