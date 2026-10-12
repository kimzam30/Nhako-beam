// Drag sessions for file stacks. Two sources feed the same magnet logic:
//  - 'os':    files dragged in from the file manager (desktop)
//  - 'stack': the staged file stack dragged with a pointer (desktop and Android)
import { magnet, type MagnetTarget } from './motion/physics';
import { app } from './state/app.svelte';
import { playHaptic } from './haptics.svelte';
import { playSound } from './sounds.svelte';

function targets(): MagnetTarget[] {
  const out: MagnetTarget[] = [];
  for (const [id, el] of app.orbEls) {
    const r = el.getBoundingClientRect();
    if (r.width === 0) continue;
    out.push({ id, x: r.left + r.width / 2, y: r.top + r.height / 2 });
  }
  return out;
}

export function beginDrag(source: 'os' | 'stack', x: number, y: number, count: number): void {
  app.drag = { source, x, y, count, nearId: null, snappedId: null };
  if (source === 'stack') {
    playHaptic('dragStart');
    playSound('lift');
  }
  moveDrag(x, y);
}

export function moveDrag(x: number, y: number, count?: number): void {
  const d = app.drag;
  if (!d) return;
  const r = magnet({ x, y }, targets(), d.snappedId);
  if (r.snappedId && r.snappedId !== d.snappedId) {
    playHaptic('magnetSnap');
    playSound('snap');
  }
  if (!r.snappedId && d.snappedId) playHaptic('thresholdDeactivate');
  app.drag = { ...d, x, y, count: count ?? d.count, nearId: r.nearId, snappedId: r.snappedId };
}

/** Where the dragged stack should be drawn right now (pointer plus magnet pull). */
export function drawnPosition(): { x: number; y: number } | null {
  const d = app.drag;
  if (!d) return null;
  return magnet({ x: d.x, y: d.y }, targets(), d.snappedId).drawn;
}

export function orbCenter(id: string): { x: number; y: number } | null {
  const el = app.orbEls.get(id);
  if (!el) return null;
  const r = el.getBoundingClientRect();
  return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
}

/** Ends the drag. Returns the device it was dropped on, if any. */
export function endDrag(): string | null {
  const d = app.drag;
  app.drag = null;
  return d?.snappedId ?? null;
}
