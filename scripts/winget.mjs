#!/usr/bin/env node
// The winget manifests for one release (docs/releasing.md, "winget"): what Windows Package Manager
// needs to install Nhako Beam with `winget install Kimzam30.NhakoBeam`.
//
//   node scripts/winget.mjs 0.2.0 path/to/nhako-beam-0.2.0-windows-x64-setup.exe [out-dir]
//
// Writes three files to out-dir (default winget/0.2.0): the version, the installer (the release
// download address and the installer's SHA-256) and the store text. The Release workflow runs it;
// you submit them once the release is published (the address only works after Publish).
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, join } from "node:path";

const [version, installer, outArg] = process.argv.slice(2);
if (!/^\d+\.\d+\.\d+$/.test(version ?? "") || !installer) {
  console.error("Usage: node scripts/winget.mjs <version> <installer.exe> [out-dir]");
  process.exit(1);
}

const id = "Kimzam30.NhakoBeam";
const repo = "https://github.com/kimzam30/Nhako-beam";
const sha256 = createHash("sha256").update(readFileSync(installer)).digest("hex").toUpperCase();
const url = `${repo}/releases/download/v${version}/${basename(installer)}`;
const schema = "1.9.0";
const head = (type) => `# yaml-language-server: $schema=https://aka.ms/winget-manifest.${type}.${schema}.schema.json\n`;

const files = {
  [`${id}.yaml`]: `${head("version")}
PackageIdentifier: ${id}
PackageVersion: ${version}
DefaultLocale: en-US
ManifestType: version
ManifestVersion: ${schema}
`,
  [`${id}.installer.yaml`]: `${head("installer")}
PackageIdentifier: ${id}
PackageVersion: ${version}
InstallerType: nullsoft
Scope: user
InstallModes:
  - interactive
  - silent
UpgradeBehavior: install
ReleaseDate: ${new Date().toISOString().slice(0, 10)}
Installers:
  - Architecture: x64
    InstallerUrl: ${url}
    InstallerSha256: ${sha256}
ManifestType: installer
ManifestVersion: ${schema}
`,
  [`${id}.locale.en-US.yaml`]: `${head("defaultLocale")}
PackageIdentifier: ${id}
PackageVersion: ${version}
PackageLocale: en-US
Publisher: kimzam30
PublisherUrl: https://github.com/kimzam30
PublisherSupportUrl: ${repo}/issues
PackageName: Nhako Beam
PackageUrl: ${repo}
License: MIT OR Apache-2.0
LicenseUrl: ${repo}/blob/main/LICENSE-MIT
ShortDescription: Send files straight between your own devices.
Description: Nhako Beam sends files, folders, text and links straight between your Windows, Linux and Android devices, nearby over Wi-Fi or across the internet, encrypted end to end. No cloud, no account, no size limit.
Moniker: nhako-beam
Tags:
  - file-transfer
  - file-sharing
  - airdrop
  - wifi
  - p2p
ReleaseNotesUrl: ${repo}/releases/tag/v${version}
ManifestType: defaultLocale
ManifestVersion: ${schema}
`,
};

const out = outArg ?? join("winget", version);
mkdirSync(out, { recursive: true });
for (const [name, body] of Object.entries(files)) writeFileSync(join(out, name), body);
console.log(`winget manifests for ${version} in ${out} (installer SHA-256 ${sha256})`);
