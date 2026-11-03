import { describe, expect, it } from 'vitest';
import type { Device } from '../engine/types';
import { arrangeRoom, deviceArt, fitRoom, ROOM, roomMax, roomSlots, roomTags, roomView } from './room';

const dev = (id: string, kind: Device['kind'], trusted = true): Device => ({ id, name: id, kind, trusted, reachable: true, wakeable: false } as Device);
const mine = [dev('a-phone', 'phone'), dev('b-laptop', 'laptop'), dev('c-pc', 'desktop'), dev('d-pc', 'desktop'), dev('e-phone', 'phone'), dev('f-tablet', 'tablet')];
const FLIP = [360, 880] as const, S24 = [384, 832] as const, TAB_UP = [823, 1317] as const, TAB_SIDE = [1317, 823] as const, FOLD = [884, 1104] as const, PHONE_SIDE = [880, 360] as const, COVER = [300, 288] as const, SMALL = [320, 640] as const, WINDOW = [900, 640] as const, LAPTOP = [1280, 800] as const, HD = [1920, 1080] as const, ULTRA = [2560, 1080] as const;
const SCREENS = [FLIP, S24, TAB_UP, TAB_SIDE, FOLD, PHONE_SIDE, COVER, SMALL, WINDOW, LAPTOP, HD, ULTRA];
const KINDS: Device['kind'][] = ['desktop', 'laptop', 'tablet', 'phone'];

/** Every mix of up to `n` devices across the kinds. */
function* mixes(n: number): Generator<Device[]> {
  for (let a = 0; a <= n; a++) for (let b = 0; a + b <= n; b++) for (let c = 0; a + b + c <= n; c++) for (let d = 0; a + b + c + d <= n; d++) {
    yield [a, b, c, d].flatMap((m, k) => Array.from({ length: m }, (_, i) => dev(`${KINDS[k]}${i}`, KINDS[k]!)));
  }
}

describe('arrangeRoom', () => {
  it('shows all 4 of 4 on a phone, with none left over', () => {
    const r = arrangeRoom(mine.slice(0, 4), null, ...FLIP);
    expect(r.room).toHaveLength(4);
    expect(r.rest).toHaveLength(0);
  });
  it('gives PCs the desk first, however they sort by name', () => {
    const r = arrangeRoom(mine.slice(0, 4), null, ...FLIP);
    expect(r.room.filter((q) => q.device.kind === 'desktop').map((q) => q.slot).sort()).toEqual(['deskL', 'deskR']);
  });
  it('keeps to your picks: unticking one does not pull another in', () => {
    const r = arrangeRoom(mine, ['a-phone', 'c-pc'], ...FLIP);
    expect(r.room.map((q) => q.device.id).sort()).toEqual(['a-phone', 'c-pc']);
    expect(r.rest).toHaveLength(4);
  });
  it('a third PC on a phone goes in the rest, not on a shelf', () => {
    const r = arrangeRoom([dev('p1', 'desktop'), dev('p2', 'desktop'), dev('p3', 'desktop')], null, ...FLIP);
    expect(r.room).toHaveLength(2);
    expect(r.rest.map((d) => d.id)).toEqual(['p3']);
  });
  it('the Flip cover screen keeps the room a picture', () => {
    expect(roomMax(...COVER)).toBe(0);
    const r = arrangeRoom(mine, null, ...COVER);
    expect(r.room).toHaveLength(0);
    expect(r.rest).toHaveLength(mine.length);
  });
});

