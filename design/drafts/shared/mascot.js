// The app's real 3D bolt, loaded from the desktop dev server (the "prototype" launch config, port 5173),
// so the drafts show the shipped model, camera and moods. New moods proposed in these drafts are added
// here only, by extending the real MascotMotion in this page. Nothing in the app changes.

const VITE = `${location.protocol}//${location.hostname === '127.0.0.1' ? '127.0.0.1' : 'localhost'}:5173`;
let mod = null;

const deg = (d) => (d * Math.PI) / 180;

/** Proposed moods. Each starts from the real idle pose (breathing, blinking) and changes what it needs. */
export const NEW_MOODS = {
  searching: {
    label: 'Searching', base: 'idle',
    spec: 'Turns slowly left and right over 2400 ms (radarSweep token), eyes lead the turn by 0.7 voxel.',
    pose(p, t) { const s = Math.sin((t * 2 * Math.PI) / 2.4); p.ry = deg(24) * s; p.eyeShift = 0.7 * Math.sign(s) * Math.min(1, Math.abs(s) * 3); },
    still(p) { p.ry = deg(18); p.eyeShift = 0.7; },
  },
  waiting: {
    label: 'Waiting to accept', base: 'idle',
    spec: 'Faces the device (turn 18°), a small 0.1 hop every 1200 ms, glances back at you every 3.6 s.',
    pose(p, t) {
      const glance = (t % 3.6) > 3.1;
      p.ry = glance ? deg(-4) : deg(18); p.eyeShift = glance ? 0 : 0.7;
      const a = (t % 1.2) / 1.2; if (a < 0.22) { const u = Math.sin((a / 0.22) * Math.PI); p.y = u * 0.1; p.sy = 1 + u * 0.03; p.sx = 1 - u * 0.015; }
    },
    still(p) { p.ry = deg(18); p.eyeShift = 0.7; },
  },
  waking: {
    label: 'Waking a device', base: 'idle',
    spec: 'Leans toward the device and sends a burst of 3 sparks every 900 ms, like knocking.',
    pose(p, t) { const a = (t % 0.9) / 0.9; const k = a < 0.3 ? Math.sin((a / 0.3) * Math.PI) : 0; p.rz = deg(-6 - 8 * k); p.ry = deg(14); if (k > 0.2) p.sparks = 'out'; },
    still(p) { p.rz = deg(-8); p.ry = deg(14); },
  },
  trust: {
    label: 'Trust check', base: 'idle',
    spec: 'Holds still with a straight face, eyes move between the two codes every 1600 ms.',
    pose(p, t) { p.eyeShift = (Math.floor(t / 1.6) % 2 ? -0.7 : 0.7); p.ry = deg(4) * Math.sign(p.eyeShift); p.sx = 1; p.sy = 1; },
    still(p) { p.eyeShift = 0; },
  },
  cancelled: {
    label: 'Cancelled', base: 'idle',
    spec: 'A shrug: squash 6% and tilt 5° over 360 ms, half-closed eyes, then back to idle.',
    pose(p, t) { const a = Math.min(1, t / 0.36); const k = Math.sin(a * Math.PI); p.sy = 1 - 0.06 * k; p.sx = 1 + 0.03 * k; p.rz = deg(5) * k; p.face = a < 1 ? 'half' : 'eyes'; p.eyeOpen = a < 1 ? 0.5 : p.eyeOpen; },
    still(p) { p.face = 'half'; p.eyeOpen = 0.5; },
  },
  refused: {
    label: 'Not allowed', base: 'idle',
    spec: 'Shakes its head twice (±14°, 560 ms), eyes stay open, no crack: nothing broke, it was a no.',
    pose(p, t) { const a = t / 0.56; p.ry = a < 1 ? deg(14) * Math.sin(a * Math.PI * 4) * (1 - a) : 0; p.sparkles = [0, 0, 0]; },
    still(p) { p.sparkles = [0, 0, 0]; },
  },
};

function patch(MascotMotion) {
  if (MascotMotion.__draft) return;
  MascotMotion.__draft = true;
  const step = MascotMotion.prototype.step, still = MascotMotion.prototype.still, set = MascotMotion.prototype.set;
  MascotMotion.prototype.set = function (state, now) {
    this.__custom = NEW_MOODS[state] ? state : null;
    if (this.__custom) { this.__since = now; return set.call(this, NEW_MOODS[state].base, now); }
    return set.call(this, state, now);
  };
  MascotMotion.prototype.step = function (now, dt, look) {
    const p = step.call(this, now, dt, look);
    if (this.__custom) NEW_MOODS[this.__custom].pose(p, (now - this.__since) / 1000);
    return p;
  };
  MascotMotion.prototype.still = function (look) {
    const p = still.call(this, look);
    if (this.__custom) NEW_MOODS[this.__custom].still(p);
    return p;
  };
}

/** Loads the real scene once. Throws a readable error when the dev server isn't running. */
export async function load() {
  if (mod) return mod;
  try {
    const [scene, pose] = await Promise.all([
      import(/* @vite-ignore */ `${VITE}/src/lib/stage/scene.ts`),
      import(/* @vite-ignore */ `${VITE}/src/lib/stage/pose.ts`),
    ]);
    patch(pose.MascotMotion);
    mod = { scene, pose };
    return mod;
  } catch (e) {
    throw new Error(`The 3D bolt comes from the desktop dev server on port 5173 (launch config "prototype"). ${e.message}`);
  }
}

/** A stage in `canvas` like Mascot3D.svelte makes it. */
export async function stage(canvas, { mood = 'idle', intro = false, reduced = false, size } = {}) {
  const { scene } = await load();
  const s = await scene.createStage(canvas, {
    camera: { eye: [0, 2.6, 8.6], target: [0, 1.45, 0], fovDeg: 30 },
    home: false, mascotScale: 1, intro, reduced,
  });
  const w = size ?? canvas.clientWidth, h = size ?? canvas.clientHeight;
  s.resize(w, h);
  s.setMood(mood);
  return s;
}
