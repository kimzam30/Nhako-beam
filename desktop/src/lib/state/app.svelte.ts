// App-wide UI state. Holds what the engine reports plus what the UI is showing.
import { breakpoint } from '../tokens';
import { MockEngine } from '../engine/mock';
import { TauriEngine, isTauri, message } from '../engine/tauri';
import type { Connection, Device, DiscoveryState, Engine, FileItem, IncomingRequest, PairingCode, ServerProblem, Snapshot, Transfer } from '../engine/types';
import * as shell from '../shell';
import { playHaptic } from '../haptics.svelte';
import { t } from '../copy';
import { summarizeFiles } from '../format';

export type Screen = 'onboarding' | 'home' | 'settings';
export type PlatformMode = 'auto' | 'desktop' | 'android';
export type Layout = 'compact' | 'medium' | 'expanded';

export type Sheet =
  | { kind: 'send' }
  | { kind: 'showCode' }
  | { kind: 'enterCode' }
  | { kind: 'scan' }
  | { kind: 'paired'; deviceId: string; via: 'shown' | 'entered'; check: string }
  | { kind: 'transfer'; id: string }
  | { kind: 'confirmRemove'; deviceId: string };

export interface Toast {
  id: number;
  text: string;
  tone: 'neutral' | 'success' | 'warning' | 'danger';
  action?: { label: string; run: () => void };
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

  /** The pretend engine's test hooks (browser prototype and UI tests only). */
  get mock(): MockEngine | null {
    return this.engine instanceof MockEngine ? this.engine : null;
  }

  isAndroid = $derived(this.platformMode === 'android' || (this.platformMode === 'auto' && this.coarsePointer));
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

  init(): void {
    if (this.started) return;
    this.started = true;
    const s = loadStored();
    this.onboarded = s.onboarded;
    this.selfName = s.selfName;
    this.saveLocation = s.saveLocation;
    this.screen = this.onboarded ? 'home' : 'onboarding';
    void this.engine.getConnection().then((c) => (this.connection = c));
    void this.loadSelf();
    if (this.native) void shell.onSendRequest((paths) => void this.sendFromFileManager(paths));

    const coarse = matchMedia('(pointer: coarse)');
    this.coarsePointer = coarse.matches;
    coarse.addEventListener('change', (e) => (this.coarsePointer = e.matches));
    this.width = innerWidth;
    addEventListener('resize', () => (this.width = innerWidth));

    this.engine.on((e) => {
      switch (e.type) {
        case 'discovery':
          this.discovery = e.state;
          break;
        case 'device': {
          const i = this.devices.findIndex((d) => d.id === e.device.id);
          if (i >= 0) this.devices[i] = e.device;
          else this.devices = [...this.devices, e.device];
          break;
        }
        case 'deviceLost':
          this.devices = this.devices.filter((d) => d.id !== e.deviceId);
          break;
        case 'incoming':
          if (!this.requests.some((r) => r.id === e.request.id)) this.requests = [...this.requests, e.request];
          break;
        case 'incomingWithdrawn': {
          const r = this.requests.find((x) => x.id === e.requestId);
          this.requests = this.requests.filter((x) => x.id !== e.requestId);
          if (r) this.toast(t('incoming.withdrawn', { device: this.deviceName(r.deviceId) }));
          break;
        }
        case 'transfer':
          this.onTransfer(e.transfer);
          break;
        case 'paired':
          this.sheet = { kind: 'paired', deviceId: e.device.id, via: e.via, check: e.check };
          playHaptic('pairMatched');
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
        case 'snapshot':
          this.applySnapshot(e.snapshot);
          break;
      }
    });
    if (this.onboarded) this.engine.start();
  }

  /** Redraws from what the engine has (window loaded, or engine restarted). Finished transfers the
   *  app already shows stay as history. */
  private applySnapshot(s: Snapshot): void {
    this.discovery = s.discovery;
    this.devices = s.devices;
    this.requests = s.requests;
    const fresh = new Map(s.transfers.map((x) => [x.id, x]));
    const kept = this.transfers.filter((x) => !fresh.has(x.id) && !ACTIVE.has(x.status));
    this.transfers = [...[...fresh.values()].sort((a, b) => b.startedAt - a.startedAt), ...kept];
    this.code = s.code;
    if (s.freeSpace !== null) this.freeSpace = s.freeSpace;
    this.serverProblem = s.serverProblem;
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

  private onTransfer(next: Transfer): void {
    const i = this.transfers.findIndex((x) => x.id === next.id);
    const prev = i >= 0 ? this.transfers[i] : undefined;
    if (i >= 0) this.transfers[i] = next;
    else {
      this.transfers = [next, ...this.transfers];
      if (next.direction === 'receive' && !this.acceptedHere.delete(next.deviceId)) {
        const d = this.devices.find((x) => x.id === next.deviceId);
        if (d?.autoAccept) this.toast(t('incoming.autoAccepted', { device: d.name }), 'success');
      }
    }
    if (prev?.status === next.status) return;
    if (next.status === 'done') {
      playHaptic('transferDone');
      this.bump(next.deviceId);
    } else if (next.status === 'failed') playHaptic('transferFailed');
    else if (next.status === 'declined') playHaptic('decline');
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
    this.engine.start();
  }

  resetOnboarding(): void {
    this.engine.stop();
    this.engine = newEngine();
    this.started = false;
    this.devices = [];
    this.transfers = [];
    this.requests = [];
    this.staged = [];
    this.sheet = null;
    this.onboarded = false;
    this.persist();
    this.init();
  }

  navigate(to: Screen): void {
    const order: Screen[] = ['onboarding', 'home', 'settings'];
    this.navDir = order.indexOf(to) >= order.indexOf(this.screen) ? 1 : -1;
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

  /** Opens a received file, or shows it in its folder. */
  async openReceived(transferId: string, reveal: boolean): Promise<void> {
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

  toast(text: string, tone: Toast['tone'] = 'neutral', action?: Toast['action']): void {
    const toast: Toast = { id: ++this.toastSeq, text, tone, action };
    this.toasts = [...this.toasts, toast].slice(-2);
    setTimeout(() => this.dismissToast(toast.id), action ? 6000 : 4000);
  }

  dismissToast(id: number): void {
    this.toasts = this.toasts.filter((x) => x.id !== id);
  }

  // ---------- Sending ----------

  stage(files: FileItem[]): void {
    if (!files.length) return;
    this.staged = [...this.staged, ...files];
  }

  stageFromFileList(list: FileList | File[]): void {
    let n = 0;
    this.stage(
      [...list].map((file) => ({ id: `u${Date.now()}-${n++}`, name: file.name, size: file.size || 1, mime: file.type || 'application/octet-stream' })),
    );
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
  }

  sendTo(deviceId: string, files: FileItem[] = this.staged): void {
    if (!files.length) return;
    const sending = [...files];
    this.staged = [];
    this.bump(deviceId);
    if (this.sheet?.kind === 'send') this.sheet = null;
    this.engine.send(deviceId, sending).catch((e) => {
      // Put the files back so nothing is lost, and say what went wrong.
      if (!this.staged.length) this.staged = sending;
      this.toast(message(e) || t('send.failed'), 'danger');
    });
  }

  stagedSummary = $derived(summarizeFiles(this.staged));

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
