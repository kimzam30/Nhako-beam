// Draft renderer for icon set B. The drawings live in design/icons/icons.mjs (shared with the apps).
import { SET } from '../../icons/icons.mjs';
export { SET };

export const NAMES = Object.keys(SET);

/** Offset color for where the icon sits. */
export const OFFSET = { navy: '#1A6FE0', deep: '#0B3E85' };

let uid = 0;
/** SVG for one icon. `deep` uses the deeper navy offset (blue surfaces, accent and active icons). */
export function iconB(name, size = 24, { deep = false, offset, label } = {}) {
  const d = SET[name];
  if (!d) throw new Error('No icon ' + name);
  const id = `nb${++uid}`;
  const knocks = (d.knock ?? []).map((k) => (typeof k === 'string' ? { d: k, w: 2.2 } : { w: 2.2, ...k }));
  const mask = knocks.length
    ? `<mask id="${id}" maskUnits="userSpaceOnUse" x="-2" y="-2" width="28" height="28"><rect x="-2" y="-2" width="28" height="28" fill="#fff"/>${knocks.map((k) => `<path d="${k.d}" fill="none" stroke="#000" stroke-width="${k.w}" stroke-linecap="round" stroke-linejoin="round"/>`).join('')}</mask>` : '';
  const m = knocks.length ? ` mask="url(#${id})"` : '';
  const body = (color) => {
    const solid = (d.solid ?? []).map((p) => `<path d="${p}" fill="${color}" fill-rule="evenodd"/>`).join('');
    const bold = (d.bold ?? []).map((b) => `<path d="${b.d}" fill="none" stroke="${color}" stroke-width="${b.w ?? 3}" stroke-linecap="round" stroke-linejoin="round"/>`).join('');
    const top = (d.top ?? []).map((t) => t.fill ? `<path d="${t.fill}" fill="${color}"/>` : `<path d="${t.d}" fill="none" stroke="${color}" stroke-width="${t.w ?? 3}" stroke-linecap="round"/>`).join('');
    return `<g${m}>${solid}${bold}</g>${top}`;
  };
  const shift = +(24 / size).toFixed(4);
  // Blue surface: always the deep navy. Active (accent-colored icon): deep in light theme only, where
  // accent equals navy; in dark the icon is light blue and the plain navy shows better (2.31 vs 1.49).
  const off = offset ?? (deep === 'active' ? 'var(--nb-active-offset)' : deep ? OFFSET.deep : OFFSET.navy);
  const shadow = size >= 20 ? `<g transform="translate(${shift} ${shift})" data-offset>${body(off)}</g>` : '';
  const a11y = label ? `role="img" aria-label="${label}"` : 'aria-hidden="true"';
  return `<svg ${a11y} width="${size}" height="${size}" viewBox="0 0 24 24" data-icon="${name}" data-style="B" data-size="${size}"${d.round ? ' data-round' : ''} style="display:block;flex:none;overflow:visible"><defs>${mask}</defs>${shadow}${body('currentColor')}</svg>`;
}
