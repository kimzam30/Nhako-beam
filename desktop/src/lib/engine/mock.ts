// A pretend engine for the prototype. It behaves like the real one will (same events, same
// timings within reason) so the UI can be built and tested before the Rust core exists.
// Everything prefixed `sim` is a test hook used by the scenario panel and Playwright.
import type {
  CheckResult,
  Connection,
  Device,
  DiscoveryState,
  Engine,
  EngineEvent,
  FileItem,
  HistoryEntry,
  IncomingRequest,
  PairingCode,
  RedeemResult,
  Route,
  SelfInfo,
  ServerCheck,
  ServerProblem,
  Transfer,
  TransferError,
} from './types';
import { isWord, randomWords } from './words';
import { parseLookup, parseRelay, parseToken } from './server';

export type SendOutcome = 'accept' | 'decline' | 'ignore' | 'unreachable';
export type SampleKind = 'photos' | 'video' | 'longname' | 'mixed' | 'big';

const TICK_MS = 100;
const CODE_LIFETIME_MS = 10 * 60 * 1000;
const ROUTE_SPEED: Record<Route, number> = { nearby: 92e6, direct: 31e6, relayed: 11e6 };
const CONNECTION_STORE = 'nhako.mock.connection';
const LAN_STORE = 'nhako.mock.lan';
const ENDED = new Set(['done', 'failed', 'cancelled', 'declined']);
const HOUR = 3_600_000;
/** How long a pretend server check takes. */
export const CHECK_MS = 900;
const ACTIVE = new Set(['waiting', 'connecting', 'transferring', 'paused', 'reconnecting']);

/**
 * Pretend server rules, so every result can be shown and tested:
 * a host containing "offline" or port 9 doesn't answer; a missing token or "wrong" is turned down.
 */
function mockCheckOne(address: string, token: string | null | undefined, kind: 'relay' | 'lookup'): CheckResult {
  const parsed = kind === 'relay' ? parseRelay(address) : parseLookup(address);
  if (!parsed.ok) return { status: 'invalid', reason: parsed.problem };
  const url = new URL(parsed.value);
  if (url.hostname.includes('offline') || url.port === '9') {
    return { status: 'unreachable', reason: 'nothing answered at that address. Is the server running, and is the port right?' };
  }
  if (kind === 'relay' && (!token || token === 'wrong')) return { status: 'tokenRejected' };
  return { status: 'ok', ms: kind === 'relay' ? 14 : 6 };
}

function loadLan(): boolean {
  try {
    const raw = localStorage.getItem(LAN_STORE);
    if (raw) return JSON.parse(raw) === true;
  } catch {
    // Blocked storage or no window (tests): on.
  }
  return true;
}

function loadConnection(): Connection {
  try {
    const raw = localStorage.getItem(CONNECTION_STORE);
    if (raw) return JSON.parse(raw) as Connection;
  } catch {
    // Blocked storage or no window (tests): start on the public servers.
  }
  return { mode: 'public' };
}

interface Seed extends Device {
  appearMs: number;
}

const SEED: Seed[] = [
  { id: 'd-aisyah', name: "Aisyah's laptop", kind: 'laptop', os: 'windows', trusted: true, autoAccept: false, reachable: true, route: 'nearby', fingerprint: '4F2A 91C3', wakeable: false, appearMs: 700 },
  { id: 'd-phone', name: 'Work phone', kind: 'phone', os: 'android', trusted: true, autoAccept: true, reachable: true, route: 'nearby', fingerprint: 'B7E0 22D9', wakeable: false, appearMs: 1300 },
  { id: 'd-studio', name: 'Studio PC', kind: 'desktop', os: 'linux', trusted: false, autoAccept: false, reachable: true, route: 'nearby', fingerprint: '0C5D E81A', wakeable: false, appearMs: 2100 },
  { id: 'd-tablet', name: "Mei Ling's tablet", kind: 'tablet', os: 'android', trusted: false, autoAccept: false, reachable: true, route: 'nearby', fingerprint: '6A19 C4F0', wakeable: false, appearMs: 2900 },
  { id: 'd-office', name: 'Office laptop', kind: 'laptop', os: 'windows', trusted: true, autoAccept: false, reachable: false, route: null, fingerprint: 'D3B8 0E57', wakeable: false, appearMs: 400 },
];

