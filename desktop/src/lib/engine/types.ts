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
  /** Not reachable right now, but can be woken up to receive (a phone Android put to sleep). */
  wakeable: boolean;
}

export interface FileItem {
  id: string;
  name: string;
  size: number;
  mime: string;
  /** The folder it's in, inside a folder that was sent ("Photos/2024", "/" between folders).
   *  Missing for files sent on their own. */
  folder?: string;
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
  /** The receiver only takes files from its own devices (Who can send to me). */
  | 'notAllowed'
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
  /** Sender: the other device was asleep and has been asked to wake up. */
  waking: boolean;
  /** A text transfer (a note, a link, a clipboard): the text. Its one file is the same text, for
   *  devices on a version without text. */
  text?: string;
}

/** A transfer that ended, as History lists it. Kept after a restart, and after the device is removed. */
export interface HistoryEntry {
  /** The transfer's id. */
  id: string;
  direction: TransferDirection;
  deviceId: string;
  /** The other device's name when the transfer ended. */
  deviceName: string;
  deviceKind: DeviceKind;
  files: FileItem[];
  totalBytes: number;
  doneBytes: number;
  route: Route;
  /** Done, failed, cancelled or declined. */
  status: TransferStatus;
  error: TransferError | null;
  cancelledBy: 'me' | 'peer' | null;
  /** Epoch ms. */
  startedAt: number;
  finishedAt: number;
  /** Bytes per second while bytes were moving (pauses left out). 0 when nothing moved. */
  averageSpeed: number;
  /** A text transfer: the text. */
  text?: string;
}

export interface IncomingRequest {
  id: string;
  deviceId: string;
  files: FileItem[];
  totalBytes: number;
  receivedAt: number;
  /** Text rather than files: the text, to read before accepting. */
  text?: string;
}

/** Another device on the same network asking to be one of your devices, and you one of its own
 *  (Trust this device, from Nearby). Both screens show `check`. */
export interface TrustRequest {
  deviceId: string;
  check: string;
  receivedAt: number;
}

/** How a trust ask ended without both devices trusting each other.
 *  Asker: declined, outdated (a version without trust asks), unreachable, busy.
 *  Asked: declined (you said no), withdrawn (the asker gave up or went away). */
export type TrustOutcome = 'declined' | 'withdrawn' | 'outdated' | 'unreachable' | 'busy';

/** A newer Nhako Beam on GitHub (Settings > About > Check for new versions). */
export interface UpdateInfo {
  /** "0.3.0". */
  version: string;
  /** Its release page, where the downloads are. */
  url: string;
}

/** Files (and or text) shared with a link, for a device without the app (Share via link). */
export interface LinkShare {
  id: string;
  /** "http://192.168.1.20:41234/k3jq.../", also in the QR code. */
  url: string;
  files: FileItem[];
  totalBytes: number;
  text?: string;
  /** Epoch ms. Downloads running then carry on, and the link stops after them. */
  expiresAt: number;
  /** Browsers that opened the link, in the order they came. */
  visitors: LinkVisitor[];
}

/** A browser that opened a shared link. */
export interface LinkVisitor {
  id: string;
  /** "Chrome on Android", from what the browser says about itself. */
  label: string;
  /** Its address on the network, "192.168.1.23". */
  address: string;
  /** Asking: waiting for you to let it in. */
  state: 'asking' | 'allowed' | 'denied';
  arrivedAt: number;
  /** Downloads running now: bytes so far, and their sizes. Both 0 when nothing is running. */
  activeBytes: number;
  activeTotal: number;
  /** Downloads finished (a file, or the zip of everything). */
  downloads: number;
}

/** The longest text sent as text, in bytes of UTF-8. Longer text goes as a file. */
export const TEXT_MAX = 64 * 1024;

export interface PairingCode {
  /** Four words (since Phase 11). */
  words: [string, string, string, string];
  /** Epoch ms. Codes live 10 minutes. */
  expiresAt: number;
  /** Encoded in the QR code. */
  uri: string;
}

export type DiscoveryState = 'searching' | 'idle' | 'blocked' | 'offline';

export type PairVia = 'shown' | 'entered' | 'nearby';

/** Settings > Receiving > Who can send to me. Trusted devices, and devices connected with a code,
 *  can always send. */
