// The Home room: an original lofi night room in one-point perspective, inked and cel shaded.
// Everything is drawn once in daylight colours; design/room/build.mjs turns it into night and lamp
// light with hard-edged colour filters, then renders it to images both apps use. No gradients.
const hex = (h) => [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16));
const mix = (a, b, t) => { const A = hex(a), B = hex(b); return '#' + A.map((v, i) => Math.round(v + (B[i] - v) * t).toString(16).padStart(2, '0')).join(''); };
const lt = (c, t) => mix(c, '#ffffff', t), dk = (c, t) => mix(c, '#000000', t);
const rng = (seed) => { let a = seed >>> 0; return () => { a = (a + 0x6d2b79f5) >>> 0; let t = a; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; }; };
const VP = [800, 330], F = 900, NS = 'style="stroke:none"', pick = (r, a) => a[Math.floor(r() * a.length)];
const n1 = (v) => (+v).toFixed(1);
const Pg = (a, c, ex = '') => `<polygon points="${a.map((p) => `${n1(p[0])},${n1(p[1])}`).join(' ')}" fill="${c}" ${ex}/>`;
const R = (x, y, w, h, c, ex = '') => `<rect x="${n1(x)}" y="${n1(y)}" width="${n1(w)}" height="${n1(h)}" fill="${c}" ${ex}/>`;
const C = (x, y, r, c, ex = '') => `<circle cx="${n1(x)}" cy="${n1(y)}" r="${n1(r)}" fill="${c}" ${ex}/>`;
const E = (x, y, a, b, c, ex = '') => `<ellipse cx="${n1(x)}" cy="${n1(y)}" rx="${n1(a)}" ry="${n1(b)}" fill="${c}" ${ex}/>`;
const Pa = (d, c, ex = '') => `<path d="${d}" fill="${c}" ${ex}/>`;
const St = (d, c, w = 1.4, ex = '') => `<path d="${d}" fill="none" stroke="${c}" stroke-width="${w}" stroke-linecap="round" stroke-linejoin="round" ${ex}/>`;
const Ln = (x1, y1, x2, y2, c, w = 1, ex = '') => `<line x1="${n1(x1)}" y1="${n1(y1)}" x2="${n1(x2)}" y2="${n1(y2)}" stroke="${c}" stroke-width="${w}" ${ex}/>`;
const anim = (c, d, delay = 0) => `class="${c}" style="--d:${d}s;animation-delay:${delay}s"`;
const ext = (p, t) => [VP[0] + (p[0] - VP[0]) * t, VP[1] + (p[1] - VP[1]) * t];
/** A box from its front face on screen, going `d` deeper; shows the sides the camera can see. */
const bx = (x, y, w, h, z, d, c, o = {}) => {
  const K = (F + z) / (F + z + d), b = (px, py) => ext([px, py], K), top = o.top ?? lt(c, 0.16), side = o.side ?? dk(c, 0.22), under = o.under ?? dk(c, 0.42); let s = '';
  if (x > VP[0]) s += Pg([[x, y], b(x, y), b(x, y + h), [x, y + h]], side);
  if (x + w < VP[0]) s += Pg([[x + w, y], b(x + w, y), b(x + w, y + h), [x + w, y + h]], side);
  if (y > VP[1]) s += Pg([[x, y], [x + w, y], b(x + w, y), b(x, y)], top);
  if (y + h < VP[1]) s += Pg([[x, y + h], [x + w, y + h], b(x + w, y + h), b(x, y + h)], under);
  return s + R(x, y, w, h, c);
};
const BOOK = ['#c9413d', '#3d73b5', '#e0ad43', '#3e9169', '#7f58b3', '#e5e0d3', '#37465f', '#d9774a', '#8f9bb0', '#2d7f86', '#b9486f'];
/** A row of upright books, each with its own bands and title marks, some leaning, some with gaps. */
function books(x0, x1, base, hMax, seed, gap = 0.1) {
  const r = rng(seed); let s = '', x = x0;
  while (x < x1 - 8) {
    if (r() < gap) { x += 7 + r() * 16; continue; }
    const w = 8 + r() * 13, h = hMax * (0.68 + r() * 0.32), c = pick(r, BOOK), lean = r() < 0.07 ? 5 : 0;
    if (x + w > x1) break;
    s += `<g ${lean ? `transform="rotate(${lean} ${n1(x)} ${n1(base)})"` : ''}>` + R(x, base - h, w, h, c) + R(x + w * 0.72, base - h + 1, w * 0.28, h - 2, dk(c, 0.18), NS);
    if (r() < 0.7) s += R(x + 1, base - h * 0.84, w - 2, 3, lt(c, 0.55), NS) + R(x + 1, base - h * 0.78, w - 2, 1.2, lt(c, 0.55), NS);
    if (r() < 0.5) s += R(x + w * 0.3, base - h * 0.62, w * 0.34, h * 0.28, lt(c, 0.4), NS);
    if (r() < 0.3) s += R(x + 1, base - 9, w - 2, 2, dk(c, 0.45), NS);
    s += '</g>'; x += w + 0.7;
  }
  return s;
}
function pile(x, base, w, n, seed) { const r = rng(seed); let s = '', y = base; for (let i = 0; i < n; i++) { const h = 7 + r() * 8, ww = w * (0.72 + r() * 0.28), dx = (r() - 0.5) * 12, c = pick(r, BOOK); y -= h; s += R(x + dx, y, ww, h, c) + R(x + dx + ww - 8, y + 1, 7, h - 2, '#efe9da', NS) + Ln(x + dx + ww - 8, y + h / 2, x + dx + ww, y + h / 2, '#c9c1ae', 0.8) + R(x + dx + 6, y + 2, 2.5, h - 4, lt(c, 0.5), NS); } return s; }
const rocket = (x, b, s, c = '#e8e4d8') => `<g transform="translate(${x} ${b}) scale(${s})">` + Pa('M-9 -6 L-22 0 L-9 -22 Z M9 -6 L22 0 L9 -22 Z', '#c9413d') + Pa('M-9 0 L-11 -46 Q-11 -74 0 -92 Q11 -74 11 -46 L9 0 Z', c) + Pa('M-11 -46 Q-11 -74 0 -92 Q2 -70 2 -46 Z', lt(c, 0.5), NS) + Pa('M-7.5 -74 Q0 -96 7.5 -74 Z', '#c9413d') + C(0, -54, 4.5, '#3d73b5') + R(-10, -26, 20, 4, '#37465f', NS) + R(-6, 0, 12, 5, '#37465f') + '</g>';
const satellite = (x, y, s) => `<g transform="translate(${x} ${y}) scale(${s})">` + R(-11, -11, 22, 22, '#d5b45c') + R(-40, -6, 25, 12, '#2d5fa8') + R(15, -6, 25, 12, '#2d5fa8') + Ln(-27, -6, -27, 6, '#9fc0ee') + Ln(27, -6, 27, 6, '#9fc0ee') + Ln(-15, 0, -11, 0, '#333', 2) + Ln(15, 0, 11, 0, '#333', 2) + Pa('M0 -11 L-7 -24 L7 -24 Z', '#c8ccd4') + C(0, -2, 4, '#7f8896') + '</g>';
const globe = (x, b, r, c = '#2f7f9c', land = '#c6a84f') => { let s = R(x - r * 0.62, b - 6, r * 1.24, 6, '#6e5832') + Ln(x, b - 6, x, b - 16, '#6e5832', 4) + C(x, b - r - 16, r, c) + Pa(`M${x - r * 0.5} ${b - r * 1.5 - 16} q${r * 0.5} ${r * 0.1} ${r * 0.6} ${r * 0.6} q-${r * 0.3} ${r * 0.5} -${r * 0.5} ${r * 0.2} Z M${x + r * 0.2} ${b - r * 0.8 - 16} q${r * 0.6} ${r * -0.1} ${r * 0.5} ${r * 0.4} q-${r * 0.4} ${r * 0.3} -${r * 0.6} ${r * 0.1} Z`, land, NS); return s + St(`M${x - r * 1.14} ${b - r - 16} A${r * 1.14} ${r * 1.14} 0 0 1 ${x + r * 0.3} ${b - r * 2.1 - 16}`, '#8a6e38', 2.6) + E(x, b - r - 16, r, r * 0.2, 'none', 'style="stroke:#d9e4e8;stroke-width:.7;opacity:.5"'); };
const plant = (x, b, s, seed) => { const r = rng(seed); let o = `<g transform="translate(${x} ${b}) scale(${s})">`; for (let i = 0; i < 8; i++) { const a = -2.8 + (i / 7) * 2.5, l = 30 + r() * 22; o += Pa(`M0 -18 Q${Math.cos(a) * l * 0.5 - 6} ${-18 + Math.sin(a) * l * 0.6} ${Math.cos(a) * l} ${-18 + Math.sin(a) * l} Q${Math.cos(a) * l * 0.5 + 6} ${-18 + Math.sin(a) * l * 0.3} 0 -18 Z`, i % 2 ? '#3f9b62' : '#58b47a'); } return o + Pa('M-16 -20 L16 -20 L12 0 L-12 0 Z', '#c9684a') + R(-17, -24, 34, 5, '#d98061') + '</g>'; };
const boxC = (x, b, w, h, c, label) => bx(x, b - h, w, h, 300, 30, c) + (label ? R(x + w * 0.2, b - h * 0.65, w * 0.6, h * 0.3, '#f1ead9', NS) + Ln(x + w * 0.27, b - h * 0.5, x + w * 0.73, b - h * 0.5, '#6d6554', 1.2) : '') + R(x + w * 0.4, b - h + 3, w * 0.2, 4, dk(c, 0.5), NS);
const cloth = (id, base, a, b) => `<pattern id="${id}" width="36" height="36" patternUnits="userSpaceOnUse" patternTransform="rotate(12)">` + R(0, 0, 36, 36, base, NS) + R(0, 0, 36, 7, a, 'style="stroke:none;opacity:.8"') + R(0, 0, 7, 36, a, 'style="stroke:none;opacity:.8"') + R(0, 22, 36, 2, b, 'style="stroke:none;opacity:.8"') + R(22, 0, 2, 36, b, 'style="stroke:none;opacity:.8"') + '</pattern>';

