import { describe, expect, it } from 'vitest';
import type { Transfer } from '../engine/types';
import { homeStatus, type StatusInput } from './status';

const transfer = (p: Partial<Transfer>): Transfer => ({
  id: 't1',
  direction: 'send',
  deviceId: 'd1',
  files: [],
  totalBytes: 200,
  doneBytes: 50,
  fileBytes: {},
  speed: 0,
  route: 'nearby',
  status: 'transferring',
  pausedBy: null,
  cancelledBy: null,
  error: null,
  startedAt: 0,
  movingSince: null,
  finishedAt: null,
  waitingLong: false,
  waking: false,
  ...p,
});

const base: StatusInput = { discovery: 'idle', active: [], reachable: 0, staging: false, deviceName: () => "Kim's XPS" };
const status = (p: Partial<StatusInput>) => homeStatus({ ...base, ...p });

describe('homeStatus', () => {
  it('says ready with nothing going on, and counts devices once found', () => {
    expect(status({}).text).toBe('Ready to beam');
    expect(status({ reachable: 1 }).text).toBe('1 device ready');
    expect(status({ reachable: 3 }).text).toBe('3 devices ready');
  });

  it('says it is looking while it has found nothing, with the sonar dot', () => {
    expect(status({ discovery: 'searching' })).toMatchObject({ text: 'Looking for devices', tone: 'searching' });
  });

  it('counts what it found even while it keeps looking', () => {
    expect(status({ discovery: 'searching', reachable: 5 })).toMatchObject({ text: '5 devices ready', tone: 'ready' });
  });

  it('puts a transfer before anything else but being offline', () => {
    const sending = status({ discovery: 'searching', staging: true, active: [transfer({})] });
    expect(sending).toMatchObject({ text: "Sending to Kim's XPS", tone: 'busy', percent: 25 });
    expect(status({ active: [transfer({ direction: 'receive', doneBytes: 200 })] })).toMatchObject({ text: "Receiving from Kim's XPS", percent: 100 });
    expect(status({ active: [transfer({ status: 'waiting' })] })).toMatchObject({ text: "Waiting for Kim's XPS", percent: null });
    expect(status({ active: [transfer({ status: 'reconnecting' })] }).text).toBe("Connecting to Kim's XPS");
    expect(status({ active: [transfer({ status: 'paused' })] }).tone).toBe('warn');
    expect(status({ active: [transfer({}), transfer({ id: 't2' })] }).text).toBe('2 transfers going');
    expect(status({ discovery: 'offline', active: [transfer({})] })).toMatchObject({ text: "You're offline", tone: 'warn' });
  });

  it('keeps the same key while only the percent moves, so the line rolls once', () => {
    expect(status({ active: [transfer({ doneBytes: 10 })] }).key).toBe(status({ active: [transfer({ doneBytes: 190 })] }).key);
    expect(status({ active: [transfer({ status: 'waiting' })] }).key).not.toBe(status({ active: [transfer({})] }).key);
  });

  it('asks for a device when files are waiting', () => {
    expect(status({ staging: true, discovery: 'searching' }).text).toBe('Pick a device');
  });
});
