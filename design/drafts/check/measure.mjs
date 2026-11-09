// Measures every draft page at every target size and theme, and reports what is out of place.
// Usage (from desktop/, with the drafts server on 4321 and the prototype server on 5173):
//   node ../design/drafts/check/measure.mjs <out-dir>
// Writes <out-dir>/report.json and screenshots, prints a summary.
import { createRequire } from 'node:module';
import { writeFileSync, mkdirSync } from 'node:fs';
const { chromium } = createRequire(new URL('../../../desktop/package.json', import.meta.url))('@playwright/test');

const out = process.argv[2] ?? 'measure-out';
mkdirSync(out, { recursive: true });

const PAGES = ['index.html', '01-icon-styles.html', '02-brand-blue.html', '03-moments.html', '04-pages.html', '05-system-icons.html', '06-cover-mini.html'];
// Widths of the screens the drafts are read on (measured): Flip5 main 360, Flip cover 374, Tab S9 FE 823, desktop 1080, wide 1280.
const WIDTHS = [360, 374, 823, 1080, 1280];
const THEMES = ['light', 'dark'];

// Runs inside the page.
function audit() {
  const R = (el) => el.getBoundingClientRect();
  const issues = [];
  const add = (kind, el, detail) => {
    const id = el ? (el.id ? '#' + el.id : el.className && typeof el.className === 'string' ? '.' + el.className.trim().split(/\s+/).join('.') : el.tagName.toLowerCase()) : '';
    const text = el ? (el.textContent || '').trim().replace(/\s+/g, ' ').slice(0, 50) : '';
    issues.push({ kind, el: id, text, detail });
  };
  const visible = (el) => { const s = getComputedStyle(el); return s.display !== 'none' && s.visibility !== 'hidden' && +s.opacity !== 0 && R(el).width > 0; };

  // 1. Page wider than the window
  if (document.documentElement.scrollWidth > innerWidth + 1) add('page-overflow-x', document.body, `${document.documentElement.scrollWidth} > ${innerWidth}`);

  // 2. Content outside its device frame (frames clip, so this would be cut off on the device)
  const frames = [...document.querySelectorAll('.phone, .cover, .flip, .scaled > .inner, .tcard, .shade')];
  for (const f of frames) {
    const fr = f.matches('.scaled > .inner') ? R(f.parentElement) : R(f);
    for (const el of f.querySelectorAll('*')) {
      if (!visible(el) || el.closest('svg') && el.tagName !== 'svg' || el.matches('.scrim, .sheet, .blue, .home, .safe, .cut, .cut *, .time, .shine')) continue;
      if (el.closest('.enter-sheet') && !el.matches('.sheet')) { /* inside a sheet: measured below */ }
      const r = R(el);
      if (r.left < fr.left - 0.5 || r.right > fr.right + 0.5 || r.top < fr.top - 0.5 || r.bottom > fr.bottom + 0.5) {
        add('outside-frame', el, `${Math.round(r.left - fr.left)},${Math.round(r.top - fr.top)} ${Math.round(r.width)}×${Math.round(r.height)} in ${Math.round(fr.width)}×${Math.round(fr.height)}`);
      }
    }
  }

  // 3. Clipped text (not counting intended ellipsis in the timeline strips)
  for (const el of document.querySelectorAll('main *')) {
    if (!visible(el) || el.closest('.timeline, svg, canvas')) continue;
    const s = getComputedStyle(el);
    if ((s.overflow === 'hidden' || s.overflowX === 'hidden') && el.scrollWidth > el.clientWidth + 1 && el.textContent.trim() && !el.matches('.phone, .cover, .flip, .scaled, .stagebox, .beam, .np, .seg, .toast, .zoom, .clip, .adaptive, .tile'))
      add('text-clipped', el, `${el.scrollWidth} > ${el.clientWidth}`);
  }

  // 4. Touch targets inside true-size frames: 48 dp minimum
  for (const el of document.querySelectorAll('.phone [data-hit], .cover [data-hit]')) {
    if (!visible(el)) continue;
    const r = R(el);
    if (r.width < 47.5 || r.height < 47.5) add('hit-small', el, `${r.width.toFixed(1)}×${r.height.toFixed(1)} dp`);
  }

  // 5. Overlapping siblings in frame bodies (overlays excluded on purpose)
  for (const box of document.querySelectorAll('.phone .body, .cover .in, .sheet')) {
    const kids = [...box.children].filter((k) => visible(k) && getComputedStyle(k).position !== 'absolute');
    for (let i = 0; i < kids.length; i++) for (let j = i + 1; j < kids.length; j++) {
      const a = R(kids[i]), b = R(kids[j]);
      const w = Math.min(a.right, b.right) - Math.max(a.left, b.left), h = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
      if (w > 1 && h > 1) add('overlap', kids[i], `with ${kids[j].className || kids[j].tagName}: ${w.toFixed(0)}×${h.toFixed(0)}`);
    }
  }

  // 6. Cover screen: nothing under the camera cutout, everything inside the safe area
  for (const c of document.querySelectorAll('.cover')) {
    const cut = R(c.querySelector('.cut')), safe = R(c.querySelector('.safe'));
    for (const el of c.querySelectorAll('.in *')) {
      if (!visible(el) || el.closest('svg') && el.tagName !== 'svg') continue;
      const r = R(el);
      if (r.right > cut.left && r.bottom > cut.top && r.left < cut.right && r.top < cut.bottom) add('cover-cutout', el, 'touches the camera cutout');
      if (r.left < safe.left - .5 || r.right > safe.right + .5 || r.top < safe.top - .5 || r.bottom > safe.bottom + .5) add('cover-unsafe', el, 'outside the safe area');
    }
  }

  // 7. Icons on whole pixels at 1x, 2x, 3x
  const icons = [...document.querySelectorAll('svg[data-icon]')].filter((s) => visible(s) && !s.closest('.zoom'));
  const offGrid = { 1: 0, 2: 0, 3: 0 };
  for (const s of icons) {
    const r = R(s);
    for (const d of [1, 2, 3]) if (Math.abs(r.left * d - Math.round(r.left * d)) > .01 || Math.abs(r.top * d - Math.round(r.top * d)) > .01) offGrid[d]++;
  }

  // 8. Icon live area: geometry plus half its stroke must sit in 2..22 (round shapes may reach 1..23)
  const live = [];
  const seen = new Set();
  for (const s of document.querySelectorAll('svg[data-icon][data-size="24"]')) {
    const key = s.dataset.icon + s.dataset.style;
    if (seen.has(key)) continue; seen.add(key);
    let x1 = 99, y1 = 99, x2 = -99, y2 = -99;
    for (const p of s.querySelectorAll('g:not([transform]) > path, svg > g > path')) {
      if (p.closest('mask, [transform]')) continue;
      const b = p.getBBox(); const sw = +(p.closest('[stroke-width]')?.getAttribute('stroke-width') ?? 0) / 2;
      x1 = Math.min(x1, b.x - sw); y1 = Math.min(y1, b.y - sw); x2 = Math.max(x2, b.x + b.width + sw); y2 = Math.max(y2, b.y + b.height + sw);
    }
    const round = /check|close/.test(s.dataset.icon);
    const lo = round ? 0.99 : 1.99, hi = round ? 23.01 : 22.01;
    live.push({ icon: s.dataset.icon, style: s.dataset.style, box: [x1, y1, x2, y2].map((v) => +v.toFixed(2)), ok: x1 >= lo && y1 >= lo && x2 <= hi && y2 <= hi });
  }

  // 9. Contrast of text and icons against what is really behind them
  const parse = (c) => {
    // Chrome reports color-mix() results as color(srgb r g b / a) with 0..1 channels.
    const srgb = c.match(/color\(srgb ([\d.e-]+) ([\d.e-]+) ([\d.e-]+)(?: \/ ([\d.e-]+))?\)/);
    if (srgb) return { r: +srgb[1] * 255, g: +srgb[2] * 255, b: +srgb[3] * 255, a: srgb[4] === undefined ? 1 : +srgb[4] };
    const m = c.match(/[\d.]+/g); if (!m) return null; const [r, g, b, a = 1] = m.map(Number); return { r, g, b, a: m.length === 4 ? a : 1 };
  };
  const lum = ({ r, g, b }) => { const f = (v) => { v /= 255; return v <= .03928 ? v / 12.92 : ((v + .055) / 1.055) ** 2.4; }; return .2126 * f(r) + .7152 * f(g) + .0722 * f(b); };
  const blend = (top, under) => ({ r: top.r * top.a + under.r * (1 - top.a), g: top.g * top.a + under.g * (1 - top.a), b: top.b * top.a + under.b * (1 - top.a), a: 1 });
  const bgOf = (el) => {
    const stack = [];
    for (let e = el; e; e = e.parentElement) { const c = parse(getComputedStyle(e).backgroundColor); if (c && c.a > 0) { stack.push(c); if (c.a >= 1) break; } }
    let base = { r: 255, g: 255, b: 255, a: 1 };
    for (const c of stack.reverse()) base = blend(c, base);
    return base;
  };
  const ratio = (a, b) => { const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p); return (x + .05) / (y + .05); };
  const lowContrast = [];
  for (const el of document.querySelectorAll('main *, .bar *')) {
    if (!visible(el) || el.closest('svg, canvas, .zoom, .adaptive, .home .tile, .panelbar, .statusbar, .big, .sizes, .window, .phone .stage')) continue;
    const own = [...el.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim());
    if (!own) continue;
    const s = getComputedStyle(el);
    const fg = parse(s.color); if (!fg) continue;
    const bg = bgOf(el); const fgB = blend({ ...fg, a: fg.a * +s.opacity }, bg);
    const size = parseFloat(s.fontSize), bold = +s.fontWeight >= 700;
    const need = size >= 24 || (bold && size >= 18.66) ? 3 : 4.5;
    const r = ratio(fgB, bg);
    if (r < need - .005) lowContrast.push({ el: el.className || el.tagName, text: el.textContent.trim().slice(0, 40), ratio: +r.toFixed(2), need });
  }
  for (const s of document.querySelectorAll('svg[data-icon]')) {
    if (!visible(s) || s.closest('.zoom')) continue;
    const fg = parse(getComputedStyle(s).color); const bg = bgOf(s);
    const r = ratio(blend(fg, bg), bg);
    if (r < 2.995) lowContrast.push({ el: `icon ${s.dataset.icon} ${s.dataset.style}`, text: '', ratio: +r.toFixed(2), need: 3 });
  }

  // 10. Motion: finite UI animations and transitions over 400 ms; endless loops listed
  const long = [], loops = new Set();
  for (const a of document.getAnimations()) {
    const t = a.effect?.getTiming?.(); const el = a.effect?.target; if (!t || !el) continue;
    const name = a.animationName || a.transitionProperty || 'anim';
    if (t.iterations === Infinity) { loops.add(`${name} on ${el.className || el.tagName}`); continue; }
    if (t.duration > 400 && !el.closest('.flip')) long.push(`${name} ${t.duration}ms on ${el.className || el.tagName}`);
  }

  return { issues, offGrid, iconsChecked: icons.length, live, lowContrast, long, loops: [...loops] };
}

