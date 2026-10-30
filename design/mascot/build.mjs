// Makes the voxel mascot from the logo's own 48 x 48 pixels (docs/brand/logo/svg/nhako-beam-bolt.svg):
// each colored pixel becomes a column of cubes, so the 3D bolt is the logo, not a redraw.
//
//   node design/mascot/build.mjs        (or `npm run mascot` in desktop/)
//
// Writes, into design/mascot/build/:
//   mascot.glb    one glTF model with a node per moving part (both apps load the same file)
//   mascot.json   the parts, the states and their timings, for the apps' animation code
// How it's built and what each state is for: design/mascot/README.md.

import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, '../..');
const OUT = join(HERE, 'build');
const SVG = join(ROOT, 'docs/brand/logo/svg/nhako-beam-bolt.svg');
const SIZE = 48;

// ---------- The logo's pixels ----------

/** grid[row][col] = '#rrggbb' or null. The SVG draws each row of a color as `M x y h w v1 h-w z`. */
function readGrid() {
  const svg = readFileSync(SVG, 'utf8');
  const grid = Array.from({ length: SIZE }, () => Array(SIZE).fill(null));
  for (const path of svg.matchAll(/<path fill="(#[0-9a-fA-F]{6})" d="([^"]+)"/g)) {
    for (const run of path[2].matchAll(/M(\d+) (\d+)h(\d+)v1h-\d+z/g)) {
      const [x, y, w] = [Number(run[1]), Number(run[2]), Number(run[3])];
      for (let i = 0; i < w; i++) grid[y][x + i] = path[1].toLowerCase();
    }
  }
  return grid;
}

// The logo's colors, by what they are.
const C = {
  outline: '#7a3010',
  rim: '#fff08a',
  face: '#ffd93b',
  shade: '#f2b51f',
  side: '#ff8a2a',
  sideDark: '#e0661a',
  shine: '#fff7c2',
  sparkle: '#bcdcff',
  white: '#ffffff',
  cheek: '#ff9a6b',
  navy: '#1a6fe0',
};

// Where the face is, in logo pixels (columns and rows, inclusive).
const EYE_L = { c0: 23, c1: 26, r0: 9, r1: 13 };
const EYE_R = { c0: 33, c1: 36, r0: 9, r1: 13 };
const MOUTH = { c0: 28, c1: 31, r0: 16, r1: 17 };
const inBox = (b, c, r) => c >= b.c0 && c <= b.c1 && r >= b.r0 && r <= b.r1;

// Depth, in voxels: the face is 4 deep, the orange side sits 1 back (the logo draws it as the
// bolt's side, so in 3D it's a step behind), the features stand a little proud of the face.
const Z = {
  body: [0, 4],
  side: [0, 3],
  feature: [4, 4.6],
  cheek: [4, 4.35],
  sparkle: [1, 2.5],
  crack: [4, 4.3],
  zzz: [1.5, 2.5],
};

