import { describe, expect, it } from 'vitest';
import { homeCamera, project, slotOnFloor, STAGE } from './view';
import { mascotMood, nextChange, type MoodInput } from './mood';
import { MascotMotion } from './pose';
import { mascot as timing } from '../tokens';

describe('the stage camera', () => {
  const cam = homeCamera();

  it('puts what it looks at in the middle', () => {
    const p = project(cam, cam.target, 400, 340);
    expect(p.x).toBeCloseTo(200, 6);
    expect(p.y).toBeCloseTo(170, 6);
  });

  it('mirrors left and right, and puts nearer things lower', () => {
    const l = project(cam, [-2, 0, 0], 400, 340);
    const r = project(cam, [2, 0, 0], 400, 340);
    expect(l.x + r.x).toBeCloseTo(400, 6);
    expect(project(cam, [0, 0, 2], 400, 340).y).toBeGreaterThan(project(cam, [0, 0, -2], 400, 340).y);
  });

  it('keeps the whole floor in the box', () => {
    const h = 400 * STAGE.heightRatio;
    for (let a = 0; a < 360; a += 15) {
      const rad = (a * Math.PI) / 180;
      const p = project(cam, [Math.cos(rad) * STAGE.floorRadius, 0, Math.sin(rad) * STAGE.floorRadius], 400, h);
      expect(p.x).toBeGreaterThan(0);
      expect(p.x).toBeLessThan(400);
    }
  });

  it('turns a radar slot at the edge into the slot reach', () => {
    const at = slotOnFloor({ x: 200, y: 0 }, 400);
    expect(at[0]).toBeCloseTo(STAGE.slotReach, 6);
    expect(at[2]).toBe(0);
  });
});

// The same cases are in android/.../MascotTest.kt.
describe('the mascot mood', () => {
  const base: MoodInput = {
    now: 100_000, started: true, offline: false, searching: false, devices: 2,
    sending: false, receiving: false, looking: false, moments: { found: 0, done: 0, failed: 0 },
  };
  const cases: [string, Partial<MoodInput>, string][] = [
    ['before the engine starts', { started: false }, 'loading'],
    ['with no network', { offline: true }, 'offline'],
    ['nothing going on', {}, 'idle'],
    ['nobody around', { devices: 0 }, 'asleep'],
    ['searching with nobody around', { devices: 0, searching: true }, 'looking'],
    ['sending', { sending: true }, 'sending'],
    ['receiving', { receiving: true }, 'receiving'],
    ['a device just turned up', { moments: { found: 99_500, done: 0, failed: 0 } }, 'happy'],
    ['a transfer just finished, while another sends', { sending: true, moments: { found: 0, done: 99_000, failed: 0 } }, 'done'],
    ['a transfer just failed', { moments: { found: 0, done: 99_900, failed: 99_000 } }, 'failed'],
    ['a moment that ran out', { moments: { found: 0, done: 100_000 - timing.doneMs - 1, failed: 0 } }, 'idle'],
    ['offline wins over a moment', { offline: true, moments: { found: 0, done: 99_900, failed: 0 } }, 'offline'],
    ['the pointer on a device', { looking: true }, 'looking'],
  ];
  for (const [name, change, want] of cases) {
    it(name, () => expect(mascotMood({ ...base, ...change })).toBe(want));
  }

  it('says when a moment runs out', () => {
    expect(nextChange(base)).toBeNull();
    expect(nextChange({ ...base, moments: { found: 0, done: 99_000, failed: 0 } })).toBe(99_000 + timing.doneMs);
  });
});

describe('the mascot pose', () => {
  it('holds still poses with reduced motion', () => {
    const m = new MascotMotion(() => 0.5);
    m.set('failed', 0);
    const p = m.still();
    expect(p.face).toBe('x');
    expect(p.crack).toBe(true);
    expect(p.sparkles).toEqual([0, 0, 0]);
    m.set('asleep', 0);
    expect(m.still().zzz).not.toBeNull();
    expect(m.still().eyeOpen).toBeLessThan(0.2);
  });

  it('bursts once when done, partway up the jump', () => {
    const m = new MascotMotion(() => 0.5);
    m.set('done', 1000);
    let bursts = 0;
    let top = 0;
    for (let t = 1000; t < 4000; t += 16) {
      const p = m.step(t, 0.016, null);
      if (p.burst) bursts++;
      top = Math.max(top, p.y);
    }
    expect(bursts).toBe(1);
    expect(top).toBeGreaterThan(0.7);
  });

  it('sends sparks while sending, about as often as the state says', () => {
    const m = new MascotMotion(() => 0.5);
    m.set('sending', 0);
    let sparks = 0;
    for (let t = 0; t < 1000; t += 10) if (m.step(t, 0.01, null).sparks === 'out') sparks++;
    expect(sparks).toBeGreaterThanOrEqual(12);
    expect(sparks).toBeLessThanOrEqual(15);
  });

  it('blinks in idle', () => {
    const m = new MascotMotion(() => 0);
    m.set('idle', 0);
    let closed = false;
    for (let t = 0; t < 6000; t += 10) if (m.step(t, 0.01, null).eyeOpen < 0.3) closed = true;
    expect(closed).toBe(true);
  });
});
