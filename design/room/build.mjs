// Renders the Home room (scene.js) to images in build/, which both apps use: the room for wide and
// tall screens, each person in layers that move on their own, the devices, and room.json saying
// where everything goes. Needs Playwright's browser (desktop's dev dependency): `npm run room`.
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'fs';
import { createRequire } from 'module';
import { dirname, join } from 'path';
import { fileURLToPath } from 'url';
import { KINDS, SLOTS, SURFACES, margin, problems, screenBox } from './layout.mjs';

const here = dirname(fileURLToPath(import.meta.url)), root = join(here, '../..'), out = join(here, 'build');
const { chromium } = createRequire(join(root, 'desktop/'))('@playwright/test');
rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });

// A trophy has no floating sparkles: those pixels go, the white of the eyes stays.
let boltSvg = readFileSync(join(root, 'docs/brand/logo/svg/nhako-beam-bolt.svg'), 'utf8').replace(/^<svg[^>]*>/, '').replace('</svg>', '').replace(/<title>.*?<\/title>/, '');
boltSvg = boltSvg.replace(/<path fill="#bcdcff" d="[^"]*"\/>/, '').replace(/(<path fill="#ffffff" d=")([^"]*)"/, (_, a, d) => a + d.split('M').filter((p) => { const x = parseInt(p, 10); return p && x > 10 && x < 38; }).map((p) => 'M' + p).join('') + '"');

// The room for each shape of screen, and what's in it. Only the camera's crop differs between them.
const VIEWS = { tall: { x: 490, y: -200, w: 620, h: 1700 }, mid: { x: 290, y: -200, w: 1020, h: 1700 }, wide: { x: 0, y: 0, w: 1600, h: 900 } };
const VIEW_SCALE = { tall: 1.75, mid: 1.4, wide: 1.5 };
const PERSON = { x: 620, y: 450, w: 290, h: 450 };
const BOLT = { x: 901, y: 246, size: 56, threadX: 922, threadY: 270 };
const FRONT = { x: 250, y: 130, w: 340, h: 410 };

const wrong = problems();
if (wrong.length) throw new Error(`The room doesn't fit:\n${wrong.join('\n')}`);

const round = (v) => Math.round(v * 10) / 10;
const data = {
  views: VIEWS,
  // The sprites were drawn at PERSON; on the desk they sit 13 lower, arms on the desk's front edge.
  person: { ...PERSON, y: PERSON.y + 13 }, bolt: BOLT, front: FRONT,
  fit: {
    // Width over height: up to tallBelow uses the tall crop, up to midBelow the mid one.
    tallBelow: 0.6, midBelow: 1.25,
    // What must stay in view, in room y: from the device shelf down past the desk's name tags.
    band: { top: 345, bottom: 660 },
    // A narrow wide screen gives up the bed first, so the side table stays.
    wideFocus: { left: 300, right: 1560 },
    // Under both of these (a flip's cover screen) the buttons fill the screen: the room stays a picture.
    coverBelow: { w: 500, h: 400 },
  },
  // What the app's own buttons cover, in screen points, measured from Home.
  chrome: { shortBelow: 500, narrowBelow: 700, short: { top: 62, bottom: 118 }, narrow: { top: 104, bottom: 190 }, wide: { top: 104, bottom: 160 }, panelW: 520 },
  tag: { w: 96, h: 24, gap: 5 },
  // Where the moving bits go: steam off the mug, the sleeper's z, dust in the lamp light, rain behind the glass.
  steam: { x: 512, y: 594 }, z: { x: 792, y: 475 }, cone: [[612, 322], [702, 342], [1000, 579], [540, 579]], window: { x: 304, y: 168, w: 232, h: 250 },
  deviceArt: Object.fromEntries(Object.entries(KINDS).map(([k, v]) => [k, v.art.length])),
  // Every place in fill order, with where each kind it holds lands on screen: x, y is the middle of
  // its front edge, box its whole body, art its picture, margin its centimetres clear of the edge.
  slots: SLOTS.map((s) => ({
    id: s.id, on: s.on, z: s.z,
    kinds: Object.fromEntries(s.kinds.map((kind) => {
      const b = screenBox(s, kind), pad = 62;
      return [kind, { x: round(b.bx), y: round(b.by), box: { x0: round(b.x0), x1: round(b.x1), y0: round(b.y0), y1: round(b.y1) }, art: { x: Math.floor(b.x0 - pad), y: Math.floor(b.y0 - pad), w: Math.ceil(b.x1 - b.x0 + 2 * pad), h: Math.ceil(b.y1 - b.y0 + 2 * pad) }, margin: round(margin(s, kind)) }];
    })),
  })),
};

