// Every text color is readable on every surface it sits on, in both themes (WCAG 2.2 AA: 4.5:1 for
// text, 3:1 for icons and large text). The screens are also checked by axe in tests/e2e/a11y.spec.ts;
// this keeps a token change from quietly breaking it.
import { describe, expect, it } from 'vitest';
import { color } from './tokens';

type Theme = Record<string, string>;

/** `#RRGGBB` or `#RRGGBBAA` over an opaque background, as linear sRGB luminance. */
function luminance(hex: string, under = '#FFFFFF'): number {
  const parse = (h: string) => [1, 3, 5, 7].map((i) => parseInt(h.slice(i, i + 2) || 'FF', 16) / 255);
  const [r, g, b, a] = parse(hex);
  const [br, bg, bb] = parse(under);
  const mix = [r! * a! + br! * (1 - a!), g! * a! + bg! * (1 - a!), b! * a! + bb! * (1 - a!)];
  const lin = mix.map((c) => (c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4));
  return 0.2126 * lin[0]! + 0.7152 * lin[1]! + 0.0722 * lin[2]!;
}

function ratio(fg: string, bg: string): number {
  const a = luminance(fg, bg);
  const b = luminance(bg);
  return (Math.max(a, b) + 0.05) / (Math.min(a, b) + 0.05);
}

/** Text tokens, the surfaces they're used on, and the ratio they need. */
const PAIRS: [fg: string, bgs: string[], min: number][] = [
  ['textPrimary', ['canvas', 'surface', 'surfaceRaised', 'surfaceSunken', 'fill'], 4.5],
  ['textSecondary', ['canvas', 'surface', 'surfaceRaised', 'surfaceSunken', 'fill'], 4.5],
  ['textTertiary', ['canvas', 'surface', 'surfaceRaised'], 4.5],
  ['accentText', ['canvas', 'surface', 'accentSubtle'], 4.5],
  ['onAccent', ['accent', 'accentPressed'], 4.5],
  ['onDanger', ['danger'], 4.5],
  ['danger', ['surface', 'dangerSubtle'], 4.5],
  ['success', ['surface', 'successSubtle'], 3],
  ['warning', ['surface', 'warningSubtle'], 3],
  ['focusRing', ['canvas', 'surface'], 3],
];

describe('contrast', () => {
  for (const theme of ['light', 'dark'] as const) {
    const c = (color as unknown as Record<string, Theme>)[theme]!;
    for (const [fg, bgs, min] of PAIRS) {
      for (const bg of bgs) {
        it(`${theme}: ${fg} on ${bg} is at least ${min}:1`, () => {
          expect(c[fg], fg).toBeDefined();
          expect(c[bg], bg).toBeDefined();
          expect(ratio(c[fg]!, c[bg]!)).toBeGreaterThanOrEqual(min);
        });
      }
    }
  }
});
