// Full-page screenshots of draft pages for review.
// Usage (from desktop/, needs the drafts server on 4321):
//   node ../design/drafts/check/shot.mjs <out-dir> <page.html> [width] [theme] [extra query]
import { createRequire } from 'node:module';
// Playwright lives in desktop/node_modules; resolve it from there.
const { chromium } = createRequire(new URL('../../../desktop/package.json', import.meta.url))('@playwright/test');

const [out, page, width = '1280', theme = 'light', extra = ''] = process.argv.slice(2);
const browser = await chromium.launch();
const p = await browser.newPage({ viewport: { width: +width, height: 900 }, deviceScaleFactor: 1 });
const errors = [];
p.on('pageerror', (e) => errors.push(e.message));
p.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
await p.goto(`http://127.0.0.1:4321/design/drafts/${page}?theme=${theme}&${extra}`, { waitUntil: 'networkidle' });
await p.waitForTimeout(1200);
const file = `${out}/${page.replace('.html', '')}-${width}-${theme}${extra ? '-' + extra.replace(/[^a-z0-9]+/gi, '_') : ''}.png`;
await p.screenshot({ path: file, fullPage: true });
console.log(file, errors.length ? 'ERRORS: ' + errors.join(' | ') : 'no errors');
await browser.close();