// 200% font size inside the Android frames: doubles every text size, then looks for spill and clipping.
function bigText() {
  const found = [];
  const frames = [...document.querySelectorAll('.phone, .cover')];
  for (const f of frames) for (const el of f.querySelectorAll('*')) {
    const s = getComputedStyle(el);
    if (el.closest('svg')) continue;
    el.style.fontSize = parseFloat(s.fontSize) * 2 + 'px';
    if (s.lineHeight !== 'normal') el.style.lineHeight = parseFloat(s.lineHeight) * 2 + 'px';
  }
  for (const f of frames) {
    const fr = f.getBoundingClientRect();
    const spill = [...f.querySelectorAll('.body > *, .in > *')].filter((el) => {
      const r = el.getBoundingClientRect(); return r.width && (r.bottom > fr.bottom + .5 || r.right > fr.right + .5);
    }).map((el) => (el.textContent || '').trim().slice(0, 30));
    if (spill.length) found.push({ frame: f.closest('figure')?.querySelector('figcaption')?.textContent ?? '', spill });
  }
  return found;
}

const browser = await chromium.launch({ args: ['--use-gl=angle', '--use-angle=swiftshader', '--enable-unsafe-swiftshader'] });
const report = [];
for (const page of PAGES) for (const width of WIDTHS) for (const theme of THEMES) {
  const ctx = await browser.newContext({ viewport: { width, height: 900 }, deviceScaleFactor: 1 });
  const p = await ctx.newPage();
  const errors = [];
  p.on('pageerror', (e) => errors.push(e.message));
  p.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
  await p.goto(`http://127.0.0.1:4321/design/drafts/${page}?theme=${theme}&spec=off&reduced=off`, { waitUntil: 'networkidle' });
  await p.waitForTimeout(1500);
  const r = await p.evaluate(audit);
  let big = [];
  if (width === 1280 && theme === 'light' && /04-pages|06-cover/.test(page)) big = await p.evaluate(bigText);
  if (width === 360 || width === 1280) await p.screenshot({ path: `${out}/${page.replace('.html', '')}-${width}-${theme}.png`, fullPage: true });
  report.push({ page, width, theme, errors, ...r, bigText: big });
  await ctx.close();
}
await browser.close();
writeFileSync(`${out}/report.json`, JSON.stringify(report, null, 1));