// ---------- The walls, floor, posters ----------
function shell(v) {
  const r = rng(5), TL = [260, 78], TR = [1340, 78], BR = [1340, 768], BL = [260, 768], T = 3.4, wall = '#6f84c2';
  let s = R(TL[0], TL[1], 1080, 690, wall, NS);
  for (let x = TL[0]; x < TR[0]; x += 54) s += R(x, TL[1], 27, 690, dk(wall, 0.05), NS);
  s += Pg([TL, BL, ext(BL, T), ext(TL, T)], '#5b6dab', NS) + Pg([TR, BR, ext(BR, T), ext(TR, T)], '#6375b0', NS) + Pg([TL, TR, ext(TR, T), ext(TL, T)], '#50619c', NS);
  for (let t = 1.1; t < 3.2; t += 0.18) s += Pg([ext([260, 78], t), ext([260, 768], t), ext([260, 768], t + 0.08), ext([260, 78], t + 0.08)], '#55669f', NS) + Pg([ext([1340, 78], t), ext([1340, 768], t), ext([1340, 768], t + 0.08), ext([1340, 78], t + 0.08)], '#5d6ea7', NS);
  // Floor: planks that run to the vanishing point, each cut at a different place.
  s += Pg([BL, BR, ext(BR, T), ext(BL, T)], '#74615a', NS);
  for (let i = 0; i < 29; i++) { const x0 = 260 + i * 40, a = [x0, 768], b = [x0 + 40, 768], t = 1 + r() * 0.1; s += Pg([ext(a, 1), ext(b, 1), ext(b, T), ext(a, T)], pick(r, ['#74615a', '#7b675f', '#6c5a54', '#806b62']), NS) + Ln(...ext(a, 1), ...ext(a, T), '#54463f', 1); for (let q = t; q < T; q += 0.28 + r() * 0.35) s += Ln(...ext(a, q), ...ext(b, q), '#54463f', 1); }
  s += R(260, 744, 1080, 24, '#b9bfd2') + Ln(260, 750, 1340, 750, '#8b92aa');
  s += Pg([BL, [260, 744], ext([260, 744], T), ext(BL, T)], '#a0a7bd') + Pg([BR, [1340, 744], ext([1340, 744], T), ext(BR, T)], '#a9b0c5');
  // A paper on a wall, drawn in that wall's perspective.
  const onL = (t0, t1, y0, y1) => (u, q) => { const t = t0 + (t1 - t0) * u; return ext([260, y0 + (y1 - y0) * q], t); };
  const onR = (t0, t1, y0, y1) => (u, q) => { const t = t0 + (t1 - t0) * u; return ext([1340, y0 + (y1 - y0) * q], t); };
  const paper = (f, c) => Pg([f(0, 0), f(1, 0), f(1, 1), f(0, 1)], c);
  const pinned = (f) => C(...f(0.5, 0.03), 2.6, '#c9413d', NS);
  // Left wall: moon phases, a long-exposure ring of stars, a launch photo, a note.
  let f = onL(1.12, 1.42, 120, 330); s += paper(f, '#111a3d'); for (let i = 0; i < 5; i++) { const q = 0.12 + i * 0.19; s += `<circle cx="${n1(f(0.5, q)[0])}" cy="${n1(f(0.5, q)[1])}" r="9" fill="#e9e2c5" ${NS}/>` + `<circle cx="${n1(f(0.5, q)[0] + (i - 2) * 6)}" cy="${n1(f(0.5, q)[1])}" r="${i === 2 ? 0 : 9}" fill="#111a3d" ${NS}/>`; } s += pinned(f);
  f = onL(1.48, 2.0, 130, 400); s += paper(f, '#0e1737'); { const c = f(0.5, 0.45); for (let i = 1; i < 11; i++) s += `<ellipse cx="${n1(c[0])}" cy="${n1(c[1])}" rx="${i * 8}" ry="${i * 12}" fill="none" stroke="#a8c1ee" stroke-width=".8" stroke-dasharray="${8 + i * 3} ${9 + i}" style="opacity:.75"/>`; } s += Pg([f(0, 0.8), f(1, 0.8), f(1, 1), f(0, 1)], '#25463e', NS) + pinned(f);
  f = onL(2.05, 2.5, 160, 310); s += paper(f, '#efe7d3') + Pg([f(0.08, 0.1), f(0.92, 0.1), f(0.92, 0.7), f(0.08, 0.7)], '#23304e', NS) + Pg([f(0.45, 0.68), f(0.5, 0.2), f(0.55, 0.68)], '#f3f0e6', NS) + Pg([f(0.3, 0.7), f(0.7, 0.7), f(0.5, 0.52)], '#e8953f', NS) + pinned(f);
  // Right wall: a ringed planet, and a hand-drawn constellation map.
  f = onR(1.14, 1.42, 110, 400); s += paper(f, '#0a1230'); { const c = f(0.5, 0.42); s += `<ellipse cx="${n1(c[0])}" cy="${n1(c[1] + 8)}" rx="56" ry="12" fill="none" stroke="#e7cb91" stroke-width="5" ${NS.replace('stroke:none', 'opacity:.9')}/>` + C(c[0], c[1], 34, '#d9a463', NS) + Pa(`M${c[0] - 33} ${c[1] - 6} Q${c[0]} ${c[1] - 14} ${c[0] + 33} ${c[1] - 6} L${c[0] + 34} ${c[1] + 3} Q${c[0]} ${c[1] - 3} ${c[0] - 34} ${c[1] + 3} Z`, '#b57c48', NS) + Pa(`M${c[0] - 56} ${c[1] + 8} A56 12 0 0 0 ${c[0] + 56} ${c[1] + 8} L${c[0] + 44} ${c[1] + 12} A44 8 0 0 1 ${c[0] - 44} ${c[1] + 12} Z`, '#e7cb91', NS); } s += pinned(f);
  f = onR(1.48, 1.92, 150, 440); s += paper(f, '#14224f'); { const q = rng(23); let last = null; for (let i = 0; i < 12; i++) { const p = f(0.12 + q() * 0.76, 0.1 + q() * 0.8); if (last && i % 4) s += Ln(last[0], last[1], p[0], p[1], '#8fb0e6', 0.9); s += C(p[0], p[1], 2.2, '#fff6cf', NS); last = p; } } s += pinned(f);
  // A mobile of planets hanging from the ceiling (only a phone sees all of it).
  s += Ln(1160, -60, 1160, 40, '#2a2f44', 1.4) + Ln(1090, 40, 1230, 40, '#2a2f44', 1.4) + Ln(1090, 40, 1090, 66, '#2a2f44', 1) + C(1090, 80, 15, '#d9774a') + Ln(1230, 40, 1230, 56, '#2a2f44', 1) + C(1230, 72, 18, '#6fa8d6');
  return s;
}

