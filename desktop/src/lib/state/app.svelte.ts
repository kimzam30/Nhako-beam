// App-wide UI state. Holds what the engine reports plus what the UI is showing.
import { breakpoint } from '../tokens';
import { MockEngine } from '../engine/mock';
import { TauriEngine, isTauri, message } from '../engine/tauri';
import type {
  Connection,
  Device,
  DiscoveryState,
  Engine,
  FileItem,
  HistoryEntry,
  IncomingRequest,
  LinkShare,
  PairingCode,
  PairVia,
  ServerProblem,
  Snapshot,
  Transfer,
  TrustOutcome,
  TrustRequest,
  UpdateInfo,
  WhoCanSend,
} from '../engine/types';
import { TEXT_MAX } from '../engine/types';
import * as shell from '../shell';
import { playHaptic } from '../haptics.svelte';
import { playSound } from '../sounds.svelte';
import { t } from '../copy';
import { isLink, isRunnable, summarizeFiles, utf8Bytes } from '../format';

export type Screen = 'onboarding' | 'home' | 'history' | 'settings' | 'licenses';
export type PlatformMode = 'auto' | 'desktop' | 'android';
export type Layout = 'compact' | 'medium' | 'expanded';

export type Sheet =
  | { kind: 'send' }
  | { kind: 'showCode' }
  | { kind: 'enterCode' }
  | { kind: 'scan' }
  | { kind: 'paired'; deviceId: string; via: PairVia; check: string }
  /** Type or paste text to send; to `deviceId` straight away when it's set. */
  | { kind: 'compose'; deviceId?: string; text?: string }
  /** A device's own sheet (right-click or long press on it): send, trust. */
  | { kind: 'device'; deviceId: string }
  /** This device asked another to trust it; `check` arrives once the other device has the ask. */
  | { kind: 'trustWait'; deviceId: string; check: string | null }
  /** Another device asks to trust this one. */
  | { kind: 'trustAsk'; deviceId: string; check: string }
  /** Text that was sent or received (a transfer, or a History entry). */
  | { kind: 'text'; id: string }
  | { kind: 'transfer'; id: string }
  | { kind: 'confirmRemove'; deviceId: string }
  | { kind: 'historyItem'; id: string }
  | { kind: 'confirmClearHistory' }
  | { kind: 'confirmOpen'; transferId: string; fileName: string; deviceName: string }
  /** The link being shared: its QR, who opened it, Stop. */
  | { kind: 'link' };

export interface Toast {
  id: number;
  text: string;
  tone: 'neutral' | 'success' | 'warning' | 'danger';
  action?: { label: string; run: () => void };
  /** How long it stays; its time-left bar drains over this. */
  ms: number;
}

/** A file stack being dragged (from the OS on desktop, or the staged stack by pointer). */
export interface DragState {
  source: 'os' | 'stack';
  x: number;
  y: number;
  count: number;
  nearId: string | null;
  snappedId: string | null;
}

const STORE = 'nhako.app.v1';
/** The update the Home banner was closed for ("Not now"), so it stays closed for that version. */
const UPDATE_SEEN = 'nhako.update.seen';

function loadStored(): { onboarded: boolean; selfName: string; saveLocation: string } {
  try {
    const raw = localStorage.getItem(STORE);
    if (raw) return { onboarded: false, selfName: '', saveLocation: 'Downloads/Nhako Beam', ...JSON.parse(raw) };
  } catch {
    // Blocked storage: start fresh.
  }
  return { onboarded: false, selfName: '', saveLocation: 'Downloads/Nhako Beam' };
}

const ACTIVE = new Set(['waiting', 'connecting', 'transferring', 'paused', 'reconnecting']);
/** The browser prototype's pretend folders (the desktop app uses the real folder picker). */
const SAVE_OPTIONS = ['Downloads/Nhako Beam', 'Documents/Nhako Beam', 'Desktop'];
/** Default names the onboarding screen fills in; the engine's own default replaces them. */
const PLACEHOLDER_NAMES = new Set(['', 'My laptop', 'My phone']);

/** Shows a home folder as ~, so long paths stay readable. */
export function prettyPath(p: string): string {
  return p.replace(/^\/home\/[^/]+(?=\/|$)/, '~').replace(/^\/root(?=\/|$)/, '~').replace(/^[A-Za-z]:\\Users\\[^\\]+(?=\\|$)/, '~');
}

function newEngine(): Engine {
  return isTauri() ? new TauriEngine() : new MockEngine();
}

class AppState {
  engine: Engine = newEngine();
  /** Inside the desktop app (real engine, native pickers, tray). */
  readonly native = isTauri();

  screen = $state<Screen>('home');
  navDir = $state<1 | -1>(1);
  onboarded = $state(false);
  selfName = $state('');
  saveLocation = $state('Downloads/Nhako Beam');

