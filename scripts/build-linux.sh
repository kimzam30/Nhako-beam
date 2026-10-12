#!/usr/bin/env bash
# Builds the Linux desktop app (.deb) on your own Debian or Ubuntu computer.
#
#   scripts/build-linux.sh           the .deb
#   scripts/build-linux.sh --check   first the same checks as the GitHub workflow (format, lints, tests)
#
# The .deb ends up in out/. Setup steps are in docs/building.md.
set -euo pipefail

cd "$(dirname "$0")/.."
ROOT=$PWD
CHECK=0
[[ "${1:-}" == "--check" ]] && CHECK=1

problems=()
command -v cargo >/dev/null 2>&1 || problems+=("Rust is missing. Install it from https://rustup.rs")
command -v npm >/dev/null 2>&1 || problems+=("Node is missing. Install Node 20 or newer (https://nodejs.org)")
if command -v pkg-config >/dev/null 2>&1; then
  pkg-config --exists webkit2gtk-4.1 || problems+=("The WebKit libraries are missing. Run: sudo apt install libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev libasound2-dev libssl-dev build-essential file")
  pkg-config --exists alsa || problems+=("The sound library is missing. Run: sudo apt install libasound2-dev")
else
  problems+=("pkg-config is missing. Run: sudo apt install pkg-config")
fi
if ((${#problems[@]})); then
  echo "Can't build yet:" >&2
  printf '  - %s\n' "${problems[@]}" >&2
  exit 1
fi

cd desktop
echo "Installing the UI's packages..."
npm ci
# The app embeds the built UI, so it's built before any Rust step.
npm run build

if ((CHECK)); then
  echo "Running the checks..."
  (cd "$ROOT" && cargo fmt --all -- --check)
  (cd "$ROOT" && CARGO_PROFILE_DEV_DEBUG=line-tables-only cargo clippy --workspace --all-targets -- -D warnings)
  (cd "$ROOT" && CARGO_PROFILE_DEV_DEBUG=line-tables-only cargo test --workspace)
  npm run check
  npm test
fi

echo "Building the .deb..."
npx tauri build

mkdir -p "$ROOT/out"
cp "$ROOT"/target/release/bundle/deb/*.deb "$ROOT/out/"
echo
echo "Done: out/$(basename "$ROOT"/target/release/bundle/deb/*.deb)"
echo "Install it with: sudo apt install --reinstall ./out/<that file>"