/** What's outside: drawn in its own night colours, so it isn't darkened twice. */
function outside() {
  const r = rng(9); let s = `<g clip-path="url(#winclip)">` + R(304, 168, 232, 250, '#0e1740') + R(304, 300, 232, 120, '#172358');
  for (let i = 0; i < 44; i++) s += C(308 + r() * 224, 172 + r() * 110, 0.6 + r() * 0.9, '#dfe9ff', '');
  s += C(478, 222, 22, '#f4eec9') + C(469, 216, 19, '#0e1740') + C(478, 222, 44, '#f4eec9', 'opacity=".07"');
  let x = 298; while (x < 540) { const w = 22 + r() * 30, h = 50 + r() * 90, c = r() < 0.5 ? '#0b1130' : '#111a44'; s += R(x, 420 - h, w, h, c, NS); for (let a = x + 4; a < x + w - 5; a += 8) for (let b = 426 - h; b < 410; b += 10) if (r() < 0.32) s += R(a, b, 4, 5, r() < 0.2 ? '#8fd6ff' : '#ffd27a', NS); x += w + 1; }
  return s + '</g>';
}

// ---------- The furniture: placed by a real camera, centimetres in, screen out ----------
const CAMS = 1440, EYE = 146;
const S3 = (z) => CAMS / z;
const P3 = (X, Y, Z) => [800 + X * S3(Z), 330 + (EYE - Y) * S3(Z)];
/** A real box in the room: x, height and depth in centimetres. Draws the faces the camera can see. */
function box3(X0, X1, Y0, Y1, Z0, Z1, c, o = {}) {
  const top = o.top ?? lt(c, 0.16), side = o.side ?? dk(c, 0.22), under = o.under ?? dk(c, 0.42); let s = '';
  if (X0 > 0) s += Pg([P3(X0, Y1, Z0), P3(X0, Y1, Z1), P3(X0, Y0, Z1), P3(X0, Y0, Z0)], side);
  if (X1 < 0) s += Pg([P3(X1, Y1, Z0), P3(X1, Y1, Z1), P3(X1, Y0, Z1), P3(X1, Y0, Z0)], side);
  if (Y1 < EYE) s += Pg([P3(X0, Y1, Z0), P3(X1, Y1, Z0), P3(X1, Y1, Z1), P3(X0, Y1, Z1)], top);
  if (Y0 > EYE) s += Pg([P3(X0, Y0, Z0), P3(X1, Y0, Z0), P3(X1, Y0, Z1), P3(X0, Y0, Z1)], under);
  return s + Pg([P3(X0, Y1, Z0), P3(X1, Y1, Z0), P3(X1, Y0, Z0), P3(X0, Y0, Z0)], c);
}
const HUTCH_S = S3(450), hutchY = (y) => EYE - (y - 330) / HUTCH_S;
const CASE_S = S3(440), caseY = (y) => EYE - (y - 330) / CASE_S;

/** Window frame, sill, curtains and what stands on the sill: everything rain must stay behind. */
function windowFront(v) {
  const girl = v === 'girl'; let s = '';
  s += R(292, 156, 256, 12, '#d6d9e3') + R(292, 156, 12, 268, '#c6cad8') + R(536, 156, 12, 268, '#c6cad8') + R(415, 168, 9, 250, '#c6cad8') + R(304, 292, 232, 8, '#c6cad8') + bx(282, 418, 276, 11, 560, 30, '#dfe2ea');
  const curtain = girl ? '#d98fa8' : '#b9544d';
  s += Pa('M274 142 L346 142 Q352 280 330 400 Q320 462 334 526 L270 526 Z', curtain) + St('M292 150 Q300 300 288 520 M312 150 Q322 300 310 520', dk(curtain, 0.25), 1.6) + Pa('M566 142 L504 142 Q498 240 520 370 Q532 450 516 526 L568 526 Z', curtain) + St('M548 150 Q540 300 552 520 M528 150 Q520 260 534 410', dk(curtain, 0.25), 1.6) + R(262, 136, 316, 8, '#7a5a3c') + C(262, 140, 6, '#5a4029') + C(578, 140, 6, '#5a4029');
  const catc = '#1b2030';
  s += `<g transform="translate(378 418)">` + E(0, -18, 25, 20, catc) + C(-20, -38, 14, catc) + Pa('M-32 -46 L-30 -60 L-20 -50 Z M-14 -50 L-5 -60 L-5 -44 Z', catc) + St('M22 -8 C48 -6 44 -28 33 -34', catc, 6) + '</g>';
  return s + plant(490, 418, 0.5, 4);
}

