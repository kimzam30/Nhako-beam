// The room's numbers: one real camera, furniture in centimetres, one list of places. build.mjs turns
// these into room.json, and refuses to build if any device stops fitting where it's put.
export const CAM = { vx: 800, vy: 330, f: 1440, eye: 146 };
export const project = (X, Y, Z) => { const s = CAM.f / Z; return [CAM.vx + X * s, CAM.vy + (CAM.eye - Y) * s]; };

// The part of each piece of furniture that can hold something, and the height free above it.
export const SURFACES = {
  desk: { x: [-80, 80], z: [410, 480], y: 75, clear: Infinity },
  hutch: { x: [-78, 78], z: [450, 480], y: 112.25, clear: 30 },
  table: { x: [118, 178], z: [330, 438], y: 75, clear: Infinity },
  stand: { x: [-112, -84], z: [445, 480], y: 52, clear: Infinity },
};

// A little larger than life, as in any lofi room, so a phone can be seen. foot: what touches the
// surface (half width, depth in front of and behind its centre). wide: half the whole body's width.
export const KINDS = {
  desktop: { foot: { x: 11, z: [-7, 7] }, wide: 22, h: 34, art: [3, 0] },
  laptop: { foot: { x: 17, z: [-10, 13.6] }, wide: 17, h: 22, art: [0, 5] },
  tablet: { foot: { x: 15, z: [-4, 6] }, wide: 15, h: 21, art: [2] },
  phone: { foot: { x: 6.5, z: [-4.5, 4.5] }, wide: 6.5, h: 27, art: [1, 4, 2] },
};

// The places, in the order devices fill them. Each is turned toward the middle of the room. A device
// keeps its size wherever it goes: a place holds it whole, or doesn't offer to.
const ANY = ['desktop', 'laptop', 'tablet', 'phone'], NOT_PC = ['laptop', 'tablet', 'phone'];
export const SLOTS = [
  { id: 'deskR', on: 'desk', x: 38, z: 442, yaw: -10, kinds: ANY },
  { id: 'deskL', on: 'desk', x: -56, z: 442, yaw: 12, kinds: ANY },
  { id: 'shelfL', on: 'hutch', x: -20, z: 465, yaw: 3, kinds: NOT_PC },
  { id: 'shelfR', on: 'hutch', x: 32, z: 465, yaw: -3, kinds: NOT_PC },
  { id: 'deskCorner', on: 'desk', x: 69, z: 428, yaw: -16, kinds: ['phone'] },
  { id: 'table', on: 'table', x: 148, z: 408, yaw: -18, kinds: ANY },
  { id: 'tableFront', on: 'table', x: 158, z: 356, yaw: -26, kinds: ['tablet', 'phone'] },
  { id: 'night', on: 'stand', x: -100, z: 462, yaw: 10, kinds: ['phone'] },
];

/** A device's own point, in centimetres, to the room. */
const toRoom = (slot, lx, ly, lz) => {
  const a = (slot.yaw * Math.PI) / 180, c = Math.cos(a), s = Math.sin(a);
  return [slot.x + lx * c + lz * s, SURFACES[slot.on].y + ly, slot.z - lx * s + lz * c];
};
const footCorners = (slot, kind) => { const f = KINDS[kind].foot; return [[-f.x, f.z[0]], [f.x, f.z[0]], [f.x, f.z[1]], [-f.x, f.z[1]]].map(([x, z]) => toRoom(slot, x, 0, z)); };

/** Where a device shows, in room units: its whole body, and the middle of its front edge. */
export function screenBox(slot, kind) {
  const K = KINDS[kind], xs = [], ys = [];
  for (const x of [-K.wide, K.wide]) for (const z of K.foot.z) for (const y of [0, K.h]) { const [a, b] = project(...toRoom(slot, x, y, z)); xs.push(a); ys.push(b); }
  const [bx, by] = project(...toRoom(slot, 0, 0, K.foot.z[0]));
  return { x0: Math.min(...xs), x1: Math.max(...xs), y0: Math.min(...ys), y1: Math.max(...ys), bx, by };
}

/** How many centimetres the footprint stays inside its surface: below 0, it's hanging off. */
export function margin(slot, kind) {
  const u = SURFACES[slot.on];
  return Math.min(...footCorners(slot, kind).map(([X, , Z]) => Math.min(X - u.x[0], u.x[1] - X, Z - u.z[0], u.z[1] - Z)));
}

/** The gap in centimetres between two footprints on one surface (turned rectangles); below 0, they overlap. */
function gap(a, b) {
  const normals = (p) => p.map(([x, , z], i) => { const [x2, , z2] = p[(i + 1) % 4], l = Math.hypot(x2 - x, z2 - z); return [(z2 - z) / l, -(x2 - x) / l]; });
  let best = -Infinity;
  for (const [nx, nz] of [...normals(a), ...normals(b)]) {
    const pa = a.map(([x, , z]) => x * nx + z * nz), pb = b.map(([x, , z]) => x * nx + z * nz);
    best = Math.max(best, Math.min(...pb) - Math.max(...pa), Math.min(...pa) - Math.max(...pb));
  }
  return best;
}

/** Every reason the numbers don't work, or nothing. */
export function problems() {
  const out = [];
  for (const s of SLOTS) for (const kind of s.kinds) {
    const m = margin(s, kind);
    if (m < 0) out.push(`${kind} hangs ${-m.toFixed(1)} cm off ${s.id}`);
    if (KINDS[kind].h > SURFACES[s.on].clear) out.push(`${kind} is too tall for ${s.id}`);
  }
  SLOTS.forEach((a, i) => SLOTS.slice(i + 1).forEach((b) => {
    if (a.on !== b.on) return;
    for (const ka of a.kinds) for (const kb of b.kinds) if (gap(footCorners(a, ka), footCorners(b, kb)) < 0) out.push(`${ka} on ${a.id} overlaps ${kb} on ${b.id}`);
  }));
  return out;
}