  platformMode = $state<PlatformMode>('auto');
  coarsePointer = $state(false);
  width = $state(1024);

  devices = $state<Device[]>([]);
  discovery = $state<DiscoveryState>('idle');
  /** Reached the engine at least once (the mascot shows loading before). Android: `started`. */
  engineReady = $state(false);
  transfers = $state<Transfer[]>([]);
  requests = $state<IncomingRequest[]>([]);
  freeSpace = $state(0);
  code = $state<PairingCode | null>(null);
  staged = $state<FileItem[]>([]);
  sheet = $state<Sheet | null>(null);
  toasts = $state<Toast[]>([]);
  drag = $state<DragState | null>(null);
  /** Bumped to make an orb play its bouncy pulse. */
  pulse = $state<Record<string, number>>({});
  /** Transfers the user dismissed (collapsed into history). */
  dismissed = $state<string[]>([]);
  devPanel = $state(false);
  /** Public servers or your own (saved by the engine). */
  connection = $state<Connection>({ mode: 'public' });
  /** Your own server's state while running on it. */
  serverProblem = $state<ServerProblem | null>(null);
  /** This device's fingerprint and the app version (from the engine). */
  fingerprint = $state<string | null>(null);
  version = $state('');
  /** Finished transfers, newest first (kept by the engine across restarts). */
  history = $state<HistoryEntry[]>([]);
  /** Announce this device and look for others on the local network (saved by the engine). */
  lanDiscovery = $state(true);
  /** Android: always ready to receive with no screen open (the phone app's ReceiverService). The
   *  prototype's phone layout shows the switch; nothing runs behind it here. */
  alwaysReady = $state(true);
  /** When something worth a reaction last happened (ms): the mascot shows it for a moment
   *  (lib/stage/mood.ts). Set where the matching sounds play. */
  moments = $state({ found: 0, done: 0, failed: 0 });
  /** Who can send to me (saved by the engine). */
  whoCanSend = $state<WhoCanSend>('yourDevices');
  /** Text waiting to be sent, instead of files (the tray shows it; a device takes it). */
  stagedText = $state<string | null>(null);
  /** On Wi-Fi or a cable, where nearby devices can be found. Android tells; the desktop app
   *  always is (off a network it's offline). The prototype pretends with the scenario panel. */
  onLan = $state(true);
  /** Trust asks from other devices, waiting for an answer here. */
  trustAsks = $state<TrustRequest[]>([]);
  /** Home opens its picker for this (from a device's sheet): files or a folder, then sends them there. */
  pickRequest = $state<{ kind: 'files' | 'folder'; deviceId: string; n: number } | null>(null);
  private pickSeq = 0;
  /** The link being shared (Share via link). */
  linkShare = $state<LinkShare | null>(null);
  /** Browsers already heard asking, so each one rings once. */
  private visitorsHeard = new Set<string>();
  /** A newer version, when the daily check found one. */
  update = $state<UpdateInfo | null>(null);
  /** Check for new versions (saved by the engine); null in builds that get updates from a store. */
  updateCheck = $state<boolean | null>(null);
  /** The version whose Home banner was closed with Not now. */
  updateSeen = $state<string | null>(null);
  /** Windows Firewall keeps nearby devices out (the desktop app on Windows, without its rule). */
  firewallBlocked = $state(false);
  /** Fix it is waiting for Windows' admin prompt. */
  fixingFirewall = $state(false);

  /** The pretend engine's test hooks (browser prototype and UI tests only). */
  get mock(): MockEngine | null {
    return this.engine instanceof MockEngine ? this.engine : null;
  }

  /** The phone layout and its parts (the QR scanner). Never in the desktop app, even on a touch screen. */
  isAndroid = $derived(!this.native && (this.platformMode === 'android' || (this.platformMode === 'auto' && this.coarsePointer)));
  layout = $derived<Layout>(this.width >= breakpoint.expanded ? 'expanded' : this.width >= breakpoint.medium ? 'medium' : 'compact');
  reachable = $derived(this.devices.filter((d) => d.reachable));
  /** Radar order: trusted first, then nearby, offline last. */
  radarDevices = $derived(
    [...this.devices].sort((a, b) => Number(b.reachable) - Number(a.reachable) || Number(b.trusted) - Number(a.trusted) || a.name.localeCompare(b.name)),
  );
  activeTransfers = $derived(this.transfers.filter((x) => ACTIVE.has(x.status)));
  visibleTransfers = $derived(this.transfers.filter((x) => !this.dismissed.includes(x.id)));