describe('every mix of devices, on every screen', () => {
  it('stays on a place it belongs on, fully on screen, and accounted for', () => {
    for (const [w, h] of SCREENS) {
      const f = fitRoom(w, h), offered = new Map(roomSlots(w, h).map((s) => [s.id, s]));
      for (const devices of mixes(8)) {
        const r = arrangeRoom(devices, null, w, h);
        expect(r.room.length + r.rest.length).toBe(devices.length);
        expect(new Set(r.room.map((q) => q.slot)).size).toBe(r.room.length);
        for (const q of r.room) {
          expect(offered.get(q.slot)?.kinds[q.device.kind], `${q.device.kind} on ${q.slot}, ${w}x${h}`).toBeTruthy();
          expect(q.margin).toBeGreaterThanOrEqual(0);
          expect(f.ox + q.box.x0 * f.k, `${q.slot} ${w}x${h}`).toBeGreaterThanOrEqual(0);
          expect(f.ox + q.box.x1 * f.k, `${q.slot} ${w}x${h}`).toBeLessThanOrEqual(w);
        }
        const tags = roomTags(r.room, f, w);
        for (const a of tags) for (const b of tags) if (a !== b) expect(Math.abs(a.x - b.x) < (a.w + b.w) / 2 && a.y < b.y + ROOM.tag.h && b.y < a.y + ROOM.tag.h, `${a.id} touches ${b.id}, ${w}x${h}`).toBe(false);
      }
    }
  });
  it('short names side by side stay on one row', () => {
    const spot = (id: string, name: string, x: number) => ({ device: { ...dev(id, 'phone'), name }, slot: id, on: 'desk', z: 0, x, y: 500, box: { x0: x, x1: x, y0: 400, y1: 500 } }) as unknown as Parameters<typeof deviceArt>[0];
    const f = { k: 1, ox: 0, oy: 0 } as Parameters<typeof roomTags>[1];
    const [a, b] = roomTags([spot('a', 'KIMA5', 560), spot('b', "Kim's S24+", 650)], f, 1200);
    expect(a!.y).toBe(b!.y);
    const [c, d] = roomTags([spot('c', 'A much longer device name', 560), spot('d', 'Another long device name', 650)], f, 1200);
    expect(d!.y).toBeGreaterThan(c!.y);
  });
  it('every place holds what it offers, with its picture', () => {
    for (const s of ROOM.slots) for (const [kind, q] of Object.entries(s.kinds)) {
      expect(q.margin, `${kind} on ${s.id}`).toBeGreaterThanOrEqual(0);
      expect(deviceArt({ ...q, device: dev('x', kind as Device['kind']), slot: s.id, on: s.on, z: s.z })).toMatch(new RegExp(`^device-${s.id}-${kind}-\\d$`));
    }
  });
  it('a screen always has a place for a phone, and for a PC unless it is a cover screen', () => {
    for (const [w, h] of SCREENS) {
      if (w === COVER[0]) continue;
      expect(roomSlots(w, h).some((s) => s.kinds.phone), `${w}x${h}`).toBe(true);
      expect(roomSlots(w, h).some((s) => s.kinds.desktop), `${w}x${h}`).toBe(true);
    }
  });
});

describe('the camera', () => {
  it('picks a crop by the shape of the screen', () => {
    expect(roomView(...FLIP)).toBe('tall');
    expect(roomView(...TAB_UP)).toBe('mid');
    expect(roomView(...FOLD)).toBe('mid');
    expect(roomView(...TAB_SIDE)).toBe('wide');
  });
  it('fits the art to any window with nothing showing past its edges', () => {
    for (const [w, h] of [...SCREENS, [390, 844], [1440, 900], [800, 800], [1600, 1000], [1024, 768], [1000, 900]] as const) {
      const f = fitRoom(w, h), v = ROOM.views[f.view];
      expect(f.ox + v.x * f.k).toBeLessThanOrEqual(0.01);
      expect(f.oy + v.y * f.k).toBeLessThanOrEqual(0.01);
      expect(f.ox + (v.x + v.w) * f.k).toBeGreaterThanOrEqual(w - 0.01);
      expect(f.oy + (v.y + v.h) * f.k).toBeGreaterThanOrEqual(h - 0.01);
    }
  });
});

describe('the pictures', () => {
  it('a device keeps its picture, and an offline one goes dark', () => {
    const q = arrangeRoom([mine[0]!], null, ...FLIP).room[0]!;
    expect(deviceArt(q)).toBe(deviceArt(q));
    expect(deviceArt({ ...q, device: { ...q.device, reachable: false } })).toBe(`device-${q.slot}-phone-off`);
  });
});
