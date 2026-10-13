// Formatting for sizes, speeds, times and file summaries. Rules from docs/design/ux-copy.md:
// never more than one decimal, "About 2 min left", sentence case.
import type { FileItem } from './engine/types';

const UNITS = ['B', 'KB', 'MB', 'GB', 'TB'];

export function formatBytes(n: number): string {
  if (!Number.isFinite(n) || n <= 0) return '0 B';
  let i = 0;
  let v = n;
  while (v >= 1000 && i < UNITS.length - 1) {
    v /= 1000;
    i++;
  }
  const digits = i === 0 || v >= 100 ? 0 : 1;
  return `${v.toFixed(digits).replace(/\.0$/, '')} ${UNITS[i]}`;
}

export function formatSpeed(bytesPerSecond: number): string {
  return `${formatBytes(bytesPerSecond)}/s`;
}

/** "About 2 min left" style. Returns null when there is not enough data. */
export function formatEta(remainingBytes: number, speed: number): string | null {
  if (speed <= 0 || remainingBytes <= 0) return null;
  const s = remainingBytes / speed;
  // Up to the next 5 seconds, so a short wait still reads "About 5 sec left".
  if (s < 60) return `${Math.ceil(s / 5) * 5} sec`;
  if (s < 3600) return `${Math.round(s / 60)} min`;
  const h = Math.floor(s / 3600);
  const m = Math.round((s % 3600) / 60);
  return m ? `${h} hr ${m} min` : `${h} hr`;
}

export function percent(done: number, total: number): number {
  if (total <= 0) return 0;
  return Math.min(100, Math.floor((done / total) * 100));
}

type Kind = 'photo' | 'video' | 'song' | 'document' | 'archive' | 'file';

export function fileKind(f: Pick<FileItem, 'mime' | 'name'>): Kind {
  const m = f.mime;
  if (m.startsWith('image/')) return 'photo';
  if (m.startsWith('video/')) return 'video';
  if (m.startsWith('audio/')) return 'song';
  if (/zip|rar|7z|tar|gzip/.test(m) || /\.(zip|rar|7z|tar|gz)$/i.test(f.name)) return 'archive';
  if (m.startsWith('text/') || /pdf|document|sheet|presentation|msword/.test(m)) return 'document';
  return 'file';
}

const plural: Record<Kind, [string, string]> = {
  photo: ['photo', 'photos'],
  video: ['video', 'videos'],
  song: ['song', 'songs'],
  document: ['document', 'documents'],
  archive: ['archive', 'archives'],
  file: ['file', 'files'],
};

const count = (n: number, k: Kind) => `${n.toLocaleString('en-US')} ${plural[k][n === 1 ? 0 : 1]}`;

/** "3 photos and 1 video", "12 files", or the file name for a single file. Files inside a sent
 *  folder count as the folder: "Trip folder, 240 files", "2 folders, 31 files", "Trip folder and 3 photos". */
export function summarizeFiles(files: Pick<FileItem, 'mime' | 'name' | 'folder'>[]): string {
  const tops = new Map<string, number>();
  const loose: Pick<FileItem, 'mime' | 'name'>[] = [];
  for (const f of files) {
    const top = f.folder?.split('/')[0];
    if (top) tops.set(top, (tops.get(top) ?? 0) + 1);
    else loose.push(f);
  }
  if (!tops.size) return summarizeLoose(files);
  const inFolders = files.length - loose.length;
  const folders = tops.size === 1 ? `${[...tops.keys()][0]} folder` : `${tops.size.toLocaleString('en-US')} folders`;
  if (!loose.length) return `${folders}, ${count(inFolders, 'file')}`;
  return `${folders} and ${summarizeLoose(loose)}`;
}

/** When every file is inside one sent folder: its name and how many files ("Trip", "240 files"). */
export function singleFolder(files: Pick<FileItem, 'folder'>[]): { name: string; files: string } | null {
  const top = files[0]?.folder?.split('/')[0];
  if (!top || !files.every((f) => f.folder?.split('/')[0] === top)) return null;
  return { name: top, files: count(files.length, 'file') };
}

function summarizeLoose(files: Pick<FileItem, 'mime' | 'name'>[]): string {
  if (files.length === 0) return 'No files';
  if (files.length === 1) return files[0]!.name;
  const groups = new Map<Kind, number>();
  for (const f of files) groups.set(fileKind(f), (groups.get(fileKind(f)) ?? 0) + 1);
  if (groups.size > 2) return count(files.length, 'file');
  const parts = [...groups.entries()].sort((a, b) => b[1] - a[1]).map(([k, n]) => count(n, k));
  return parts.join(' and ');
}

