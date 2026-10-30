# Releasing Nhako Beam

A release is one version tag, like `v0.2.0`. Pushing it starts the Release workflow
(`.github/workflows/release.yml`), which:

1. checks the tag matches the version in every file, and that the tag is on `main`,
2. runs every check and build in the Build workflow (format, lints, all tests, then the apps),
3. makes a **draft** release with the files below, plus notes from `.github/release-notes.md` and the list of changes since the last release.

| File | For |
|---|---|
| `nhako-beam-0.2.0-windows-x64-setup.exe` | Windows 10 and 11 |
| `nhako-beam-0.2.0-linux-amd64.deb` | Ubuntu, Debian, Zorin, Mint |
| `nhako-beam-0.2.0-android-arm64-v8a.apk` | almost every Android phone |
| `nhako-beam-0.2.0-android-armeabi-v7a.apk` | older or cheaper phones |
| `nhako-beam-0.2.0-android-x86_64.apk` | emulators, some Chromebooks |
| `SHA256SUMS.txt` | a checksum for each file |

Nothing is public until you press **Publish** on the draft. Everything here is free on a public repository.

## One-time setup: the Android release key

Android only installs an update if it's signed with the same key as the app already on the phone.
So every APK you publish must be signed with one key, kept safe forever. Without it, the Release
workflow stops instead of publishing an APK nobody could update.

### 1. Make the key

On your computer, outside the repository:

```bash
keytool -genkeypair -v -keystore ~/nhako-beam-release.jks -alias nhako-beam -keyalg RSA -keysize 4096 -validity 10000
```

It asks for a password (use a long one and save it in your password manager) and a name; your
own name or "Nhako Beam" is fine. The key is valid for about 27 years.

### 2. Back it up

Copy `nhako-beam-release.jks` to two places you control, for example a USB stick and an encrypted
cloud folder, and keep the password with it. **If you lose the key or its password, you can never
update the Android app again**: everyone would have to uninstall and pair their devices again.
Never commit it; `.gitignore` already skips `*.jks` and `*.keystore`.

### 3. Give it to GitHub

The workflow reads the key from three repository secrets. GitHub encrypts them, and they're
never shown in logs. From the repository folder:

```bash
base64 -w0 ~/nhako-beam-release.jks | gh secret set ANDROID_KEYSTORE_BASE64
```

```bash
gh secret set ANDROID_KEYSTORE_PASSWORD
```

```bash
gh secret set ANDROID_KEY_ALIAS --body nhako-beam
```

The second command asks for the password. You can also add them on GitHub: Settings > Secrets and
variables > Actions > New repository secret.

### 4. Use it on your computer too (optional)

So `scripts/build-android.sh` signs with the same key and your own builds update the published
app, create `android/keystore.properties` (ignored by git):

```properties
storeFile=/home/you/nhako-beam-release.jks
storePassword=your password
keyAlias=nhako-beam
```

Without that file, local builds use your computer's debug key, as before.

### The one time everyone reinstalls

Builds made before the release key were signed with each computer's debug key. Phones with one of
those need to uninstall Nhako Beam once before installing a release, then pair again. After that,
every release installs over the last one.

## Every release

1. **Pick the version.** Plain numbers, `MAJOR.MINOR.PATCH`:
   - `0.2.1`: only fixes.
   - `0.3.0`: new features.
   - `1.0.0`: when you call it finished and stable. Before 1.0, every release is marked "Pre-release".

2. **Set it everywhere** on `dev`, then commit and merge to `main` as usual:

   ```bash
   node scripts/bump-version.mjs 0.2.0
   ```

   This writes the version into `Cargo.toml`, `Cargo.lock`, `desktop/package.json`,
   `desktop/package-lock.json`, `desktop/src-tauri/tauri.conf.json` and
   `android/app/build.gradle.kts`, and raises Android's `versionCode` by one. Without a
   version, it shows what each file says now.

3. **Tag `main` and push the tag:**

   ```bash
   git fetch origin && git tag -a v0.2.0 -m "Nhako Beam 0.2.0" origin/main
   ```

   ```bash
   git push origin v0.2.0
   ```

4. **Wait for the workflow:** GitHub > Actions > Release, or `gh run watch`. About an hour, mostly the Windows and Android builds.