  /** Element registry for magnet targets (not reactive on purpose). */
  orbEls = new Map<string, HTMLElement>();
  /** Set by DragLayer so drops can play the land / spring-back animation. */
  dragLayer: { land(id: string): Promise<void>; cancel(to?: { x: number; y: number }): Promise<void> } | null = null;
  private toastSeq = 0;
  /** Devices whose request was accepted on the card, so no extra "Receiving" toast. */
  private acceptedHere = new Set<string>();
  private started = false;
  /** Window listeners and the file manager hookup, set up once for the page's life. */
  private wired = false;
  /** Stops listening to the current engine (Reset onboarding swaps it for a new one). */
  private offEngine: (() => void) | null = null;

  /** `popover`: the tray popover's copy of the app, which leaves files from the file manager to the
   *  main window. */
  init(opts: { popover?: boolean } = {}): void {
    if (this.started) return;
    this.started = true;
    const s = loadStored();
    this.onboarded = s.onboarded;
    this.selfName = s.selfName;
    this.saveLocation = s.saveLocation;
    this.screen = this.onboarded ? 'home' : 'onboarding';
    const unread = (what: string) => (e: unknown) => console.warn(`Could not read ${what}:`, message(e));
    void this.engine.getConnection().then((c) => (this.connection = c), unread('the connection'));
    void this.engine.getLanDiscovery().then((on) => (this.lanDiscovery = on), unread('Find nearby devices'));
    void this.engine.getWhoCanSend().then((who) => (this.whoCanSend = who), unread('Who can send'));
    void this.engine.getUpdateCheck().then((on) => (this.updateCheck = on), unread('the update check'));
    try {
      this.updateSeen = localStorage.getItem(UPDATE_SEEN);
    } catch {
      // Blocked storage: the banner shows again next time.
    }
    void this.loadSelf();
    if (this.native) void this.checkFirewall();
    this.wire(opts);

    this.offEngine = this.engine.on((e) => {
      switch (e.type) {
        case 'discovery':
          this.discovery = e.state;
          break;
        case 'device': {
          const i = this.devices.findIndex((d) => d.id === e.device.id);
          const was = i >= 0 ? this.devices[i] : undefined;
          if (i >= 0) this.devices[i] = e.device;
          else this.devices = [...this.devices, e.device];
          if (e.device.reachable && !was?.reachable) this.foundSound();
          break;
        }
        case 'deviceLost':
          this.devices = this.devices.filter((d) => d.id !== e.deviceId);
          break;
        case 'incoming':
          if (!this.requests.some((r) => r.id === e.request.id)) {
            this.requests = [...this.requests, e.request];
            playSound('incoming');
          }
          break;
        case 'incomingWithdrawn': {
          const r = this.requests.find((x) => x.id === e.requestId);
          this.requests = this.requests.filter((x) => x.id !== e.requestId);
          if (r) {
            this.toast(t('incoming.withdrawn', { device: this.deviceName(r.deviceId) }));
            playSound('cancel');
          }
          break;
        }
        case 'transfer':
          this.onTransfer(e.transfer);
          break;
        case 'transferProgress': {
          const x = this.transfers.find((t) => t.id === e.id);
          if (x) {
            x.doneBytes = e.doneBytes;
            x.speed = e.speed;
            x.movingSince = e.movingSince;
            for (const [file, bytes] of Object.entries(e.fileBytes)) x.fileBytes[file] = bytes;
          }
          break;
        }
        case 'paired':
          this.trustAsks = this.trustAsks.filter((r) => r.deviceId !== e.device.id);
          this.sheet = { kind: 'paired', deviceId: e.device.id, via: e.via, check: e.check };
          playHaptic('pairMatched');
          playSound('matched');
          this.moments.found = Date.now();
          break;
        case 'trustAsk':
          this.trustAsks = [...this.trustAsks.filter((r) => r.deviceId !== e.request.deviceId), e.request];
          // Someone is holding the other device and waiting: this comes first.
          this.sheet = { kind: 'trustAsk', deviceId: e.request.deviceId, check: e.request.check };
          playSound('incoming');
          break;
        case 'trustWaiting':
          if (this.sheet?.kind === 'trustWait' && this.sheet.deviceId === e.deviceId) this.sheet = { ...this.sheet, check: e.check };
          break;
        case 'trustEnded':
          this.trustEnded(e.deviceId, e.outcome);
          break;
        case 'code':
          this.code = e.code;
          break;
        case 'freeSpace':
          this.freeSpace = e.bytes;
          break;
        case 'server':
          this.serverProblem = e.problem;
          break;
        case 'history':
          this.history = [e.entry, ...this.history.filter((h) => h.id !== e.entry.id)].sort((a, b) => b.finishedAt - a.finishedAt);
          break;
        case 'linkShare':
          this.onLinkShare(e.share);
          break;
        case 'update':
          this.update = e.update;
          break;
        case 'snapshot':
          this.applySnapshot(e.snapshot);
          break;
      }
    });
    if (this.onboarded) this.startEngine();
  }

