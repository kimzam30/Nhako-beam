// Drives the real desktop app (Tauri window + the Rust engine) with the `nhako` command line tool
// as the other device, both on this machine, finding each other over mDNS like two laptops on the
// same Wi-Fi. Linux only: it talks WebDriver to `tauri-driver`, which runs WebKitWebDriver.
//
//   npm run build && npx tauri build --no-bundle && cargo build --release -p nhako-cli
//   dbus-run-session -- npm run test:app      (inside `xvfb-run -a` on a machine without a screen)
//
// The D-Bus session lets a second launch hand its files to the running app, as on a desktop.
//
// Needs: tauri-driver (`cargo install tauri-driver --locked`) and WebKitWebDriver
// (`webkit2gtk-driver` on Debian and Ubuntu). Every run uses fresh temporary home folders, so it
// never touches your own Nhako Beam settings. Screenshots land in test-results/app/.
import { spawn, execFile } from 'node:child_process';
import { createHash, randomBytes } from 'node:crypto';
import { mkdirSync, mkdtempSync, readFileSync, writeFileSync, existsSync, readdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { setTimeout as sleep } from 'node:timers/promises';

const root = resolve(import.meta.dirname, '../../..');
const APP = process.env.NHAKO_APP ?? join(root, 'target/release/nhako-beam');
const CLI = process.env.NHAKO_CLI ?? join(root, 'target/release/nhako');
const SHOTS = resolve(import.meta.dirname, '../../test-results/app');
const DRIVER = 'http://127.0.0.1:4444';
// Your own server (the Docker setup in relay/), for machines where devices can't see each other
// on the local network: NHAKO_TEST_RELAY=http://127.0.0.1:3340 NHAKO_RELAY_TOKEN=... NHAKO_TEST_LOOKUP=http://127.0.0.1:3341
const OWN = process.env.NHAKO_TEST_RELAY
  ? { relay: process.env.NHAKO_TEST_RELAY, token: process.env.NHAKO_RELAY_TOKEN ?? '', lookup: process.env.NHAKO_TEST_LOOKUP }
  : null;
/** On your own server, the command line skips local discovery too, so both sides really use the server. */
const cliNet = OWN ? ['--no-mdns'] : [];

for (const [what, path] of [['app', APP], ['command line tool', CLI]]) {
  if (!existsSync(path)) throw new Error(`No ${what} at ${path}. Build it first (see the top of this file).`);
}
mkdirSync(SHOTS, { recursive: true });

// ---------- Two devices with their own homes ----------

const work = mkdtempSync(join(tmpdir(), 'nhako-app-test-'));
const appHome = join(work, 'app-home');
const cliConfig = join(work, 'cli-config');
const cliSave = join(work, 'cli-save');
const outbox = join(work, 'outbox');
for (const d of [appHome, cliConfig, cliSave, outbox]) mkdirSync(d, { recursive: true });
/** Where the app saves files: read from the onboarding screen (it depends on the XDG folders). */
let appSave = '';

/** A file of random bytes, and its hash. */
function makeFile(name, size) {
  const path = join(outbox, name);
  writeFileSync(path, randomBytes(size));
  return { path, name, hash: sha(path) };
}
const sha = (p) => createHash('sha256').update(readFileSync(p)).digest('hex');

const hideToken = (text) => (OWN?.token ? text.replaceAll(OWN.token, '[token]') : text);

/** Runs the command line tool as the other device ("Test phone"). */
function cli(args, { timeout = 90_000 } = {}) {
  return new Promise((ok, fail) => {
    execFile(CLI, ['--config-dir', cliConfig, '--save-dir', cliSave, ...args], { timeout }, (err, stdout, stderr) =>
      err ? fail(new Error(`nhako ${hideToken(args.join(' '))} failed:\n${stdout}\n${stderr}`)) : ok(stdout),
    );
  });
}
/** Background command line runs, stopped at the end even if a step fails (a leftover one would
 *  show up as a second "Test phone" in the next run). */
const background = new Set();
/** Starts the command line tool and keeps it running in the background. */
function cliBackground(args) {
  const p = spawn(CLI, ['--config-dir', cliConfig, '--save-dir', cliSave, ...args], { stdio: ['pipe', 'pipe', 'pipe'] });
  background.add(p);
  p.on('exit', () => background.delete(p));
  let out = '';
  p.stdout.on('data', (d) => (out += d));
  p.stderr.on('data', (d) => (out += d));
  const done = new Promise((ok) => p.on('exit', (code) => ok(code)));
  return { proc: p, output: () => out, done };
}

// ---------- A small WebDriver client ----------

let session = '';
async function wd(method, path, body) {
  const res = await fetch(`${DRIVER}${path}`, { method, headers: { 'content-type': 'application/json' }, body: body ? JSON.stringify(body) : undefined });
  const json = await res.json();
  if (!res.ok) throw new Error(`${method} ${path}: ${JSON.stringify(json.value)}`);
  return json.value;
}
const s = (path) => `/session/${session}${path}`;
const exec = (script, ...args) => wd('POST', s('/execute/sync'), { script, args });

async function waitFor(what, fn, timeout = 30_000) {
  const until = Date.now() + timeout;
  let last;
  while (Date.now() < until) {
    try {
      last = await fn();
      if (last) return last;
    } catch (e) {
      last = e;
    }
    await sleep(250);
  }
  await shot(`timeout-${what.replace(/\W+/g, '-')}`);
  throw new Error(`Timed out waiting for ${what} (last: ${last})`);
}
const visible = (testid) => exec(`const e = document.querySelector('[data-testid="${testid}"]'); return !!e && e.getClientRects().length > 0;`);
const text = (testid) => exec(`return document.querySelector('[data-testid="${testid}"]')?.textContent?.trim() ?? null;`);
async function click(testid) {
  await waitFor(`${testid} to show`, () => visible(testid));
  // A toast or a sheet that's still leaving can be in the way for a moment: try again.
  await waitFor(`${testid} to take a click`, async () => {
    const el = await wd('POST', s('/element'), { using: 'css selector', value: `[data-testid="${testid}"]` });
    await wd('POST', s(`/element/${Object.values(el)[0]}/click`), {});
    return true;
  });
}
async function type(testid, value) {
  // Fields can still be opening (height animation); retry until they take input.
  await waitFor(`${testid} to take input`, async () => {
    const el = await wd('POST', s('/element'), { using: 'css selector', value: `[data-testid="${testid}"]` });
    const id = Object.values(el)[0];
    await wd('POST', s(`/element/${id}/clear`), {});
    await wd('POST', s(`/element/${id}/value`), { text: value });
    return true;
  });
}
async function shot(name) {
  try {
    const png = await wd('GET', s('/screenshot'));
    writeFileSync(join(SHOTS, `${name}.png`), Buffer.from(png, 'base64'));
  } catch {
    // A screenshot is a nice-to-have.
  }
}
async function clickCss(css) {
  const el = await waitFor(css, () => wd('POST', s('/element'), { using: 'css selector', value: css }));
  await wd('POST', s(`/element/${Object.values(el)[0]}/click`), {});
}
/** Flips a switch and waits until it shows the new state (screens can still be sliding in). */
async function flip(testid, on) {
  const state = () => exec(`return document.querySelector('[data-testid="${testid}"]')?.getAttribute('aria-checked');`);
  await waitFor(`${testid} to be ${on ? 'on' : 'off'}`, async () => {
    if ((await state()) === String(on)) return true;
    await click(testid);
    await sleep(600);
    return (await state()) === String(on);
  });
}
const toasts = () => exec(`return [...document.querySelectorAll('[data-testid="toast"]')].map((t) => t.textContent.trim());`);

// ---------- The run ----------

const steps = [];
async function step(name, fn) {
  const t0 = Date.now();
  process.stdout.write(`- ${name} ... `);
  await fn();
  const ms = Date.now() - t0;
  steps.push({ name, ms });
  console.log(`ok (${(ms / 1000).toFixed(1)} s)`);
}

const driverEnv = { ...process.env, HOME: appHome, XDG_CONFIG_HOME: join(appHome, '.config'), XDG_DATA_HOME: join(appHome, '.local/share'), RUST_LOG: 'info,iroh=warn' };
const driver = spawn('tauri-driver', [], { env: driverEnv, stdio: ['ignore', 'pipe', 'pipe'] });
const appLog = [];
driver.stdout.on('data', (d) => appLog.push(String(d)));
driver.stderr.on('data', (d) => appLog.push(String(d)));

let failed = false;
try {
  await waitFor('tauri-driver', () => fetch(`${DRIVER}/status`).then((r) => r.ok), 15_000);
  session = (await wd('POST', '/session', { capabilities: { alwaysMatch: { 'tauri:options': { application: APP } } } })).sessionId;
  await cli(['name', 'Test phone']);
  if (OWN) await cli(['server', 'set', OWN.relay, '--token', OWN.token, ...(OWN.lookup ? ['--lookup', OWN.lookup] : [])]);

  await step('onboarding names the device and starts the engine', async () => {
    await waitFor('onboarding', () => visible('onboarding'));
    await shot('01-onboarding');
    await click('onboarding-next');
    await waitFor('name input', () => visible('name-input'));
    await type('name-input', 'Test laptop');
    await click('onboarding-next');
    await waitFor('save step', () => exec(`return document.querySelector('[data-testid="onboarding"]')?.dataset.step === 'save';`));
    appSave = await exec(`return document.querySelector('.location .path')?.getAttribute('title') ?? '';`);
    if (!appSave.startsWith(appHome) || !appSave.endsWith('Nhako Beam')) throw new Error(`save folder is "${appSave}"`);
    await shot('02-save-folder');
    await click('onboarding-next');
    await waitFor('home', () => visible('home'));
    await shot('03-home');
  });

  if (OWN) {
    await step('switches to your own server in Settings > Connection', async () => {
      await click('open-settings');
      await clickCss('[data-testid="conn-mode"] button[role="radio"]:nth-of-type(2)');
      await waitFor('relay field', () => visible('conn-relay'));
      await type('conn-relay', OWN.relay);
      await type('conn-token', OWN.token);
      if (OWN.lookup) await type('conn-lookup', OWN.lookup);
      await click('conn-save');
      await waitFor('"Now using your own server"', async () => (await toasts()).some((t) => t.includes('Now using your own server')), 30_000);
      await waitFor('Save to settle (it is the saved setting now)', () => exec(`return document.querySelector('[data-testid="conn-save"]').disabled;`));
      await shot('04-own-server');
      await click('settings-back');
      await waitFor('home', () => visible('home'));
    });
  }

  await step('pairs with a code shown by the app', async () => {
    await click('send-code');
    // The words, from the label screen readers get: "Code: magpie, firefly, berry".
    const label = await waitFor('code words', () => exec(`return document.querySelector('[data-testid="code-words"]')?.getAttribute('aria-label');`));
    const words = label.replace('Code: ', '').split(', ');
    await shot('05-code');
    const pair = cliBackground([...cliNet, 'pair', ...words]);
    await waitFor('paired sheet', () => visible('sheet-paired'), 60_000);
    const check = await text('pair-check');
    await shot('06-paired');
    const code = await pair.done;
    if (code !== 0) throw new Error(`nhako pair exited ${code}\n${pair.output()}`);
    if (!pair.output().includes(check)) throw new Error(`both sides should show ${check}\n${pair.output()}`);
    await click('paired-continue');
    await waitFor('Test phone on the radar', () => exec(`return [...document.querySelectorAll('[data-testid^="orb-"]')].some((o) => o.textContent.includes('Test phone'));`));
  });

  const incoming = makeFile('holiday.mp4', 24 * 1024 * 1024);
  await step('receives 24 MiB from the command line after Accept', async () => {
    const send = cliBackground([...cliNet, 'send', incoming.path, '--to', 'Test laptop']);
    await waitFor('the request card', () => visible('incoming-card'), 60_000);
    await shot('07-incoming');
    await click('accept');
    const code = await send.done;
    if (code !== 0) throw new Error(`nhako send exited ${code}\n${send.output()}`);
    await waitFor('Open button', () => visible('open'));
    const saved = join(appSave, incoming.name);
    await waitFor('the file on disk', () => existsSync(saved));
    if (sha(saved) !== incoming.hash) throw new Error('received file differs');
    await shot('08-received');
  });

  const outgoing = makeFile('report.pdf', 5 * 1024 * 1024);
  await step('sends a file dropped on the device', async () => {
    const recv = cliBackground([...cliNet, 'receive', '--yes']);
    await sleep(2000);
    // The same events Tauri sends when files are dragged in from the file manager.
    const target = await exec(`
      const orb = [...document.querySelectorAll('[data-testid^="orb-"]')].find((o) => o.textContent.includes('Test phone'));
      const r = orb.getBoundingClientRect();
      return { x: (r.left + r.width / 2) * devicePixelRatio, y: (r.top + r.height / 2) * devicePixelRatio };`);
    const emit = (event, payload) =>
      exec(`return window.__TAURI_INTERNALS__.invoke('plugin:event|emit', { event: arguments[0], payload: arguments[1] }).then(() => true);`, event, payload);
    await emit('tauri://drag-enter', { paths: [outgoing.path], position: target });
    await emit('tauri://drag-over', { position: target });
    await sleep(500);
    await shot('09-drag-over-device');
    await emit('tauri://drag-drop', { paths: [outgoing.path], position: target });
    const got = join(cliSave, outgoing.name);
    await waitFor('the file at the command line', () => existsSync(got) && sha(got) === outgoing.hash, 90_000);
    await sleep(800);
    await shot('10-sent');
    recv.proc.kill('SIGINT');
    await recv.done;
  });

  await step('settings show the real folder, fingerprint and desktop options', async () => {
    await click('open-settings');
    await waitFor('desktop section', () => visible('settings-desktop'));
    const fp = await text('fingerprint');
    if (!/^[0-9A-F]{4} [0-9A-F]{4}$/.test(fp ?? '')) throw new Error(`fingerprint "${fp}"`);
    await shot('10-settings');
  });

  await step('start at login writes the autostart entry with --hidden', async () => {
    await flip('start-at-login', true);
    const dir = join(appHome, '.config', 'autostart');
    const entry = await waitFor('autostart entry', () => existsSync(dir) && readdirSync(dir).find((f) => f.endsWith('.desktop')));
    const body = readFileSync(join(dir, entry), 'utf8');
    if (!body.includes('--hidden')) throw new Error(`autostart entry has no --hidden:\n${body}`);
    await shot('11-start-at-login');
    await flip('start-at-login', false);
    await waitFor('autostart entry removed', () => !existsSync(join(dir, entry)));
  });

  const fromFiles = makeFile('from files menu.zip', 3 * 1024 * 1024);
  await step('"Send with Nhako Beam" in Files hands the file to the running app', async () => {
    await flip('file-menu', true);
    const script = join(appHome, '.local', 'share', 'nautilus', 'scripts', 'Send with Nhako Beam');
    await waitFor('the Files script', () => existsSync(script));
    await shot('12-file-menu-setting');
    await click('settings-back');
    await waitFor('home', () => visible('home'));
    // Coming back from the long Settings page must not leave Home taller than the window.
    await sleep(1200);
    const fit = await exec(`return { page: document.scrollingElement.scrollHeight, window: innerHeight };`);
    if (fit.page > fit.window + 1) throw new Error(`Home is ${fit.page} px tall in a ${fit.window} px window after Settings`);
    const recv = cliBackground([...cliNet, 'receive', '--yes']);
    await sleep(2000);
    // What Files does: run the script in the file's folder with the selected names. It starts a
    // second copy of the app, which hands the file to this one and exits.
    const second = spawn(script, [fromFiles.name], { cwd: outbox, env: driverEnv, stdio: 'ignore' });
    const secondExit = new Promise((ok) => second.on('exit', ok));
    await waitFor('the Send to sheet', () => visible('sheet-send'), 20_000);
    const staged = await exec(`return document.querySelector('[data-testid="staged-tray"]')?.textContent ?? '';`);
    if (!staged.includes('from files menu.zip')) throw new Error(`staged tray shows "${staged}"`);
    await shot('13-send-from-files');
    const rows = await exec(`return [...document.querySelectorAll('[data-testid^="send-row-"]')].filter((r) => r.textContent.includes('Test phone')).map((r) => r.dataset.testid);`);
    if (rows.length !== 1) throw new Error(`Test phone should be listed once in Send to, not ${rows.length} times: ${rows.join(', ')}`);
    await waitFor('the second launch to hand over and exit', () => Promise.race([secondExit.then(() => true), sleep(500).then(() => false)]), 15_000);
    await exec(`[...document.querySelectorAll('[data-testid^="send-row-"]')].find((r) => r.textContent.includes('Test phone')).click(); return true;`);
    const got = join(cliSave, fromFiles.name);
    await waitFor('the file at the command line', () => existsSync(got) && sha(got) === fromFiles.hash, 60_000);
    recv.proc.kill('SIGINT');
    await recv.done;
    await click('open-settings');
    await flip('file-menu', false);
    await waitFor('the Files script removed', () => !existsSync(script));
  });

  await step('no error toasts along the way', async () => {
    const bad = (await toasts()).filter((t) => /couldn't|can't|wrong/i.test(t));
    if (bad.length) throw new Error(`error toasts: ${bad.join(' | ')}`);
  });
} catch (e) {
  failed = true;
  console.log('FAILED');
  console.error(e);
  await shot('failure');
} finally {
  if (session) await wd('DELETE', `/session/${session}`).catch(() => {});
  driver.kill();
  for (const p of background) p.kill('SIGKILL');
  writeFileSync(join(SHOTS, 'app.log'), appLog.join(''));
  const panics = appLog.join('').split('\n').filter((l) => l.includes('panicked'));
  if (panics.length) console.error(`The app crashed:\n${panics.join('\n')}`);
}
console.log(failed ? '\nThe real app test failed. See test-results/app/.' : `\nAll ${steps.length} steps passed. Screenshots: test-results/app/`);
process.exit(failed ? 1 : 0);
