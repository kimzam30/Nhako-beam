import { describe, expect, it } from 'vitest';
import type { Transfer } from '../engine/types';
import { activityItems, groupProgress } from './groups';

const tr = (id: string, p: Partial<Transfer> = {}): Transfer => ({ id, direction: 'send', deviceId: `d-${id}`, files: [], totalBytes: 100, doneBytes: 0, status: 'transferring', speed: 0, ...p }) as Transfer;

describe('activityItems', () => {
  it('folds a group into one card where its first transfer was', () => {
    const items = activityItems([tr('a'), tr('b'), tr('c'), tr('d')], { g1: ['b', 'd'] });
    expect(items.map((x) => (x.kind === 'group' ? `${x.id}:${x.transfers.map((y) => y.id).join('')}` : x.transfer.id))).toEqual(['a', 'g1:bd', 'c']);
  });
  it('a group with one transfer left shown is that transfer again', () => {
    expect(activityItems([tr('a')], { g1: ['a', 'b'] })).toEqual([{ kind: 'one', transfer: tr('a') }]);
  });
});

describe('groupProgress', () => {
  it('counts every byte, so a fast device does not hide a slow one', () => {
    expect(groupProgress([tr('a', { doneBytes: 100, status: 'done' }), tr('b', { doneBytes: 0 })])).toBe(0.5);
    expect(groupProgress([tr('a', { totalBytes: 0, status: 'done' }), tr('b', { totalBytes: 0, status: 'done' })])).toBe(1);
  });
});
