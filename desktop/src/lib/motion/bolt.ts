// The bolt from the logo, as motion: its shape, the morph into a check, the arc between two
// points, and the sparks thrown when something commits. "Charge, zap, settle".
// The Kotlin twin is android/.../ui/components/Bolt.kt. Spec: docs/design/motion-spec.md, 4.15.
import { bolt as tokens, brand } from '../tokens';
import { prefs } from '../state/prefs.svelte';

type Pt = readonly [number, number];

/** The bolt's seven corners, in a 1024 box: the vector bolt from the first logo, kept as the
 * motion shape now that the logo is the pixel mascot. */
export const BOLT: readonly Pt[] = [
  [676.16, 161.6],
  [829.76, 244.16],
  [608.96, 464.96],
  [743.36, 503.36],
  [194.24, 862.4],
  [436.16, 574.4],
  [301.76, 536],
];

/** A thick check with the same seven corners in the same order, so the bolt morphs into it. */
export const CHECK: readonly Pt[] = [
  [774, 248],
  [866, 332],
  [648, 570],
  [430, 807],
  [166, 544],
  [254, 456],
  [430, 633],
];

/** The two halves of a cracked bolt: the line runs through the bolt's own zig (corners 2 and 5). */
export const CRACK_LINE: readonly [Pt, Pt] = [
  [0, 640],
  [1024, 380],
];

/** The shape between the bolt (0) and the check (1), as an SVG path in the 1024 box. */
export function shapePath(morph = 0): string {
  const m = Math.max(0, Math.min(1, morph));
  return (
    BOLT.map(([x, y], i) => {
      const [cx, cy] = CHECK[i]!;
      return `${i ? 'L' : 'M'}${(x + (cx - x) * m).toFixed(1)},${(y + (cy - y) * m).toFixed(1)}`;
    }).join(' ') + ' Z'
  );
}

/**
 * A jagged lightning line from a to b: alternate corners pushed off the straight line, smaller
 * near the ends so it leaves and lands cleanly. `seed` picks the zig pattern.
 */
export function arcPath(a: { x: number; y: number }, b: { x: number; y: number }, seed = 0, amp = 7): string {
  const dx = b.x - a.x, dy = b.y - a.y;
  const len = Math.hypot(dx, dy) || 1;
  const nx = -dy / len, ny = dx / len;
  const steps = Math.max(3, Math.min(8, Math.round(len / 22)));
  let d = `M${a.x.toFixed(1)},${a.y.toFixed(1)}`;
  for (let i = 1; i < steps; i++) {
    const f = i / steps;
    const side = (i + seed) % 2 ? 1 : -1;
    const k = amp * Math.sin(Math.PI * f) * (0.7 + 0.3 * (((i * 7 + seed * 3) % 5) / 4));
    d += ` L${(a.x + dx * f + nx * k * side).toFixed(1)},${(a.y + dy * f + ny * k * side).toFixed(1)}`;
  }
  return `${d} L${b.x.toFixed(1)},${b.y.toFixed(1)}`;
}

/** One small bolt as inline SVG, yellow with the logo's dark blue offset under it. */
function shard(size: number): string {
  const d = shapePath(0);
  return (
    `<svg viewBox="150 120 760 780" width="${size}" height="${size}" aria-hidden="true">` +
    `<path d="${d}" transform="translate(40 50)" fill="${brand.navy}" stroke="${brand.navy}" stroke-width="40" stroke-linejoin="round"/>` +
    `<path d="${d}" fill="${brand.yellow}" stroke="${brand.yellow}" stroke-width="40" stroke-linejoin="round"/></svg>`
  );
}

let layer: HTMLElement | null = null;
function sparkLayer(): HTMLElement {
  if (layer?.isConnected) return layer;
  layer = document.createElement('div');
  layer.className = 'spark-layer';
  layer.setAttribute('aria-hidden', 'true');
  Object.assign(layer.style, { position: 'fixed', inset: '0', pointerEvents: 'none', zIndex: '2147483000', overflow: 'hidden' });
  document.body.append(layer);
  return layer;
}

export interface SparkOptions {
  count?: number;
  reach?: number;
  size?: number;
  /** Throw them in one direction (radians, 0 = right) within a 120 degree fan instead of all round. */
  toward?: number;
}

let seq = 0;

