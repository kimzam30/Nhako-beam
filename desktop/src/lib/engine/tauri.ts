// The real engine (core/) inside the desktop app. Each method is a Tauri command in
// src-tauri/src/engine.rs, and engine events arrive as `nhako://event` with the same JSON the mock
// produces. When the window (re)loads, the engine sends one snapshot so the app can redraw.
import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { Connection, Engine, EngineEvent, FileItem, LinkShare, PairingCode, RedeemResult, SelfInfo, ServerCheck, Snapshot, WhoCanSend } from './types';

const EVENT = 'nhako://event';
const RESYNC = 'nhako://resync';

/** True inside the desktop app (Tauri injects this before the page loads). */
export function isTauri(): boolean {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

interface Started {
  snapshot: Snapshot;
  me: SelfInfo;
}

/** Tauri rejects with the command's error string; turn anything else into one too. */
export function message(e: unknown): string {
  if (typeof e === 'string') return e;
  if (e instanceof Error) return e.message;
  return 'Something went wrong.';
}

export class TauriEngine implements Engine {
  private listeners = new Set<(e: EngineEvent) => void>();
  private unlisten: UnlistenFn[] = [];
  private booting: Promise<void> | null = null;
  /** A name picked before the engine started (onboarding), saved before the network starts. */
  private pendingName: string | null = null;
  private codeCall: Promise<PairingCode> | null = null;
  /** Set once the engine has started. */
  started = false;

  on(listener: (e: EngineEvent) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  private emit(e: EngineEvent): void {
    for (const l of this.listeners) l(e);
  }

  /** Commands whose answer the app doesn't wait for. Problems show up as events, or are logged. */
  private fire(cmd: string, args: Record<string, unknown> = {}): void {
    invoke(cmd, args).catch((e) => console.warn(`${cmd} failed:`, message(e)));
  }

  start(): Promise<void> {
    if (this.booting) return this.booting;
    const booting = this.boot().catch((e) => {
      console.error('The engine did not start:', message(e));
      if (this.booting === booting) this.booting = null;
      this.emit({ type: 'discovery', state: 'offline' });
      throw e;
    });
    this.booting = booting;
    return booting;
  }

  /** Bumped by `stop()`, so a start still waiting when it's stopped lets go of what it set up. */
  private run = 0;

  private async boot(): Promise<void> {
    const run = this.run;
    // Listen first, hold events until the snapshot is drawn, then let them through in order.
    let held: EngineEvent[] | null = [];
    const pass = (e: EngineEvent) => (held ? held.push(e) : this.emit(e));
    const mine: UnlistenFn[] = [];
    const letGo = () => {
      for (const u of mine) u();
      this.unlisten = this.unlisten.filter((u) => !mine.includes(u));
    };
    try {
      mine.push(await listen<EngineEvent>(EVENT, (m) => pass(m.payload)));
      mine.push(await listen<Snapshot>(RESYNC, (m) => pass({ type: 'snapshot', snapshot: m.payload })));
      if (run !== this.run) return letGo();
      this.unlisten.push(...mine);
      const started = await invoke<Started>('engine_start', { name: this.pendingName });
      if (run !== this.run) return;
      this.pendingName = null;
      this.started = true;
      this.emit({ type: 'snapshot', snapshot: started.snapshot });
      const queue = held;
      held = null;
      for (const e of queue) this.emit(e);
    } catch (e) {
      letGo();
      throw e;
    }
  }

  stop(): void {
    this.run++;
    for (const u of this.unlisten.splice(0)) u();
    this.booting = null;
    this.started = false;
    this.fire('engine_stop');
  }

  rescan(): void {
    this.fire('rescan');
  }

  send(deviceId: string, files: FileItem[]): Promise<string> {
    return invoke<string>('send', { deviceId, fileIds: files.map((f) => f.id) });
  }

  sendText(deviceId: string, text: string): Promise<string> {
    return invoke<string>('send_text', { deviceId, text });
  }

  accept(requestId: string, opts: { trust?: boolean; autoAccept?: boolean } = {}): Promise<string | null> {
    return invoke<string | null>('accept', { requestId, trust: !!opts.trust, autoAccept: !!opts.autoAccept });
  }

  decline(requestId: string): void {
    this.fire('decline', { requestId });
  }

  pause(transferId: string): void {
    this.fire('pause', { transferId });
  }

  resume(transferId: string): void {
    this.fire('resume', { transferId });
  }

  cancel(transferId: string): void {
    this.fire('cancel', { transferId });
  }

  retry(transferId: string): void {
    this.fire('retry', { transferId });
  }

  createCode(): Promise<PairingCode> {
    // The sheet can ask twice while the first answer is on its way; one code is enough.
    this.codeCall ??= invoke<PairingCode>('create_code').finally(() => (this.codeCall = null));
    return this.codeCall;
  }

  cancelCode(): void {
    this.fire('cancel_code');
  }

  redeemCode(words: string[]): Promise<RedeemResult> {
    return invoke<RedeemResult>('redeem_code', { words });
  }

  setTrusted(deviceId: string, trusted: boolean): void {
    this.fire('set_trusted', { deviceId, trusted });
  }

  askTrust(deviceId: string): Promise<void> {
    return invoke('ask_trust', { deviceId });
  }

  cancelTrust(deviceId: string): void {
    this.fire('cancel_trust', { deviceId });
  }

  answerTrust(deviceId: string, yes: boolean): void {
    this.fire('answer_trust', { deviceId, yes });
  }

  setAutoAccept(deviceId: string, on: boolean): void {
    this.fire('set_auto_accept', { deviceId, on });
  }

  renameSelf(name: string): void {
    if (!this.started) this.pendingName = name;
    this.fire('rename_self', { name });
  }

  getSelf(): Promise<SelfInfo> {
    return invoke<SelfInfo>('get_self');
  }

  setSaveDir(dir: string): Promise<void> {
    return invoke('set_save_dir', { dir });
  }

  getConnection(): Promise<Connection> {
    return invoke<Connection>('get_connection');
  }

  setConnection(connection: Connection): Promise<void> {
    return invoke('set_connection', { connection });
  }

  checkServer(server: Extract<Connection, { mode: 'own' }>): Promise<ServerCheck> {
    const { mode: _mode, ...own } = server;
    return invoke<ServerCheck>('check_server', { server: own });
  }

  getLanDiscovery(): Promise<boolean> {
    return invoke<boolean>('get_lan_discovery');
  }

  setLanDiscovery(on: boolean): Promise<void> {
    return invoke('set_lan_discovery', { on });
  }

  getWhoCanSend(): Promise<WhoCanSend> {
    return invoke<WhoCanSend>('get_who_can_send');
  }

  setWhoCanSend(who: WhoCanSend): Promise<void> {
    return invoke('set_who_can_send', { who });
  }

  shareLink(files: FileItem[], text?: string): Promise<LinkShare> {
    return invoke<LinkShare>('share_link', { fileIds: files.map((f) => f.id), text: text ?? null });
  }

  stopLink(): void {
    this.fire('stop_link');
  }

  answerVisitor(visitorId: string, allow: boolean): void {
    this.fire('answer_visitor', { visitorId, allow });
  }

  getUpdateCheck(): Promise<boolean | null> {
    return invoke<boolean | null>('get_update_check');
  }

  setUpdateCheck(on: boolean): Promise<void> {
    return invoke('set_update_check', { on });
  }

  clearHistory(): void {
    this.fire('clear_history');
  }

  diagnostics(): Promise<string> {
    return invoke<string>('diagnostics');
  }
}
