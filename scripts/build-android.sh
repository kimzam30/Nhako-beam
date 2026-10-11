#!/usr/bin/env bash
# Builds the Android app (.apk) on your own computer, the same way the GitHub workflow does.
#
#   scripts/build-android.sh            tests, then an .apk for every phone (arm64, arm32, x86_64)
#   scripts/build-android.sh --fast     arm64 only (most phones), no tests: quicker for trying things
#
# The .apk ends up in out/. Setup steps are in docs/building.md.
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
  ./gradlew assembleDebug -Pnhako.abis=arm64-v8a
else
  echo "Running the tests..."
  ./gradlew testDebugUnitTest
  echo "Building the .apk for every phone..."
  ./gradlew assembleDebug
fi

mkdir -p "$ROOT/out"
cp app/build/outputs/apk/debug/app-debug.apk "$ROOT/out/nhako-beam-debug.apk"
echo
echo "Done: out/nhako-beam-debug.apk"
((FAST)) || echo "Screenshots of every screen: android/app/build/screenshots/"
echo "To install it, see 'Install the .apk on your phone' in docs/building.md."
