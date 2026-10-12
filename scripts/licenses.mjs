// Lists the open source packages each app ships, with their license texts, for About > Open
// source licenses. Run after changing dependencies (Cargo.lock, desktop/package-lock.json,
// android/gradle/libs.versions.toml) and commit the output:
//
//   node scripts/licenses.mjs
//
//  - desktop/src/lib/licenses.json     the desktop app (Rust for Linux and Windows, plus the UI's npm packages)
//  - android/app/src/main/assets/licenses.json   the Android app (Rust for phones, plus the Android libraries)
//
// Each package points at its own license files (LICENSE, COPYING, NOTICE...), so copyright lines
// stay with the right package. Identical texts are stored once.
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';

const root = resolve(import.meta.dirname, '..');
const LICENSE_FILE = /^(licen[sc]e|copying|notice|unlicense|copyright)([-._].*)?$/i;

// ---------- Texts ----------

const texts = new Map();
function addText(text) {
  const clean = text.replace(/\r\n/g, '\n').replace(/[ \t]+$/gm, '').trim();
  const id = createHash('sha1').update(clean).digest('hex').slice(0, 12);
  texts.set(id, clean);
  return id;
}
function filesIn(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir)
    .filter((f) => LICENSE_FILE.test(f))
    .sort()
    .map((f) => readFileSync(join(dir, f), 'utf8'));
}

// Standard texts for packages that only name their license. Taken from packages that ship them.
const standard = {};
function rememberStandard(spdx, text) {
  if (!standard[spdx]) standard[spdx] = text;
}

// ---------- Rust ----------

const cargo = (args) => execFileSync('cargo', args, { cwd: root, encoding: 'utf8', maxBuffer: 256 * 1024 * 1024 });
const metadata = JSON.parse(cargo(['metadata', '--format-version', '1', '--locked']));
const byId = new Map(metadata.packages.map((p) => [`${p.name} ${p.version}`, p]));
const workspace = new Set(metadata.workspace_members.map((id) => metadata.packages.find((p) => p.id === id)?.name));