  private wire(opts: { popover?: boolean }): void {
    if (this.wired) return;
    this.wired = true;
    if (this.native && !opts.popover) {
      shell.onSendRequest((paths) => void this.sendFromFileManager(paths)).catch((e) => console.warn('"Send with Nhako Beam" is not connected:', message(e)));
    }
    const coarse = matchMedia('(pointer: coarse)');
    this.coarsePointer = coarse.matches;
    coarse.addEventListener('change', (e) => (this.coarsePointer = e.matches));
    this.width = innerWidth;
    addEventListener('resize', () => (this.width = innerWidth));
  }

  /** Starts the engine; when it can't start, says why (as Android does). */
  private startEngine(): void {
    this.engine.start().then(
      () => (this.engineReady = true),
      (e) => this.toast(message(e), 'danger'),
    );
  }

  /** Redraws from what the engine has (window loaded, or engine restarted). Finished transfers the
   *  app already shows stay as history. */
  private applySnapshot(s: Snapshot): void {
    this.engineReady = true;
    this.discovery = s.discovery;
    this.devices = s.devices;
    this.requests = s.requests;
    const fresh = new Map(s.transfers.map((x) => [x.id, x]));
    const kept = this.transfers.filter((x) => !fresh.has(x.id) && !ACTIVE.has(x.status));
    this.transfers = [...[...fresh.values()].sort((a, b) => b.startedAt - a.startedAt), ...kept];
    this.code = s.code;
    if (s.freeSpace !== null) this.freeSpace = s.freeSpace;
    this.serverProblem = s.serverProblem;
    this.history = s.history;
    this.trustAsks = s.trustAsks ?? [];
    this.linkShare = s.linkShare ?? null;
    for (const v of this.linkShare?.visitors ?? []) this.visitorsHeard.add(v.id);
    this.update = s.update ?? null;
    void this.loadSelf();
  }

  /** Name, save folder and fingerprint from the engine. The browser prototype keeps its own. */
  private async loadSelf(): Promise<void> {
    try {
      const me = await this.engine.getSelf();
      this.fingerprint = me.fingerprint;
      this.version = me.version;
      if (!this.native) return;
      this.saveLocation = me.saveDir;
      if (this.onboarded || PLACEHOLDER_NAMES.has(this.selfName)) this.selfName = me.name;
    } catch (e) {
      console.warn('Could not read this device:', message(e));
    }
  }

  /** Devices arriving all at once (just after launch, or back on Wi-Fi) make one sound, not many. */
  private foundQuietUntil = Date.now() + 2500;
  private foundSound(): void {
    const now = Date.now();
    if (now < this.foundQuietUntil) return;
    this.foundQuietUntil = now + 2000;
    playSound('found');
    this.moments.found = now;
  }

  private onTransfer(next: Transfer): void {
    const i = this.transfers.findIndex((x) => x.id === next.id);
    const prev = i >= 0 ? this.transfers[i] : undefined;
    if (i >= 0) this.transfers[i] = next;
    else {
      this.transfers = [next, ...this.transfers];
      if (next.direction === 'receive' && !this.acceptedHere.delete(next.deviceId)) {
        const d = this.devices.find((x) => x.id === next.deviceId);
        if (d?.autoAccept && next.text !== undefined) this.textArrived(next, d.name);
        else if (d?.autoAccept) {
          this.toast(t('incoming.autoAccepted', { device: d.name }), 'success');
          playSound('incoming');
        }
      }
    }
    if (prev?.status === next.status) return;
    if (next.status === 'done') {
      playHaptic('transferDone');
      playSound('done');
      this.moments.done = Date.now();
      this.bump(next.deviceId);
    } else if (next.status === 'failed') {
      playHaptic('transferFailed');
      playSound('failed');
      this.moments.failed = Date.now();
    } else if (next.status === 'declined') {
      playHaptic('decline');
      playSound('decline');
    } else if (next.status === 'cancelled' && next.cancelledBy === 'peer') playSound('cancel');
    else if (prev?.status === 'reconnecting' && next.status === 'transferring') this.toast(t('transfer.reconnected'), 'success');
  }

  persist(): void {
    try {
      localStorage.setItem(STORE, JSON.stringify({ onboarded: this.onboarded, selfName: this.selfName, saveLocation: this.saveLocation }));
    } catch {
      // Ignore.
    }
  }

  finishOnboarding(): void {
    this.onboarded = true;
    this.persist();
    this.engine.renameSelf(this.selfName);
    this.navigate('home');
    this.startEngine();
  }