/** Splits the logo into parts. Each part: { name, pixels: [{c, r, color}], z, hidden }. */
function parts(grid) {
  const body = [];
  const side = [];
  const eyeL = [];
  const eyeR = [];
  const mouth = [];
  const cheeks = [];
  const loose = [];
  for (let r = 0; r < SIZE; r++) {
    for (let c = 0; c < SIZE; c++) {
      const color = grid[r][c];
      if (!color) continue;
      const px = { c, r, color };
      if (inBox(EYE_L, c, r) || inBox(EYE_R, c, r) || inBox(MOUTH, c, r)) {
        if (color === C.outline || color === C.white) {
          (inBox(EYE_L, c, r) ? eyeL : inBox(EYE_R, c, r) ? eyeR : mouth).push(px);
          // The face goes on behind it, so a blink shows yellow, not a hole.
          body.push({ c, r, color: C.face });
        } else body.push(px);
      } else if (color === C.cheek) {
        cheeks.push(px);
        body.push({ c, r, color: C.face });
      } else if (color === C.side || color === C.sideDark) side.push(px);
      else if (color === C.sparkle || color === C.white) loose.push(px);
      else body.push(px);
    }
  }
  // The three sparkles around the bolt, each its own part so they twinkle apart.
  const sparkles = [];
  const seen = new Set();
  for (const p of loose) {
    const key = `${p.c},${p.r}`;
    if (seen.has(key)) continue;
    const group = [];
    const todo = [p];
    seen.add(key);
    while (todo.length) {
      const q = todo.pop();
      group.push(q);
      for (const n of loose) {
        const k = `${n.c},${n.r}`;
        if (!seen.has(k) && Math.abs(n.c - q.c) + Math.abs(n.r - q.r) === 1) {
          seen.add(k);
          todo.push(n);
        }
      }
    }
    sparkles.push(group);
  }
  sparkles.sort((a, b) => a[0].r - b[0].r || a[0].c - b[0].c);

  const onBody = new Set(body.map((p) => `${p.c},${p.r}`));
  const center = (b) => ({ c: (b.c0 + b.c1) / 2, r: (b.r0 + b.r1) / 2 });
  return [
    { name: 'body', pixels: body, z: Z.body },
    { name: 'side', pixels: side, z: Z.side },
    { name: 'eyeL', pixels: eyeL, z: Z.feature },
    { name: 'eyeR', pixels: eyeR, z: Z.feature },
    { name: 'mouth', pixels: mouth, z: Z.feature },
    { name: 'cheeks', pixels: cheeks, z: Z.cheek },
    ...sparkles.map((pixels, i) => ({ name: `sparkle${i + 1}`, pixels, z: Z.sparkle })),
    // Faces for other states, hidden until a state shows them.
    { name: 'eyeXL', pixels: cross(center(EYE_L)), z: Z.feature, hidden: true },
    { name: 'eyeXR', pixels: cross(center(EYE_R)), z: Z.feature, hidden: true },
    { name: 'eyeHappyL', pixels: arch(center(EYE_L)), z: Z.feature, hidden: true },
    { name: 'eyeHappyR', pixels: arch(center(EYE_R)), z: Z.feature, hidden: true },
    { name: 'mouthOpen', pixels: openMouth(), z: Z.feature, hidden: true },
    { name: 'crack', pixels: crack(onBody), z: Z.crack, hidden: true },
    { name: 'zzz', pixels: zzz(), z: Z.zzz, hidden: true },
  ];
}

/** An x for each eye (failed). */
function cross({ c, r }) {
  const out = [];
  for (let i = -2; i <= 2; i++) {
    out.push({ c: Math.round(c + i), r: Math.round(r + i), color: C.outline });
    if (i !== 0) out.push({ c: Math.round(c + i), r: Math.round(r - i), color: C.outline });
  }
  return out;
}

/** A happy ^ for each eye (done, found a device). */
function arch({ c, r }) {
  const cc = Math.round(c);
  const rr = Math.round(r);
  return [
    [-2, 1], [-1, 0], [0, -1], [1, -1], [2, 0], [3, 1],
  ].map(([dc, dr]) => ({ c: cc - 1 + dc, r: rr + dr, color: C.outline }));
}

/** An open smile (happy, done). */
function openMouth() {
  const out = [];
  for (let c = 27; c <= 32; c++) out.push({ c, r: 16, color: C.outline });
  for (let c = 28; c <= 31; c++) out.push({ c, r: 17, color: c === 29 || c === 30 ? C.cheek : C.outline });
  for (let c = 29; c <= 30; c++) out.push({ c, r: 18, color: C.outline });
  return out;
}

