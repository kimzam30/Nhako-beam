// The contract between the UI and the transfer engine. Keep this file the single definition.
// - MockEngine (mock.ts): the pretend engine for the browser prototype and the UI tests.
// - TauriEngine (tauri.ts): the Rust core (core/) through the desktop app's commands and events.
// - Android (Phase 7): the same core through UniFFI.
// Methods that answer with a value are async, because the real engine runs in another process.

export type DeviceKind = 'laptop' | 'desktop' | 'phone' | 'tablet';
export type OsPlatform = 'windows' | 'linux' | 'android' | 'macos' | 'ios' | 'unknown';

/** How bytes travel: same network, direct over the internet, or through a relay. */
export type Route = 'nearby' | 'direct' | 'relayed';

export interface Device {
  id: string;
  name: string;
  kind: DeviceKind;
  os: OsPlatform;
  /** Saved after pairing; can find each other without a code. */
  trusted: boolean;
  /** Trusted devices only: incoming files skip the request card. */
  autoAccept: boolean;
  /** Currently reachable (visible nearby, or reachable through the internet). */
  reachable: boolean;
  /** Best known route right now, if reachable. */
  route: Route | null;
  /** Short public key fingerprint shown when pairing, for example "4F2A 91C3". */
  fingerprint: string;
}

export interface FileItem {
  id: string;
  name: string;
  size: number;
  mime: string;
}

export type TransferDirection = 'send' | 'receive';

export type TransferStatus =
  | 'waiting' // sender: waiting for the other side to accept
  | 'connecting'
  | 'transferring'
  | 'paused'
  | 'reconnecting'
  | 'done'
  | 'failed'
  | 'cancelled'
  | 'declined';

export type TransferError =
  | 'peerGone'
  | 'network'
  | 'fileChanged'
  | 'integrity'
  | 'relayUnreachable'
  | 'permission'
  | 'diskFull'
  | 'diskFullPeer'
  | 'unknown';

export interface Transfer {
  id: string;
  direction: TransferDirection;
  deviceId: string;
  files: FileItem[];
  totalBytes: number;
  /** Bytes confirmed by the receiver (only goes backwards after a reconnect rollback). */
  doneBytes: number;
  /** Per file confirmed bytes, keyed by file id. */
  fileBytes: Record<string, number>;
  /** Smoothed bytes per second (3 second average). 0 when not moving. */
  speed: number;
  route: Route;
  status: TransferStatus;
  pausedBy: 'me' | 'peer' | null;
  cancelledBy: 'me' | 'peer' | null;
  error: TransferError | null;
  startedAt: number;
  /** When bytes first moved; ETA is hidden until 3 s after this. */
  movingSince: number | null;
  finishedAt: number | null;
  /** Sender is still waiting after 60 s. */
  waitingLong: boolean;
}

export interface IncomingRequest {
  id: string;
  deviceId: string;
  files: FileItem[];
  totalBytes: number;
  receivedAt: number;
}

export interface PairingCode {
  words: [string, string, string];
  /** Epoch ms. Codes live 10 minutes. */
  expiresAt: number;
  /** Encoded in the QR code. */
  uri: string;
}

export type DiscoveryState = 'searching' | 'idle' | 'blocked' | 'offline';

export type RedeemResult =
  | { ok: true; device: Device }
  | { ok: false; reason: 'unknownWord'; index: number }
  /** `tokenRejected`: your own relay turned this device away, so the other device couldn't be reached. */
  | { ok: false; reason: 'noMatch' | 'expired' | 'tokenRejected' };

/** Something wrong with your own server, found while running on it. */
export type ServerProblem = 'unreachable' | 'tokenRejected';

/** Everything needed to draw the app, for when the window (re)loads after the engine started. */
export interface Snapshot {
  discovery: DiscoveryState;
  devices: Device[];
  transfers: Transfer[];
  requests: IncomingRequest[];
  code: PairingCode | null;
  freeSpace: number | null;
  serverProblem: ServerProblem | null;
}

/** This device, as the engine knows it. */
export interface SelfInfo {
  name: string;
  /** Where received files go (a full path on the real engine). */
  saveDir: string;
  /** Short public key fingerprint; null before the engine has started. */
  fingerprint: string | null;
  version: string;
}

/** Which servers help devices find and reach each other over the internet. Nearby discovery works either way. */
export type Connection =
  | { mode: 'public' }
  | {
      mode: 'own';
      /** Relay address, for example "http://192.168.1.20:3340". */
      relay: string;
      /** Sent to the relay when it only lets in devices that know it. */
      token?: string;
      /** Address lookup server. When unset, n0's public lookup is used. */
      lookup?: string;
    };

export type CheckResult =
  | { status: 'ok'; ms: number }
  /** The address is fine but nothing usable answered. `reason` is plain English from the engine. */
  | { status: 'unreachable'; reason: string }
  /** The relay answered and turned the token down (or wanted one). */
  | { status: 'tokenRejected' }
  | { status: 'invalid'; reason: string };

export interface ServerCheck {
  relay: CheckResult;
  /** null when no lookup server is set. */
  lookup: CheckResult | null;
}

export type EngineEvent =
  | { type: 'discovery'; state: DiscoveryState }
  | { type: 'device'; device: Device }
  | { type: 'deviceLost'; deviceId: string }
  | { type: 'incoming'; request: IncomingRequest }
  | { type: 'incomingWithdrawn'; requestId: string }
  | { type: 'transfer'; transfer: Transfer }
  /** `check` is the same short value on both screens ("Check that both screens show"). */
  | { type: 'paired'; device: Device; via: 'shown' | 'entered'; check: string }
  /** The code this device is showing changed (new, expiring, or gone). */
  | { type: 'code'; code: PairingCode | null }
  | { type: 'freeSpace'; bytes: number }
  /** Your own server's state changed. null when it's fine (always null on public servers). */
  | { type: 'server'; problem: ServerProblem | null }
  /** Redraw from this (the engine restarted, or the window fell behind). Replaces devices and requests. */
  | { type: 'snapshot'; snapshot: Snapshot };

export interface Engine {
  start(): void;
  stop(): void;
  on(listener: (e: EngineEvent) => void): () => void;
  rescan(): void;

  /** Resolves to the transfer id. Rejects with a plain-language message (for example a file that was moved).
   *  On the desktop app the FileItem ids come from the shell, which knows each file's path. */
  send(deviceId: string, files: FileItem[]): Promise<string>;
  accept(requestId: string, opts?: { trust?: boolean; autoAccept?: boolean }): Promise<string | null>;
  decline(requestId: string): void;
  pause(transferId: string): void;
  resume(transferId: string): void;
  cancel(transferId: string): void;
  /** Resumes a failed transfer from the last confirmed chunk. */
  retry(transferId: string): void;

  createCode(): Promise<PairingCode>;
  cancelCode(): void;
  redeemCode(words: string[]): Promise<RedeemResult>;

  setTrusted(deviceId: string, trusted: boolean): void;
  setAutoAccept(deviceId: string, on: boolean): void;
  renameSelf(name: string): void;
  getSelf(): Promise<SelfInfo>;
  /** Rejects with a plain-language message if the folder can't be used. */
  setSaveDir(dir: string): Promise<void>;

  getConnection(): Promise<Connection>;
  /** Saves the setting and reconnects through it. Rejects with a plain-language message if it can't
   *  (a setting that can't be used, or transfers still running). */
  setConnection(c: Connection): Promise<void>;
  /** Checks a server before saving it. Never rejects: problems come back in the result. */
  checkServer(c: Extract<Connection, { mode: 'own' }>): Promise<ServerCheck>;
}