5. **Check the draft:** GitHub > Releases. Download one file and check it if you like
   (`sha256sum --check --ignore-missing SHA256SUMS.txt`). Add a few lines on what's new at the
   top of the notes, then press **Publish release**.

## Nightly builds

Every push to `dev` that changes more than docs starts the Nightly workflow
(`.github/workflows/nightly.yml`). It runs the same checks and builds as the Build workflow, then
replaces one pre-release, tagged `latest` and titled "Nhako Beam nightly", with the new files. You
can also start it by hand: GitHub > Actions > Nightly > Run workflow.

The files keep the same names every time, so these links always give the newest build:

| File | For |
|---|---|
| [`nhako-beam-nightly-windows-x64-setup.exe`](https://github.com/kimzam30/Nhako-beam/releases/download/latest/nhako-beam-nightly-windows-x64-setup.exe) | Windows 10 and 11 |
| [`nhako-beam-nightly-linux-amd64.deb`](https://github.com/kimzam30/Nhako-beam/releases/download/latest/nhako-beam-nightly-linux-amd64.deb) | Ubuntu, Debian, Zorin, Mint |
| [`nhako-beam-nightly-android-arm64-v8a.apk`](https://github.com/kimzam30/Nhako-beam/releases/download/latest/nhako-beam-nightly-android-arm64-v8a.apk) | almost every Android phone |
| [`nhako-beam-nightly-android-armeabi-v7a.apk`](https://github.com/kimzam30/Nhako-beam/releases/download/latest/nhako-beam-nightly-android-armeabi-v7a.apk) | older or cheaper phones |
| [`nhako-beam-nightly-android-x86_64.apk`](https://github.com/kimzam30/Nhako-beam/releases/download/latest/nhako-beam-nightly-android-x86_64.apk) | emulators, some Chromebooks |
| `SHA256SUMS.txt` | a checksum for each file |

- **Only the newest run counts.** A push while one is running stops the older run.
- **The same Android key** signs nightly and release APKs (the secrets above), so either installs
  over the other without losing your devices.
- **No update prompts:** the apps' update check skips the `latest` tag (`core/src/update.rs`), and
  `--latest=false` keeps GitHub's "Latest" badge on the newest real release.
- **The version number** inside a nightly is whatever `dev` says; it's not bumped.
- **If it fails**, nothing is replaced: the previous nightly stays until a run succeeds. A
  failure in the last step (after the old one is deleted) leaves no nightly until the next run;
  press "Re-run failed jobs".
- **A Build run by hand on `dev`** at the same time shares its cancel group with the nightly's
  build, so one of them stops. Run the nightly by hand instead.
- **Not in it:** the F-Droid variant (F-Droid signs it, so the build here is unsigned and won't
  install) and AppImage (not built yet, see "Not done yet").

## When something goes wrong

- **"These files don't say 0.2.0"**: the version wasn't bumped, or the bump isn't on `main` yet.
  Fix that, then move the tag (step below).
- **"points at a commit that isn't on main"**: you tagged `dev` or a local commit. Tag `origin/main` instead.
- **"The ANDROID_KEYSTORE_BASE64 secret is missing"**: do the one-time setup above.
- **A check or build failed:** fix it on `main`, delete the draft release if one was made, and move the tag.
- **A runner hiccup** (a download timed out): on the run's page, press "Re-run failed jobs". The draft gets the new files.

To move a tag that was never published:

```bash
git tag -d v0.2.0 && git push origin :refs/tags/v0.2.0
```

Then tag again from step 3. **Never move a tag after you publish**: people may already have those
files. Release `0.2.1` instead.

## Stores and signing

Everything below is free. The files are in the repository; the steps marked **you** need your
accounts, so they're yours to do. Do them once, after the first release you're happy with.

### Signing the Windows installer (SignPath)

[SignPath Foundation](https://signpath.org) signs installers of open-source projects for free, so
SmartScreen stops saying "Windows protected your PC". The Build workflow already has the steps; they
stay skipped until SignPath is set up.

1. **You:** apply at signpath.org/apply with the repository address. They check the project is open
   source and builds on GitHub Actions. It can take a few weeks.
2. **You:** once accepted, in SignPath create the project `nhako-beam` with an artifact configuration
   for a single `.exe` (Authenticode), and the signing policies `test-signing` and `release-signing`.
   Link it to this repository as a "trusted build system" (GitHub.com).
3. **You:** in GitHub, Settings > Secrets and variables > Actions: add the secret `SIGNPATH_API_TOKEN`
   (from SignPath, a CI user's token) and the variable `SIGNPATH_ORGANIZATION_ID`.

From then on each build sends the installer to SignPath and gets it back signed: test-signed for
ordinary builds, release-signed for a release (SignPath may ask you to approve each release-signing
request in their website). Only the installer is signed, not the app inside it; SmartScreen judges
the installer you download.

### winget

`winget install Kimzam30.NhakoBeam` on Windows. Each release run makes the three manifest files for
that version (`scripts/winget.mjs`): on the run's page under Artifacts, **winget-manifests**.

1. Publish the release first (the manifests point at its download address).
2. **You:** submit them. Easiest: install `wingetcreate` (`winget install wingetcreate`), then
   `wingetcreate submit --token <a GitHub token> <the unzipped folder>`. It opens a pull request on
   microsoft/winget-pkgs, which their bots check (it downloads and scans the installer). A person
   reviews the first one; later versions usually go through by themselves.

### Flathub

`packaging/flatpak/` has the manifest, built from source and offline as Flathub requires, and
`packaging/linux/` the AppStream file (the store page) and the desktop entry. The screenshots on the
page are `packaging/screenshots/` (made with `STORE_SHOTS=1 npx playwright test store-shots
--project=desktop` in `desktop/`), loaded from the `main` branch.

To try it here: `packaging/flatpak/build-local.sh` builds this checkout and installs it, then
`flatpak run io.github.kimzam30.nhakobeam`.

In the Flatpak, two things are left out because the sandbox can't do them: "Send with Nhako Beam" in
the file manager, and "Start when you log in". The update check is off too: Flathub updates the app.

1. **You:** fork github.com/flathub/flathub and open a pull request to its `new-pr` branch with
   `io.github.kimzam30.nhakobeam.yml`, `cargo-sources.json` and `node-sources.json` (make the last two
   with `packaging/flatpak/generate-sources.sh`), and Flathub's shared modules as a git submodule
   named `shared-modules`. Set the manifest's `tag:` to the release, and add its `commit:`.
2. Their reviewers comment on the pull request; answer there. Once merged, Flathub makes a repository
   `flathub/io.github.kimzam30.nhakobeam` for you, and each later release is a pull request there with
   the new tag and fresh source lists.
3. Verifying the app id (io.github.kimzam30) is done on Flathub's website by adding a file to the
   GitHub Pages of `kimzam30`, or they accept the GitHub account itself; they'll say which.

### F-Droid

F-Droid only takes free software, so it builds its own APK with `-Pnhako.fdroid`: no Firebase (so no
wake-up while closed) and no update check. The Build workflow builds that variant on every run and
fails if Firebase code gets into it. The store text and screenshots are `fastlane/metadata/android/`
in this repository, which F-Droid reads from each release tag.

1. **You:** open a merge request on gitlab.com/fdroid/fdroiddata adding
   `metadata/io.github.kimzam30.nhakobeam.yml`, starting from `packaging/fdroid/` here (set the
   version, versionCode and tag of the release). Their reviewers usually ask for changes; answer
   there. Builds take a few days to appear after merging.
2. Their build is arm64 only to start with (almost every phone). Adding armeabi-v7a later needs a
   second build entry with its own versionCode.
3. F-Droid signs with its own key, so the F-Droid app and the one from GitHub can't update each
   other: someone switching between them uninstalls first.

### Google Play (later)

Each Build run also makes an app bundle (`.aab`), under Artifacts as **android-app-bundle**, signed
with the release key (the "upload key" for Play App Signing). A Play developer account has a one-time
fee, so it waits until you decide. Then: the store listing (reuse `fastlane/metadata/android/`), a
privacy policy page, and the Data safety form (Nhako Beam collects nothing; files go device to
device).

## Not done yet

- **More Linux formats** (AppImage, .rpm): add them to `bundle.targets` in `desktop/src-tauri/tauri.linux.conf.json` and to the file list in `release.yml`.
- **Protecting `v*` tags** so only you can create them: Settings > Rules > Rulesets > New tag ruleset.
- **AppStream in the .deb:** the .deb names its desktop entry after the product (`Nhako Beam.desktop`), so the Flatpak's AppStream file doesn't fit it as it is.
