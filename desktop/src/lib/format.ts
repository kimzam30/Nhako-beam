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
  if (s < 10) return 'a few seconds';
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

const count = (n: number, k: Kind) => `${n} ${plural[k][n === 1 ? 0 : 1]}`;

/** "3 photos and 1 video", "12 files", or the file name for a single file. */
export function summarizeFiles(files: Pick<FileItem, 'mime' | 'name'>[]): string {
  if (files.length === 0) return 'No files';
  if (files.length === 1) return files[0]!.name;
  const groups = new Map<Kind, number>();
  for (const f of files) groups.set(fileKind(f), (groups.get(fileKind(f)) ?? 0) + 1);
  if (groups.size > 2) return count(files.length, 'file');
  const parts = [...groups.entries()].sort((a, b) => b[1] - a[1]).map(([k, n]) => count(n, k));
  return parts.join(' and ');
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
