// The Home stage's camera, as plain math: where a point on the floor shows up on screen. The
// three.js scene uses the same numbers (design/mascot/build/mascot.json, `stage`), so each
// device's button sits exactly on its drawn device, even before three.js has loaded.
// The Kotlin twin is android/.../stage/StageView.kt.

import mascot from '../../../../design/mascot/build/mascot.json';

export const STAGE = mascot.stage;

export type Vec3 = [number, number, number];

const sub = (a: Vec3, b: Vec3): Vec3 => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const dot = (a: Vec3, b: Vec3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross = (a: Vec3, b: Vec3): Vec3 => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
const norm = (a: Vec3): Vec3 => {
  const l = Math.hypot(a[0], a[1], a[2]) || 1;
  return [a[0] / l, a[1] / l, a[2] / l];
};

/** The camera: looking from `eye` at `target`, y up, with a vertical field of view. */
export interface Camera {
  eye: Vec3;
  target: Vec3;
  fovDeg: number;
}

export const homeCamera = (): Camera => ({ eye: STAGE.eye as Vec3, target: STAGE.target as Vec3, fovDeg: STAGE.fovDeg });

/**
 * Where `p` lands in a `width` x `height` box, in pixels from its top left, and how far from the
 * camera it is (for sizing and stacking). The same as three.js's PerspectiveCamera with lookAt.
 */
export function project(cam: Camera, p: Vec3, width: number, height: number): { x: number; y: number; depth: number } {
  const f = norm(sub(cam.target, cam.eye));
  const r = norm(cross(f, [0, 1, 0]));
  const u = cross(r, f);
  const d = sub(p, cam.eye);
  const cx = dot(d, r);
  const cy = dot(d, u);
  const cz = dot(d, f);
  const k = 1 / Math.tan(((cam.fovDeg * Math.PI) / 180) / 2);
  const aspect = width / height;
  const nx = (cx * k) / aspect / cz;
  const ny = (cy * k) / cz;
  return { x: ((nx + 1) / 2) * width, y: ((1 - ny) / 2) * height, depth: cz };
}

/** Pixels per unit at depth `depth`, for sizing things that stand at that distance. */
export function scaleAt(cam: Camera, depth: number, height: number): number {
  const k = 1 / Math.tan(((cam.fovDeg * Math.PI) / 180) / 2);
  return (k * height) / 2 / depth;
}

/**
 * A radar slot (pixels from the radar's centre, in a radar `size` across) as a spot on the floor:
 * x stays x, and down the screen becomes towards the camera.
 */
export function slotOnFloor(slot: { x: number; y: number }, size: number): Vec3 {
  const k = (STAGE.slotReach * 2) / size;
  return [slot.x * k, 0, slot.y * k];
}

/** A token like `#1214181A` as a color three.js takes (it has no alpha) and that alpha, 0 to 1. */
export function splitAlpha(hex: string): { color: string; alpha: number } {
  const m = /^#([0-9a-f]{6})([0-9a-f]{2})?$/i.exec(hex.trim());
  if (!m) return { color: hex, alpha: 1 };
  return { color: `#${m[1]}`, alpha: m[2] ? parseInt(m[2], 16) / 255 : 1 };
}