  resetOnboarding(): void {
    this.offEngine?.();
    this.offEngine = null;
    this.engine.stop();
    this.engine = newEngine();
    this.started = false;
    this.engineReady = false;
    this.devices = [];
    this.transfers = [];
    this.requests = [];
    this.history = [];
    this.staged = [];
    this.sheet = null;
    this.linkShare = null;
    this.onboarded = false;
    this.persist();
    this.init();
  }

  navigate(to: Screen): void {
    const order: Screen[] = ['onboarding', 'home', 'history', 'settings', 'licenses'];
    this.navDir = order.indexOf(to) >= order.indexOf(this.screen) ? 1 : -1;
    if (to !== this.screen && to !== 'onboarding' && this.screen !== 'onboarding') playSound(this.navDir > 0 ? 'open' : 'close');
    this.screen = to;
  }

  deviceName(id: string): string {
    return this.devices.find((d) => d.id === id)?.name ?? 'The other device';
  }

  device(id: string): Device | undefined {
    return this.devices.find((d) => d.id === id);
  }

  transferFor(deviceId: string): Transfer | undefined {
    return this.activeTransfers.find((x) => x.deviceId === deviceId);
  }

  bump(deviceId: string): void {
    this.pulse = { ...this.pulse, [deviceId]: (this.pulse[deviceId] ?? 0) + 1 };
  }

  /** Saves the server setting; the engine reconnects through it. False (with a toast) if it couldn't. */
  async setConnection(c: Connection): Promise<boolean> {
    try {
      await this.engine.setConnection(c);
    } catch (e) {
      this.toast(message(e), 'danger');
      return false;
    }
    this.connection = c;
    this.toast(t(c.mode === 'own' ? 'settings.connection.saved.own' : 'settings.connection.saved.public'), 'success');
    return true;
  }

  /** Turns nearby search on or off; the engine reconnects. False (with a toast) if it couldn't. */
  async setLanDiscovery(on: boolean): Promise<boolean> {
    try {
      await this.engine.setLanDiscovery(on);
    } catch (e) {
      this.toast(message(e), 'danger');
      return false;
    }
    this.lanDiscovery = on;
    this.toast(t(on ? 'settings.network.lan.on' : 'settings.network.lan.off'), 'success');
    return true;
  }

  /** Settings > Receiving > Who can send to me. False (with a toast) if it couldn't be saved. */
  async setWhoCanSend(who: WhoCanSend): Promise<boolean> {
    const before = this.whoCanSend;
    this.whoCanSend = who;
    try {
      await this.engine.setWhoCanSend(who);
    } catch {
      this.whoCanSend = before;
      this.toast(t('settings.receive.who.failed'), 'danger');
      return false;
    }
    return true;
  }

  clearHistory(): void {
    this.engine.clearHistory();
    this.history = [];
    this.sheet = null;
    this.toast(t('history.cleared'), 'success');
  }

  historyEntry(id: string): HistoryEntry | undefined {
    return this.history.find((h) => h.id === id);
  }

  /** Picks where received files go: the folder picker in the desktop app, pretend folders in the prototype. */
  async changeSaveLocation(): Promise<void> {
    if (!this.native) {
      this.saveLocation = SAVE_OPTIONS[(SAVE_OPTIONS.indexOf(this.saveLocation) + 1) % SAVE_OPTIONS.length]!;
      this.persist();
      return;
    }
    try {
      const dir = await shell.pickFolder(this.saveLocation);
      if (!dir) return;
      await this.engine.setSaveDir(dir);
      this.saveLocation = dir;
    } catch (e) {
      this.toast(`${t('settings.device.save.failed')} ${message(e)}`, 'danger');
    }
  }

  /** Shows a new pairing code (it arrives as a 'code' event). */
  newCode(): void {
    this.engine.createCode().catch((e) => this.toast(message(e), 'danger'));
  }

  /**
   * Opens a received file, or shows it in its folder. A program or script (opening it runs it)
   * asks first; `confirmed` is the answer.
   */
  async openReceived(transferId: string, reveal: boolean, confirmed = false): Promise<void> {
    const entry = this.transfers.find((x) => x.id === transferId) ?? this.historyEntry(transferId);
    const only = entry?.files.length === 1 ? entry.files[0] : undefined;
    if (!reveal && !confirmed && only && isRunnable(only.name)) {
      const deviceName = this.device(entry!.deviceId)?.name ?? (entry && 'deviceName' in entry ? entry.deviceName : 'another device');
      this.sheet = { kind: 'confirmOpen', transferId, fileName: only.name, deviceName };
      return;
    }
    if (confirmed && this.sheet?.kind === 'confirmOpen') this.sheet = null;
    if (!this.native) {
      this.toast(reveal ? 'Prototype: would show the file in its folder' : 'Prototype: would open the file');
      return;
    }
    try {
      await shell.openReceived(transferId, reveal);
    } catch (e) {
      this.toast(message(e) || t('transfer.openFailed'), 'danger');
    }
  }

