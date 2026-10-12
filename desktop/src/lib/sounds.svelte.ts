// The app's sounds (design/sounds/README.md). In the desktop app the Rust side plays them
// (src-tauri/src/sound.rs: WebKit keeps a page silent until the first click); the browser
// prototype plays the same files itself. Every play is logged so tests and testers can see it.
//
// Taps: any click on something you can press plays `tap`, unless that control has its own sound
// (a `data-sound` attribute, or code that plays one while handling the click). The tap waits one
// turn of the event loop, and any other sound in that time replaces it, so a press never makes
// two sounds.
import { invoke } from '@tauri-apps/api/core';
import type { SoundName } from '../../../design/sounds/build/sounds.js';
import { sounds as all } from '../../../design/sounds/build/sounds.js';
import { isTauri } from './engine/tauri';
import { prefs } from './state/prefs.svelte';

export type { SoundName };

/** The same sound again this soon is one event seen twice (a button and the code behind it). */
const REPEAT_MS = 400;
/** Soft plays everything at this share of Loud. */
export const SOFT = 0.4;
const volumeOf = () => (prefs.sounds === 'loud' ? 1 : SOFT);

// Only the browser prototype plays the files itself. The desktop build leaves them out: the app
// already carries them in sound.rs.
const files = (import.meta.env.TAURI_ENV_PLATFORM ? {} : import.meta.glob('../../../design/sounds/build/res/raw/*.wav', { query: '?url', import: 'default' })) as Record<string, () => Promise<string>>;
const urlFor = (name: SoundName) => files[`../../../design/sounds/build/res/raw/${all[name].file}`]?.();

/** Plays in the page, for the browser prototype. */
class WebAudio {
  private ctx: AudioContext | null = null;
  private buffers = new Map<SoundName, Promise<AudioBuffer>>();

  play(name: SoundName, volume: number): void {
    try {
      this.ctx ??= new AudioContext();
      const ctx = this.ctx;
      if (ctx.state === 'suspended') void ctx.resume();
      const url = urlFor(name);
      if (!url) return;
      let b = this.buffers.get(name);
      if (!b) {
        b = url.then((u) => fetch(u)).then((r) => r.arrayBuffer()).then((a) => ctx.decodeAudioData(a));
        this.buffers.set(name, b);
      }
      void b.then((buffer) => {
        const src = ctx.createBufferSource();
        src.buffer = buffer;
        const gain = ctx.createGain();
        gain.gain.value = volume;
        src.connect(gain).connect(ctx.destination);
        src.start();
      }).catch(() => {});
    } catch {
      // No audio here (a test browser, or blocked): the log still shows what would play.
    }
  }
}

export class SoundPlayer {
  entries = $state<{ id: number; name: SoundName; at: number }[]>([]);
  private n = 0;
  private pendingTap: ReturnType<typeof setTimeout> | null = null;
  private last = new Map<SoundName, number>();

  constructor(
    private out: (name: SoundName, volume: number) => void,
    private enabled: () => boolean = () => prefs.sounds !== 'off',
    private volume: () => number = volumeOf,
  ) {}

  play(name: SoundName): void {
    if (name === 'tap') {
      // Whether to play is decided when it plays: the press may be the one that changes the
      // level, and then it's heard at the new one (or not at all, for Off).
      this.pendingTap ??= setTimeout(() => {
        this.pendingTap = null;
        if (this.enabled()) this.emit('tap');
      }, 0);
      return;
    }
    if (!this.enabled()) return;
    if (this.pendingTap) {
      clearTimeout(this.pendingTap);
      this.pendingTap = null;
    }
    const now = Date.now();
    if (now - (this.last.get(name) ?? -Infinity) < REPEAT_MS) return;
    this.last.set(name, now);
    this.emit(name);
  }

  private emit(name: SoundName): void {
    this.entries = [{ id: ++this.n, name, at: Date.now() }, ...this.entries].slice(0, 12);
    this.out(name, this.volume());
  }
}

function output(): (name: SoundName, volume: number) => void {
  if (isTauri()) return (name, volume) => void invoke('play_sound', { name, volume }).catch(() => {});
  const web = new WebAudio();
  return (name, volume) => web.play(name, volume);
}

export const sounds = new SoundPlayer(output());
export const playSound = (name: SoundName) => sounds.play(name);

/** Things you can press. Their click plays `tap` unless they say otherwise. */
const PRESSABLE = 'button, a[href], [role="button"], [role="switch"], [role="tab"], [role="radio"], [role="option"], summary, label, select';

/** One listener for the whole window: every press gets a sound. Returns a function that removes it. */
export function listenForTaps(root: Document = document): () => void {
  const onClick = (e: MouseEvent) => {
    const el = (e.target as Element | null)?.closest?.(PRESSABLE);
    if (!el || (el as HTMLButtonElement).disabled || el.getAttribute('aria-disabled') === 'true') return;
    const own = el.closest('[data-sound]')?.getAttribute('data-sound') as SoundName | 'none' | null;
    if (own === 'none') return;
    playSound(own && own in all ? own : 'tap');
  };
  // Capture, so the tap is queued before the control's own handler can replace it.
  root.addEventListener('click', onClick, true);
  return () => root.removeEventListener('click', onClick, true);
}
