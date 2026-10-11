// User preferences that change how the UI looks and moves.
// Theme and motion follow the system by default, with an in-app override.

export type ThemePref = 'system' | 'light' | 'dark';
export type MotionPref = 'system' | 'full' | 'reduced';

const KEY = 'nhako.prefs.v1';

function load(): { theme: ThemePref; motion: MotionPref } {
  try {
    const raw = localStorage.getItem(KEY);
    if (raw) return { theme: 'system', motion: 'system', ...JSON.parse(raw) };
  } catch {
    // Storage can be blocked; defaults are fine.
  }
  return { theme: 'system', motion: 'system' };
}

class Prefs {
  theme = $state<ThemePref>('system');
  motion = $state<MotionPref>('system');
  systemReduced = $state(false);
  systemDark = $state(false);

  reducedMotion = $derived(this.motion === 'reduced' || (this.motion === 'system' && this.systemReduced));
  dark = $derived(this.theme === 'dark' || (this.theme === 'system' && this.systemDark));

  init(): void {
    const saved = load();
    this.theme = saved.theme;
    this.motion = saved.motion;
    const rm = matchMedia('(prefers-reduced-motion: reduce)');
    const dm = matchMedia('(prefers-color-scheme: dark)');
    this.systemReduced = rm.matches;
    this.systemDark = dm.matches;
    rm.addEventListener('change', (e) => (this.systemReduced = e.matches));
    dm.addEventListener('change', (e) => (this.systemDark = e.matches));
  }

  save(): void {
    try {
      localStorage.setItem(KEY, JSON.stringify({ theme: this.theme, motion: this.motion }));
    } catch {
      // Ignore: prefs just will not persist.
    }
  }
}

export const prefs = new Prefs();