  /** Text from one of your own devices with "Accept files automatically" on: it goes straight to
   *  the clipboard (the desktop app's Rust side already put it there, even with the window hidden). */
  private textArrived(tr: Transfer, device: string): void {
    const text = tr.text ?? '';
    if (!this.native) void navigator.clipboard?.writeText(text).catch(() => {});
    const link = isLink(text);
    this.toast(
      t(link ? 'text.copied.link' : 'text.copied.from', { device }),
      'success',
      link ? { label: t('text.open'), run: () => void this.openLink(text.trim()) } : { label: t('text.show'), run: () => (this.sheet = { kind: 'text', id: tr.id }) },
    );
  }

  /** Copies text you chose to copy (a button), with a toast (`said`, or "Copied"). */
  async copyText(text: string, said = t('text.copied')): Promise<boolean> {
    try {
      if (this.native) await shell.writeClipboard(text);
      else await navigator.clipboard.writeText(text);
    } catch {
      this.toast(t('text.copyFailed'), 'danger');
      return false;
    }
    playHaptic('buttonPress');
    this.toast(said, 'success');
    return true;
  }

  async openLink(url: string): Promise<void> {
    if (!isLink(url)) return;
    if (this.native) await shell.openLink(url.trim()).catch((e) => this.toast(message(e), 'danger'));
    else window.open(url.trim(), '_blank', 'noopener');
  }

  /** What's on the clipboard as text ("" when nothing, or it can't be read). */
  async readClipboard(): Promise<string> {
    try {
      return this.native ? await shell.readClipboard() : await navigator.clipboard.readText();
    } catch {
      return '';
    }
  }

  /** Settings > About: copies the diagnostics report. False (with a toast) if it couldn't. */
  async copyDiagnostics(): Promise<boolean> {
    try {
      const report = await this.engine.diagnostics();
      if (this.native) await shell.writeClipboard(report);
      else await navigator.clipboard.writeText(report);
    } catch {
      this.toast(t('settings.about.diagnostics.failed'), 'danger');
      return false;
    }
    this.toast(t('settings.about.diagnostics.copied'), 'success');
    return true;
  }

  toast(text: string, tone: Toast['tone'] = 'neutral', action?: Toast['action']): void {
    const toast: Toast = { id: ++this.toastSeq, text, tone, action, ms: action ? 6000 : 4000 };
    if (tone === 'danger') playSound('error');
    this.toasts = [...this.toasts, toast].slice(-2);
    setTimeout(() => this.dismissToast(toast.id), toast.ms);
  }

  dismissToast(id: number): void {
    this.toasts = this.toasts.filter((x) => x.id !== id);
  }

  // ---------- Sending ----------

  stage(files: FileItem[]): void {
    if (!files.length) return;
    this.stagedText = null;
    this.staged = [...this.staged, ...files];
  }

  /** Text to send instead of files. False (with a toast) when it's empty or too long. */
  stageText(text: string): boolean {
    if (!text.trim()) {
      this.toast(t('send.clipboard.empty'), 'warning');
      return false;
    }
    if (utf8Bytes(text) > TEXT_MAX) {
      this.toast(t('send.text.tooLong'), 'warning');
      return false;
    }
    this.staged = [];
    this.stagedText = text;
    return true;
  }

  /** Home > Clipboard: whatever text is on the clipboard, ready to go to a device. */
  async stageClipboard(): Promise<boolean> {
    return this.stageText(await this.readClipboard());
  }

  /** Files from the browser's pickers and drops. A folder picker gives each file's path inside
   *  the folder, which becomes its `folder`. */
  stageFromFileList(list: FileList | File[]): void {
    let n = 0;
    this.stage(
      [...list].map((file) => {
        const rel = (file as File & { webkitRelativePath?: string }).webkitRelativePath ?? '';
        const cut = rel.lastIndexOf('/');
        const item: FileItem = { id: `u${Date.now()}-${n++}`, name: file.name, size: file.size || 1, mime: file.type || 'application/octet-stream' };
        if (cut > 0) item.folder = rel.slice(0, cut);
        return item;
      }),
    );
  }

  /** Desktop app: the native "choose folders" dialog. Resolves to what was staged. */
  async pickFolders(): Promise<FileItem[]> {
    try {
      const picked = await shell.pickFoldersToSend();
      return picked ? this.stagePicked(picked) : [];
    } catch (e) {
      this.toast(message(e), 'danger');
      return [];
    }
  }

