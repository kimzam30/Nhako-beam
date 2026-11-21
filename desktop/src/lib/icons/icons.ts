// The Nhako Beam icon set (style B, "Bolt solid"). The drawings come from design/icons/icons.mjs
// (run `npm run icons` after editing it); Android draws the same data in PhIcon (ui/theme/Theme.kt).
import { ICONS, OFFSET } from '../../../../design/icons/build/icons.js';

/**
 * Kept so existing callers still type-check. Style B has one drawing per icon: an active icon is
 * shown by its color (accent on the soft accent circle), not by a heavier weight.
 */
export type IconWeight = 'duotone' | 'fill' | 'bold';

export const iconNames = Object.keys(ICONS);

export function hasIcon(name: string): boolean {
  return name in ICONS;
}

/**
 * The offset sits on blue, or the icon is blue itself (the accent is the brand navy in light theme),
 * when the icon's color is the accent or white. Then the offset uses the deeper navy, so it still shows.
 */
export function wantsDeepOffset(cssColor: string): boolean {
  return cssColor === 'rgb(26, 111, 224)' || cssColor === 'rgb(255, 255, 255)';
}

let uid = 0;

/** Inner SVG markup (viewBox 0 0 24 24) for an icon at `size` CSS px; '' for an unknown name. */
export function iconMarkup(name: string, size: number, deep = false): string {
  const i = ICONS[name];
  if (!i) return '';
  const id = `nbi${++uid}`;
  const cut = i.k.length ? ` mask="url(#${id})"` : '';
  const mask = i.k.length
    ? `<defs><mask id="${id}" maskUnits="userSpaceOnUse" x="-2" y="-2" width="28" height="28"><rect x="-2" y="-2" width="28" height="28" fill="#fff"/>${i.k
        .map((k) => `<path d="${k.d}" fill="none" stroke="#000" stroke-width="${k.w}" stroke-linecap="round" stroke-linejoin="round"/>`)
        .join('')}</mask></defs>`
    : '';
  const draw = (color: string) =>
    `<g${cut}>${i.s.map((d) => `<path d="${d}" fill="${color}" fill-rule="evenodd"/>`).join('')}${i.b
      .map((b) => `<path d="${b.d}" fill="none" stroke="${color}" stroke-width="${b.w}" stroke-linecap="round" stroke-linejoin="round"/>`)
      .join('')}</g>${i.f.map((d) => `<path d="${d}" fill="${color}"/>`).join('')}${i.t
      .map((t) => `<path d="${t.d}" fill="none" stroke="${color}" stroke-width="${t.w}" stroke-linecap="round"/>`)
      .join('')}`;
  // The navy offset: exactly 1 px down and right, from 20 px up; at smaller sizes it would only blur.
  const shift = 24 / size;
  const offset = size >= 20 ? `<g transform="translate(${shift} ${shift})">${draw(deep ? OFFSET.deep : OFFSET.navy)}</g>` : '';
  return mask + offset + draw('currentColor');
}
