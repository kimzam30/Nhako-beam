// Loads the app's real brand files so the drafts show what ships, not a redraw.

const ROOT = '../../';

export async function text(path) {
  const r = await fetch(ROOT + path);
  if (!r.ok) throw new Error(`${path}: ${r.status}`);
  return r.text();
}

/** Android #AARRGGBB or #RRGGBB to CSS. */
function color(c) {
  if (!c || c.startsWith('@')) return null;
  if (c.length === 9) return `#${c.slice(3)}${c.slice(1, 3)}`;
  return c;
}

/** Converts an Android vector drawable to an SVG string (paths and groups with transforms). */
export function vectorToSvg(xml, { size, recolor = {} } = {}) {
  const doc = new DOMParser().parseFromString(xml, 'application/xml');
  const v = doc.documentElement;
  const A = (el, n) => el.getAttribute('android:' + n);
  const vw = A(v, 'viewportWidth'), vh = A(v, 'viewportHeight');
  const walk = (el) => [...el.children].map((c) => {
    if (c.tagName === 'group') {
      const t = [];
      const tx = +(A(c, 'translateX') ?? 0), ty = +(A(c, 'translateY') ?? 0);
      const sx = +(A(c, 'scaleX') ?? 1), sy = +(A(c, 'scaleY') ?? 1);
      const px = +(A(c, 'pivotX') ?? 0), py = +(A(c, 'pivotY') ?? 0), rot = +(A(c, 'rotation') ?? 0);
      t.push(`translate(${tx + px} ${ty + py})`, `rotate(${rot})`, `scale(${sx} ${sy})`, `translate(${-px} ${-py})`);
      return `<g transform="${t.join(' ')}">${walk(c)}</g>`;
    }
    if (c.tagName === 'path') {
      let fill = color(A(c, 'fillColor')) ?? 'none';
      let stroke = color(A(c, 'strokeColor')) ?? 'none';
      fill = recolor[fill.toUpperCase()] ?? fill;
      stroke = recolor[stroke.toUpperCase()] ?? stroke;
      const sw = A(c, 'strokeWidth') ?? 0;
      const join = A(c, 'strokeLineJoin') ?? 'miter';
      return `<path d="${A(c, 'pathData')}" fill="${fill}" stroke="${stroke}" stroke-width="${sw}" stroke-linejoin="${join}"/>`;
    }
    return '';
  }).join('');
  const s = size ? `width="${size}" height="${size}"` : '';
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${vw} ${vh}" ${s} aria-hidden="true">${walk(v)}</svg>`;
}

/** Bounding box of the drawn parts of an SVG string, in its own viewBox units. */
export function bbox(svgString) {
  const holder = document.createElement('div');
  holder.style.cssText = 'position:absolute;left:-9999px;top:0;width:500px;height:500px';
  holder.innerHTML = svgString;
  document.body.append(holder);
  const svg = holder.querySelector('svg');
  svg.setAttribute('width', '500'); svg.setAttribute('height', '500');
  let x1 = Infinity, y1 = Infinity, x2 = -Infinity, y2 = -Infinity;
  const root = svg.getScreenCTM().inverse();
  svg.querySelectorAll('path,rect,circle').forEach((el) => {
    const b = el.getBoundingClientRect();
    if (!b.width && !b.height) return;
    const p1 = new DOMPoint(b.left, b.top).matrixTransform(root);
    const p2 = new DOMPoint(b.right, b.bottom).matrixTransform(root);
    x1 = Math.min(x1, p1.x); y1 = Math.min(y1, p1.y); x2 = Math.max(x2, p2.x); y2 = Math.max(y2, p2.y);
  });
  holder.remove();
  return { x: x1, y: y1, w: x2 - x1, h: y2 - y1 };
}

/** Furthest distance of any drawn pixel from a centre, sampled by rasterising. Used for safe-circle checks. */
export async function reach(svgString, viewSize, cx, cy) {
  const n = 432; // 4 samples per unit for a 108 viewport
  const img = new Image();
  img.src = 'data:image/svg+xml;charset=utf-8,' + encodeURIComponent(
    svgString.replace('<svg ', `<svg width="${n}" height="${n}" `));
  await img.decode();
  const c = new OffscreenCanvas(n, n), g = c.getContext('2d');
  g.drawImage(img, 0, 0, n, n);
  const d = g.getImageData(0, 0, n, n).data, k = viewSize / n;
  let far = 0;
  for (let y = 0; y < n; y++) for (let x = 0; x < n; x++) {
    if (d[(y * n + x) * 4 + 3] > 24) far = Math.max(far, Math.hypot((x + .5) * k - cx, (y + .5) * k - cy));
  }
  return far;
}

/** WCAG contrast ratio of two #RRGGBB colors. */
export function contrast(a, b) {
  const L = (h) => {
    const v = [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16) / 255)
      .map((c) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
    return 0.2126 * v[0] + 0.7152 * v[1] + 0.0722 * v[2];
  };
  const [x, y] = [L(a), L(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
}
