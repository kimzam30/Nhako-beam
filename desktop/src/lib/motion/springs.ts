// Interruptible, velocity-preserving spring values built on Motion.
// Use for anything a gesture drives (drag, swipe, magnet). Retargeting mid-flight keeps
// the current velocity, so motion never jumps or restarts.
import { animate } from 'motion';
import { spring, type SpringName } from '../tokens';
import { prefs } from '../state/prefs.svelte';

type Controls = ReturnType<typeof animate>;

export class SpringValue {
  private controls: Controls | null = null;
  private lastT = 0;
  private lastV = 0;
  value: number;
  velocity = 0;

  constructor(
    initial: number,
    private onChange: (v: number) => void,
  ) {
    this.value = initial;
  }

  /** Jump to a value with no animation (for 1:1 finger tracking). */
  set(v: number, velocity = 0): void {
    this.stop();
    this.value = v;
    this.velocity = velocity;
    this.onChange(v);
  }

  /** Animate to `target`. Starts from the current value and velocity unless one is given. */
  to(target: number, preset: SpringName = 'smooth', velocity?: number): Promise<void> {
    this.stop();
    const from = this.value;
    const v0 = velocity ?? this.velocity;
    if (prefs.reducedMotion) {
      // Reduced motion: no travel. The caller fades instead.
      this.set(target);
      return Promise.resolve();
    }
    const s = spring[preset];
    this.lastT = performance.now();
    this.lastV = from;
    return new Promise((resolve) => {
      this.controls = animate(from, target, {
        type: 'spring',
        stiffness: s.stiffness,
        damping: s.damping,
        mass: s.mass,
        velocity: v0,
        restDelta: 0.01,
        restSpeed: 1,
        onUpdate: (v: number) => {
          const now = performance.now();
          const dt = (now - this.lastT) / 1000;
          if (dt > 0) this.velocity = (v - this.lastV) / dt;
          this.lastT = now;
          this.lastV = v;
          this.value = v;
          this.onChange(v);
        },
        onComplete: () => {
          this.velocity = 0;
          this.controls = null;
          resolve();
        },
      });
      this.controls.then(() => resolve(), () => resolve());
    });
  }

  stop(): void {
    if (this.controls) {
      this.controls.stop();
      this.controls = null;
    }
  }

  get animating(): boolean {
    return this.controls !== null;
  }
}

/** Plays a short horizontal shake (wrong code, failed transfer) using the snappy spring. */
export async function shake(el: HTMLElement, distance = 6, cycles = 3): Promise<void> {
  if (prefs.reducedMotion) return;
  const v = new SpringValue(0, (x) => (el.style.translate = `${x}px 0`));
  for (let i = 0; i < cycles; i++) {
    const d = distance * (1 - i / (cycles + 1));
    await Promise.race([v.to(d, 'snappy'), wait(60)]);
    await Promise.race([v.to(-d, 'snappy'), wait(60)]);
  }
  await v.to(0, 'snappy');
  el.style.translate = '';
}

/** Animates element properties with a token spring using Motion (WAAPI, GPU friendly). */
export function springAnimate(
  el: Element,
  keyframes: Record<string, number | string | (number | string)[]>,
  preset: SpringName = 'smooth',
): Controls {
  const s = spring[preset];
  return animate(el, keyframes, { type: 'spring', stiffness: s.stiffness, damping: s.damping, mass: s.mass });
}

export const wait = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));