function room(v) {
  const r = rng(12), girl = v === 'girl'; let s = windowFront(v);
  // The hutch stands on the desk's back edge: sides go all the way down to the desk top.
  const hb = '#a78c6b', X = 78, Zf = 450, Zb = 480, top = hutchY(108), rows = [228, 335, 438].map(hutchY);
  s += Pg([P3(-X, top, Zb), P3(X, top, Zb), P3(X, 75, Zb), P3(-X, 75, Zb)], dk(hb, 0.36));
  // Row one is above the eye: its shelf hides the bottom of what stands there.
  s += books(562, 722, 228, 92, 31) + pile(730, 228, 60, 5, 8) + rocket(828, 228, 0.88) + rocket(860, 228, 0.6, '#c8d4e6') + satellite(930, 180, 0.7) + (girl ? plant(1005, 228, 0.62, 8) : R(986, 180, 44, 48, '#d9b65a') + R(990, 184, 36, 40, '#1b2852', NS) + C(1008, 202, 10, '#6fa8d6', NS));
  s += box3(-X, X, top, top + 4, Zf, Zb, hb) + box3(-X, X, rows[0] - 3, rows[0], Zf, Zb, hb) + box3(-X, X, rows[1] - 3, rows[1], Zf, Zb, hb) + box3(-X, X, rows[2] - 3, rows[2], Zf, Zb, hb);
  s += books(562, 628, 333, 84, 42, 0.04) + pile(640, 333, 66, 4, 11) + books(726, 786, 333, 90, 33, 0.14) + Pg([[906, 248], [954, 248], [972, 322], [888, 322]], '#ffe7a8', 'style="stroke:none;opacity:.14"') + R(922, 238, 16, 5, '#c9ccd4') + bx(892, 322, 76, 13, 480, 26, '#5f3f28') + bx(902, 310, 56, 12, 485, 18, '#74502f') + R(915, 324, 30, 8, '#d9b65a') + Ln(919, 328, 941, 328, '#7a5f22', 1) + R(925, 302, 10, 8, '#d9b65a');
  // Row three is the device shelf: books only at its ends, the middle stays free.
  s += books(562, 618, 434, 82, 53, 0.02) + books(966, 1036, 434, 84, 34, 0.1);
  const up = (x, y0, y1) => box3((x - 800) / HUTCH_S, (x - 800) / HUTCH_S + 2.4, y1, y0, Zf, Zb, hb);
  s += up(720, rows[0], top) + up(880, rows[0], top) + up(628, rows[1], rows[0]) + up(980, rows[1], rows[0]) + up(622, rows[2], rows[1]) + up(958, rows[2], rows[1]);
  s += box3(-X - 3, -X, 75, top + 4, Zf, Zb, hb) + box3(X, X + 3, 75, top + 4, Zf, Zb, hb);
  // A cork board on the wall under the hutch.
  { const q = rng(55); s += R(588, 452, 300, 84, '#b98d5e') + R(592, 456, 292, 76, '#c79d6d', NS);
    const pts = []; for (let i = 0; i < 9; i++) { const x = 600 + (i % 5) * 56 + q() * 10, y = 459 + Math.floor(i / 5) * 35 + q() * 3, w = 34 + q() * 12, h = 22 + q() * 7, c = pick(q, ['#f4dc74', '#f2f0e6', '#a9d4e8', '#f2a0b0', '#1b2852']), a = (q() - 0.5) * 10; pts.push([x + w / 2, y + 2]); s += `<g transform="rotate(${n1(a)} ${n1(x)} ${n1(y)})">` + R(x, y, w, h, c); if (c === '#1b2852') for (let k = 0; k < 7; k++) s += C(x + 3 + q() * (w - 6), y + 3 + q() * (h - 6), 0.9, '#fff', NS); else for (let k = 1; k < 4; k++) s += Ln(x + 4, y + k * h / 4, x + w - 4 - q() * 8, y + k * h / 4, dk(c, 0.42), 0.7); s += '</g>' + C(x + w / 2, y + 2, 2.2, pick(q, ['#c9413d', '#3d73b5', '#e0ad43']), NS); }
    for (let i = 0; i < pts.length - 1; i += 2) s += Ln(pts[i][0], pts[i][1], pts[i + 1][0], pts[i + 1][1], '#a82f2f', 0.9); }

  // The tall bookcase on the right: stands on the floor, its base at the floor line for its depth.
  const bc = '#9d8362', Bx0 = 88, Bx1 = 164, Bz0 = 440, Bz1 = 480, caseTop = caseY(135), caseRows = [240, 340, 440, 540, 640, 740].map(caseY);
  s += Pg([P3(Bx0, caseTop, Bz1), P3(Bx1, caseTop, Bz1), P3(Bx1, 0, Bz1), P3(Bx0, 0, Bz1)], dk(bc, 0.42));
  for (const y of caseRows) s += box3(Bx0, Bx1, y - 3, y, Bz0, Bz1, bc);
  s += box3(Bx0, Bx1, caseTop, caseTop + 4, Bz0, Bz1, bc) + box3(Bx0, Bx1, 0, 8, Bz0, Bz1, dk(bc, 0.1));
  s += books(1106, 1226, 238, 92, 61, 0.05) + rocket(1262, 238, 0.74, '#e8e4d8') + `<g transform="translate(1188 338)">` + E(0, -34, 32, 34, '#1a1740') + (() => { let o = ''; const q = rng(70); for (let i = 0; i < 26; i++) { const a = q() * 6.28, d = q() * 28; o += C(Math.cos(a) * d, -34 + Math.sin(a) * d * 1.05, 1.1 + q() * 1.2, pick(q, ['#ffe48a', '#8fd6ff', '#ffffff'])); } return o; })() + R(-14, -8, 28, 8, '#6e5832') + '</g>';
  s += books(1108, 1160, 338, 82, 62, 0.2) + books(1236, 1312, 338, 80, 63, 0.05);
  s += pile(1110, 438, 74, 5, 14) + books(1194, 1312, 438, 88, 64, 0.08);
  s += books(1106, 1312, 538, 92, 65, 0.1);
  s += globe(1156, 638, 34, '#34658f') + boxC(1226, 638, 74, 64, '#d4a85a', true);
  s += boxC(1108, 738, 88, 72, '#7b6243', false) + books(1208, 1312, 738, 76, 66, 0.15);
  s += box3(Bx0 - 3, Bx0, 0, caseTop + 4, Bz0, Bz1, bc) + box3(Bx1, Bx1 + 3, 0, caseTop + 4, Bz0, Bz1, bc);
  if (girl) s += plant(1282, 538, 0.5, 9);
  else s += `<g transform="translate(14 56) rotate(-11 1060 750)">` + R(1036, 560, 34, 190, '#d9774a', 'rx="16"') + R(1044, 590, 18, 130, '#23304e', 'rx="8" style="stroke:none"') + C(1032, 610, 7, '#efe7d3') + C(1074, 610, 7, '#efe7d3') + C(1032, 700, 7, '#efe7d3') + C(1074, 700, 7, '#efe7d3') + '</g>';

  // The bed: headboard on the wall, running toward you, on the floor.
  const sheetC = girl ? '#c7a6cf' : '#5f7fa5', duvet = girl ? '#a886b6' : '#46638a', Lx = -178, Rx = -114;
  s += Pg([P3(Lx, 74, 480), P3(Rx, 74, 480), P3(Rx, 0, 480), P3(Lx, 0, 480)], '#6e5639') + Pg([P3(Lx + 3, 71, 480), P3(Rx - 3, 71, 480), P3(Rx - 3, 42, 480), P3(Lx + 3, 42, 480)], '#7c6242', NS);
  s += Pg([P3(Rx, 22, 480), P3(Rx, 22, 230), P3(Rx, 0, 230), P3(Rx, 0, 480)], '#54422c');
  s += Pg([P3(Rx, 46, 480), P3(Rx, 46, 230), P3(Rx, 22, 230), P3(Rx, 22, 480)], dk(sheetC, 0.3));
  s += Pg([P3(Lx, 46, 480), P3(Rx, 46, 480), P3(Rx, 46, 230), P3(Lx, 46, 230)], sheetC);
  s += Pg([P3(Lx, 47, 425), P3(Rx, 47, 425), P3(Rx, 47, 230), P3(Lx, 47, 230)], duvet) + Pg([P3(Rx, 47, 425), P3(Rx, 30, 425), P3(Rx, 30, 230), P3(Rx, 47, 230)], dk(duvet, 0.28));
  for (let z = 400; z > 240; z -= 26) { const a = P3(Lx, 47, z), b = P3(Rx, 47, z), m = P3((Lx + Rx) / 2, 46, z); s += St(`M${n1(a[0])} ${n1(a[1])} Q${n1(m[0])} ${n1(m[1] + 5)} ${n1(b[0])} ${n1(b[1])}`, dk(duvet, 0.2), 1.3); }
  s += `<g transform="translate(-16 -46)">` + Pa('M286 660 Q282 636 310 636 L366 638 Q388 640 384 666 Q382 692 356 692 L304 690 Q286 688 286 660 Z', '#efe8d8') + St('M300 650 Q334 666 370 650', '#bdb49f', 1.3) + Pa('M382 662 Q380 638 406 638 L446 640 Q466 642 462 666 Q460 692 438 692 L398 690 Q382 688 382 662 Z', '#e5dcc9') + St('M396 652 Q424 664 450 652', '#b5ac97', 1.3) + '</g>';

  // The nightstand: on the floor beside the bed, with a mug on it.
  s += box3(-112, -84, 0, 49, 445, 480, '#94785a') + box3(-112, -84, 49, 52, 445, 480, '#a58865');
  { const a = P3(-109, 46, 445), b = P3(-87, 26, 445), c = P3(-109, 23, 445), d = P3(-87, 4, 445); s += R(a[0], a[1], b[0] - a[0], b[1] - a[1], '#a58865') + R(c[0], c[1], d[0] - c[0], d[1] - c[1], '#a58865') + C((a[0] + b[0]) / 2, (a[1] + b[1]) / 2, 3, '#55422b') + C((c[0] + d[0]) / 2, (c[1] + d[1]) / 2, 3, '#55422b'); }
  { const m = P3(-89, 52, 455); s += R(m[0] - 11, m[1] - 25, 22, 25, girl ? '#e9a6bb' : '#e8e4d8', 'rx="3"'); }

  // The desk: legs and drawers reach the floor, cables reach the desk.
  const dc = '#ab8e6c', Dz0 = 410, Dz1 = 480, Dy = 71.5;
  s += Pg([P3(-80, Dy, 476), P3(80, Dy, 476), P3(80, 0, 476), P3(-80, 0, 476)], dk(dc, 0.5));
  s += box3(-74, -48, 0, 22, 440, 472, '#8a6d48'); for (let i = 0; i < 12; i++) { const a = P3(-72 + i * 1.9, 46 - (i % 3) * 3, 440), b = P3(-70.3 + i * 1.9, 22, 440); s += R(a[0], a[1], b[0] - a[0], b[1] - a[1], pick(r, BOOK)); }
  { const a = P3(20, 45, 432), w = 14 * S3(432), h = 45 * S3(432); s += box3(20, 34, 0, 45, 432, 472, '#262c3e') + R(a[0] + 5, a[1] + 5, w - 10, 2, '#515a75', NS); [0.25, 0.5, 0.75].forEach((q, i) => (s += C(a[0] + w / 2, a[1] + h * q, 11, ['#3c8db0', '#7a5fb0', '#b0568a'][i]) + C(a[0] + w / 2, a[1] + h * q, 6.5, '#141827') + C(a[0] + w / 2, a[1] + h * q, 2, '#d2d8e6'))); }
  s += box3(36, 76, 0, Dy, 412, 478, dk(dc, 0.06)); { for (let i = 0; i < 3; i++) { const a = P3(39, Dy - 3 - i * 23, 412), b = P3(73, Dy - 23 - i * 23, 412); s += R(a[0], a[1], b[0] - a[0], b[1] - a[1], dc) + R(a[0] + (b[0] - a[0]) * 0.32, a[1] + (b[1] - a[1]) * 0.42, (b[0] - a[0]) * 0.36, 7, '#5c4730'); } }
  s += box3(-80, -75, 0, Dy, 412, 478, dk(dc, 0.1)) + box3(-80, 80, Dy, 75, Dz0, Dz1, dc);
  // The lamp: its foot stands on the desk's back corner.
  { const f = P3(-73, 75, 470); s += E(f[0], f[1], 18, 5, '#23283a') + St(`M${n1(f[0])} ${n1(f[1])} L540 420 L632 306`, '#23283a', 6) + St(`M${n1(f[0] + 6)} ${n1(f[1])} L548 426`, '#44506d', 1.5) + C(540, 420, 8, '#44506d'); }
  s += Pa('M618 278 L668 290 L708 346 L608 324 Z', '#23283a') + Pa('M618 278 L668 290 L680 306 L624 292 Z', '#3a4562', NS);
  // Under the sleeper's arms: an open book, lying on the desk.
  { const a = P3(-40, 75, 430), b = P3(12, 75, 430), c = P3(14, 75, 412), d = P3(-42, 75, 412); s += Pg([a, b, c, d], '#f0ead8'); }

  // The side table along the right wall, on four legs that reach the floor, with a shelf.
  const tb = '#a98d6c', Tx0 = 118, Tx1 = 178, Tz0 = 330, Tz1 = 438;
  s += box3(Tx1 - 5, Tx1 - 2, 0, 72, Tz1 - 6, Tz1 - 3, '#4a5266') + box3(Tx0 + 2, Tx0 + 5, 0, 72, Tz1 - 6, Tz1 - 3, '#4a5266');
  s += box3(Tx0 + 2, Tx1 - 2, 28, 31, Tz0 + 4, Tz1 - 4, tb);
  { const q = P3(150, 31, 395); s += pile(q[0] - 28, q[1], 54, 4, 91); }
  s += box3(Tx1 - 5, Tx1 - 2, 0, 72, Tz0 + 3, Tz0 + 6, '#4a5266') + box3(Tx0 + 2, Tx0 + 5, 0, 72, Tz0 + 3, Tz0 + 6, '#4a5266');
  s += box3(Tx0, Tx1, 72, 75, Tz0, Tz1, tb);
  // A bin and a bag, both on the floor.
  { const f = P3(78, 0, 398), k = S3(398) / 3.6; s += `<g transform="translate(${n1(f[0])} ${n1(f[1])}) scale(${n1(k)})">` + Pa('M-24 -64 L24 -64 L16 0 L-16 0 Z', '#59647f') + C(-10, -68, 8, '#efe9da') + C(6, -70, 9, '#e7e0ce') + Ln(-20, -50, 20, -50, '#3d465e', 1) + Ln(-18, -35, 18, -35, '#3d465e', 1) + '</g>'; }
  { const f = P3(-104, 0, 425), k = S3(425) / 3.4; s += `<g transform="translate(${n1(f[0])} ${n1(f[1])}) scale(${n1(k)})">` + Pa('M-38 0 Q-44 -116 0 -122 Q40 -116 34 0 Z', girl ? '#a06c9f' : '#47696b') + St('M-20 -116 Q-2 -156 16 -116', '#2a3942', 4) + R(-30, -66, 52, 40, girl ? '#8a5a89' : '#3b5a5c', 'rx="6"') + Ln(-28, -56, 20, -56, '#222', 1) + '</g>'; }
  return s;
}