const EXTRA: Omit<Device, 'id'>[] = [
  { name: "Ravi's desktop", kind: 'desktop', os: 'windows', trusted: false, autoAccept: false, reachable: true, route: 'nearby', fingerprint: '91AE 3B70', wakeable: false },
  { name: 'Living room tablet', kind: 'tablet', os: 'android', trusted: false, autoAccept: false, reachable: true, route: 'nearby', fingerprint: '27CC F104', wakeable: false },
  { name: "Daniel's phone", kind: 'phone', os: 'android', trusted: false, autoAccept: false, reachable: true, route: 'nearby', fingerprint: 'E05B 6D92', wakeable: false },
  { name: 'Lab laptop', kind: 'laptop', os: 'linux', trusted: false, autoAccept: false, reachable: true, route: 'nearby', fingerprint: '5F81 A2C6', wakeable: false },
];

const PAIR_DEVICE: Omit<Device, 'id'> = { name: "Nadia's laptop", kind: 'laptop', os: 'linux', trusted: false, autoAccept: false, reachable: true, route: 'direct', fingerprint: '8C4E 17B2', wakeable: false };
/** What both screens show after pairing with a code (the real engine derives it from both device keys). */
const PAIR_CHECK = '5D19 E04A';

/** A few earlier transfers, so History has something to show in the prototype. */
function seedHistory(now: number): HistoryEntry[] {
  const at = (hoursAgo: number) => now - hoursAgo * HOUR;
  const entry = (e: Partial<HistoryEntry> & Pick<HistoryEntry, 'id' | 'direction' | 'deviceId' | 'deviceName' | 'deviceKind' | 'files'>, hoursAgo: number, seconds: number): HistoryEntry => {
    const total = e.files.reduce((n, x) => n + x.size, 0);
    const done = e.doneBytes ?? total;
    return {
      totalBytes: total,
      doneBytes: done,
      route: 'nearby',
      status: 'done',
      error: null,
      cancelledBy: null,
      startedAt: at(hoursAgo) - seconds * 1000,
      finishedAt: at(hoursAgo),
      averageSpeed: seconds > 0 ? done / seconds : 0,
      ...e,
    };
  };
  return [
    entry({ id: 'h1', direction: 'receive', deviceId: 'd-phone', deviceName: 'Work phone', deviceKind: 'phone', files: [f('IMG_1180.jpg', 3_400_000, 'image/jpeg'), f('IMG_1181.jpg', 3_100_000, 'image/jpeg')] }, 2, 1.4),
    entry({ id: 'h2', direction: 'send', deviceId: 'd-aisyah', deviceName: "Aisyah's laptop", deviceKind: 'laptop', files: [f('Quarterly report.pdf', 2_200_000, 'application/pdf')] }, 5, 0.6),
    entry({ id: 'h3', direction: 'send', deviceId: 'd-studio', deviceName: 'Studio PC', deviceKind: 'desktop', files: [f('Raw footage day 2.mov', 6_800_000_000, 'video/quicktime')], status: 'cancelled', cancelledBy: 'peer', doneBytes: 2_100_000_000 }, 27, 92),
    entry({ id: 'h4', direction: 'receive', deviceId: 'd-office', deviceName: 'Office laptop', deviceKind: 'laptop', files: [f('Slides.key', 48_000_000, 'application/octet-stream')], route: 'relayed' }, 30, 5.2),
    entry({ id: 'h5', direction: 'send', deviceId: 'd-tablet', deviceName: "Mei Ling's tablet", deviceKind: 'tablet', files: [f('Recipe.txt', 4_000, 'text/plain')], status: 'declined', doneBytes: 0 }, 75, 0),
  ];
}

let fileSeq = 0;
const f = (name: string, size: number, mime: string): FileItem => ({ id: `f${++fileSeq}`, name, size, mime });

