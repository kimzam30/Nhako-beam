// Home's room, as data (design/room/build.mjs writes room.json next to the images). Where things go,
// and which device stands where. The Kotlin twin is android/.../stage/RoomModel.kt.
import data from '../../../../design/room/build/room.json';
import type { Device } from '../engine/types';

export const ROOM = data;
const urls = import.meta.glob('../../../../design/room/build/*.webp', { eager: true, query: '?url', import: 'default' }) as Record<string, string>;
export const roomImage = (name: string) => urls[`../../../../design/room/build/${name}.webp`] ?? '';

export type RoomPerson = 'boy' | 'girl';
export type RoomView = keyof typeof data.views;
type Box = { x0: number; x1: number; y0: number; y1: number };
type Rect = { x: number; y: number; w: number; h: number };
type Standing = { x: number; y: number; box: Box; art: Rect; margin: number };
export type RoomSlot = { id: string; on: string; z: number; kinds: Partial<Record<Device['kind'], Standing>> };
export interface RoomSpot extends Standing { device: Device; slot: string; on: string; z: number }
export interface RoomLayout { room: RoomSpot[]; rest: Device[] }
export interface RoomFit { view: RoomView; k: number; ox: number; oy: number; top: number; bottom: number; panelW: number }

/** Trusted first, then by name, never by whether it's online: nothing moves when a device sleeps. */
export function roomOrder(devices: Device[]): Device[] {
  return [...devices].sort((a, b) => Number(b.trusted) - Number(a.trusted) || a.name.toLowerCase().localeCompare(b.name.toLowerCase()) || a.id.localeCompare(b.id));
}

/** Which crop of the room a screen uses: rotating a screen just changes crop. */
export function roomView(width: number, height: number): RoomView {
  const a = width / height;
  return a <= data.fit.tallBelow ? 'tall' : a <= data.fit.midBelow ? 'mid' : 'wide';
}

/** The room fills the box: world units to pixels. What must stay in view sits between the top buttons and the bottom panel. */
export function fitRoom(width: number, height: number): RoomFit {
  const view = roomView(width, height), v = data.views[view], k = Math.max(width / v.w, height / v.h);
  const c = data.chrome, covers = height < c.shortBelow ? c.short : width < c.narrowBelow ? c.narrow : c.wide;
  const seenW = width / k, seenH = height / k, f = data.fit.wideFocus;
  const left = view === 'wide' ? Math.max(v.x, Math.min(v.x + v.w - seenW, f.right - seenW, f.left)) : v.x + (v.w - seenW) / 2;
  const wanted = (data.fit.band.top + data.fit.band.bottom) / 2 - (covers.top + height - covers.bottom) / 2 / k;
  const top = Math.max(v.y, Math.min(v.y + v.h - seenH, wanted));
  return { view, k, ox: -left * k, oy: -top * k, top: covers.top, bottom: covers.bottom, panelW: width < c.narrowBelow ? width : Math.min(c.panelW, width) };
}

/** The places this screen offers: a kind stays only if the device and its name tag land on clear screen. */
export function roomSlots(width: number, height: number): RoomSlot[] {
  if (width < data.fit.coverBelow.w && height < data.fit.coverBelow.h) return [];
  const f = fitRoom(width, height), panelL = (width - f.panelW) / 2, panelR = (width + f.panelW) / 2, panelY = height - f.bottom;
  const tagRoom = height < data.chrome.shortBelow ? 34 : 54, above = 2 * data.tag.h + 8;
  return (data.slots as RoomSlot[]).flatMap((s) => {
    const kinds = Object.fromEntries(Object.entries(s.kinds).filter(([, q]) => {
      const x0 = f.ox + q.box.x0 * f.k, x1 = f.ox + q.box.x1 * f.k, y0 = f.oy + q.box.y0 * f.k, y1 = f.oy + q.box.y1 * f.k + tagRoom;
      if (x0 < 4 || x1 > width - 4 || y0 - (s.on === 'hutch' ? above : 0) < f.top * 0.75 || y1 > height) return false;
      return !(y1 > panelY && x1 > panelL && x0 < panelR);
    }));
    return Object.keys(kinds).length ? [{ ...s, kinds }] : [];
  });
}

const SIZE_ORDER = ['desktop', 'laptop', 'tablet', 'phone'];

/** Your picks (or, until you've picked, the first devices) get the first free place that holds them. With none left, a device goes in the rest. */
export function arrangeRoom(devices: Device[], picks: string[] | null, width: number, height: number): RoomLayout {
  const free = roomSlots(width, height), ordered = roomOrder(devices);
  const picked = (picks ? ordered.filter((d) => picks.includes(d.id)) : ordered).slice(0, free.length);
  // Big things choose first: a PC needs a desk, a phone fits nearly anywhere.
  const room: RoomSpot[] = [];
  for (const device of [...picked].sort((a, b) => SIZE_ORDER.indexOf(a.kind) - SIZE_ORDER.indexOf(b.kind))) {
    const at = free.findIndex((f) => f.kinds[device.kind]);
    if (at < 0) continue;
    const [slot] = free.splice(at, 1);
    room.push({ ...slot!.kinds[device.kind]!, device, slot: slot!.id, on: slot!.on, z: slot!.z });
  }
  return { room, rest: ordered.filter((d) => !room.some((q) => q.device === d)) };
}

/** How many devices this screen's room can hold. */
export const roomMax = (width: number, height: number) => roomSlots(width, height).length;

/** Each name goes under its device, on the hutch above it so it never covers the desk's monitor. If two would touch, the later moves one row away from its device. A tag is centred in its box of `tag.w`, and only as wide as its name, so `w` is the room it really takes. */
export function roomTags(room: RoomSpot[], f: RoomFit, width: number): { id: string; x: number; y: number; w: number }[] {
  const { w, h, gap } = data.tag, out: { id: string; x: number; y: number; w: number }[] = [];
  for (const q of [...room].sort((a, b) => a.x - b.x)) {
    const above = q.on === 'hutch', x = Math.max(6, Math.min(width - w - 6, f.ox + q.x * f.k - w / 2));
    const wide = Math.min(w, Math.max(44, q.device.name.length * 7 + 22));
    let y = above ? f.oy + q.box.y0 * f.k - h - 3 : f.oy + q.y * f.k + gap;
    while (out.some((o) => Math.abs(x - o.x) < (wide + o.w) / 2 + 2 && y < o.y + h + 2 && o.y < y + h + 2)) y += above ? -(h + 3) : h + 3;
    out.push({ id: q.device.id, x, y, w: wide });
  }
  return out;
}

/** Which picture a device gets at its place: the same wallpaper every time, so a phone keeps its own. */
export function deviceArt(q: RoomSpot): string {
  const d = q.device;
  if (!d.reachable && !d.wakeable) return `device-${q.slot}-${d.kind}-off`;
  let h = 0;
  for (const c of d.id) h = (Math.imul(h, 31) + c.charCodeAt(0)) | 0;
  return `device-${q.slot}-${d.kind}-${Math.abs(h) % data.deviceArt[d.kind]}`;
}
