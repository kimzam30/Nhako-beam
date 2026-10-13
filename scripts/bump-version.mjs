#!/usr/bin/env node
// The app's version lives in six files. This keeps them in step (docs/releasing.md).
//
//   node scripts/bump-version.mjs              show the version in each file
//   node scripts/bump-version.mjs 0.2.0        set 0.2.0 everywhere, and raise Android's versionCode by 1
//   node scripts/bump-version.mjs --check 0.2.0   fail unless every file says 0.2.0 (the Release workflow)
//
// Versions are plain Semantic Versioning numbers, like 1.4.2. No "-beta" endings: Debian sorts them
// oddly and Windows installers want numbers only. While the version starts with 0, every release
// is marked as a pre-release on GitHub anyway.
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const SEMVER = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)$/;

/** The crates in this workspace, whose entries in Cargo.lock carry the version too. */
const crates = ["core", "cli", "ffi", "desktop/src-tauri"].map(
  (dir) => read(`${dir}/Cargo.toml`).match(/^name = "([^"]+)"/m)[1],
);

/** A "version" key at the top level of a JSON file (two spaces in, as npm and Tauri write them). */
const TOP_LEVEL_JSON = /^( {2}"version": ")([^"]+)(")/m;

/**
 * Each place a version is written. `pattern` captures what comes before the version (group 1),
 * the version (group 2) and what comes after (group 3).
 */
const places = [
  { file: "Cargo.toml", pattern: /(\[workspace\.package\][^[]*?\nversion = ")([^"]+)(")/ },
  ...crates.map((name) => ({
    file: "Cargo.lock",
    label: `Cargo.lock (${name})`,
    pattern: new RegExp(`(\\nname = "${name}"\\nversion = ")([^"]+)(")`),
  })),
  { file: "desktop/package.json", pattern: TOP_LEVEL_JSON },
  // The lock file names the version twice: at the top, and for the package itself under "packages".
  { file: "desktop/package-lock.json", pattern: TOP_LEVEL_JSON },
  {
    file: "desktop/package-lock.json",
    label: 'desktop/package-lock.json (packages."")',
    pattern: /("packages": \{\s*"": \{\s*"name": "[^"]+",\s*"version": ")([^"]+)(")/,
  },
  { file: "desktop/src-tauri/tauri.conf.json", pattern: TOP_LEVEL_JSON },
  { file: "android/app/build.gradle.kts", pattern: /(\n\s*versionName = ")([^"]+)(")/ },
];

const VERSION_CODE = /(\n\s*versionCode = )(\d+)/;

function read(file) {
  return readFileSync(join(root, file), "utf8");
}

function find(place, text = read(place.file)) {
  const match = text.match(place.pattern);
  if (!match) throw new Error(`Couldn't find the version in ${place.label ?? place.file}. Has its layout changed?`);
  return match[2];
}

function versionCode() {
  const match = read("android/app/build.gradle.kts").match(VERSION_CODE);
  if (!match) throw new Error("Couldn't find versionCode in android/app/build.gradle.kts.");
  return Number(match[2]);
}

function show() {
  for (const place of places) console.log(`${find(place).padEnd(16)} ${place.label ?? place.file}`);
  console.log(`${String(versionCode()).padEnd(16)} android/app/build.gradle.kts (versionCode)`);
}

function check(wanted) {
  const wrong = places.filter((place) => find(place) !== wanted);
  if (wrong.length) {
    console.error(`These files don't say ${wanted}:`);
    for (const place of wrong) console.error(`  - ${place.label ?? place.file}: ${find(place)}`);
    console.error(`Run: node scripts/bump-version.mjs ${wanted}, then merge that to main and tag again.`);
    process.exit(1);
  }
  console.log(`Every file says ${wanted}.`);
}

function bump(next) {
  const current = find(places[0]);
  const texts = new Map();
  for (const place of places) {
    const text = texts.get(place.file) ?? read(place.file);
    find(place, text);
    texts.set(place.file, text.replace(place.pattern, `$1${next}$3`));
  }
  if (current !== next) {
    const gradle = "android/app/build.gradle.kts";
    texts.set(gradle, texts.get(gradle).replace(VERSION_CODE, (_, before, code) => `${before}${Number(code) + 1}`));
  }
  for (const [file, text] of texts) writeFileSync(join(root, file), text);

  console.log(`${current} -> ${next}`);
  show();
  if (current === next) console.log("Same version as before, so versionCode stays where it was.");
}

const args = process.argv.slice(2);
try {
  if (args.length === 0) {
    show();
  } else if (args[0] === "--check" && args.length === 2 && SEMVER.test(args[1])) {
    check(args[1]);
  } else if (args.length === 1 && SEMVER.test(args[0])) {
    bump(args[0]);
  } else {
    console.error("Usage: node scripts/bump-version.mjs [--check] <version>, for example 0.2.0");
    process.exit(1);
  }
} catch (error) {
  console.error(error.message);
  process.exit(1);
}
