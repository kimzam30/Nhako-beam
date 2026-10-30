// The mascot's motion, frame by frame: for a state and the time since it began, where the bolt
// is (hop, lean, turn, squash) and what its face shows. Plain math with no drawing, so both apps
// move it the same way: the Kotlin twin is android/.../stage/MascotMotion.kt. The states and
// their timings come from design/mascot/build/mascot.json; motion-spec.md section 4.18.

import mascot from '../../../../design/mascot/build/mascot.json';
import { mascot as timing } from '../tokens';

export type MascotState = 'idle' | 'looking' | 'happy' | 'sending' | 'receiving' | 'done' | 'failed' | 'asleep' | 'offline' | 'loading';
export type Face = 'eyes' | 'happy' | 'x' | 'closed' | 'half';

export interface Pose {
  /** Lift off the floor, in model units (the bolt is 3 high). */
  y: number;
  /** Radians: nod (x), turn (y), lean (z). */
  rx: number;
  ry: number;
  rz: number;
  /** Squash and stretch. */
  sx: number;
  sy: number;
  face: Face;
  /** How open the eyes are, 0 to 1 (blinks, sleep). */
  eyeOpen: number;
  /** Eyes moved sideways, in voxels (looking at something). */
  eyeShift: number;
  crack: boolean;
  /** The Zs: how far they've risen (0 to 1) and how visible; null when hidden. */
  zzz: { rise: number; opacity: number } | null;
  /** The three sparkles' sizes; 0 hides them. */
  sparkles: [number, number, number];
  /** Light level, 1 normal. */
  light: number;
  /** Sparks to send this frame: from the bolt to the device, or the other way. */
  sparks: 'out' | 'in' | null;
  /** The done burst, true on the one frame it fires. */
  burst: boolean;
}

interface StateSpec {
  id: MascotState;
  face: Face;
  motion: Record<string, number | number[] | boolean>;
  pose: Record<string, number | boolean>;
}

export const STATES = mascot.states as unknown as StateSpec[];
const spec = (id: MascotState) => STATES.find((s) => s.id === id)!;
const deg = (d: number) => (d * Math.PI) / 180;
const easeInOut = (t: number) => (t < 0.5 ? 4 * t * t * t : 1 - (-2 * t + 2) ** 3 / 2);
const num = (v: unknown, fallback = 0) => (typeof v === 'number' ? v : fallback);

/** Where the pointer or the device is, from -1 (left, up) to 1 (right, down); null: nothing. */
export type Look = { x: number; y: number } | null;

/**
 * Keeps what carries from frame to frame (the next blink, the smoothed turn) and gives a pose.
 * `random` is there for tests.
 */
export class MascotMotion {
  state: MascotState = 'idle';
  private since = 0;
  private nextBlink = 0;
  private yaw = 0;
  private pitch = 0;
  private burstDone = false;
  private sparkDebt = 0;

  constructor(private random: () => number = Math.random) {}

  /** Starts `state` at `now` (ms). The same state again changes nothing. */
  set(state: MascotState, now: number): void {
    if (state === this.state && this.since) return;
    this.state = state;
    this.since = now;
    this.burstDone = false;
    this.nextBlink = now + 1200;
  }

  /** The still pose for reduced motion: the face, a held lean or tilt, nothing moving. */
  still(look: Look = null): Pose {
    const s = spec(this.state);
    const p = base(s.face);
    if (num(s.pose.turnDeg)) p.ry = deg(num(s.pose.turnDeg)) * (look && look.x < 0 ? -1 : 1);
    if (num(s.pose.leanDeg)) p.rz = deg(num(s.pose.leanDeg));
    if (num(s.pose.tiltDeg)) p.rz = deg(num(s.pose.tiltDeg));
    if (this.state === 'looking') p.eyeShift = num(s.motion.eyeShift) * (look && look.x < 0 ? -1 : 1);
    p.crack = !!s.pose.crack;
    p.zzz = s.pose.zzz ? { rise: 0.3, opacity: 1 } : null;
    p.sparkles = sparklesFor(this.state, 1);
    p.light = num(s.pose.dim, 1);
    return p;
  }