const TONE = `<filter id="night" color-interpolation-filters="sRGB" x="0" y="0" width="100%" height="100%"><feColorMatrix values=".16 .02 .01 0 .008  .03 .2 .03 0 .012  .05 .08 .4 0 .03  0 0 0 1 0"/></filter>
<filter id="mid" color-interpolation-filters="sRGB" x="0" y="0" width="100%" height="100%"><feColorMatrix values=".5 .03 .01 0 .01  .03 .42 .03 0 .008  .02 .06 .36 0 .018  0 0 0 1 0"/></filter>
<filter id="hot" color-interpolation-filters="sRGB" x="0" y="0" width="100%" height="100%"><feColorMatrix values="1.12 0 0 0 .02  0 1.03 0 0 .01  0 0 .82 0 0  0 0 0 1 0"/></filter>
<filter id="g18" x="-60%" y="-60%" width="220%" height="220%"><feGaussianBlur stdDeviation="20"/></filter>
<clipPath id="winclip"><rect x="304" y="168" width="232" height="250"/></clipPath>
<clipPath id="band1"><polygon points="612,322 702,342 1000,552 1060,579 1060,596 524,596 540,579"/></clipPath>
<clipPath id="band2"><polygon points="590,318 730,350 1056,500 1056,596 524,596 540,579 572,452 572,404"/></clipPath>
<clipPath id="pers1"><polygon points="620,440 880,440 880,588 800,600 745,604 690,600 620,588"/></clipPath>
<clipPath id="pers2"><polygon points="620,440 880,440 880,740 800,760 745,764 690,760 620,740"/></clipPath>
<clipPath id="pool"><ellipse cx="770" cy="880" rx="350" ry="100"/></clipPath>`;

const browser = await chromium.launch();
const scene = readFileSync(join(here, 'scene.js'), 'utf8');
let total = 0;
async function shoot(name, build, view, scale, opaque) {
  const page = await browser.newPage({ viewport: { width: Math.ceil(view.w), height: Math.ceil(view.h) }, deviceScaleFactor: scale });
  await page.setContent(`<!doctype html><style>html,body{margin:0;background:transparent}.ink polygon,.ink rect,.ink path,.ink circle,.ink ellipse{stroke:#12141f;stroke-width:1.3;stroke-linejoin:round}</style>
    <svg width="0" height="0" style="position:absolute"><defs><symbol id="bolt" viewBox="0 0 48 48">${boltSvg}</symbol></defs></svg>
    <svg id="s" xmlns="http://www.w3.org/2000/svg" viewBox="${view.x} ${view.y} ${view.w} ${view.h}" width="${view.w}" height="${view.h}" style="display:block"></svg>`);
  await page.addScriptTag({ content: scene + `;window.__draw = (build, TONE) => { const view = 0; document.getElementById('s').innerHTML = eval(build)(TONE); };` });
  await page.evaluate(([b, t]) => window.__draw(b, t), [build, TONE]);
  const png = await page.screenshot({ omitBackground: !opaque, type: 'png' });
  // Chromium's own encoder, so nothing extra has to be installed.
  const url = await page.evaluate(async ([b64, q]) => { const i = new Image(); i.src = 'data:image/png;base64,' + b64; await i.decode(); const c = document.createElement('canvas'); c.width = i.width; c.height = i.height; c.getContext('2d').drawImage(i, 0, 0); return c.toDataURL('image/webp', q); }, [png.toString('base64'), opaque ? 0.86 : 0.9]);
  const bytes = Buffer.from(url.split(',')[1], 'base64');
  writeFileSync(join(out, `${name}.webp`), bytes);
  total += bytes.length;
  await page.close();
}