  /** Desktop app: stages files by path (picked in the dialog, or dropped from the file manager). */
  async stagePaths(paths: string[]): Promise<FileItem[]> {
    try {
      return this.stagePicked(await shell.filesFromPaths(paths));
    } catch (e) {
      this.toast(message(e), 'danger');
      return [];
    }
  }

  /** "Send with Nhako Beam" in Explorer or Files: stage the files and ask where to send them. */
  async sendFromFileManager(paths: string[]): Promise<void> {
    const files = await this.stagePaths(paths);
    // Before onboarding is finished they wait in the tray above the radar.
    if (!files.length || !this.onboarded) return;
    if (this.screen !== 'home') this.navigate('home');
    if (!this.sheet || this.sheet.kind === 'send') this.sheet = { kind: 'send' };
  }

  /** Desktop app: the native "choose files" dialog. Resolves to what was staged. */
  async pickFiles(): Promise<FileItem[]> {
    try {
      const picked = await shell.pickFiles();
      return picked ? this.stagePicked(picked) : [];
    } catch (e) {
      this.toast(message(e), 'danger');
      return [];
    }
  }

  private stagePicked(p: shell.Picked): FileItem[] {
    if (p.skipped) this.toast(p.skipped === 1 ? t('send.skipped.one') : t('send.skipped.many', { n: p.skipped }), 'warning');
    this.stage(p.files);
    return p.files;
  }

  clearStaged(): void {
    this.staged = [];
    this.stagedText = null;
  }

  /** Asks Home to open its file or folder picker, then send what's picked to `deviceId`. */
  requestPick(kind: 'files' | 'folder', deviceId: string): void {
    this.pickRequest = { kind, deviceId, n: ++this.pickSeq };
  }

  /** Something is waiting to be sent: files or text. */
  get hasStaged(): boolean {
    return this.staged.length > 0 || this.stagedText !== null;
  }

  sendTo(deviceId: string, files: FileItem[] = this.staged): void {
    if (this.stagedText !== null && files === this.staged) {
      this.sendTextTo(deviceId, this.stagedText);
      return;
    }
    if (!files.length) return;
    const sending = [...files];
    this.staged = [];
    this.bump(deviceId);
    if (this.sheet?.kind === 'send') this.sheet = null;
    playSound('send');
    this.engine.send(deviceId, sending).catch((e) => {
      // Put the files back so nothing is lost, and say what went wrong.
      if (!this.staged.length) this.staged = sending;
      this.toast(message(e) || t('send.failed'), 'danger');
    });
  }

  sendTextTo(deviceId: string, text: string): void {
    this.stagedText = null;
    this.bump(deviceId);
    if (this.sheet?.kind === 'send' || this.sheet?.kind === 'compose') this.sheet = null;
    playSound('send');
    this.engine.sendText(deviceId, text).catch((e) => {
      if (this.stagedText === null && !this.staged.length) this.stagedText = text;
      this.toast(message(e) || t('send.failed'), 'danger');
    });
  }

  stagedSummary = $derived(summarizeFiles(this.staged));

  // ---------- Share via link ----------

  /** Shares what's waiting (files or text) with a link, for a device without the app. */
  async shareLink(): Promise<void> {
    if (!this.hasStaged) return;
    try {
      const share = await this.engine.shareLink(this.staged, this.stagedText ?? undefined);
      this.linkShare = share;
      this.staged = [];
      this.stagedText = null;
      this.sheet = { kind: 'link' };
      playSound('send');
    } catch (e) {
      this.toast(message(e), 'danger');
    }
  }

  stopLink(): void {
    this.engine.stopLink();
    this.linkShare = null;
    if (this.sheet?.kind === 'link') this.sheet = null;
    this.toast(t('link.stopped'));
    playSound('cancel');
  }

  answerVisitor(visitorId: string, allow: boolean): void {
    const v = this.linkShare?.visitors.find((x) => x.id === visitorId);
    if (v) v.state = allow ? 'allowed' : 'denied';
    this.engine.answerVisitor(visitorId, allow);
    playHaptic(allow ? 'accept' : 'decline');
    playSound(allow ? 'accept' : 'decline');
  }

  private onLinkShare(share: LinkShare | null): void {
    const had = this.linkShare;
    this.linkShare = share;
    if (!share) {
      this.visitorsHeard.clear();
      if (this.sheet?.kind === 'link') this.sheet = null;
      // Stopped here says so already; this is the link ending by itself.
      if (had) this.toast(t('link.ended'));
      return;
    }
    for (const v of share.visitors) {
      if (v.state !== 'asking' || this.visitorsHeard.has(v.id)) continue;
      this.visitorsHeard.add(v.id);
      playSound('incoming');
      // Someone is standing there with a browser: show who, unless something else is open.
      if (!this.sheet) this.sheet = { kind: 'link' };
      else if (this.sheet.kind !== 'link') this.toast(t('link.asks', { browser: v.label }), 'neutral', { label: t('text.show'), run: () => (this.sheet = { kind: 'link' }) });
    }
  }