// Summary, one line per finding kind, deduplicated across sizes and themes
const sum = new Map();
const note = (k, v) => { if (!sum.has(k)) sum.set(k, new Set()); sum.get(k).add(v); };
for (const r of report) {
  for (const e of r.errors) note('console error', `${r.page}: ${e}`);
  for (const i of r.issues) note(i.kind, `${r.page} @${r.width}: ${i.el} "${i.text}" ${i.detail}`);
  for (const d of [1, 2, 3]) if (r.offGrid[d]) note(`icons off whole pixels at ${d}x`, `${r.page} @${r.width}: ${r.offGrid[d]} of ${r.iconsChecked}`);
  for (const l of r.live) if (!l.ok) note('icon outside live area', `${l.icon} ${l.style} box ${l.box.join(', ')}`);
  for (const c of r.lowContrast) note('low contrast', `${r.page} ${r.theme}: ${c.el} "${c.text}" ${c.ratio} < ${c.need}`);
  for (const m of r.long) note('UI motion over 400 ms', `${r.page}: ${m}`);
  for (const m of r.loops) note('endless loop (needs idle stop in the app)', `${r.page}: ${m}`);
  for (const b of r.bigText) note('spills at 200% font', `${r.page}: ${b.frame}: ${b.spill.join(' | ')}`);
}
const liveAll = report.find((r) => r.live.length)?.live ?? [];
console.log(`Checked ${PAGES.length} pages × ${WIDTHS.length} widths × ${THEMES.length} themes = ${report.length} renders.`);
console.log(`Icon live area: ${liveAll.filter((l) => l.ok).length} of ${liveAll.length} sample icons inside.`);
if (!sum.size) console.log('No findings.');
for (const [k, v] of sum) { console.log(`\n## ${k} (${v.size})`); [...v].slice(0, 14).forEach((x) => console.log('  ' + x)); if (v.size > 14) console.log(`  … ${v.size - 14} more`); }
