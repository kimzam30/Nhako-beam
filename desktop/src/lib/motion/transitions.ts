// Svelte transitions driven by the token springs. Entrances use springs; exits use a quick
// fade plus a short move (exits are quicker than entrances). Reduced motion turns every
// transition into a plain fade.
import { cubicOut } from 'svelte/easing';
import type { TransitionConfig } from 'svelte/transition';
import { distance, duration as dur, easing as ease, scale as scaleTokens, type SpringName } from '../tokens';
import { prefs } from '../state/prefs.svelte';
import { springEasing } from './physics';

const bezier = (p: readonly [number, number, number, number]) => {
  // Small cubic bezier solver (x -> y) for token easings.
  const [x1, y1, x2, y2] = p;
  const cx = 3 * x1, bx = 3 * (x2 - x1) - cx, ax = 1 - cx - bx;
  const cy = 3 * y1, by = 3 * (y2 - y1) - cy, ay = 1 - cy - by;
  const sx = (t: number) => ((ax * t + bx) * t + cx) * t;
  const sy = (t: number) => ((ay * t + by) * t + cy) * t;
  const dx = (t: number) => (3 * ax * t + 2 * bx) * t + cx;
  return (x: number) => {
    let t = x;
    for (let i = 0; i < 6; i++) {
      const d = dx(t);
      if (Math.abs(d) < 1e-6) break;
      t -= (sx(t) - x) / d;
    }
    return sy(Math.min(1, Math.max(0, t)));
  };
};

export const easeStandard = bezier(ease.standard);
export const easeEnter = bezier(ease.enter);
export const easeExit = bezier(ease.exit);

function reducedFade(ms: number = dur.fast): TransitionConfig {
  return { duration: ms, easing: cubicOut, css: (t) => `opacity: ${t}` };
}

export interface SpringInParams {
  y?: number;
  x?: number;
  scale?: number;
  opacity?: number;
  preset?: SpringName;
  delay?: number;
}

/** Entrance: moves from an offset and scale into place on a spring, fading in quickly. */
export function springIn(_node: Element, p: SpringInParams = {}): TransitionConfig {
  if (prefs.reducedMotion) return { ...reducedFade(dur.base), delay: (p.delay ?? 0) / 2 };
  const { duration, easing } = springEasing(p.preset ?? 'smooth');
  const y = p.y ?? distance.enterOffset;
  const x = p.x ?? 0;
  const s0 = p.scale ?? scaleTokens.enterFrom;
  const o0 = p.opacity ?? 0;
  const fadePart = Math.min(1, dur.base / duration);
  return {
    duration,
    delay: p.delay ?? 0,
    easing: (t) => t, // We ease per property below so opacity does not overshoot.
    css: (t) => {
      const e = easing(t);
      const o = o0 + (1 - o0) * easeEnter(Math.min(1, t / fadePart));
      return `opacity: ${o}; transform: translate(${(1 - e) * x}px, ${(1 - e) * y}px) scale(${s0 + (1 - s0) * e});`;
    },
  };
}

export interface QuickOutParams {
  y?: number;
  x?: number;
  scale?: number;
  duration?: number;
}

/** Exit: short move plus fade, using the exit easing. */
export function quickOut(_node: Element, p: QuickOutParams = {}): TransitionConfig {
  if (prefs.reducedMotion) return reducedFade(dur.fast);
  const y = p.y ?? 8;
  const x = p.x ?? 0;
  const s1 = p.scale ?? 0.98;
  return {
    duration: p.duration ?? dur.fast,
    easing: easeExit,
    css: (t, u) => `opacity: ${t}; transform: translate(${u * x}px, ${u * y}px) scale(${1 - u * (1 - s1)});`,
  };
}

/** Plain fade with token timing. */
export function fadeT(_node: Element, p: { duration?: number; delay?: number } = {}): TransitionConfig {
  return { duration: p.duration ?? dur.base, delay: p.delay ?? 0, easing: easeStandard, css: (t) => `opacity: ${t}` };
}

/** Bottom sheet / dialog: rises from below on the smooth spring. */
export function sheetIn(node: Element, p: { dialog?: boolean } = {}): TransitionConfig {
  if (prefs.reducedMotion) return reducedFade(dur.base);
  if (p.dialog) return springIn(node, { y: 16, scale: 0.96, preset: 'smooth' });
  const { duration, easing } = springEasing('smooth');
  return { duration, easing: (t) => t, css: (t) => `transform: translateY(${(1 - easing(t)) * 100}%);` };
}

export function sheetOut(node: Element, p: { dialog?: boolean } = {}): TransitionConfig {
  if (prefs.reducedMotion) return reducedFade(dur.fast);
  if (p.dialog) return quickOut(node, { y: 8, scale: 0.97 });
  return { duration: dur.slow, easing: easeExit, css: (_t, u) => `transform: translateY(${u * 100}%);` };
}

/** Stagger delay for the i-th item (30 ms steps, capped at 6 items; halved in reduced motion). */
export function stagger(i: number, step = 30, max = 6): number {
  const d = Math.min(i, max - 1) * step;
  return prefs.reducedMotion ? d / 2 : d;
}

/** Svelte animate: FLIP with the smooth spring. */
export function springFlip(_node: Element, { from, to }: { from: DOMRect; to: DOMRect }) {
  const dx = from.left - to.left;
  const dy = from.top - to.top;
  if (prefs.reducedMotion || (dx === 0 && dy === 0)) return { duration: 0 };
  const { duration, easing } = springEasing('smooth');
  return {
    duration,
    easing: (t: number) => t,
    css: (t: number) => {
      const e = easing(t);
      return `transform: translate(${(1 - e) * dx}px, ${(1 - e) * dy}px);`;
    },
  };
}

/** Opens or closes a block by its height (no overshoot, so nothing below jumps), fading the content. */
export function expand(node: Element, p: { delay?: number } = {}): TransitionConfig {
  if (prefs.reducedMotion) return reducedFade(dur.base);
  const el = node as HTMLElement;
  const style = getComputedStyle(el);
  const h = el.offsetHeight;
  const pt = parseFloat(style.paddingTop), pb = parseFloat(style.paddingBottom);
  const mt = parseFloat(style.marginTop), mb = parseFloat(style.marginBottom);
  const { duration } = springEasing('smooth');
  return {
    duration,
    delay: p.delay ?? 0,
    easing: easeStandard,
    css: (t) =>
      `overflow: hidden; height: ${t * h}px; padding-top: ${t * pt}px; padding-bottom: ${t * pb}px; ` +
      `margin-top: ${t * mt}px; margin-bottom: ${t * mb}px; opacity: ${easeEnter(Math.min(1, t * 1.6))};`,
  };
}