/** What lies on the floor: a true circle of rug, seen from this eye height, and things lying on it. */
function floorItems(v) {
  const r = rng(3), girl = v === 'girl'; let s = '';
  [['#5e4a78', 78], ['#c9a662', 73], ['#3f6c84', 66], ['#d9d0ba', 58], ['#7d4a58', 50]].forEach(([c, rad]) => { const pts = []; for (let i = 0; i < 64; i++) { const a = (i / 64) * Math.PI * 2; pts.push(P3(-6 + Math.cos(a) * rad, 0, 360 + Math.sin(a) * rad)); } s += Pg(pts, c, NS); });
  [[-52, 395], [34, 368], [58, 402], [-34, 338], [10, 312]].forEach(([X, Z], i) => { const a = P3(X, 0, Z), w = 18 * S3(Z), t = (r() - 0.5) * 22; s += `<g transform="translate(${n1(a[0])} ${n1(a[1])}) skewX(${n1(t)})">` + R(0, 0, w, w * 0.34, '#f0ead8'); for (let k = 1; k < 4; k++) s += Ln(5, k * w * 0.075, w - 5, k * w * 0.075, '#a8a08c', 0.7); s += '</g>'; });
  { const c = girl ? '#d98fa8' : '#6f8c94', a = P3(66, 0, 352), k = S3(352) / 3.6; s += `<g transform="translate(${n1(a[0])} ${n1(a[1])}) scale(${n1(k)})">` + Pa('M0 0 Q6 -24 40 -18 Q64 -12 58 0 Q30 10 0 0 Z', c) + Pa('M66 8 Q72 -16 106 -10 Q130 -4 124 8 Q96 18 66 8 Z', c) + '</g>'; }
  return s;
}
/** The telescope, moved to the gap between the desk and the side table, its feet on the floor. */
function foreground(v) {
  let s = `<g transform="translate(160 170) scale(.8)">`;
  s += St('M1254 656 L1198 868 M1254 656 L1250 882 M1254 656 L1316 866', '#d8d9dd', 6) + St('M1254 656 L1198 868 M1254 656 L1250 882 M1254 656 L1316 866', '#6f7380', 1) + St('M1226 770 L1250 782 L1286 768', '#6f7380', 2);
  s += `<g transform="rotate(-38 1250 620)">` + R(1158, 596, 170, 46, '#eceef2') + R(1158, 596, 170, 12, '#ffffff', NS) + R(1140, 586, 32, 66, '#c6cad2') + R(1316, 604, 40, 30, '#2a2f44') + R(1214, 590, 16, 58, '#30384e') + R(1266, 590, 10, 58, '#30384e') + '</g>' + C(1252, 648, 12, '#30384e') + R(1236, 642, 32, 18, '#30384e') + '</g>';
  return s;
}

