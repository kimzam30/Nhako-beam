import { describe, expect, it } from 'vitest';
import type { Device } from '../engine/types';
import { canTrust, sendGroups } from './sendList';

const dev = (id: string, name: string, p: Partial<Device> = {}): Device => ({ id, name, kind: 'phone', os: 'android', trusted: false, autoAccept: false, reachable: true, route: 'nearby', nearby: true, fingerprint: '', wakeable: false, ...p });

describe('sendGroups', () => {
  it('keeps rows in name order whatever order the engine lists them in', () => {
    const a = dev('a', 'Kims S24'), b = dev('b', 'kims flip'), c = dev('c', 'Kims Tab');
    const order = (l: Device[]) => sendGroups(l).nearby.map((d) => d.id);
    expect(order([a, b, c])).toEqual(['b', 'a', 'c']);
    expect(order([c, a, b])).toEqual(['b', 'a', 'c']);
  });

  it('does not move a device of yours when it goes offline or changes route', () => {
    const a = dev('a', 'Alpha', { trusted: true }), b = dev('b', 'Beta', { trusted: true });
    expect(sendGroups([b, a]).trusted.map((d) => d.id)).toEqual(['a', 'b']);
    expect(sendGroups([{ ...a, reachable: false, route: null }, { ...b, route: 'direct' }]).trusted.map((d) => d.id)).toEqual(['a', 'b']);
  });

  it('breaks a tie between equal names by id', () => {
    const list = [dev('z', 'Same'), dev('y', 'Same')];
    expect(sendGroups(list).nearby.map((d) => d.id)).toEqual(['y', 'z']);
  });

  it('leaves out strangers that are gone', () => {
    expect(sendGroups([dev('a', 'A', { reachable: false })]).nearby).toEqual([]);
  });
});

describe('canTrust', () => {
  it('goes by being on this network, not by the route in use', () => {
    expect(canTrust(dev('a', 'A', { route: 'direct' }))).toBe(true);
    expect(canTrust(dev('a', 'A', { route: 'relayed', nearby: false }))).toBe(false);
    expect(canTrust(dev('a', 'A', { trusted: true }))).toBe(false);
    expect(canTrust(dev('a', 'A', { reachable: false }))).toBe(false);
  });
});