/**
 * Throws a handful of tiny bolts out from a point (a click, or the center of an element) on top
 * of everything. Transform and opacity only; gone in about 300 ms. Reduced motion: nothing.
 */
export function spark(at: Element | { x: number; y: number }, o: SparkOptions = {}): void {
  if (prefs.reducedMotion || typeof document === 'undefined') return;
  let x: number, y: number;
  if ('getBoundingClientRect' in at) {
    const r = at.getBoundingClientRect();
    x = r.left + r.width / 2;
    y = r.top + r.height / 2;
  } else ({ x, y } = at);
  const count = o.count ?? tokens.sparkCount;
  const reach = o.reach ?? tokens.sparkReach;
  const size = o.size ?? tokens.sparkSize;
  const burst = document.createElement('div');
  burst.dataset.testid = 'spark';
  Object.assign(burst.style, { position: 'absolute', left: `${x}px`, top: `${y}px`, width: '0', height: '0' });
  const turn = (seq++ * 0.9) % (Math.PI * 2);
  const done: Promise<unknown>[] = [];
  for (let i = 0; i < count; i++) {
    const s = document.createElement('span');
    s.innerHTML = shard(size * (0.75 + 0.25 * ((i * 3) % 4) / 3));
    Object.assign(s.style, { position: 'absolute', left: `${-size / 2}px`, top: `${-size / 2}px`, display: 'block', lineHeight: '0' });
    burst.append(s);
    const a = o.toward === undefined ? turn + (i / count) * Math.PI * 2 : o.toward + ((i / Math.max(1, count - 1)) - 0.5) * (Math.PI * 2) / 3;
    const r = reach * (0.8 + 0.2 * ((i * 5) % 3) / 2);
    const tilt = (a * 180) / Math.PI + 90;
    const anim = s.animate(
      [
        // Out fast and full size, then shrink and fade the last stretch.
        { transform: `translate(0px, 0px) rotate(${tilt}deg) scale(0.5)`, opacity: 1, easing: 'cubic-bezier(0, 0, 0, 1)' },
        { transform: `translate(${Math.cos(a) * r * 0.8}px, ${Math.sin(a) * r * 0.8}px) rotate(${tilt}deg) scale(1)`, opacity: 1, offset: 0.6 },
        { transform: `translate(${Math.cos(a) * r}px, ${Math.sin(a) * r}px) rotate(${tilt}deg) scale(0.6)`, opacity: 0 },
      ],
      { duration: tokens.sparkMs, easing: 'linear', fill: 'forwards' },
    );
    done.push(anim.finished.catch(() => undefined));
  }
  sparkLayer().append(burst);
  void Promise.all(done).then(() => burst.remove());
}

/** Where a pointer event happened, or the element's center when it came from the keyboard. */
export function pointOf(e: MouseEvent | undefined, el: Element): Element | { x: number; y: number } {
  return e && (e.clientX || e.clientY) && e.detail > 0 ? { x: e.clientX, y: e.clientY } : el;
}

/** The yellow rim flash on release: opacity only, so it stays in reduced motion. */
export function flash(rim: Element | undefined | null): void {
  if (!rim) return;
  const keys = prefs.reducedMotion ? [{ opacity: 0.8 }, { opacity: 0 }] : [{ opacity: 0.9, transform: 'scale(0.98)' }, { opacity: 0, transform: 'scale(1.06)' }];
  rim.animate(keys, { duration: tokens.flashMs, easing: 'cubic-bezier(0, 0, 0, 1)' });
}

/** Draws a path in along its length, holds, then fades it. For arcs and the done check. */
export function strike(path: SVGPathElement, delay = 0): Promise<void> {
  if (prefs.reducedMotion) {
    path.style.opacity = '0';
    return Promise.resolve();
  }
  const len = path.getTotalLength?.() || 200;
  path.style.strokeDasharray = `${len}`;
  path.style.strokeDashoffset = `${len}`;
  const ms = tokens.arcMs;
  const anim = path.animate(
    [
      { strokeDashoffset: len, opacity: 1 },
      { strokeDashoffset: 0, opacity: 1, offset: 0.4 },
      { strokeDashoffset: 0, opacity: 1, offset: 0.6 },
      { strokeDashoffset: 0, opacity: 0 },
    ],
    { duration: ms, delay, easing: 'linear', fill: 'both' },
  );
  return anim.finished.then(
    () => undefined,
    () => undefined,
  );
}
