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

## Not done yet

- **Signing the Windows installer**, so SmartScreen stops warning. [SignPath Foundation](https://signpath.org) does it for free for open-source projects, after an application. Certificates you buy cost money every year.
- **Google Play.** The release key above can be the "upload key" for Play App Signing. A Play developer account has a one-time fee, so ask first.
- **More Linux formats** (AppImage, .rpm): add them to `bundle.targets` in `desktop/src-tauri/tauri.linux.conf.json` and to the file list in `release.yml`.
- **Protecting `v*` tags** so only you can create them: Settings > Rules > Rulesets > New tag ruleset.
