// Pure motion and gesture math. No DOM, no timers, so it is unit tested and can be
// ported 1:1 to Kotlin for the Android app.
import { gesture, spring, type SpringName } from '../tokens';

/** Position of a unit spring (0 -> 1, starting at rest) at time `t` seconds. Mass 1. */
export function springPosition(dampingRatio: number, stiffness: number): (t: number) => number {
  const w0 = Math.sqrt(stiffness);
  const z = dampingRatio;
  if (z < 1) {
    const wd = w0 * Math.sqrt(1 - z * z);
    const b = -(z * w0) / wd;
    return (t) => 1 + Math.exp(-z * w0 * t) * (-Math.cos(wd * t) + b * Math.sin(wd * t));
  }
  if (z === 1) return (t) => 1 + (-1 - w0 * t) * Math.exp(-w0 * t);
  const s = Math.sqrt(z * z - 1);
  const r1 = -w0 * (z - s);
  const r2 = -w0 * (z + s);
  const c2 = r1 / (r2 - r1);
  const c1 = -1 - c2;
  return (t) => 1 + c1 * Math.exp(r1 * t) + c2 * Math.exp(r2 * t);
}

/** Easing function (0..1 in, may overshoot 1 out) for a named spring, plus its duration. */
export function springEasing(name: SpringName): { duration: number; easing: (p: number) => number } {
  const s = spring[name];
  const pos = springPosition(s.dampingRatio, s.stiffness);
  const seconds = s.settleMs / 1000;
  return { duration: s.settleMs, easing: (p) => (p >= 1 ? 1 : pos(p * seconds)) };
}

/** Apple style resistance past an edge. `overshoot` is how far past the edge the pointer is. */
export function rubberBand(overshoot: number, dimension: number, coefficient: number = gesture.rubberBandCoefficient): number {
  if (dimension <= 0) return 0;
  const sign = Math.sign(overshoot);
  const x = Math.abs(overshoot);
  return sign * (1 - 1 / ((x * coefficient) / dimension + 1)) * dimension;
}

/** Clamps `value` into [min, max], rubber banding anything outside. */
export function rubberClamp(value: number, min: number, max: number, dimension: number): number {
  if (value < min) return min + rubberBand(value - min, dimension);
  if (value > max) return max + rubberBand(value - max, dimension);
  return value;
}

export interface Point {
  x: number;
  y: number;
}

export interface MagnetTarget extends Point {
  id: string;
}

export interface MagnetResult {
  /** Where the dragged item should be drawn. */
  drawn: Point;
  /** Target the item leans toward (inside magnet radius), if any. */
  nearId: string | null;
  /** Target the item is locked onto (inside snap radius), if any. */
  snappedId: string | null;
}

/**
 * Magnetic pull toward the nearest target.
 * `previousSnapId` adds hysteresis so the lock does not flicker on the edge.
 */
export function magnet(
  pointer: Point,
  targets: MagnetTarget[],
  previousSnapId: string | null = null,
  opts = { radius: gesture.magnetRadius, snapRadius: gesture.magnetSnapRadius, pull: gesture.magnetPull, release: 12 },
): MagnetResult {
  let best: MagnetTarget | null = null;
  let bestDist = Infinity;
  for (const t of targets) {
    const d = Math.hypot(t.x - pointer.x, t.y - pointer.y);
    if (d < bestDist) {
      best = t;
      bestDist = d;
    }
  }
  if (!best || bestDist > opts.radius) return { drawn: { ...pointer }, nearId: null, snappedId: null };

  const snapLimit = best.id === previousSnapId ? opts.snapRadius + opts.release : opts.snapRadius;
  if (bestDist <= snapLimit) return { drawn: { x: best.x, y: best.y }, nearId: best.id, snappedId: best.id };

  const strength = opts.pull * (1 - bestDist / opts.radius);
  return {
    drawn: { x: pointer.x + (best.x - pointer.x) * strength, y: pointer.y + (best.y - pointer.y) * strength },
    nearId: best.id,
    snappedId: null,
  };
}

export type SwipeOutcome = 'accept' | 'decline' | 'cancel';

/** Decides what a released horizontal swipe does. Right = accept, left = decline. */
export function swipeDecision(offsetX: number, velocityX: number, width: number): SwipeOutcome {
  const fraction = offsetX / width;
  const fast = Math.abs(velocityX) >= gesture.swipeCommitVelocity;
  // A fast flick wins, unless it goes against a big drag (user changed their mind).
  if (fast && Math.sign(velocityX) === Math.sign(offsetX || velocityX)) return velocityX > 0 ? 'accept' : 'decline';
  if (fraction >= gesture.swipeCommitFraction && velocityX > -gesture.swipeCommitVelocity) return 'accept';
  if (fraction <= -gesture.swipeCommitFraction && velocityX < gesture.swipeCommitVelocity) return 'decline';
  return 'cancel';
}

/** True when the drag is past the commit distance (used for the threshold haptic). */
export function pastSwipeThreshold(offsetX: number, width: number): boolean {
  return Math.abs(offsetX / width) >= gesture.swipeCommitFraction;
}

/** Tracks pointer velocity over a short window (px per second). */
export class VelocityTracker {
  private samples: { t: number; x: number; y: number }[] = [];
  constructor(private windowMs = 100) {}

  reset(): void {
    this.samples = [];
  }

  add(t: number, x: number, y: number): void {
    this.samples.push({ t, x, y });
    const cutoff = t - this.windowMs;
    while (this.samples.length > 2 && this.samples[0]!.t < cutoff) this.samples.shift();
  }

  velocity(): Point {
    const n = this.samples.length;
    if (n < 2) return { x: 0, y: 0 };
    const a = this.samples[0]!;
    const b = this.samples[n - 1]!;
    const dt = (b.t - a.t) / 1000;
    if (dt <= 0) return { x: 0, y: 0 };
    return { x: (b.x - a.x) / dt, y: (b.y - a.y) / dt };
  }
}

/** Places `count` items on rings around a center. Returns offsets from the center. */
export function radarSlots(count: number, size: number): Point[] {
  const inner = Math.min(count, 6);
  const outer = Math.max(0, count - 6);
  const slots: Point[] = [];
  const r1 = size * 0.36;
  const r2 = size * 0.47;
  for (let i = 0; i < inner; i++) {
    // Start at the top-left so the first device sits where the eye lands first.
    const a = -Math.PI * 0.75 + (i * 2 * Math.PI) / Math.max(inner, 1);
    slots.push({ x: Math.cos(a) * r1, y: Math.sin(a) * r1 });
  }
  for (let i = 0; i < outer; i++) {
    const a = -Math.PI * 0.5 + Math.PI / Math.max(outer, 1) / 2 + (i * 2 * Math.PI) / Math.max(outer, 1);
    slots.push({ x: Math.cos(a) * r2, y: Math.sin(a) * r2 });
  }
  return slots;
}
