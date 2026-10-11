// Checks every color pair listed in tokens.json `contrast.pairs` against WCAG 2.1.
// Usage: node design/tokens/check-contrast.mjs
// Exits with code 1 if any pair is below its minimum.

import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const tokens = JSON.parse(readFileSync(join(here, 'tokens.json'), 'utf8'));

export function parseHex(hex) {
  const h = hex.replace('#', '');
  const n = (i) => parseInt(h.slice(i, i + 2), 16);
  return { r: n(0), g: n(2), b: n(4), a: h.length === 8 ? n(6) / 255 : 1 };
}

// Places a see-through color on top of an opaque one.
function over(fg, bg) {
  const mix = (f, b) => Math.round(f * fg.a + b * (1 - fg.a));
  return { r: mix(fg.r, bg.r), g: mix(fg.g, bg.g), b: mix(fg.b, bg.b), a: 1 };
}

function luminance({ r, g, b }) {
  const lin = (c) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

export function contrast(fgHex, bgHex, baseHex) {
  let bg = parseHex(bgHex);
  if (bg.a < 1) bg = over(bg, parseHex(baseHex));
  const fg = over(parseHex(fgHex), bg);
  const [hi, lo] = [luminance(fg), luminance(bg)].sort((a, b) => b - a);
  return (hi + 0.05) / (lo + 0.05);
}

let failures = 0;
const rows = [];
for (const theme of Object.keys(tokens.color)) {
  const c = tokens.color[theme];
  for (const pair of tokens.contrast.pairs) {
    for (const bg of pair.bg) {
      const ratio = contrast(c[pair.fg], c[bg], c.canvas);
      const ok = ratio >= pair.min;
      if (!ok) failures++;
      rows.push({ theme, fg: pair.fg, bg, ratio: ratio.toFixed(2), min: pair.min, result: ok ? 'pass' : 'FAIL' });
    }
  }
}

console.table(rows);
console.log(failures === 0 ? `All ${rows.length} pairs pass.` : `${failures} of ${rows.length} pairs fail.`);
process.exit(failures === 0 ? 0 : 1);