  // ---------- Updates ----------

  async setUpdateCheck(on: boolean): Promise<boolean> {
    try {
      await this.engine.setUpdateCheck(on);
    } catch {
      this.toast(t('settings.about.updates.failed'), 'danger');
      return false;
    }
    this.updateCheck = on;
    if (!on) this.update = null;
    return true;
  }

  async checkFirewall(): Promise<void> {
    try {
      this.firewallBlocked = (await shell.firewall.status()) === false;
    } catch (e) {
      console.warn('Could not check Windows Firewall:', message(e));
    }
  }

  /** The firewall banner's Fix it: one Windows admin prompt, then a fresh look for devices. */
  async fixFirewall(): Promise<void> {
    if (this.fixingFirewall) return;
    this.fixingFirewall = true;
    try {
      const fixed = this.native ? await shell.firewall.fix() : true;
      this.firewallBlocked = !fixed;
      if (fixed) {
        this.toast(t('firewall.fixed'), 'success');
        this.engine.rescan();
      } else {
        this.toast(t('firewall.notFixed'), 'danger');
      }
    } catch (e) {
      this.toast(message(e), 'danger');
    } finally {
      this.fixingFirewall = false;
    }
  }

  /** The Home banner's Not now: hidden until the next version. */
  hideUpdate(): void {
    if (!this.update) return;
    this.updateSeen = this.update.version;
    try {
      localStorage.setItem(UPDATE_SEEN, this.update.version);
    } catch {
      // Blocked storage: it shows again next time.
    }
  }

  /** Opens the release page, where the downloads are. */
  openUpdate(): void {
    if (this.update) void this.openLink(this.update.url);
  }

  // ---------- Trust from Nearby ----------

  /** Asks a nearby device to trust this one, and to be trusted back; the sheet waits for it. */
  askTrust(deviceId: string): void {
    this.sheet = { kind: 'trustWait', deviceId, check: null };
    this.engine.askTrust(deviceId).catch((e) => {
      if (this.sheet?.kind === 'trustWait' && this.sheet.deviceId === deviceId) this.sheet = null;
      this.toast(message(e), 'danger');
    });
  }

  cancelTrust(deviceId: string): void {
    this.engine.cancelTrust(deviceId);
    if (this.sheet?.kind === 'trustWait' && this.sheet.deviceId === deviceId) this.sheet = null;
  }

  answerTrust(deviceId: string, yes: boolean): void {
    this.trustAsks = this.trustAsks.filter((r) => r.deviceId !== deviceId);
    if (this.sheet?.kind === 'trustAsk' && this.sheet.deviceId === deviceId) this.sheet = null;
    this.engine.answerTrust(deviceId, yes);
    // A yes opens the paired sheet when both have saved each other.
  }

  private trustEnded(deviceId: string, outcome: TrustOutcome): void {
    this.trustAsks = this.trustAsks.filter((r) => r.deviceId !== deviceId);
    const open = this.sheet && (this.sheet.kind === 'trustWait' || this.sheet.kind === 'trustAsk') && this.sheet.deviceId === deviceId;
    const asking = this.sheet?.kind === 'trustWait';
    if (open) this.sheet = null;
    const device = this.deviceName(deviceId);
    // Saying no here needs no toast; everything else says what happened.
    if (outcome === 'declined' && !asking) return;
    if (!open && outcome !== 'withdrawn') return;
    this.toast(t(`trust.ended.${outcome}`, { device }), outcome === 'declined' || outcome === 'withdrawn' ? 'neutral' : 'warning');
    playSound(outcome === 'declined' ? 'decline' : 'cancel');
  }

  // ---------- Receiving ----------

  accept(requestId: string, opts: { autoAccept?: boolean } = {}): void {
    const r = this.requests.find((x) => x.id === requestId);
    this.requests = this.requests.filter((x) => x.id !== requestId);
    if (r) this.acceptedHere.add(r.deviceId);
    this.engine.accept(requestId, { trust: !!opts.autoAccept, autoAccept: !!opts.autoAccept }).catch((e) => this.toast(message(e), 'danger'));
    if (r) this.bump(r.deviceId);
  }

  decline(requestId: string): void {
    this.requests = this.requests.filter((x) => x.id !== requestId);
    this.engine.decline(requestId);
  }

  dismissTransfer(id: string): void {
    if (!this.dismissed.includes(id)) this.dismissed = [...this.dismissed, id];
    if (this.sheet?.kind === 'transfer' && this.sheet.id === id) this.sheet = null;
  }
}

export const app = new AppState();