export type WhoCanSend = 'yourDevices' | 'everyoneNearby';

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
  /** Newest first. */
  history: HistoryEntry[];
  /** Trust asks waiting for an answer here. */
  trustAsks: TrustRequest[];
  /** The link being shared, if any. */
  linkShare: LinkShare | null;
  /** A newer version, when the last check found one. */
  update: UpdateInfo | null;
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
  /** A transfer moved: only what changed since its last `transfer` or `transferProgress` event, to merge in.
   * `fileBytes` lists only the files whose count changed. */
  | { type: 'transferProgress'; id: string; doneBytes: number; speed: number; movingSince: number | null; fileBytes: Record<string, number> }
  /** `check` is the same short value on both screens ("Check that both screens show").
   *  `nearby`: Trust this device, from Nearby, agreed on both. */
  | { type: 'paired'; device: Device; via: PairVia; check: string }
  /** The code this device is showing changed (new, expiring, or gone). */
  | { type: 'code'; code: PairingCode | null }
  | { type: 'freeSpace'; bytes: number }
  /** Your own server's state changed. null when it's fine (always null on public servers). */
  | { type: 'server'; problem: ServerProblem | null }
  /** A transfer ended and was added to History (or replaced its own entry after a retry). */
  | { type: 'history'; entry: HistoryEntry }
  /** A device nearby asks to trust this one, and this one it. Answer with `answerTrust`. */
  | { type: 'trustAsk'; request: TrustRequest }
  /** On the device that asked: the other device has the ask, and both screens show `check`. */
  | { type: 'trustWaiting'; deviceId: string; check: string }
  /** A trust ask ended without a match (on either device). A match ends with `paired` instead. */
  | { type: 'trustEnded'; deviceId: string; outcome: TrustOutcome }
  /** The link being shared changed: started, a browser came or moved, or it stopped (null). */
  | { type: 'linkShare'; share: LinkShare | null }
  /** A newer version is out (or, null, none is). */
  | { type: 'update'; update: UpdateInfo | null }
  /** Redraw from this (the engine restarted, or the window fell behind). Replaces devices and requests. */
  | { type: 'snapshot'; snapshot: Snapshot };

export interface Engine {
  /** Resolves once the engine runs; rejects with the reason (plain words) when it can't start. */
  start(): Promise<void>;
  stop(): void;
  on(listener: (e: EngineEvent) => void): () => void;
  rescan(): void;

  /** Resolves to the transfer id. Rejects with a plain-language message (for example a file that was moved).
   *  On the desktop app the FileItem ids come from the shell, which knows each file's path. */
  send(deviceId: string, files: FileItem[]): Promise<string>;
  /** Sends text (a note, a link, a clipboard). Resolves to the transfer id. Rejects with a
   *  plain-language message (for example text over TEXT_MAX). */
  sendText(deviceId: string, text: string): Promise<string>;
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
  /** Asks a nearby device to trust this one, and to be trusted back. `trustWaiting` follows, then
   *  `paired` (via nearby) or `trustEnded`. Rejects if the device isn't nearby. */
  askTrust(deviceId: string): Promise<void>;
  cancelTrust(deviceId: string): void;
  /** Answers another device's ask. */
  answerTrust(deviceId: string, yes: boolean): void;
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

  /** Whether this device announces itself and looks for others on the local network. */
  getLanDiscovery(): Promise<boolean>;
  /** Saves it and reconnects. Rejects with a plain-language message if it can't (transfers still running). */
  setLanDiscovery(on: boolean): Promise<void>;

  /** Who may send requests here. Applies to the next request, no reconnect. */
  getWhoCanSend(): Promise<WhoCanSend>;
  setWhoCanSend(who: WhoCanSend): Promise<void>;

  /** Starts sharing files and or text with a link, replacing one already shared. Rejects with a
   *  plain-language message (off a local network, a file that was moved). */
  shareLink(files: FileItem[], text?: string): Promise<LinkShare>;
  stopLink(): void;
  /** Lets a browser in to the shared link, or turns it away. */
  answerVisitor(visitorId: string, allow: boolean): void;

  /** Settings > About > Check for new versions. null in builds that get updates from a store. */
  getUpdateCheck(): Promise<boolean | null>;
  setUpdateCheck(on: boolean): Promise<void>;

  /** Forgets every History entry. Received files stay where they are. */
  clearHistory(): void;

  /** Settings > About > Copy diagnostics: a plain-text report on the network and nearby
   *  discovery, with recent logs. Leaves out device names, file names and internet addresses. */
  diagnostics(): Promise<string>;
}
