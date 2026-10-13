#!/bin/sh
# Builds the Flatpak from this checkout (not a release tag) and installs it for you, to try before
# a release: flatpak run io.github.kimzam30.nhakobeam
# Needs flatpak-builder (flatpak install --user flathub org.flatpak.Builder) and the sources from
# generate-sources.sh. Flathub's shared modules are fetched into a temporary folder.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
# The builder runs sandboxed and sees your home folder, not /tmp.
work=${NHAKO_FLATPAK_WORK:-${XDG_CACHE_HOME:-$HOME/.cache}/nhako-beam-flatpak}
mkdir -p "$work"
[ -f "$here/cargo-sources.json" ] || "$here/generate-sources.sh"
[ -d "$work/shared-modules" ] || git clone -q --depth 1 https://github.com/flathub/shared-modules.git "$work/shared-modules"
cp "$here/cargo-sources.json" "$here/node-sources.json" "$work/"
# The same manifest, building this folder instead of the release tag.
# Build output, installed packages and the Android app stay out of the copy.
sed -e "s#^      - type: git#      - type: dir#" \
  -e "s#^        url: https://github.com/kimzam30/Nhako-beam.git#        path: $root\\n        skip: [target, .git, desktop/node_modules, desktop/dist, desktop/test-results, android, docs/brand]#" \
  -e "/^        tag: v/d" \
  "$here/io.github.kimzam30.nhakobeam.yml" > "$work/io.github.kimzam30.nhakobeam.yml"
cd "$work"
flatpak run org.flatpak.Builder --user --install --force-clean --install-deps-from=flathub build io.github.kimzam30.nhakobeam.yml
echo "Installed. Run: flatpak run io.github.kimzam30.nhakobeam"