function rustPackages(pkg, targets) {
  const seen = new Set();
  for (const target of targets) {
    const out = cargo(['tree', '-p', pkg, '-e', 'normal', '--target', target, '--prefix', 'none', '--format', '{p}', '--locked']);
    for (const line of out.split('\n')) {
      const m = line.match(/^(\S+) v(\S+)/);
      if (m && !workspace.has(m[1])) seen.add(`${m[1]} ${m[2]}`);
    }
  }
  return [...seen].map((key) => {
    const p = byId.get(key);
    if (!p) throw new Error(`cargo metadata doesn't know ${key}`);
    const dir = dirname(p.manifest_path);
    const found = filesIn(dir);
    for (const text of found) {
      if (/apache license\s+version 2\.0/i.test(text) && text.length > 9000) rememberStandard('Apache-2.0', text);
      if (/^MIT License|Permission is hereby granted, free of charge/m.test(text)) rememberStandard('MIT-template', text);
    }
    // A package under exactly one license with one file: a good standard text for that license.
    if (p.license && !/ OR | AND |\/|\(/.test(p.license) && found.length === 1) rememberStandard(p.license, found[0]);
    return {
      name: p.name,
      version: p.version,
      license: (p.license ?? p.license_file ?? 'See the license text').replace(/\//g, ' OR '),
      url: p.repository ?? p.homepage ?? `https://crates.io/crates/${p.name}`,
      found,
      authors: p.authors ?? [],
    };
  });
}

// ---------- npm (desktop UI) ----------

function npmPackages() {
  const lock = JSON.parse(readFileSync(join(root, 'desktop/package-lock.json'), 'utf8'));
  return Object.entries(lock.packages)
    .filter(([path, info]) => path && !info.dev)
    .map(([path, info]) => {
      const dir = join(root, 'desktop', path);
      const pkg = JSON.parse(readFileSync(join(dir, 'package.json'), 'utf8'));
      const name = path.replace(/^.*node_modules\//, '');
      const repo = typeof pkg.repository === 'string' ? pkg.repository : pkg.repository?.url;
      return {
        name,
        version: info.version,
        license: pkg.license ?? info.license ?? 'See the license text',
        url: (repo ?? pkg.homepage ?? `https://www.npmjs.com/package/${name}`).replace(/^git\+/, '').replace(/\.git$/, ''),
        found: filesIn(dir),
        authors: [],
      };
    });
}

// ---------- Android libraries ----------

function androidPackages() {
  const toml = readFileSync(join(root, 'android/gradle/libs.versions.toml'), 'utf8');
  const versions = Object.fromEntries([...toml.matchAll(/^(\w+)\s*=\s*"([^"]+)"/gm)].map((m) => [m[1], m[2]]));
  const apache = (name, version, url) => ({ name, version, license: 'Apache-2.0', url, found: [], authors: [] });
  const list = [
    apache('Jetpack Compose (UI, Foundation, Animation)', `BOM ${versions.composeBom}`, 'https://developer.android.com/jetpack/compose'),
    apache('AndroidX Core and Splash Screen', `${versions.coreKtx}, ${versions.coreSplashscreen}`, 'https://developer.android.com/jetpack/androidx/releases/core'),
    apache('AndroidX Activity', versions.activityCompose, 'https://developer.android.com/jetpack/androidx/releases/activity'),
    apache('AndroidX Lifecycle', versions.lifecycle, 'https://developer.android.com/jetpack/androidx/releases/lifecycle'),
    apache('CameraX', versions.camerax, 'https://developer.android.com/jetpack/androidx/releases/camera'),
    apache('ZXing', versions.zxing, 'https://github.com/zxing/zxing'),
    apache('Kotlin standard library', versions.kotlin, 'https://kotlinlang.org'),
    apache('kotlinx.coroutines', versions.coroutines, 'https://github.com/Kotlin/kotlinx.coroutines'),
    apache('kotlinx.serialization', versions.serialization, 'https://github.com/Kotlin/kotlinx.serialization'),
    { name: 'JNA', version: versions.jna, license: 'Apache-2.0 OR LGPL-2.1-or-later', url: 'https://github.com/java-native-access/jna', found: [], authors: [] },
  ];
  if (versions.firebaseBom) {
    list.push(apache('Firebase Cloud Messaging', `BOM ${versions.firebaseBom}`, 'https://firebase.google.com/docs/cloud-messaging'));
  }
  return list;
}

/** Things bundled as files rather than packages: the font and the icons. */
function assets() {
  return [
    {
      name: 'Inter (font)',
      version: '4',
      license: 'OFL-1.1',
      url: 'https://rsms.me/inter/',
      found: [readFileSync(join(root, 'android/third-party/Inter-OFL.txt'), 'utf8')],
      authors: [],
    },
    {
      name: 'Phosphor Icons',
      version: JSON.parse(readFileSync(join(root, 'desktop/node_modules/@phosphor-icons/core/package.json'), 'utf8')).version,
      license: 'MIT',
      url: 'https://phosphoricons.com',
      found: filesIn(join(root, 'desktop/node_modules/@phosphor-icons/core')),
      authors: [],
    },
  ];
}

// ---------- Output ----------

function build(groups) {
  texts.clear();
  const packages = [];
  const seen = new Set();
  for (const [ecosystem, list] of groups) {
    for (const p of list) {
      const key = `${ecosystem}:${p.name}@${p.version}`;
      if (seen.has(key)) continue;
      seen.add(key);
      let found = p.found;
      if (!found.length) {
        // Only the name of the license is given: the standard text, with this package's authors.
        const owner = p.authors.length ? p.authors.join(', ').replace(/ <[^>]*>/g, '') : `the ${p.name} authors`;
        const ids = p.license.split(/ OR | AND |[()]/).map((x) => x.trim()).filter(Boolean);
        const text = ids
          .map((id) => {
            if (id === 'MIT') return `MIT License\n\nCopyright (c) ${owner}\n\n${standardMitBody()}`;
            const std = standard[id];
            return std && std.replace(/^.*copyright \(c\).*$/im, `Copyright (c) ${owner}`);
          })
          .find(Boolean);
        if (!text) throw new Error(`no license text for ${p.name} (${p.license}); add one to scripts/licenses.mjs`);
        found = [text];
      }
      packages.push({
        name: p.name,
        version: p.version,
        license: p.license,
        url: p.url,
        ecosystem,
        texts: found.map(addText),
      });
    }
  }
  packages.sort((a, b) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()));
  const used = new Set(packages.flatMap((p) => p.texts));
  return {
    note: 'GENERATED by scripts/licenses.mjs. Do not edit by hand.',
    packages,
    texts: Object.fromEntries([...texts].filter(([id]) => used.has(id))),
  };
}

function standardMitBody() {
  const mit = standard['MIT-template'] ?? '';
  const at = mit.indexOf('Permission is hereby granted');
  if (at < 0) throw new Error('no MIT text found among the crates');
  return mit.slice(at).trim();
}

function write(path, data) {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, JSON.stringify(data) + '\n');
  const kb = Math.round(JSON.stringify(data).length / 1024);
  console.log(`${path.replace(root + '/', '')}: ${data.packages.length} packages, ${Object.keys(data.texts).length} texts, ${kb} KB`);
}

const desktopRust = rustPackages('nhako-desktop', ['x86_64-unknown-linux-gnu', 'x86_64-pc-windows-msvc']);
const androidRust = rustPackages('nhako-ffi', ['aarch64-linux-android', 'armv7-linux-androideabi', 'x86_64-linux-android']);

write(
  join(root, 'desktop/src/lib/licenses.json'),
  build([
    ['rust', desktopRust],
    ['npm', npmPackages()],
    ['asset', assets()],
  ]),
);
write(
  join(root, 'android/app/src/main/assets/licenses.json'),
  build([
    ['rust', androidRust],
    ['android', androidPackages()],
    ['asset', assets()],
  ]),
);