// ---------- The person: seen from behind, head down on folded arms, no face ----------
function person(v) {
  const girl = v === 'girl';
  const top = girl ? '#a58cc4' : '#b5483c', topD = dk(top, 0.26), topL = lt(top, 0.14), trouser = girl ? '#39466a' : '#3f3a33', hair = girl ? '#2c2029' : '#1b1a24', hairL = girl ? '#55394a' : '#3b3a4c', skin = girl ? '#e8bb9a' : '#d9a680', sock = girl ? '#efece2' : '#c2c6d0';
  let s = '';
  // The floor shadow, the lower legs going forward under the seat, then the chair's frame in front
  // of them: five spokes from one hub, a caster on the end of each, the lift, the seat.
  s += E(745, 852, 112, 13, '#05070f', 'style="stroke:none;opacity:.4"');
  s += Pa('M700 762 Q694 788 698 806 L728 806 Q732 788 736 762 Z', trouser) + Pa('M754 762 Q758 788 762 806 L792 806 Q796 788 790 762 Z', trouser);
  s += E(713, 808, 18, 8, sock) + E(777, 808, 18, 8, sock);
  const legs = [[745, 814], [644, 838], [846, 838], [680, 866], [810, 866]];
  for (const [x, y] of legs) s += Pa(`M745 826 L${x} ${y - 4} L${x} ${y + 4} L745 840 Z`, '#2a3044');
  for (const [x, y] of legs.slice(1)) s += R(x - 2, y + 3, 4, 6, '#232838') + E(x, y + 11, 9, 6, '#161a26');
  s += E(745, 833, 16, 8, '#1b2030') + R(736, 768, 18, 62, '#2a3044') + R(740, 768, 5, 62, '#454e68', NS);
  s += Pa('M672 764 Q745 780 818 764 L812 774 Q745 790 678 774 Z', '#1b2030') + Pa('M668 738 Q745 722 822 738 Q830 752 820 766 Q745 782 670 766 Q660 752 668 738 Z', '#2c3347');
  s += '@@';
  // Arms folded on the desk top: upper arms up over the desk's edge to the elbows, forearms in under the head.
  s += Pa('M708 604 Q672 594 654 566 Q646 550 660 545 Q676 541 690 552 L730 584 Z', top) + Pa('M782 604 Q818 594 836 566 Q844 550 830 545 Q814 541 800 552 L760 584 Z', top);
  s += St('M668 566 L682 576 M660 556 L676 570 M822 566 L808 576 M830 556 L814 570', dk(top, 0.42), 1.4);
  s += Pa('M660 552 Q700 537 748 538 L748 560 Q704 562 670 566 Q652 562 660 552 Z', topD) + Pa('M830 552 Q790 537 742 538 L742 560 Q786 562 820 566 Q838 562 830 552 Z', topD);
    // A back leaning in to the desk: shoulders about two and a half heads wide, tapering to the waist.
  s += `<g transform="translate(0 -24)">`;
  s += Pa('M700 750 Q698 708 694 676 Q674 646 670 622 Q678 604 706 598 Q745 586 784 598 Q812 604 820 622 Q816 646 796 676 Q792 708 790 750 Q745 762 700 750 Z', top);
  s += Pa('M780 596 Q812 604 820 622 Q816 646 796 676 Q792 708 790 750 Q776 755 760 756 Q790 700 780 596 Z', topD, NS);
  s += Pa('M682 618 Q694 606 714 600 Q700 622 696 656 Q682 640 682 618 Z', topL, NS);
  s += St('M745 640 Q744 700 746 754', topD, 1.4) + St('M704 716 Q718 726 734 722 M762 726 Q776 724 788 712 M698 682 Q710 690 724 688 M792 680 Q780 690 766 690', topD, 1.3);
  for (let i = 0; i < 9; i++) s += Ln(704 + i * 10.3, 744 + Math.sin(i / 2.8) * 5, 704 + i * 10.3, 755 + Math.sin(i / 2.8) * 5, dk(top, 0.36), 1);
  s += '</g>';
  // The head, from behind and tipped forward: neck, ears, hair. Nothing of the face shows.
  s += '@@' + `<g transform="translate(745 526) scale(.86) translate(-745 -540)">` + R(729, 556, 32, 68, dk(skin, 0.1)) + E(705, 540, 6, 11, skin) + E(785, 540, 6, 11, skin);
  if (girl) {
    s += Pa('M706 530 Q706 486 745 484 Q784 486 784 530 Q790 590 780 648 Q765 664 745 658 Q726 664 710 648 Q700 590 706 530 Z', hair);
    s += St('M722 520 Q716 586 722 648 M735 500 Q732 586 738 656 M750 498 Q752 586 752 656 M766 512 Q772 586 768 648', hairL, 1.6) + Pa('M718 510 Q745 490 772 510 Q775 520 770 524 Q745 508 720 524 Q715 518 718 510 Z', hairL, NS);
    s += Pa('M770 498 L775 490 L778 500 L788 500 L780 506 L784 516 L775 510 L767 516 L770 506 L762 500 Z', '#f2cb54');
  } else {
    s += Pa('M707 544 Q702 494 745 486 Q788 494 783 544 Q784 562 774 570 L768 562 L762 576 L754 564 L746 580 L738 564 L730 576 L724 562 L716 570 Q706 562 707 544 Z', hair);
    s += St('M745 496 Q722 506 716 540 M745 496 Q738 520 736 558 M745 496 Q756 520 756 558 M745 496 Q770 508 774 540 M726 500 Q712 514 712 532 M764 500 Q778 514 778 532', hairL, 1.5) + Pa('M716 514 Q745 494 774 514 Q776 522 772 526 Q745 510 718 526 Q714 520 716 514 Z', hairL, NS);
    s += St('M706 534 Q708 478 745 476 Q782 478 784 534', '#14161f', 6) + St('M710 516 Q716 484 745 481', '#59627a', 1.2) + R(696, 524, 16, 34, '#22283a', 'rx="7"') + R(778, 524, 16, 34, '#22283a', 'rx="7"');
  }
  s += '</g>@@';
  // The hood slumps at the nape (the girl's hair covers hers).
  if (!girl) s += `<g transform="translate(0 -24)">` + Pa('M712 596 Q745 582 778 596 Q782 620 758 630 Q745 634 732 630 Q708 620 712 596 Z', topL) + Pa('M724 596 Q745 588 766 596 Q764 610 745 615 Q726 610 724 596 Z', dk(top, 0.42)) + St('M718 608 Q745 634 772 608', topD, 1.4) + '</g>';
  // The blanket slipping off one shoulder, hanging outside the chair's back, down to the floor.
  s += Pa('M772 576 Q806 580 818 614 Q830 690 832 770 Q842 810 856 846 Q826 860 800 842 Q804 760 802 700 Q800 640 762 604 Z', 'url(#plaid)') + St('M798 610 Q816 660 818 744 M814 780 Q826 816 838 844', dk('#6b7c9a', 0.28), 1.4);
  s += '@@';
  // The chair's back, a mesh panel, in front of the person.
  s += `<g transform="translate(-23 6)">` + Pa('M712 700 Q712 660 728 650 L808 650 Q824 660 824 700 L824 722 Q768 736 712 722 Z', '#2a3044') + Pa('M724 700 Q724 668 736 662 L800 662 Q812 668 812 700 L812 712 Q768 724 724 712 Z', '#3a435c', NS);
  for (let i = 0; i < 8; i++) s += Ln(732 + i * 10, 664, 732 + i * 10, 716, '#232838', 0.8);
  for (let i = 0; i < 5; i++) s += Ln(726, 668 + i * 10, 810, 668 + i * 10, '#232838', 0.8);
  return (s + R(758, 722, 22, 24, '#232838') + '</g>').split('@@'); // under, torso, head, over, chair
}