export function sampleFiles(kind: SampleKind): FileItem[] {
  switch (kind) {
    case 'photos':
      return [
        f('IMG_2041.jpg', 4_200_000, 'image/jpeg'),
        f('IMG_2042.jpg', 3_900_000, 'image/jpeg'),
        f('IMG_2047.heic', 2_700_000, 'image/heic'),
        f('Sunset at Teluk Cempedak.jpg', 5_100_000, 'image/jpeg'),
      ];
    case 'video':
      return [f('Beach trip 4K.mov', 1_400_000_000, 'video/quicktime')];
    case 'longname':
      return [f('20260429_203821 family trip to Langkawi, day two.mp4', 790_000_000, 'video/mp4')];
    case 'mixed':
      return [
        f('IMG_3310.jpg', 3_800_000, 'image/jpeg'),
        f('IMG_3311.jpg', 4_100_000, 'image/jpeg'),
        f('IMG_3312.jpg', 3_600_000, 'image/jpeg'),
        f('Wedding highlights final final.mp4', 820_000_000, 'video/mp4'),
      ];
    case 'big':
      return [f('Project backup 2026-10.zip', 48_000_000_000, 'application/zip')];
  }
}

interface Internal extends Transfer {
  /** Sender side: what happens when the other side answers. */
  outcome: SendOutcome;
  answerAt: number | null;
  connectAt: number | null;
  reconnectAt: number | null;
  noise: number;
}

export class MockEngine implements Engine {
  private listeners = new Set<(e: EngineEvent) => void>();
  private devices = new Map<string, Device>();
  private transfers = new Map<string, Internal>();
  private requests = new Map<string, IncomingRequest>();
  private timers: ReturnType<typeof setTimeout>[] = [];
  private tickTimer: ReturnType<typeof setInterval> | null = null;
  private seq = 0;
  private extraIndex = 0;
  private code: PairingCode | null = null;
  private discovery: DiscoveryState = 'idle';
  private connection: Connection = loadConnection();
  private serverProblem: ServerProblem | null = null;
  private saveDir = 'Downloads/Nhako Beam';
  private lan = loadLan();
  private history: HistoryEntry[] | null = null;
  /** The status each transfer's History entry was written with. */
  private recorded = new Map<string, string>();

  /** What the next send to an untrusted device does. */
  nextSendOutcome: SendOutcome = 'accept';
  /** Free space on this device, bytes. */
  freeSpace = 32e9;
  /** The code "the other device" is showing, for Receive with a code. */
  peerCode: [string, string, string] = ['river', 'lantern', 'seven'];
  selfName = 'This device';
  now: () => number = () => Date.now();
  /** Speed multiplier so demos and tests can run faster. */
  speedScale = 1;

