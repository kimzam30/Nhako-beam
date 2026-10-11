// Haptics for the prototype. The real Android app calls NhakoHaptic.X.perform(view).
// Here we use navigator.vibrate where it exists (Android Chrome), and always log the event so
// testers on desktop can see which haptic would fire.
import { haptic } from './tokens';

export type HapticName = keyof typeof haptic;

// Rough vibration equivalents of the Android constants (ms on, off, on...).
const patterns: Partial<Record<HapticName, number | number[]>> = {
  dragStart: 12,
  thresholdActivate: 8,
  thresholdDeactivate: 5,
  magnetSnap: 8,
  accept: [10, 40, 14],
  decline: [24, 50, 24],
  transferDone: [10, 40, 14],
  transferFailed: [24, 50, 24],
  pairMatched: [10, 40, 14],
  toggleOn: 6,
  toggleOff: 4,
};

class HapticLog {
  entries = $state<{ id: number; name: HapticName; at: number }[]>([]);
  private n = 0;

  play(name: HapticName): void {
    const spec = haptic[name];
    if (spec.android === 'none') return;
    this.entries = [{ id: ++this.n, name, at: Date.now() }, ...this.entries].slice(0, 8);
    const p = patterns[name];
    try {
      if (p !== undefined && 'vibrate' in navigator) navigator.vibrate(p);
    } catch {
      // Some browsers throw if vibrate is called without a user gesture. Not important.
    }
  }
}

export const haptics = new HapticLog();
export const playHaptic = (name: HapticName) => haptics.play(name);
