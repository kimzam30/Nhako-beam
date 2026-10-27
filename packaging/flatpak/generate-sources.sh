#!/bin/sh
# Makes cargo-sources.json and node-sources.json next to the Flatpak manifest: every crate and npm
# package the build needs, by checksum, so Flathub can build offline. Run it from the repository
# for each release, after the version bump, then copy both files to the Flathub repository.
# Needs Python 3 and an internet connection; it sets up its tools in a temporary folder.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
tools=$(mktemp -d)
trap 'rm -rf "$tools"' EXIT
python3 -m venv "$tools/venv"
# The npm generator from flatpak-builder-tools (the "flatpak-node-generator" on PyPI is an old, different one).
"$tools/venv/bin/pip" install -q aiohttp tomlkit "git+https://github.com/flatpak/flatpak-builder-tools.git#subdirectory=node"
curl -sSfL -o "$tools/flatpak-cargo-generator.py" \
  https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py
"$tools/venv/bin/python" -I "$tools/flatpak-cargo-generator.py" "$root/Cargo.lock" -o "$here/cargo-sources.json"
"$tools/venv/bin/flatpak-node-generator" npm "$root/desktop/package-lock.json" -o "$here/node-sources.json"
echo "Written: $here/cargo-sources.json and $here/node-sources.json"