// ---------- Devices: drawn in the room's own ink, their screens are the only bright things, each at its real place ----------
const WALLS = [['#2251d6', '#7a49e6', '#ff8a6b', '#ffd166'], ['#0f7f79', '#22b8a0', '#f4d35e', '#ef9a50'], ['#3a0ca3', '#8a1bbd', '#f72585', '#4cc9f0'], ['#14305d', '#2a7fa8', '#7ec4de', '#fcae30'], ['#a8324f', '#e8667f', '#ffd166', '#74e0bd'], ['#284f5a', '#2ea69a', '#ecc96f', '#ea7d5c']];
const ink = (s) => `<g class="ink">${s}</g>`;
function appScreen(x, y, w, h, wall, mode, on, pct, sc = 1) {
  if (!on) return R(x, y, w, h, '#090b13', NS) + Pa(`M${x + w * 0.56} ${y} L${x + w * 0.82} ${y} L${x + w * 0.42} ${y + h} L${x + w * 0.16} ${y + h} Z`, '#fff', 'style="stroke:none;opacity:.05"');
  let s = R(x, y, w, h, wall[1], NS) + C(x + w * 0.74, y + h * 0.3, h * 0.13, wall[3], NS);
  s += Pa(`M${x} ${y + h * 0.66} Q${x + w * 0.3} ${y + h * 0.42} ${x + w * 0.56} ${y + h * 0.62} T${x + w} ${y + h * 0.58} L${x + w} ${y + h} L${x} ${y + h} Z`, wall[2], NS) + Pa(`M${x} ${y + h * 0.82} Q${x + w * 0.4} ${y + h * 0.66} ${x + w} ${y + h * 0.84} L${x + w} ${y + h} L${x} ${y + h} Z`, dk(wall[0], 0.28), NS);
  if (mode === 'wide') {
    const win = (a, b, c, d, dark) => { let o = R(x + a * w, y + b * h, c * w, d * h, dark ? '#171b24' : '#f3f4f7', NS) + R(x + a * w, y + b * h, c * w, 0.075 * h, dark ? '#2a3140' : '#d9dce3', NS); const q = rng(Math.round(a * 91 + b * 37)); for (let k = 1; k <= Math.floor(d / 0.075); k++) o += R(x + (a + 0.02 + (dark ? q() * 0.05 : 0)) * w, y + (b + 0.03 + k * 0.075) * h, c * w * (0.28 + q() * 0.56), Math.max(1, h * 0.022), dark ? pick(q, [wall[3], wall[2], '#8ab4f8', '#9aa3b2']) : '#a9b0bd', NS); return o; };
    s += win(0.05, 0.08, 0.5, 0.7, true) + win(0.46, 0.2, 0.47, 0.54, false) + R(x + w * 0.2, y + h * 0.88, w * 0.6, h * 0.08, '#fff', 'style="stroke:none;opacity:.5"');
    for (let i = 0; i < 8; i++) s += R(x + w * (0.225 + i * 0.07), y + h * 0.895, w * 0.05, h * 0.055, ['#ff5f57', '#febc2e', '#28c840', '#4cc9f0', wall[3], wall[2], '#fff', '#a78bfa'][i], NS);
  } else if (mode === 'phone') {
    s += R(x + w * 0.24, y + h * 0.1, w * 0.52, h * 0.05, '#fff', NS) + R(x + w * 0.36, y + h * 0.17, w * 0.28, h * 0.018, '#fff', 'style="stroke:none;opacity:.75"');
    for (let i = 0; i < 12; i++) s += R(x + w * (0.11 + (i % 4) * 0.2), y + h * (0.3 + Math.floor(i / 4) * 0.12), w * 0.15, w * 0.15, ['#ff5f57', '#febc2e', '#28c840', '#4cc9f0', wall[3], wall[2], '#f5f6f8', '#a78bfa', '#34d399', '#f472b6', '#60a5fa', '#fb923c'][i], 'rx="1.6" style="stroke:none"');
    for (let i = 0; i < 4; i++) s += R(x + w * (0.11 + i * 0.2), y + h * 0.84, w * 0.15, w * 0.15, ['#28c840', '#60a5fa', '#f5f6f8', '#fb923c'][i], 'rx="1.6" style="stroke:none"');
  } else {
    s += R(x, y, w * 0.26, h, '#f5f6f8', NS);
    for (let i = 0; i < 6; i++) s += R(x + w * 0.03, y + h * (0.1 + i * 0.13), w * (0.12 + i % 3 * 0.03), h * 0.03, i === 1 ? wall[0] : '#9aa3b2', NS);
    const cs = [wall[0], wall[2], wall[3], '#4cc9f0', wall[1], '#f472b6', '#34d399', '#fb923c', '#e2e5ea'];
    for (let i = 0; i < 9; i++) s += R(x + w * (0.3 + (i % 3) * 0.23), y + h * (0.08 + Math.floor(i / 3) * 0.3), w * 0.21, h * 0.26, cs[i], NS);
  }
  if (pct !== null) s += R(x + w * 0.12, y + h * 0.42, w * 0.76, h * 0.16, '#0b0e16', 'style="stroke:none;opacity:.9"') + R(x + w * 0.16, y + h * 0.48, w * 0.68, Math.max(2, h * 0.04), '#ffffff40', NS) + `<rect class="pbar" data-w="${n1(w * 0.68)}" x="${n1(x + w * 0.16)}" y="${n1(y + h * 0.48)}" width="${n1((w * 0.68 * pct) / 100)}" height="${n1(Math.max(2, h * 0.04))}" fill="#ffd23f" ${NS}/>`;
  return s + Pa(`M${x + w * 0.58} ${y} L${x + w * 0.8} ${y} L${x + w * 0.46} ${y + h} L${x + w * 0.24} ${y + h} Z`, '#fff', 'style="stroke:none;opacity:.08"');
}
const glow = (x, y, a, b, c, op = 0.34) => E(x, y, a, b, c, `filter="url(#g18)" style="mix-blend-mode:screen;opacity:${op};stroke:none"`);
const BODY = '#3a4258', BODY2 = '#2a3044';
const quad = (a, b, c, d, col, ex = '') => Pg([a, b, c, d], col, ex);
/** Maps a flat screen drawing onto a device's face, so turned devices turn their picture too. */
function onFace(tl, tr, bl, w, h, inner, id) {
  const a = (tr[0] - tl[0]) / w, b = (tr[1] - tl[1]) / w, c = (bl[0] - tl[0]) / h, d = (bl[1] - tl[1]) / h;
  return `<clipPath id="${id}"><rect x="0" y="0" width="${w}" height="${h}"/></clipPath><g transform="matrix(${a.toFixed(4)} ${b.toFixed(4)} ${c.toFixed(4)} ${d.toFixed(4)} ${tl[0].toFixed(2)} ${tl[1].toFixed(2)})"><g clip-path="url(#${id})">${inner}</g></g>`;
}
function device3(kind, slot, surfaceY, wallIndex, on) {
  const wall = WALLS[wallIndex % WALLS.length], a = (slot.yaw * Math.PI) / 180, ca = Math.cos(a), sa = Math.sin(a), id = `f${slot.id}${kind}`;
  const W = (x, y, z) => P3(slot.x + x * ca + z * sa, surfaceY + y, slot.z - x * sa + z * ca);
  const plate = (x0, x1, z0, z1, y0, y1, c, topC) => {
    // A flat slab on the surface: its top and the edges that face the camera.
    let s = '';
    s += quad(W(x0, y0, z0), W(x1, y0, z0), W(x1, y1, z0), W(x0, y1, z0), c);
    const cx = slot.x + (x0 + x1) / 2 * ca + (z0 + z1) / 2 * sa;
    if (cx > 0) s += quad(W(x0, y0, z0), W(x0, y0, z1), W(x0, y1, z1), W(x0, y1, z0), dk(c, 0.2)); else s += quad(W(x1, y0, z0), W(x1, y0, z1), W(x1, y1, z1), W(x1, y1, z0), dk(c, 0.2));
    return s + quad(W(x0, y1, z0), W(x1, y1, z0), W(x1, y1, z1), W(x0, y1, z1), topC);
  };
  const shadow = (x0, x1, z0, z1) => quad(W(x0 - 1, 0, z0 - 1), W(x1 + 1, 0, z0 - 1), W(x1 + 1, 0, z1 + 1), W(x0 - 1, 0, z1 + 1), '#05060c', 'style="stroke:none;opacity:.5"');
  const lightOn = (c, x, y, z, rx, ry, op) => { const p = W(x, y, z); return on ? glow(p[0], p[1], rx, ry, c, op) : ''; };
  let o = '';
  if (kind === 'desktop') {
    const sc = S3(slot.z);
    o += lightOn(wall[0], 0, 20, 0, 22 * sc * 1.25, 15 * sc * 1.25, 0.6) + shadow(-11, 11, -7, 7);
    o += ink(plate(-11, 11, -7, 7, 0, 1.2, BODY2, '#40485f'));
    o += ink(quad(W(-2.6, 1.2, 1), W(2.6, 1.2, 1), W(2.6, 9, 1), W(-2.6, 9, 1), BODY));
    const tl = W(-22, 34, 0), tr = W(22, 34, 0), bl = W(-22, 7, 0), br = W(22, 7, 0), side = slot.x > 0 ? -22 : 22;
    o += ink(quad(W(side, 34, 0), W(side, 34, 2), W(side, 7, 2), W(side, 7, 0), '#1a1e2b') + quad(tl, tr, br, bl, '#0c0e15'));
    o += onFace(W(-21, 33, 0), W(21, 33, 0), W(-21, 8.5, 0), 156, 82, appScreen(0, 0, 156, 82, wall, 'wide', on, null), id);
    o += `<g class="ink">${quad(W(-3, 7.6, 0), W(3, 7.6, 0), W(3, 7, 0), W(-3, 7, 0), '#2a3044')}</g>`;
  } else if (kind === 'laptop') {
    const sc = S3(slot.z), hingeZ = 10, topZ = 13.6, topY = 22;
    o += lightOn(wall[0], 0, 14, 8, 17 * sc * 1.3, 14 * sc * 1.2, 0.6) + shadow(-17, 17, -10, 11);
    const tl = W(-17, topY, topZ), tr = W(17, topY, topZ), bl = W(-17, 1.2, hingeZ), br = W(17, 1.2, hingeZ);
    o += ink(quad(tl, tr, br, bl, '#0c0e15'));
    o += onFace(W(-16, topY - 1, topZ - 0.2), W(16, topY - 1, topZ - 0.2), W(-16, 2.4, hingeZ + 0.2), 124, 83, appScreen(0, 0, 124, 83, wall, 'wide', on, null), id);
    o += ink(plate(-17, 17, -10, 10, 0, 1.2, '#2a3044', '#4b546c'));
    for (let r0 = 0; r0 < 4; r0++) for (let k = 0; k < 13; k++) { const x0 = -14.5 + k * 2.23, z0 = 8.8 - r0 * 2.6; o += quad(W(x0, 1.3, z0 - 2.1), W(x0 + 1.9, 1.3, z0 - 2.1), W(x0 + 1.9, 1.3, z0), W(x0, 1.3, z0), '#a0a9c2', NS); }
    o += quad(W(-5, 1.3, -9), W(5, 1.3, -9), W(5, 1.3, -3), W(-5, 1.3, -3), '#6c7691', NS);
  } else if (kind === 'phone') {
    const sc = S3(slot.z);
    o += lightOn(wall[0], 0, 15, 0, 15 * sc, 18 * sc, 0.65) + shadow(-6.5, 6.5, -4.5, 4.5);
    // A dock with a back rest: the phone leans into it, so it can't look as if it's balancing.
    o += ink(plate(-6.5, 6.5, -4.5, 4.5, 0, 1.6, '#2a3044', '#515a75'));
    o += ink(quad(W(-3.2, 1.6, 2.2), W(3.2, 1.6, 2.2), W(3.2, 11, 3.8), W(-3.2, 11, 3.8), '#3a4258'));
    const lean = (y) => 0.2 + (y - 1.6) * 0.17;
    const tl = W(-6, 27, lean(27)), tr = W(6, 27, lean(27)), bl = W(-6, 1.6, lean(1.6)), br = W(6, 1.6, lean(1.6));
    o += ink(quad(tl, tr, br, bl, '#0c0e15'));
    o += `<g>${onFace(W(-5.3, 26.2, lean(26.2)), W(5.3, 26.2, lean(26.2)), W(-5.3, 2.4, lean(2.4)), 44, 88, appScreen(0, 0, 44, 88, wall, 'phone', on, null), id)}</g>`;
    { const p = W(0, 25.7, lean(25.7)); o += C(p[0], p[1], 1.5, '#07080d', NS); }
    // The dock's front lip, in front of the phone's bottom edge.
    o += ink(quad(W(-6.5, 1.6, -0.6), W(6.5, 1.6, -0.6), W(6.5, 3.2, -0.6), W(-6.5, 3.2, -0.6), '#515a75'));
  } else {
    const sc = S3(slot.z), lean = (y) => -0.6 + (y - 1.4) * 0.25;
    const cover = wall[2] ? dk(wall[0], 0.15) : '#3a4258';
    o += lightOn(wall[0], 0, 11, 0, 15 * sc * 1.35, 13 * sc * 1.25, 0.65) + shadow(-15, 15, -4, 6);
    // A folio: the cover folds into a wedge behind the tablet and a tray in front of it.
    o += ink(plate(-15, 15, -4, 2, 0, 1.4, cover, lt(cover, 0.18)));
    o += ink(quad(W(-15, 19, lean(19) + 0.9), W(15, 19, lean(19) + 0.9), W(15, 1.4, 6), W(-15, 1.4, 6), dk(cover, 0.2)));
    const tl = W(-15, 21, lean(21)), tr = W(15, 21, lean(21)), bl = W(-15, 1.4, lean(1.4)), br = W(15, 1.4, lean(1.4));
    o += ink(quad(tl, tr, br, bl, '#0c0e15'));
    o += onFace(W(-14, 20.2, lean(20.2)), W(14, 20.2, lean(20.2)), W(-14, 2.2, lean(2.2)), 118, 76, appScreen(0, 0, 118, 76, wall, 'tablet', on, null), id);
    { const p = W(0, 20.6, lean(20.6)); o += C(p[0], p[1], 1.1, '#07080d', NS); }
    // A pen in its loop on the tablet's right side: nothing else in the room has one.
    o += ink(quad(W(15, 3, lean(3)), W(16.3, 3, lean(3)), W(16.3, 19, lean(19)), W(15, 19, lean(19)), '#e8e4d8'));
  }
  return o;
}
