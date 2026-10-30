// The desktop app's own abilities around the engine: native file and folder pickers, files dragged
// in from the file manager (with their real paths), opening received files, start at login, and
// staying in the tray. Only used inside the desktop app (see `isTauri`); the browser prototype
// keeps its <input type="file"> and pretend folders.
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { disable, enable, isEnabled } from '@tauri-apps/plugin-autostart';
import type { FileItem } from './engine/types';

export interface Picked {
  files: FileItem[];
  /** Unreadable items and empty folders. */
  skipped: number;
}

export interface DropHandlers {
  enter(x: number, y: number, count: number): void;
  over(x: number, y: number): void;
  drop(x: number, y: number, paths: string[]): void;
  leave(): void;
}

/**
 * Turns dropped paths (or files from "Send with Nhako Beam") into file items; the desktop app
 * remembers which path each item id is. It only takes paths it saw you hand over itself, so
 * anything else comes back as skipped.
 */
export function filesFromPaths(paths: string[]): Promise<Picked> {
  return invoke<Picked>('files_from_paths', { paths });
}

/** The native "choose files" dialog (opened by the app, not the page). Null when cancelled. */
export function pickFiles(): Promise<Picked | null> {
  return invoke<Picked | null>('pick_files');
}

/** The native "choose folders" dialog, for folders to send (everything inside, keeping its
 *  shape). Null when cancelled. */
export function pickFoldersToSend(): Promise<Picked | null> {
  return invoke<Picked | null>('pick_folders');
}

/** What's on the clipboard as text ("" when there's none). */
export function readClipboard(): Promise<string> {
  return invoke<string>('read_clipboard');
}

/** Puts text on the clipboard, with the window in front or not. */
export function writeClipboard(text: string): Promise<void> {
  return invoke('write_clipboard', { text });
}

/** The native "choose folder" dialog, starting at `current`. Null when cancelled. */
export function pickFolder(current?: string): Promise<string | null> {
  return invoke<string | null>('pick_save_dir', { current: current ?? null });
}

/** Opens a received transfer's file (or its folder when there are several), or shows it in its folder. */
export function openReceived(transferId: string, reveal: boolean): Promise<void> {
  return invoke('open_received', { transferId, reveal });
}

export const startAtLogin = {
  get: (): Promise<boolean> => isEnabled(),
  set: (on: boolean): Promise<void> => (on ? enable() : disable()),
};

/** "Send with Nhako Beam" in the file manager's right-click menu. `manager` is null where there's none. */
export interface FileMenu {
  manager: 'explorer' | 'files' | null;
  on: boolean;
  /** Running as a Flatpak: the file manager menu and start at login can't reach outside it. */
  sandboxed?: boolean;
}

export const fileMenu = {
  get: (): Promise<FileMenu> => invoke<FileMenu>('get_file_menu'),
  set: (on: boolean): Promise<FileMenu> => invoke<FileMenu>('set_file_menu', { on }),
};

/**
 * Files from "Send with Nhako Beam" (the app started with `--send`, or a later launch handed them
 * over). Calls `handle` with whatever is waiting now, and again each time more arrive.
 */
export async function onSendRequest(handle: (paths: string[]) => void): Promise<() => void> {
  const take = async () => {
    const paths = await invoke<string[]>('take_send_paths');
    if (paths.length) handle(paths);
  };
  const off = await listen('nhako://send', () => void take());
  await take();
  return off;
}

export const keepRunning = {
  get: (): Promise<boolean> => invoke<boolean>('get_background'),
  set: (on: boolean): Promise<void> => invoke('set_background', { on }),
};

/**
 * Files dragged in from the file manager. The window gets the real paths from Tauri (the browser's
 * drag events don't carry them), with positions in physical pixels, converted to CSS pixels here.
 */
export function onFileDrop(h: DropHandlers): Promise<() => void> {
  const css = (p: { x: number; y: number }) => ({ x: p.x / devicePixelRatio, y: p.y / devicePixelRatio });
  return getCurrentWebview().onDragDropEvent((e) => {
    const p = e.payload;
    if (p.type === 'enter') {
      const { x, y } = css(p.position);
      h.enter(x, y, p.paths.length || 1);
    } else if (p.type === 'over') {
      const { x, y } = css(p.position);
      h.over(x, y);
    } else if (p.type === 'drop') {
      const { x, y } = css(p.position);
      h.drop(x, y, p.paths);
    } else {
      h.leave();
    }
  });
}

/** Windows Firewall and Nhako Beam's own rule (core/src/firewall.rs). */
export const firewall = {
  /** Whether nearby devices can reach this PC: false without the rule, null off Windows. */
  status: (): Promise<boolean | null> => invoke<boolean | null>('firewall_status'),
  /** Asks Windows for admin rights once and adds the rule. True when it's there afterwards. */
  fix: (): Promise<boolean> => invoke<boolean>('fix_firewall'),
};

/** Opens a web page in the default browser. */
export function openLink(url: string): Promise<void> {
  return invoke('open_link', { url });
}
