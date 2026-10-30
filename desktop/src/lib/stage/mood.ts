// Which state the mascot is in, from what the app knows. The Kotlin twin is
// android/.../stage/MascotMood.kt; both are tested with the same cases.

import { mascot as timing } from '../tokens';
import type { MascotState } from './pose';

export interface MoodInput {
  now: number;
  /** The engine has started (before that: loading). */
  started: boolean;
  offline: boolean;
  searching: boolean;
  /** Devices on the stage. */
  devices: number;
  sending: boolean;
  receiving: boolean;
  /** Something the mascot is looking at: the pointer over a device, or files dragged near one. */
  looking: boolean;
  moments: { found: number; done: number; failed: number };
}

/** Moments come first while they last, then what's going on, then rest. */
export function mascotMood(i: MoodInput): MascotState {
  const recent = (at: number, ms: number) => at > 0 && i.now - at < ms;
  if (!i.started) return 'loading';
  if (i.offline) return 'offline';
  if (recent(i.moments.failed, timing.failedMs)) return 'failed';
  if (recent(i.moments.done, timing.doneMs)) return 'done';
  if (i.sending) return 'sending';
  if (i.receiving) return 'receiving';
  if (recent(i.moments.found, timing.foundMs)) return 'happy';
  if (i.looking) return 'looking';
  if (i.devices === 0) return i.searching ? 'looking' : 'asleep';
  return 'idle';
}

/** When the mood will next change by itself (a moment running out), so the caller can check again. */
export function nextChange(i: MoodInput): number | null {
  const ends = [i.moments.failed + timing.failedMs, i.moments.done + timing.doneMs, i.moments.found + timing.foundMs].filter((at) => at > i.now);
  return ends.length ? Math.min(...ends) : null;
}
