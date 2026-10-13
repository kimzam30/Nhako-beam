Nhako Beam sends files straight between your own devices: no cloud, no size limit, no account.

## Which file do I need?

| Your device | Download |
|---|---|
| Windows 10 or 11 | `nhako-beam-{{VERSION}}-windows-x64-setup.exe` |
| Ubuntu, Debian, Zorin, Mint | `nhako-beam-{{VERSION}}-linux-amd64.deb` |
| Almost every Android phone | `nhako-beam-{{VERSION}}-android-arm64-v8a.apk` |
| Older or cheaper Android phones | `nhako-beam-{{VERSION}}-android-armeabi-v7a.apk` |
| Android emulators, some Chromebooks | `nhako-beam-{{VERSION}}-android-x86_64.apk` |

## Installing

- **Windows:** the installer isn't signed yet, so Windows may say "Windows protected your PC". Choose More info, then Run anyway.
- **Linux:** `sudo apt install ./nhako-beam-{{VERSION}}-linux-amd64.deb`
- **Android:** open the .apk on your phone and allow installs from that app when asked. If Google Play Protect warns about an app it hasn't seen, choose Install anyway. If Android says "App not installed", you have an older build signed by a different computer: uninstall it first, then pair your devices again.

## Checking your download

`SHA256SUMS.txt` lists a checksum for every file. On Linux, in the folder with your downloads:

```
sha256sum --check --ignore-missing SHA256SUMS.txt
```

Every Android build is signed with the same certificate. Its SHA-256 fingerprint:

```
{{APK_CERT_SHA256}}
```