/** The start of a text, on one line, for titles and lists: "Meet at 6 by the…". */
export function textPreview(text: string, max = 48): string {
  const line = text.trim().split('\n').find((l) => l.trim())?.trim() ?? '';
  const flat = line.replace(/\s+/g, ' ');
  return flat.length > max ? `${flat.slice(0, max - 1).trimEnd()}…` : flat;
}

/** The text is one web link and nothing else (the engine's `text::is_link`). */
export function isLink(text: string): boolean {
  const t = text.trim();
  return /^https?:\/\//.test(t) && t.length > 8 && !/\s/.test(t);
}

/** A transfer, request or History entry in a few words: the text's start for text, otherwise
 *  `summarizeFiles`. */
export function summarize(x: { files: Pick<FileItem, 'mime' | 'name' | 'folder'>[]; text?: string | null }): string {
  return x.text ? textPreview(x.text) : summarizeFiles(x.files);
}

/** UTF-8 length, the way the engine counts TEXT_MAX. */
export function utf8Bytes(text: string): number {
  return new TextEncoder().encode(text).length;
}

/** Truncates in the middle so the extension stays visible: "holiday-vid…final.mp4". */
export function middleTruncate(name: string, max = 28): string {
  if (name.length <= max) return name;
  const dot = name.lastIndexOf('.');
  const ext = dot > 0 && name.length - dot <= 6 ? name.slice(dot) : '';
  const keep = max - ext.length - 1;
  const head = Math.ceil(keep * 0.6);
  const tail = keep - head;
  const base = ext ? name.slice(0, dot) : name;
  return `${base.slice(0, head)}…${base.slice(base.length - tail)}${ext}`;
}

/** Fills {placeholders} in a copy string. */
export function fill(template: string, vars: Record<string, string | number> = {}): string {
  return template.replace(/\{(\w+)\}/g, (_, k: string) => String(vars[k] ?? `{${k}}`));
}

/** How long something took: "under a second", "12 sec", "4 min 5 sec", "1 hr 20 min". */
export function formatDuration(ms: number): string {
  const s = Math.round(ms / 1000);
  if (s < 1) return 'under a second';
  if (s < 60) return `${s} sec`;
  if (s < 3600) {
    const m = Math.floor(s / 60);
    const rest = s % 60;
    return rest ? `${m} min ${rest} sec` : `${m} min`;
  }
  const h = Math.floor(s / 3600);
  const m = Math.round((s % 3600) / 60);
  return m ? `${h} hr ${m} min` : `${h} hr`;
}

/** Local midnight before `at`, as epoch ms: the key History groups a day's entries under. */
export function dayStart(at: number): number {
  const d = new Date(at);
  d.setHours(0, 0, 0, 0);
  return d.getTime();
}

/** "Today", "Yesterday", or a date like "Mon 12 Oct" ("Mon 12 Oct 2025" in another year). */
export function dayLabel(at: number, now: number, today: string, yesterday: string): string {
  const day = dayStart(at);
  const start = dayStart(now);
  if (day === start) return today;
  if (day === dayStart(start - 1)) return yesterday;
  const d = new Date(at);
  const sameYear = d.getFullYear() === new Date(now).getFullYear();
  return d.toLocaleDateString(undefined, { weekday: 'short', day: 'numeric', month: 'short', ...(sameYear ? {} : { year: 'numeric' }) });
}

/** The time of day, in the system's own style ("14:05" or "2:05 PM"). */
export function timeOfDay(at: number): string {
  return new Date(at).toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' });
}

/** File types that run something when opened, so the app asks first (Windows, Linux and Android). */
const RUNNABLE = new Set([
  'exe', 'msi', 'msix', 'appx', 'bat', 'cmd', 'com', 'scr', 'pif', 'cpl', 'ps1', 'psm1', 'vbs', 'vbe', 'js', 'jse', 'wsf', 'wsh', 'hta',
  'lnk', 'url', 'reg', 'jar', 'sh', 'bash', 'run', 'bin', 'appimage', 'desktop', 'deb', 'rpm', 'py', 'pl', 'apk', 'xapk', 'apks',
]);

/** Whether opening `name` would run it (a program, script, installer or shortcut). */
export function isRunnable(name: string): boolean {
  const dot = name.lastIndexOf('.');
  return dot > 0 && RUNNABLE.has(name.slice(dot + 1).toLowerCase());
}