  on(listener: (e: EngineEvent) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private emit(e: EngineEvent): void {
    for (const l of this.listeners) l(e);
  }

  private later(ms: number, fn: () => void): void {
    this.timers.push(setTimeout(fn, ms));
  }

  start(): void {
    this.emit({ type: 'freeSpace', bytes: this.freeSpace });
    this.history ??= seedHistory(this.now());
    for (const entry of [...this.history].reverse()) this.emit({ type: 'history', entry: { ...entry } });
    this.scan(this.seeds());
    this.tickTimer = setInterval(() => this.tick(TICK_MS), TICK_MS);
  }

  stop(): void {
    for (const t of this.timers) clearTimeout(t);
    this.timers = [];
    if (this.tickTimer) clearInterval(this.tickTimer);
    this.tickTimer = null;
  }

  private setDiscovery(state: DiscoveryState): void {
    this.discovery = state;
    this.emit({ type: 'discovery', state });
  }

  /** With nearby search off, only trusted devices (reached over the internet) show up. */
  private seeds(list: Seed[] = SEED): Seed[] {
    return this.lan ? list : list.filter((d) => d.trusted).map((d) => ({ ...d, route: d.reachable ? 'direct' : null }));
  }

  private scan(seeds: Seed[]): void {
    this.setDiscovery('searching');
    for (const s of seeds) {
      const { appearMs, ...device } = s;
      this.later(appearMs, () => {
        if (this.discovery === 'offline' || this.discovery === 'blocked') return;
        this.devices.set(device.id, { ...device });
        this.emit({ type: 'device', device: { ...device } });
      });
    }
    const last = Math.max(0, ...seeds.map((s) => s.appearMs));
    this.later(last + 1500, () => {
      if (this.discovery === 'searching') this.setDiscovery('idle');
    });
  }

  rescan(): void {
    const known = [...this.devices.values()].map((d, i) => ({ ...d, appearMs: 300 + i * 250 }));
    for (const d of known) {
      this.devices.delete(d.id);
      this.emit({ type: 'deviceLost', deviceId: d.id });
    }
    this.scan(known.length ? this.seeds(known) : this.seeds());
  }

  getDevice(id: string): Device | undefined {
    return this.devices.get(id);
  }

  // ---------- Sending ----------

  async send(deviceId: string, files: FileItem[]): Promise<string> {
    return this.sendNow(deviceId, files);
  }

  /** `send` without the promise, for tests and the scenario panel. */
  sendNow(deviceId: string, files: FileItem[]): string {
    const device = this.devices.get(deviceId);
    const id = `t${++this.seq}`;
    const now = this.now();
    const totalBytes = files.reduce((n, x) => n + x.size, 0);
    const trustedPeer = !!device?.trusted;
    const t: Internal = {
      id,
      direction: 'send',
      deviceId,
      files,
      totalBytes,
      doneBytes: 0,
      fileBytes: Object.fromEntries(files.map((x) => [x.id, 0])),
      speed: 0,
      route: device?.route ?? 'relayed',
      status: trustedPeer ? 'connecting' : 'waiting',
      pausedBy: null,
      cancelledBy: null,
      error: null,
      startedAt: now,
      movingSince: null,
      finishedAt: null,
      waitingLong: false,
      waking: false,
      outcome: trustedPeer ? 'accept' : this.nextSendOutcome,
      answerAt: trustedPeer ? null : now + 2200,
      connectAt: trustedPeer ? now + 700 : null,
      reconnectAt: null,
      noise: Math.random(),
    };
    this.transfers.set(id, t);
    this.push(t);
    return id;
  }

  private push(t: Internal): void {
    const { outcome: _o, answerAt: _a, connectAt: _c, reconnectAt: _r, noise: _n, ...pub } = t;
    this.emit({ type: 'transfer', transfer: { ...pub, fileBytes: { ...pub.fileBytes } } });
    if (ENDED.has(t.status) && this.recorded.get(t.id) !== t.status) this.record(t);
  }

  /** Like the engine: a transfer gets its History entry when it ends, replaced if it ends again. */
  private record(t: Internal): void {
    this.recorded.set(t.id, t.status);
    const d = this.devices.get(t.deviceId);
    const finishedAt = t.finishedAt ?? this.now();
    const seconds = Math.max(0.001, (finishedAt - t.startedAt) / 1000);
    const entry: HistoryEntry = {
      id: t.id,
      direction: t.direction,
      deviceId: t.deviceId,
      deviceName: d?.name ?? 'Unknown device',
      deviceKind: d?.kind ?? 'laptop',
      files: t.files.map((x) => ({ ...x })),
      totalBytes: t.totalBytes,
      doneBytes: t.doneBytes,
      route: t.route,
      status: t.status,
      error: t.error,
      cancelledBy: t.cancelledBy,
      startedAt: t.startedAt,
      finishedAt,
      averageSpeed: t.doneBytes ? t.doneBytes / seconds : 0,
    };
    this.history = [entry, ...(this.history ?? []).filter((h) => h.id !== t.id)];
    this.emit({ type: 'history', entry: { ...entry } });
  }

  clearHistory(): void {
    this.history = [];
  }

  getHistory(): HistoryEntry[] {
    return [...(this.history ?? [])];
  }

  private find(id: string): Internal | undefined {
    return this.transfers.get(id);
  }

  pause(id: string): void {
    const t = this.find(id);
    if (!t || (t.status !== 'transferring' && t.status !== 'reconnecting')) return;
    t.status = 'paused';
    t.pausedBy = 'me';
    t.movingSince = null;
    this.push(t);
  }

  resume(id: string): void {
    const t = this.find(id);
    if (!t || t.status !== 'paused') return;
    if (t.error === 'diskFull' && this.freeSpace < t.totalBytes - t.doneBytes) return;
    t.status = 'connecting';
    t.pausedBy = null;
    t.error = null;
    t.connectAt = this.now() + 500;
    this.push(t);
  }

  cancel(id: string): void {
    const t = this.find(id);
    if (!t || ['done', 'failed', 'cancelled', 'declined'].includes(t.status)) return;
    t.status = 'cancelled';
    t.cancelledBy = 'me';
    t.speed = 0;
    t.finishedAt = this.now();
    this.push(t);
  }

  retry(id: string): void {
    const t = this.find(id);
    if (!t || (t.status !== 'failed' && t.status !== 'cancelled' && t.status !== 'declined')) return;
    const declined = t.status === 'declined';
    t.error = null;
    t.cancelledBy = null;
    t.finishedAt = null;
    t.waitingLong = false;
    if (declined) {
      t.status = 'waiting';
      t.outcome = this.nextSendOutcome;
      t.answerAt = this.now() + 2200;
    } else {
      t.status = 'connecting';
      t.connectAt = this.now() + 600;
    }
    this.push(t);
  }

  // ---------- Receiving ----------

  async accept(requestId: string, opts: { trust?: boolean; autoAccept?: boolean } = {}): Promise<string | null> {
    return this.acceptNow(requestId, opts);
  }

  acceptNow(requestId: string, opts: { trust?: boolean; autoAccept?: boolean } = {}): string | null {
    const r = this.requests.get(requestId);
    if (!r) return null;
    this.requests.delete(requestId);
    if (opts.trust) this.setTrusted(r.deviceId, true);
    if (opts.autoAccept) this.setAutoAccept(r.deviceId, true);
    return this.startReceive(r.deviceId, r.files);
  }

  decline(requestId: string): void {
    this.requests.delete(requestId);
  }

  private startReceive(deviceId: string, files: FileItem[]): string {
    const device = this.devices.get(deviceId);
    const id = `t${++this.seq}`;
    const now = this.now();
    const t: Internal = {
      id,
      direction: 'receive',
      deviceId,
      files,
      totalBytes: files.reduce((n, x) => n + x.size, 0),
      doneBytes: 0,
      fileBytes: Object.fromEntries(files.map((x) => [x.id, 0])),
      speed: 0,
      route: device?.route ?? 'relayed',
      status: 'connecting',
      pausedBy: null,
      cancelledBy: null,
      error: null,
      startedAt: now,
      movingSince: null,
      finishedAt: null,
      waitingLong: false,
      waking: false,
      outcome: 'accept',
      answerAt: null,
      connectAt: now + 500,
      reconnectAt: null,
      noise: Math.random(),
    };
    this.transfers.set(id, t);
    this.push(t);
    return id;
  }

  // ---------- Codes ----------

  async createCode(): Promise<PairingCode> {
    return this.createCodeNow();
  }

  createCodeNow(): PairingCode {
    const words = randomWords();
    this.code = { words, expiresAt: this.now() + CODE_LIFETIME_MS, uri: `nhakobeam://pair/${words.join('-')}` };
    this.emit({ type: 'code', code: { ...this.code } });
    return { ...this.code };
  }

  cancelCode(): void {
    this.code = null;
    this.emit({ type: 'code', code: null });
  }

  redeemCode(words: string[]): Promise<RedeemResult> {
    return new Promise((resolve) => {
      const bad = words.findIndex((w) => !isWord(w));
      if (bad >= 0) return this.later(250, () => resolve({ ok: false, reason: 'unknownWord', index: bad }));
      const match = words.length === 3 && words.every((w, i) => w.trim().toLowerCase() === this.peerCode[i]);
      this.later(match ? 900 : 700, () => {
        if (this.serverProblem === 'tokenRejected') return resolve({ ok: false, reason: 'tokenRejected' });
        if (!match) return resolve({ ok: false, reason: 'noMatch' });
        const device = this.addPairDevice();
        this.emit({ type: 'paired', device: { ...device }, via: 'entered', check: PAIR_CHECK });
        resolve({ ok: true, device: { ...device } });
      });
    });
  }

  private addPairDevice(): Device {
    const existing = [...this.devices.values()].find((d) => d.fingerprint === PAIR_DEVICE.fingerprint);
    if (existing) return existing;
    const device: Device = { ...PAIR_DEVICE, id: `d-pair-${++this.seq}` };
    this.devices.set(device.id, device);
    this.emit({ type: 'device', device: { ...device } });
    return device;
  }

  // ---------- Devices ----------

  setTrusted(deviceId: string, trusted: boolean): void {
    const d = this.devices.get(deviceId);
    if (!d) return;
    d.trusted = trusted;
    if (!trusted) d.autoAccept = false;
    // Like the engine: an untrusted device that's away leaves the list at once.
    if (!trusted && !d.reachable) {
      this.devices.delete(deviceId);
      this.emit({ type: 'deviceLost', deviceId });
      return;
    }
    this.emit({ type: 'device', device: { ...d } });
  }

  setAutoAccept(deviceId: string, on: boolean): void {
    const d = this.devices.get(deviceId);
    if (!d || !d.trusted) return;
    d.autoAccept = on;
    this.emit({ type: 'device', device: { ...d } });
  }

  renameSelf(name: string): void {
    this.selfName = name;
  }

  async getSelf(): Promise<SelfInfo> {
    return { name: this.selfName, saveDir: this.saveDir, fingerprint: '2E7F A0D4', version: '0.1.0 prototype' };
  }

  async setSaveDir(dir: string): Promise<void> {
    this.saveDir = dir;
  }

  // ---------- Servers ----------

  async getConnection(): Promise<Connection> {
    return this.getConnectionNow();
  }

  getConnectionNow(): Connection {
    return this.connection;
  }

  async setConnection(c: Connection): Promise<void> {
    this.setConnectionNow(c);
  }

  setConnectionNow(c: Connection): void {
    if (c.mode === 'own') {
      const bad = [parseRelay(c.relay), parseToken(c.token ?? ''), c.lookup ? parseLookup(c.lookup) : null].find((r) => r && !r.ok);
      if (bad && !bad.ok) throw new Error(`the server setting can't be used: ${bad.problem}`);
    }
    if ([...this.transfers.values()].some((t) => ACTIVE.has(t.status))) {
      throw new Error('Wait for the transfers in progress to finish (or cancel them), then save again.');
    }
    this.connection = c;
    try {
      localStorage.setItem(CONNECTION_STORE, JSON.stringify(c));
    } catch {
      // Not saved across reloads; fine for the prototype.
    }
    // The real engine restarts its network here; nearby search starts over, and the new
    // server gets checked (a host with "offline" in it never answers, like checkServer).
    const problem: ServerProblem | null = c.mode === 'own' && mockCheckOne(c.relay, null, 'relay').status === 'unreachable' ? 'unreachable' : null;
    this.simServer(problem);
    const before = this.discovery;
    this.setDiscovery('searching');
    this.later(1200, () => this.setDiscovery(before === 'searching' ? 'idle' : before));
  }

  async getLanDiscovery(): Promise<boolean> {
    return this.lan;
  }

  async setLanDiscovery(on: boolean): Promise<void> {
    if ([...this.transfers.values()].some((t) => ACTIVE.has(t.status))) {
      throw new Error('Wait for the transfers in progress to finish (or cancel them), then save again.');
    }
    this.lan = on;
    try {
      localStorage.setItem(LAN_STORE, JSON.stringify(on));
    } catch {
      // Not saved across reloads; fine for the prototype.
    }
    // The real engine restarts here: everything is looked for again, nearby devices only if on.
    for (const d of [...this.devices.values()]) {
      this.devices.delete(d.id);
      this.emit({ type: 'deviceLost', deviceId: d.id });
    }
    this.scan(this.seeds());
  }

  checkServer(c: Extract<Connection, { mode: 'own' }>): Promise<ServerCheck> {
    const token = parseToken(c.token ?? '');
    const relay: CheckResult = token.ok ? mockCheckOne(c.relay, token.value, 'relay') : { status: 'invalid', reason: `token: ${token.problem}` };
    const lookup = c.lookup?.trim() ? mockCheckOne(c.lookup, null, 'lookup') : null;
    return new Promise((resolve) => this.later(CHECK_MS, () => resolve({ relay, lookup })));
  }

  // ---------- Clock ----------

  /** Advances the simulation. Called every 100 ms; tests call it directly. */
  tick(dtMs: number): void {
    const now = this.now();
    for (const t of this.transfers.values()) {
      let changed = false;

      if (t.status === 'waiting' && t.answerAt !== null && now >= t.answerAt) {
        if (t.outcome === 'accept') {
          t.status = 'connecting';
          t.connectAt = now + 600;
          t.answerAt = null;
        } else if (t.outcome === 'decline') {
          t.status = 'declined';
          t.finishedAt = now;
          t.answerAt = null;
        } else if (t.outcome === 'unreachable') {
          t.status = 'failed';
          t.error = 'peerGone';
          t.finishedAt = now;
          t.answerAt = null;
        } else if (now - t.startedAt >= 60_000 && !t.waitingLong) {
          t.waitingLong = true;
        } else {
          continue;
        }
        changed = true;
      }

      if (t.status === 'connecting' && t.connectAt !== null && now >= t.connectAt) {
        t.status = 'transferring';
        t.connectAt = null;
        t.movingSince = now;
        changed = true;
      }

      if (t.status === 'reconnecting' && t.reconnectAt !== null && now >= t.reconnectAt) {
        // Resume from the last confirmed chunk: roll back up to 1.5%.
        const rollback = Math.min(t.doneBytes, t.totalBytes * 0.015);
        this.addBytes(t, -rollback);
        t.status = 'transferring';
        t.reconnectAt = null;
        t.movingSince = now;
        changed = true;
      }

      if (t.status === 'transferring') {
        t.noise += dtMs / 1000;
        const wobble = 0.85 + 0.15 * Math.sin(t.noise * 1.7) + 0.08 * Math.sin(t.noise * 5.3);
        const inst = ROUTE_SPEED[t.route] * wobble * this.speedScale;
        t.speed += (inst - t.speed) * Math.min(1, dtMs / 900);
        let add = t.speed * (dtMs / 1000);
        if (t.direction === 'receive') {
          const room = this.freeSpace - t.doneBytes;
          if (room <= 0) {
            t.status = 'paused';
            t.pausedBy = null;
            t.error = 'diskFull';
            t.speed = 0;
            this.push(t);
            continue;
          }
          add = Math.min(add, room);
        }
        this.addBytes(t, add);
        if (t.doneBytes >= t.totalBytes) {
          t.status = 'done';
          t.speed = 0;
          t.finishedAt = now;
        }
        changed = true;
      } else if (t.speed > 0) {
        t.speed = t.speed < 1000 ? 0 : t.speed * 0.6;
        changed = true;
      }

      if (changed) this.push(t);
    }
  }

  private addBytes(t: Internal, delta: number): void {
    t.doneBytes = Math.max(0, Math.min(t.totalBytes, t.doneBytes + delta));
    let left = t.doneBytes;
    for (const file of t.files) {
      const b = Math.min(file.size, left);
      t.fileBytes[file.id] = b;
      left -= b;
    }
  }

  // ---------- Simulation hooks ----------

  private active(): Internal | undefined {
    return [...this.transfers.values()].reverse().find((t) => ['transferring', 'connecting', 'reconnecting', 'paused', 'waiting'].includes(t.status));
  }

  simIncoming(kind: SampleKind = 'mixed', deviceId = 'd-aisyah'): void {
    const device = this.devices.get(deviceId) ?? [...this.devices.values()].find((d) => d.reachable);
    if (!device) return;
    const files = sampleFiles(kind);
    if (device.trusted && device.autoAccept) {
      this.startReceive(device.id, files);
      return;
    }
    const request: IncomingRequest = {
      id: `r${++this.seq}`,
      deviceId: device.id,
      files,
      totalBytes: files.reduce((n, x) => n + x.size, 0),
      receivedAt: this.now(),
    };
    this.requests.set(request.id, request);
    this.emit({ type: 'incoming', request });
  }

  simWithdraw(): void {
    const last = [...this.requests.values()].pop();
    if (!last) return;
    this.requests.delete(last.id);
    this.emit({ type: 'incomingWithdrawn', requestId: last.id });
  }

  simDropConnection(): void {
    const t = this.active();
    if (!t || t.status !== 'transferring') return;
    t.status = 'reconnecting';
    t.reconnectAt = this.now() + 2600;
    t.movingSince = null;
    this.push(t);
  }

  simSwitchRoute(route: Route): void {
    const t = this.active();
    if (!t) return;
    t.route = route;
    const d = this.devices.get(t.deviceId);
    if (d) {
      d.route = route;
      this.emit({ type: 'device', device: { ...d } });
    }
    this.push(t);
  }

  simFail(error: TransferError = 'network'): void {
    const t = this.active();
    if (!t) return;
    t.status = 'failed';
    t.error = error;
    t.speed = 0;
    t.finishedAt = this.now();
    this.push(t);
  }

  simDiskFull(): void {
    const t = this.active();
    if (!t) return;
    t.status = 'paused';
    t.pausedBy = null;
    t.error = t.direction === 'receive' ? 'diskFull' : 'diskFullPeer';
    t.speed = 0;
    t.movingSince = null;
    this.push(t);
  }

  simPeerPause(): void {
    const t = this.active();
    if (!t || t.status !== 'transferring') return;
    t.status = 'paused';
    t.pausedBy = 'peer';
    t.movingSince = null;
    this.push(t);
  }

  simPeerResume(): void {
    const t = this.active();
    if (!t || t.status !== 'paused' || t.pausedBy !== 'peer') return;
    t.pausedBy = null;
    t.status = 'connecting';
    t.connectAt = this.now() + 400;
    this.push(t);
  }

  simPeerCancel(): void {
    const t = this.active();
    if (!t) return;
    t.status = 'cancelled';
    t.cancelledBy = 'peer';
    t.speed = 0;
    t.finishedAt = this.now();
    this.push(t);
  }

  simWaitLong(): void {
    const t = this.active();
    if (!t || t.status !== 'waiting') return;
    t.outcome = 'ignore';
    t.answerAt = this.now();
    t.startedAt = this.now() - 60_000;
  }

  simPeerRedeemsCode(): void {
    if (!this.code) return;
    const device = this.addPairDevice();
    this.code = null;
    this.emit({ type: 'code', code: null });
    this.emit({ type: 'paired', device: { ...device }, via: 'shown', check: PAIR_CHECK });
  }

  simExpireCodeSoon(seconds = 25): void {
    if (!this.code) return;
    this.code = { ...this.code, expiresAt: this.now() + seconds * 1000 };
    this.emit({ type: 'code', code: { ...this.code } });
  }

  simDiscovery(state: DiscoveryState): void {
    if (state === 'offline' || state === 'blocked') {
      for (const d of this.devices.values()) {
        if (!d.trusted) {
          this.devices.delete(d.id);
          this.emit({ type: 'deviceLost', deviceId: d.id });
        } else if (d.reachable) {
          d.reachable = false;
          d.route = null;
          this.emit({ type: 'device', device: { ...d } });
        }
      }
      this.setDiscovery(state);
      return;
    }
    this.setDiscovery(state);
    if (state === 'searching') this.rescan();
  }

  simAddDevice(): void {
    const base = EXTRA[this.extraIndex % EXTRA.length]!;
    this.extraIndex++;
    const device: Device = { ...base, id: `d-extra-${++this.seq}` };
    this.devices.set(device.id, device);
    this.emit({ type: 'device', device: { ...device } });
  }

  simRemoveDevice(): void {
    const d = [...this.devices.values()].reverse().find((x) => !x.trusted);
    if (!d) return;
    this.devices.delete(d.id);
    this.emit({ type: 'deviceLost', deviceId: d.id });
  }

  simSetFreeSpace(bytes: number): void {
    this.freeSpace = bytes;
    this.emit({ type: 'freeSpace', bytes });
  }

  /** Your own server goes wrong (or recovers). */
  simServer(problem: ServerProblem | null): void {
    if (this.serverProblem === problem) return;
    this.serverProblem = problem;
    this.emit({ type: 'server', problem });
  }
}
