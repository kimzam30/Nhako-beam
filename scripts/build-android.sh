#!/usr/bin/env bash
# Builds the Android app (.apk) on your own computer, the same way the GitHub workflow does.
#
#   scripts/build-android.sh            tests, then one .apk per kind of phone (arm64, arm32, x86_64)
#   scripts/build-android.sh --fast     arm64 only (most phones), no tests: quicker for trying things
#
# The .apk files end up in out/. Setup steps are in docs/building.md. With NHAKO_WAKE_URL set (see
# wake/README.md), the engine can wake your phones while Nhako Beam is closed.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT=$PWD
NDK_VERSION=29.0.14206865
FAST=0
[[ "${1:-}" == "--fast" ]] && FAST=1

problems=()
need() { command -v "$1" >/dev/null 2>&1 || problems+=("$2"); }

need cargo "Rust is missing. Install it from https://rustup.rs"
need java "Java is missing. Install JDK 17 or newer (Debian: sudo apt install openjdk-21-jdk, or openjdk-17-jdk)"
command -v cargo-ndk >/dev/null 2>&1 || problems+=("cargo-ndk is missing. Run: cargo install cargo-ndk")

SDK=${ANDROID_HOME:-${ANDROID_SDK_ROOT:-$HOME/Android/Sdk}}
if [[ ! -d "$SDK" ]]; then
  problems+=("The Android SDK wasn't found. Set ANDROID_HOME to its folder (see docs/building.md)")
elif [[ ! -d "$SDK/ndk/$NDK_VERSION" ]]; then
  problems+=("The Android NDK $NDK_VERSION is missing. Run: \"$SDK/cmdline-tools/latest/bin/sdkmanager\" \"ndk;$NDK_VERSION\"")
fi

if command -v rustup >/dev/null 2>&1; then
  installed=$(rustup target list --installed)
  for t in aarch64-linux-android armv7-linux-androideabi x86_64-linux-android; do
    grep -qx "$t" <<<"$installed" || problems+=("The Rust target $t is missing. Run: rustup target add $t")
  done
fi

if ((${#problems[@]})); then
  echo "Can't build yet:" >&2
  printf '  - %s\n' "${problems[@]}" >&2
  exit 1
fi

export ANDROID_HOME=$SDK
# Smaller debug info for the host copy of the engine the Kotlin bindings are made from.
export CARGO_PROFILE_DEV_DEBUG=${CARGO_PROFILE_DEV_DEBUG:-line-tables-only}

cd android
if ((FAST)); then
  echo "Building the .apk for arm64 phones (no tests)..."
  ./gradlew assembleRelease -Pnhako.abis=arm64-v8a
else
  echo "Running the tests..."
  ./gradlew testDebugUnitTest
  echo "Building the .apk for every phone..."
  ./gradlew assembleRelease
fi

mkdir -p "$ROOT/out"
rm -f "$ROOT"/out/nhako-beam-{arm64-v8a,armeabi-v7a,x86_64}{,-debug}.apk
echo
echo "Done:"
for apk in app/build/outputs/apk/release/app-*-release.apk; do
  abi=$(basename "$apk" | sed -e 's/^app-//' -e 's/-release\.apk$//')
  cp "$apk" "$ROOT/out/nhako-beam-$abi.apk"
  echo "  out/nhako-beam-$abi.apk ($(( $(stat -c%s "$apk") / 1000000 )) MB)"
done
echo "Most phones need the arm64-v8a one. Not sure? Settings > About phone usually says 'ARM64' or '64-bit'."
[[ -f app/google-services.json ]] || echo "Push wake-up is off in this build (no app/google-services.json; see wake/README.md)."
((FAST)) || echo "Screenshots of every screen: android/app/build/screenshots/"
echo "To install it, see 'Install the .apk on your phone' in docs/building.md."