/** A zigzag crack down the bolt (failed), only where there's bolt under it. */
function crack(onBody) {
  const path = [
    [38, 4], [37, 5], [37, 6], [36, 7], [37, 8], [38, 9], [38, 10], [37, 11], [36, 12],
    [36, 13], [35, 14], [36, 15], [36, 16], [35, 17], [34, 18], [33, 19], [33, 20], [32, 21],
  ];
  return path.filter(([c, r]) => onBody.has(`${c},${r}`)).map(([c, r]) => ({ c, r, color: C.outline }));
}

/** Three Zs above the head, big to small (asleep). */
function zzz() {
  const glyph = (c0, r0, n) => {
    const out = [];
    for (let i = 0; i < n; i++) {
      out.push({ c: c0 + i, r: r0, color: C.navy });
      out.push({ c: c0 + i, r: r0 + n - 1, color: C.navy });
      if (i > 0 && i < n - 1) out.push({ c: c0 + n - 1 - i, r: r0 + i, color: C.navy });
    }
    return out;
  };
  return [...glyph(40, -1, 5), ...glyph(44, -6, 4), ...glyph(47, -10, 3)];
}

// ---------- Mesh ----------

/** sRGB hex to linear RGB, which glTF vertex colors are in. */
function linear(hex) {
  return [1, 3, 5].map((i) => {
    const v = parseInt(hex.slice(i, i + 2), 16) / 255;
    return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  });
}

/**
 * One part's mesh: each pixel is a box from z0 to z1, drawn only where a face shows (sides only
 * where the neighbour in the same part is empty). Faces of one color next to each other merge
 * into one rectangle (a run along the row, or down the column for left and right sides), which
 * keeps the file small. Positions are relative to the part's centre, which becomes the node's
 * translation, so a part scales and turns about its own middle.
 */
