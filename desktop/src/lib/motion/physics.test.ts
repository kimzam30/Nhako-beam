import { describe, expect, it } from 'vitest';
import { spring, gesture } from '../tokens';
import { magnet, radarSlots, rubberBand, rubberClamp, springEasing, springPosition, swipeDecision, VelocityTracker } from './physics';

describe('springPosition', () => {
  it('starts at 0 and settles at 1 for every preset', () => {
    for (const s of Object.values(spring)) {
      const pos = springPosition(s.dampingRatio, s.stiffness);
      expect(pos(0)).toBeCloseTo(0, 6);
      expect(Math.abs(pos(s.settleMs / 1000) - 1)).toBeLessThan(0.003);
    }
  });

  it('matches the overshoot reported by the token build', () => {
    for (const s of Object.values(spring)) {
      const pos = springPosition(s.dampingRatio, s.stiffness);
      let peak = 0;
      for (let ms = 0; ms <= s.settleMs; ms++) peak = Math.max(peak, pos(ms / 1000));
      expect(Math.max(0, peak - 1) * 100).toBeCloseTo(s.overshootPct, 0);
    }
  });

  it('springEasing maps 0..1 progress and ends exactly at 1', () => {
    const { duration, easing } = springEasing('bouncy');
    expect(duration).toBe(spring.bouncy.settleMs);
    expect(easing(0)).toBeCloseTo(0, 6);
    expect(easing(1)).toBe(1);
    expect(Math.max(...Array.from({ length: 50 }, (_, i) => easing(i / 49)))).toBeGreaterThan(1.05);
  });
});

describe('rubberBand', () => {
  it('is zero at the edge and never passes the dimension', () => {
    expect(rubberBand(0, 300)).toBe(0);
    expect(rubberBand(10_000, 300)).toBeLessThan(300);
    expect(rubberBand(-10_000, 300)).toBeGreaterThan(-300);
  });

  it('moves less than the finger', () => {
    expect(rubberBand(100, 300)).toBeLessThan(100);
    expect(rubberBand(100, 300)).toBeGreaterThan(0);
  });

  it('rubberClamp passes values inside the range untouched', () => {
    expect(rubberClamp(50, 0, 100, 300)).toBe(50);
    expect(rubberClamp(150, 0, 100, 300)).toBeLessThan(150);
    expect(rubberClamp(-50, 0, 100, 300)).toBeGreaterThan(-50);
  });
});

describe('magnet', () => {
  const targets = [{ id: 'a', x: 0, y: 0 }];

  it('does nothing outside the radius', () => {
    const r = magnet({ x: gesture.magnetRadius + 1, y: 0 }, targets);
    expect(r).toEqual({ drawn: { x: gesture.magnetRadius + 1, y: 0 }, nearId: null, snappedId: null });
  });

  it('leans toward the target inside the radius', () => {
    const r = magnet({ x: 80, y: 0 }, targets);
    expect(r.nearId).toBe('a');
    expect(r.snappedId).toBeNull();
    expect(r.drawn.x).toBeLessThan(80);
    expect(r.drawn.x).toBeGreaterThan(0);
  });

  it('snaps inside the snap radius', () => {
    const r = magnet({ x: gesture.magnetSnapRadius - 1, y: 0 }, targets);
    expect(r.snappedId).toBe('a');
    expect(r.drawn).toEqual({ x: 0, y: 0 });
  });

  it('holds the snap a little longer (no flicker on the edge)', () => {
    const edge = { x: gesture.magnetSnapRadius + 6, y: 0 };
    expect(magnet(edge, targets, null).snappedId).toBeNull();
    expect(magnet(edge, targets, 'a').snappedId).toBe('a');
  });

  it('picks the nearest target', () => {
    const r = magnet({ x: 70, y: 0 }, [...targets, { id: 'b', x: 100, y: 0 }]);
    expect(r.nearId).toBe('b');
  });
});

describe('swipeDecision', () => {
  const w = 320;
  it('commits by distance', () => {
    expect(swipeDecision(w * 0.45, 0, w)).toBe('accept');
    expect(swipeDecision(-w * 0.45, 0, w)).toBe('decline');
  });
  it('commits by a fast flick even when short', () => {
    expect(swipeDecision(30, 900, w)).toBe('accept');
    expect(swipeDecision(-30, -900, w)).toBe('decline');
  });
  it('cancels a slow short drag', () => {
    expect(swipeDecision(60, 100, w)).toBe('cancel');
  });
  it('cancels when flicked back against a long drag', () => {
    expect(swipeDecision(w * 0.5, -900, w)).toBe('cancel');
  });
});

describe('VelocityTracker', () => {
  it('measures px per second over the window', () => {
    const v = new VelocityTracker(100);
    v.add(0, 0, 0);
    v.add(50, 50, 0);
    v.add(100, 100, 0);
    expect(v.velocity().x).toBeCloseTo(1000, 0);
  });
});

describe('radarSlots', () => {
  it('returns one slot per device, all inside the radar', () => {
    const slots = radarSlots(9, 400);
    expect(slots).toHaveLength(9);
    for (const s of slots) expect(Math.hypot(s.x, s.y)).toBeLessThanOrEqual(200);
  });
});
