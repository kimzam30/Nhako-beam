import { describe, expect, it } from 'vitest';
import { arcPath, BOLT, CHECK, shapePath } from './bolt';

// The same numbers as android/.../BoltTest.kt.
const corners = (d: string) => [...d.matchAll(/[ML](-?[\d.]+),(-?[\d.]+)/g)].map((m) => [Number(m[1]), Number(m[2])]);

describe('the bolt', () => {
  it('morphs from the bolt to the check, corner by corner', () => {
    expect(corners(shapePath(0))).toEqual(BOLT.map(([x, y]) => [Number(x.toFixed(1)), Number(y.toFixed(1))]));
    expect(corners(shapePath(1))).toEqual(CHECK.map(([x, y]) => [x, y]));
    const [x, y] = corners(shapePath(0.5))[4]!;
    expect(x).toBeCloseTo((194.24 + 166) / 2, 0);
    expect(y).toBeCloseTo((862.4 + 544) / 2, 0);
    expect(shapePath(3)).toBe(shapePath(1));
  });

  it('draws an arc that leaves and lands on its points and zigs both ways', () => {
    const pts = corners(arcPath({ x: 10, y: 40 }, { x: 200, y: 40 }, 1, 7));
    expect(pts[0]).toEqual([10, 40]);
    expect(pts.at(-1)).toEqual([200, 40]);
    expect(pts.length).toBeGreaterThanOrEqual(5);
    const off = pts.slice(1, -1).map(([, y]) => y! - 40);
    expect(off.some((o) => o > 1) && off.some((o) => o < -1)).toBe(true);
    expect(off.every((o) => Math.abs(o) <= 7)).toBe(true);
  });
});