function mesh(part) {
  const [z0, z1] = part.z;
  const at = new Map(part.pixels.map((p) => [`${p.c},${p.r}`, p.color]));
  const cs = part.pixels.map((p) => p.c);
  const rs = part.pixels.map((p) => p.r);
  // Voxel (c, r) spans x c-24..c-23 and y 23-r..24-r; z is shifted so the face is centred on 0.
  const cx = (Math.min(...cs) + Math.max(...cs) + 1) / 2 - 24;
  const cy = 24 - (Math.min(...rs) + Math.max(...rs) + 1) / 2;
  const cz = (z0 + z1) / 2 - 2;
  const pos = [];
  const nrm = [];
  const col = [];
  const idx = [];
  const quad = (a, b, c, d, n, hex) => {
    const base = pos.length / 3;
    const rgb = linear(hex).map((v) => Math.round(v * 255));
    for (const v of [a, b, c, d]) {
      pos.push(v[0] - cx, v[1] - cy, v[2] - 2 - cz);
      nrm.push(...n);
      col.push(...rgb, 255);
    }
    idx.push(base, base + 1, base + 2, base, base + 2, base + 3);
  };
  /** Runs of same-colored pixels that pass `keep`, along rows (or columns when `down`). */
  const runs = (keep, down) => {
    const out = [];
    const [a0, a1] = down ? [Math.min(...cs), Math.max(...cs)] : [Math.min(...rs), Math.max(...rs)];
    const [b0, b1] = down ? [Math.min(...rs), Math.max(...rs)] : [Math.min(...cs), Math.max(...cs)];
    for (let a = a0; a <= a1; a++) {
      let start = null;
      let color = null;
      for (let b = b0; b <= b1 + 1; b++) {
        const [c, r] = down ? [a, b] : [b, a];
        const here = b <= b1 ? at.get(`${c},${r}`) : undefined;
        const ok = here !== undefined && keep(c, r);
        if (start !== null && (!ok || here !== color)) {
          out.push({ a, from: start, to: b - 1, color });
          start = null;
        }
        if (ok && start === null) {
          start = b;
          color = here;
        }
      }
    }
    return out;
  };
  const empty = (c, r) => !at.has(`${c},${r}`);
  // Front and back, and top and bottom, as runs along each row.
  for (const { a: r, from, to, color } of runs(() => true, false)) {
    const [x0, x1, y1] = [from - 24, to - 23, 24 - r];
    const y0 = y1 - 1;
    quad([x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1], [0, 0, 1], color);
    quad([x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0], [0, 0, -1], color);
  }
  for (const { a: r, from, to, color } of runs((c, r) => empty(c, r - 1), false)) {
    const [x0, x1, y1] = [from - 24, to - 23, 24 - r];
    quad([x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0], [0, 1, 0], color);
  }
  for (const { a: r, from, to, color } of runs((c, r) => empty(c, r + 1), false)) {
    const [x0, x1, y0] = [from - 24, to - 23, 23 - r];
    quad([x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1], [0, -1, 0], color);
  }
  // Left and right, as runs down each column.
  for (const { a: c, from, to, color } of runs((c, r) => empty(c - 1, r), true)) {
    const [x0, y0, y1] = [c - 24, 23 - to, 24 - from];
    quad([x0, y0, z0], [x0, y0, z1], [x0, y1, z1], [x0, y1, z0], [-1, 0, 0], color);
  }
  for (const { a: c, from, to, color } of runs((c, r) => empty(c + 1, r), true)) {
    const [x1, y0, y1] = [c - 23, 23 - to, 24 - from];
    quad([x1, y0, z1], [x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [1, 0, 0], color);
  }
  return { translation: [cx, cy, cz], pos, nrm, col, idx };
}

// ---------- glTF binary ----------

function glb(partList) {
  const chunks = [];
  let offset = 0;
  const bufferViews = [];
  const accessors = [];
  const add = (typed, target) => {
    const bytes = Buffer.from(typed.buffer, typed.byteOffset, typed.byteLength);
    const pad = (4 - (bytes.length % 4)) % 4;
    bufferViews.push({ buffer: 0, byteOffset: offset, byteLength: bytes.length, target });
    chunks.push(bytes, Buffer.alloc(pad));
    offset += bytes.length + pad;
    return bufferViews.length - 1;
  };
  const meshes = [];
  const nodes = [];
  for (const part of partList) {
    const m = mesh(part);
    const pos = new Float32Array(m.pos);
    const min = [0, 1, 2].map((i) => Math.min(...m.pos.filter((_, j) => j % 3 === i)));
    const max = [0, 1, 2].map((i) => Math.max(...m.pos.filter((_, j) => j % 3 === i)));
    const count = m.pos.length / 3;
    accessors.push({ bufferView: add(pos, 34962), componentType: 5126, count, type: 'VEC3', min, max });
    const p = accessors.length - 1;
    accessors.push({ bufferView: add(new Float32Array(m.nrm), 34962), componentType: 5126, count, type: 'VEC3' });
    const n = accessors.length - 1;
    // Colors as bytes (0 to 255, read as 0 to 1; four of them, as glTF wants each vertex's
    // values on 4-byte boundaries), indices as 16 bits.
    accessors.push({ bufferView: add(new Uint8Array(m.col), 34962), componentType: 5121, normalized: true, count, type: 'VEC4' });
    const c = accessors.length - 1;
    accessors.push({ bufferView: add(new Uint16Array(m.idx), 34963), componentType: 5123, count: m.idx.length, type: 'SCALAR' });
    const i = accessors.length - 1;
    meshes.push({ name: part.name, primitives: [{ attributes: { POSITION: p, NORMAL: n, COLOR_0: c }, indices: i, material: 0 }] });
    nodes.push({ name: part.name, mesh: meshes.length - 1, translation: m.translation, extras: { hidden: !!part.hidden } });
  }
  // The root: 48 voxels high is 3 units, standing on y = 0.
  const parts = nodes.map((_, k) => k);
  nodes.push({ name: 'mascot', children: parts, scale: [1 / 16, 1 / 16, 1 / 16], translation: [0, 1.5, 0] });
  const bin = Buffer.concat(chunks);
  const gltf = {
    asset: { version: '2.0', generator: 'Nhako Beam design/mascot/build.mjs' },
    scene: 0,
    scenes: [{ name: 'Nhako Beam mascot', nodes: [nodes.length - 1] }],
    nodes,
    meshes,
    materials: [{ name: 'voxel', pbrMetallicRoughness: { baseColorFactor: [1, 1, 1, 1], metallicFactor: 0, roughnessFactor: 0.85 } }],
    accessors,
    bufferViews,
    buffers: [{ byteLength: bin.length }],
  };
  let json = Buffer.from(JSON.stringify(gltf));
  json = Buffer.concat([json, Buffer.alloc((4 - (json.length % 4)) % 4, 0x20)]);
  const header = Buffer.alloc(12);
  const total = 12 + 8 + json.length + 8 + bin.length;
  header.writeUInt32LE(0x46546c67, 0);
  header.writeUInt32LE(2, 4);
  header.writeUInt32LE(total, 8);
  const chunkHead = (len, type) => {
    const b = Buffer.alloc(8);
    b.writeUInt32LE(len, 0);
    b.writeUInt32LE(type, 4);
    return b;
  };
  return Buffer.concat([header, chunkHead(json.length, 0x4e4f534a), json, chunkHead(bin.length, 0x004e4942), bin]);
}

// ---------- States ----------

// What the mascot does, and when. Times in ms. Reduced motion keeps only the pose: which face
// parts show, and a still lean or tilt; nothing moves.
const STATES = [
  { id: 'idle', when: 'Home with nothing going on', face: 'eyes', motion: { breatheMs: 3200, blinkEveryMs: [2600, 5200], blinkMs: 140, swayDeg: 6, followPointer: true }, pose: {} },
  { id: 'looking', when: 'The pointer is over a device, or a device was just tapped', face: 'eyes', motion: { followPointer: true, turnDeg: 28, eyeShift: 0.7 }, pose: { turnDeg: 18 } },
  { id: 'happy', when: 'A device was found, or pairing worked', face: 'happy', motion: { hopMs: 520, hops: 2, squash: 0.12, twinkleMs: 900 }, pose: { sparkles: true } },
  { id: 'sending', when: 'Files are going out', face: 'eyes', motion: { leanDeg: -12, sparksPerSecond: 14, bobMs: 700 }, pose: { leanDeg: -10 } },
  { id: 'receiving', when: 'Files are coming in', face: 'eyes', motion: { leanDeg: 8, sparksPerSecond: 14, bobMs: 700 }, pose: { leanDeg: 6 } },
  { id: 'done', when: 'A transfer finished', face: 'happy', motion: { jumpMs: 900, spinDeg: 360, burst: 18 }, pose: { sparkles: true } },
  { id: 'failed', when: 'A transfer failed, or a device was lost mid-transfer', face: 'x', motion: { wobbleMs: 1100, wobbleDeg: 9 }, pose: { crack: true, tiltDeg: -6 } },
  { id: 'asleep', when: 'Nothing nearby, or history is empty', face: 'closed', motion: { breatheMs: 4600, zzzMs: 2400, droopDeg: 7 }, pose: { zzz: true, tiltDeg: 7 } },
  { id: 'offline', when: 'No network', face: 'half', motion: { breatheMs: 5200, dim: 0.55, droopDeg: 4 }, pose: { dim: 0.55 } },
  { id: 'loading', when: 'Starting up, or connecting', face: 'eyes', motion: { spinMs: 1400 }, pose: {} },
];

// ---------- Devices and the stage ----------

// Your other devices, as voxel boxes (in voxels, like the bolt: 16 to a unit). Each box: size
// [w, h, d], centre [x, y, z] above the floor, a tilt back in degrees, and its color role:
// `case` (the body), `screen` (blue when the device can be reached, grey when it can't), `key`.
const DEVICES = {
  laptop: [
    { size: [15, 1, 10], at: [0, 0.5, 1], role: 'case' },
    { size: [13, 0.2, 6], at: [0, 1.1, 1.5], role: 'key' },
    { size: [15, 10, 1], at: [0, 5.6, -4.2], tilt: 12, role: 'case' },
    { size: [13, 8, 0.2], at: [0, 5.7, -3.6], tilt: 12, role: 'screen' },
  ],
  desktop: [
    { size: [9, 1, 6], at: [0, 0.5, 0], role: 'case' },
    { size: [2, 4, 2], at: [0, 3, -1], role: 'case' },
    { size: [17, 11, 1.5], at: [0, 10, -1], role: 'case' },
    { size: [15, 9, 0.2], at: [0, 10.2, -0.15], role: 'screen' },
  ],
  phone: [
    { size: [4, 1, 4], at: [0, 0.5, -0.5], role: 'case' },
    { size: [7, 13, 1], at: [0, 7, 0], tilt: 14, role: 'case' },
    { size: [6, 11, 0.2], at: [0, 7.1, 0.6], tilt: 14, role: 'screen' },
  ],
  tablet: [
    { size: [6, 1, 5], at: [0, 0.5, -0.5], role: 'case' },
    { size: [13, 10, 1], at: [0, 6, 0], tilt: 16, role: 'case' },
    { size: [11, 8, 0.2], at: [0, 6.1, 0.6], tilt: 16, role: 'screen' },
  ],
};

const DEVICE_COLORS = { case: '#3a4250', key: '#4b5463', screen: '#2f8fff', screenAway: '#5f6877' };

// The Home stage: the camera, the floor, and where the radar's slots land on it. Both apps
// project with these numbers, so the devices' buttons sit exactly on the drawn devices.
const STAGE = {
  fovDeg: 30,
  // About 42 degrees down, 16 away: the whole floor fits a box 0.85 as tall as it's wide.
  eye: [0, 10.7, 11.9],
  target: [0, 1.0, 0],
  heightRatio: 0.85,
  floorRadius: 4.5,
  // A radar slot at the radar's edge lands this far out (units); the y of a slot becomes depth.
  slotReach: 3.9,
  mascotScale: 0.7,
  deviceScale: 1.3,
}

// ---------- Write ----------

const grid = readGrid();
const list = parts(grid);
mkdirSync(OUT, { recursive: true });
const model = glb(list);
writeFileSync(join(OUT, 'mascot.glb'), model);
const voxels = list.filter((p) => !p.hidden).reduce((n, p) => n + p.pixels.length * Math.ceil(p.z[1] - p.z[0]), 0);
writeFileSync(
  join(OUT, 'mascot.json'),
  `${JSON.stringify(
    {
      source: 'docs/brand/logo/svg/nhako-beam-bolt.svg',
      model: 'mascot.glb',
      pixels: list.filter((p) => !p.hidden).reduce((n, p) => n + p.pixels.length, 0),
      voxels,
      parts: list.map((p) => ({ name: p.name, pixels: p.pixels.length, hidden: !!p.hidden })),
      faces: {
        eyes: ['eyeL', 'eyeR', 'mouth'],
        happy: ['eyeHappyL', 'eyeHappyR', 'mouthOpen'],
        x: ['eyeXL', 'eyeXR', 'mouth'],
        closed: ['eyeL', 'eyeR', 'mouth'],
        half: ['eyeL', 'eyeR', 'mouth'],
      },
      states: STATES,
      devices: DEVICES,
      deviceColors: DEVICE_COLORS,
      stage: STAGE,
    },
    null,
    2,
  )}\n`,
);
console.log(`mascot.glb: ${(model.length / 1024).toFixed(1)} KB, ${list.length} parts, ${voxels} voxels.`);
