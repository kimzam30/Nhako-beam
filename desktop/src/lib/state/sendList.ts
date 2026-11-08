// Who's in the Send to list, and in what order. Android has the same rules in state/SendList.kt;
// change both together.
import type { Device } from '../engine/types';

const byName = (a: Device, b: Device) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()) || a.id.localeCompare(b.id);

/** Same name, type and system: a reinstall gives a device a new key, and this is all that's left. */
const looksLike = (a: Device, b: Device) => a.id !== b.id && a.name === b.name && a.kind === b.kind && a.os === b.os;

/** Without a trusted device that's offline while one just like it is online: most likely it from
 *  before a reinstall. Settings still lists it, with Remove. */
export function withoutTwins(devices: Device[]): Device[] {
  const online = devices.filter((d) => d.trusted && d.reachable);
  return devices.filter((d) => !(d.trusted && !d.reachable && online.some((o) => looksLike(o, d))));
}

/** By name, never by state: a row doesn't move under a finger when a device changes route or sleeps. */
export function sendGroups(devices: Device[]): { trusted: Device[]; nearby: Device[] } {
  return {
    trusted: withoutTwins(devices).filter((d) => d.trusted).sort(byName),
    nearby: devices.filter((d) => !d.trusted && d.reachable).sort(byName),
  };
}

/** The engine only asks a device on this network. Which path a connection uses doesn't matter. */
export const canTrust = (d: Device) => !d.trusted && d.reachable && d.nearby;