// The lit room, without the person. The bolt trophy, dim, is part of it.
const roomArt = (v) => `(TONE) => { const v = ${JSON.stringify(v)};
  return \`<defs>\${TONE}\${cloth('plaid', '#7c8ba6', '#4d5d7a', '#d8c9a4')}<g id="a">\${shell(v)}\${floorItems(v)}</g><g id="b" class="ink">\${room(v)}</g><g id="c" class="ink">\${foreground(v)}</g></defs>
  <rect x="-2000" y="-2000" width="6000" height="6000" fill="#060a1e"/>
  <g filter="url(#night)"><use href="#a"/></g>\${outside()}<g filter="url(#night)"><use href="#b"/><use href="#c"/></g>
  <g clip-path="url(#pool)"><g filter="url(#mid)"><use href="#a"/></g></g>
  <g clip-path="url(#band2)"><g filter="url(#mid)"><use href="#a"/><use href="#b"/></g></g>
  <g clip-path="url(#band1)"><g filter="url(#hot)"><use href="#a"/><use href="#b"/></g></g>
  <polygon points="612,322 702,342 1000,552 1060,579 540,579" fill="#ffd98a" opacity=".05"/><polygon points="616,322 700,342 708,348 610,326" fill="#fff7d6"/>
  \${E(655, 335, 90, 40, '#ffe7a8', 'filter="url(#g18)" style="mix-blend-mode:screen;opacity:.6"')}\${E(1188, 305, 40, 40, '#7fb4ff', 'filter="url(#g18)" style="mix-blend-mode:screen;opacity:.5"')}
  <g filter="url(#mid)"><use href="#bolt" x="${BOLT.x}" y="${BOLT.y}" width="${BOLT.size}" height="${BOLT.size}"/></g>\`; }`;
// One layer of the person, lit the same way as the room.
const personArt = (v, i) => `(TONE) => { const p = person(${JSON.stringify(v)}), part = p[${i}];
  const g = '<g class="ink">' + part + '</g>';
  return \`<defs>\${TONE}\${cloth('plaid', '#7c8ba6', '#4d5d7a', '#d8c9a4')}</defs><g filter="url(#night)">\${g}</g><g clip-path="url(#pers2)"><g filter="url(#mid)">\${g}</g></g><g clip-path="url(#pers1)"><g filter="url(#hot)">\${g}</g></g>\`; }`;
for (const v of ['boy', 'girl']) {
  for (const [name, view] of Object.entries(VIEWS)) await shoot(`room-${v}-${name}`, roomArt(v), view, VIEW_SCALE[name], true);
  // The window's frame, curtains and sill, again on their own: drawn over the rain, so it stays behind the glass.
  await shoot(`front-${v}`, `(TONE) => '<defs>' + TONE + '</defs><g filter="url(#night)"><g class="ink">' + windowFront(${JSON.stringify(v)}) + '</g></g>'`, FRONT, 1.75, false);
  for (const [i, part] of ['under', 'torso', 'head', 'over', 'chair'].entries()) await shoot(`${v}-${part}`, personArt(v, i), PERSON, 2, false);
}
// Every device at every place it can stand, drawn by the room's own camera, each with its wallpapers and dark.
for (const [i, s] of SLOTS.entries()) for (const kind of s.kinds) {
  const draw = (wall, on) => `(TONE) => '<defs>' + TONE + '</defs>' + device3(${JSON.stringify(kind)}, ${JSON.stringify(s)}, ${SURFACES[s.on].y}, ${wall}, ${on})`, view = data.slots[i].kinds[kind].art;
  for (const [k, wall] of KINDS[kind].art.entries()) await shoot(`device-${s.id}-${kind}-${k}`, draw(wall, true), view, 2, false);
  await shoot(`device-${s.id}-${kind}-off`, draw(0, false), view, 2, false);
}
// The bolt when it lights up during a transfer: same place as the dim one in the room.
await shoot('bolt-lit', `(TONE) => '<defs>' + TONE + '</defs>' + E(24, 24, 44, 44, '#ffd23f', 'filter="url(#g18)" style="opacity:.75"') + '<use href="#bolt" x="0" y="0" width="48" height="48"/>'`, { x: -40, y: -40, w: 128, h: 128 }, 2.5, false);
data.boltLit = { x: -40, y: -40, w: 128, unit: 48 };
await browser.close();
writeFileSync(join(out, 'room.json'), JSON.stringify(data, null, 1) + '\n');
console.log(`Room: ${(total / 1048576).toFixed(1)} MB of images in design/room/build.`);
