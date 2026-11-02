// Who's in the Send to list, and in what order. Android has the same rules in state/SendList.kt;
// change both together.
import type { Device } from '../engine/types';

const byName = (a: Device, b: Device) => a.name.toLowerCase().localeCompare(b.name.toLowerCase()) || a.id.localeCompare(b.id);

/** By name, never by state: a row doesn't move under a finger when a device changes route or sleeps. */
export function sendGroups(devices: Device[]): { trusted: Device[]; nearby: Device[] } {
  return {
    trusted: devices.filter((d) => d.trusted).sort(byName),
    nearby: devices.filter((d) => !d.trusted && d.reachable).sort(byName),
  };
}

/** The engine only asks a device on this network. Which path a connection uses doesn't matter. */
export const canTrust = (d: Device) => !d.trusted && d.reachable && d.nearby;