  /** The pose at `now` (ms), `dt` seconds after the last one, turning towards `look`. */
  step(now: number, dt: number, look: Look): Pose {
    const s = spec(this.state);
    const m = s.motion;
    const t = (now - this.since) / 1000;
    const p = base(s.face);
    p.sparkles = sparklesFor(this.state, 1);
    p.light = num(s.pose.dim, 1);
    let yaw = 0;
    let pitch = 0;

    const breathe = (ms: number) => {
      const b = Math.sin((t * 1000 * 2 * Math.PI) / ms);
      p.sx = 1 - b * 0.008;
      p.sy = 1 + b * 0.016;
    };
    const blink = () => {
      if (now > this.nextBlink) this.nextBlink = now + timing.blinkMinMs + this.random() * (timing.blinkMaxMs - timing.blinkMinMs);
      const into = timing.blinkMs - (this.nextBlink - now);
      if (into > 0 && into < timing.blinkMs) p.eyeOpen = 1 - Math.sin((into / timing.blinkMs) * Math.PI) * 0.88;
    };
    const follow = (strength: number) => {
      if (!look) return;
      yaw += look.x * deg(timing.followDeg) * strength;
      pitch += look.y * 0.12 * strength;
    };
    const twinkle = (ms: number) => {
      p.sparkles = [0, 1, 2].map((i) => 0.75 + 0.35 * Math.sin((t * 1000 * 2 * Math.PI) / ms + i * 2.1)) as Pose['sparkles'];
    };
    const sparksAt = (perSecond: number, dir: 'out' | 'in') => {
      this.sparkDebt += perSecond * dt;
      if (this.sparkDebt >= 1) {
        this.sparkDebt -= 1;
        p.sparks = dir;
      }
    };

    switch (this.state) {
      case 'idle':
        breathe(timing.breatheMs);
        blink();
        yaw += deg(num(m.swayDeg)) * Math.sin(t * 0.9) * 0.5;
        follow(1);
        twinkle(2200);
        break;
      case 'looking': {
        blink();
        const side = look && look.x < 0 ? -1 : 1;
        yaw += deg(num(m.turnDeg)) * side + Math.sin(t * 0.8) * 0.06;
        p.eyeShift = num(m.eyeShift) * side;
        follow(0.4);
        break;
      }
      case 'happy': {
        const hopMs = num(m.hopMs);
        const hops = num(m.hops);
        if (t * 1000 < hopMs * hops) hop(p, ((t * 1000) % hopMs) / hopMs, 0.45, 0.78);
        twinkle(num(m.twinkleMs));
        follow(0.6);
        break;
      }
      case 'sending':
      case 'receiving': {
        const bob = Math.sin((t * 1000 * 2 * Math.PI) / num(m.bobMs));
        p.y = Math.abs(bob) * 0.05;
        p.rz = deg(num(m.leanDeg)) * (0.85 + 0.15 * bob);
        blink();
        follow(0.7);
        sparksAt(num(m.sparksPerSecond), this.state === 'sending' ? 'out' : 'in');
        break;
      }
      case 'done': {
        const a = (t * 1000) / num(m.jumpMs);
        if (a < 1) {
          const up = Math.sin(a * Math.PI);
          p.y = up * 0.85;
          yaw += deg(num(m.spinDeg)) * easeInOut(a);
          p.sx = 1 - up * 0.03;
          p.sy = 1 + up * 0.08;
          if (a > 0.45 && !this.burstDone) {
            this.burstDone = true;
            p.burst = true;
          }
        } else {
          const land = Math.max(0, 1 - (a - 1) * 4);
          p.sx = 1 + land * 0.06;
          p.sy = 1 - land * 0.12;
          if (land === 0) breathe(timing.breatheMs);
        }
        twinkle(700);
        break;
      }
      case 'failed': {
        const a = (t * 1000) / num(m.wobbleMs);
        const w = a < 1 ? Math.exp(-a * 3.2) * Math.sin(a * Math.PI * 6) : 0;
        p.rz = deg(-6) + deg(num(m.wobbleDeg)) * w;
        p.y = -0.03;
        break;
      }
      case 'asleep': {
        breathe(num(m.breatheMs));
        p.rz = deg(num(m.droopDeg));
        pitch += 0.08;
        const a = ((t * 1000) % num(m.zzzMs)) / num(m.zzzMs);
        p.zzz = { rise: a, opacity: a < 0.15 ? a / 0.15 : 1 - (a - 0.15) / 0.85 };
        break;
      }
      case 'offline':
        breathe(num(m.breatheMs));
        p.rz = deg(num(m.droopDeg));
        pitch += 0.12;
        break;
      case 'loading':
        yaw += ((t * 1000) / num(m.spinMs)) * Math.PI * 2;
        p.y = 0.08 + Math.sin(t * 3) * 0.04;
        break;
    }

    // Turns follow softly; the spins are exact.
    const k = 1 - Math.exp(-dt * 7);
    const spinning = this.state === 'loading' || (this.state === 'done' && t * 1000 < num(m.jumpMs));
    this.yaw = spinning ? yaw : this.yaw + (yaw - this.yaw) * k;
    this.pitch += (pitch - this.pitch) * k;
    p.ry = this.yaw;
    p.rx = this.pitch;
    return p;
  }
}

function base(face: Face): Pose {
  return {
    y: 0, rx: 0, ry: 0, rz: 0, sx: 1, sy: 1, face,
    eyeOpen: face === 'closed' ? 0.12 : face === 'half' ? 0.5 : 1,
    eyeShift: 0, crack: false, zzz: null, sparkles: [1, 1, 1], light: 1, sparks: null, burst: false,
  };
}

/** Sparkles show unless something's wrong or it's asleep. */
function sparklesFor(state: MascotState, size: number): Pose['sparkles'] {
  const shown = state !== 'failed' && state !== 'offline' && state !== 'asleep';
  return shown ? [size, size, size] : [0, 0, 0];
}

/** A hop: up and down by `phase` of the way through, then a squash on landing. */
function hop(p: Pose, a: number, height: number, phase: number) {
  const up = Math.sin(Math.min(1, a / phase) * Math.PI);
  p.y = up * height;
  const land = a > phase ? Math.sin(Math.min(1, (a - phase) / 0.18) * Math.PI) : 0;
  const sy = 1 + up * 0.06 - land * 0.12;
  p.sy = sy;
  p.sx = 1 + (1 - sy) * 0.5;
}

/** Which parts of the model show for a face (mascot.json `faces`). */
export const FACE_PARTS = mascot.faces as Record<Face, string[]>;
export const ALL_FACE_PARTS = ['eyeL', 'eyeR', 'mouth', 'eyeXL', 'eyeXR', 'eyeHappyL', 'eyeHappyR', 'mouthOpen'];
