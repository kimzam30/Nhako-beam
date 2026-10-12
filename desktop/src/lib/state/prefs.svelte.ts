// User preferences that change how the UI looks, moves and sounds.
// Theme and motion follow the system by default, with an in-app override. Sounds are soft by default.

export type ThemePref = 'system' | 'light' | 'dark';
export type MotionPref = 'system' | 'full' | 'reduced';
export type SoundPref = 'loud' | 'soft' | 'off';

const KEY = 'nhako.prefs.v1';

type Saved = { theme: ThemePref; motion: MotionPref; sounds: SoundPref };
const DEFAULTS: Saved = { theme: 'system', motion: 'system', sounds: 'soft' };

function load(): Saved {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) {
      const saved = { ...DEFAULTS, ...JSON.parse(raw) };
      // Sounds was an on/off switch before Loud, Soft and Off.
      if (typeof saved.sounds === 'boolean') saved.sounds = saved.sounds ? 'soft' : 'off';
      return saved;
    }
  } catch {
    // Storage can be blocked; defaults are fine.
  }
  return DEFAULTS;
}

class Prefs {
  theme = $state<ThemePref>('system');
  motion = $state<MotionPref>('system');
  sounds = $state<SoundPref>('soft');
  systemReduced = $state(false);
  systemDark = $state(false);

  reducedMotion = $derived(this.motion === 'reduced' || (this.motion === 'system' && this.systemReduced));
  dark = $derived(this.theme === 'dark' || (this.theme === 'system' && this.systemDark));

  init(): void {
    const saved = load();
    this.theme = saved.theme;
    this.motion = saved.motion;
    this.sounds = saved.sounds;
    const rm = matchMedia('(prefers-reduced-motion: reduce)');
    const dm = matchMedia('(prefers-color-scheme: dark)');
    this.systemReduced = rm.matches;
    this.systemDark = dm.matches;
    // The tray popover is a second window on the same storage: follow changes made in the other.
    addEventListener('storage', (e) => {
      if (e.key !== KEY) return;
      const next = load();
      this.theme = next.theme;
      this.motion = next.motion;
      this.sounds = next.sounds;
    });
    rm.addEventListener('change', (e) => (this.systemReduced = e.matches));
    dm.addEventListener('change', (e) => (this.systemDark = e.matches));
  }

  save(): void {
    try {
      localStorage.setItem(KEY, JSON.stringify({ theme: this.theme, motion: this.motion, sounds: this.sounds }));
    } catch {
      // Ignore: prefs just will not persist.
    }
  }
}

export const prefs = new Prefs();
