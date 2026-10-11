import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { CHECK_MS, MockEngine, sampleFiles } from './mock';
import type { EngineEvent, Transfer } from './types';

let engine: MockEngine;
let clock = 0;
let events: EngineEvent[] = [];

const transfers = () => events.filter((e): e is Extract<EngineEvent, { type: 'transfer' }> => e.type === 'transfer').map((e) => e.transfer);
const latest = (id: string): Transfer => transfers().filter((t) => t.id === id).pop()!;

function run(ms: number): void {
  for (let t = 0; t < ms; t += 100) {
    clock += 100;
    vi.advanceTimersByTime(100); // also fires the engine's own 100 ms tick
  }
}

beforeEach(() => {
  vi.useFakeTimers();
  clock = 0;
  events = [];
  engine = new MockEngine();
  engine.now = () => clock;
  engine.on((e) => events.push(e));
  engine.start();
  run(3000); // let seeded devices appear
});

afterEach(() => {
  engine.stop();
  vi.useRealTimers();
});

describe('MockEngine', () => {
  it('discovers the seeded devices', () => {
    const ids = new Set(events.filter((e) => e.type === 'device').map((e) => (e as { device: { id: string } }).device.id));
    expect(ids.has('d-aisyah')).toBe(true);
    expect(ids.has('d-studio')).toBe(true);
  });

  it('sends to a trusted device without waiting and finishes', () => {
    const id = engine.sendNow('d-aisyah', sampleFiles('mixed'));
    expect(latest(id).status).toBe('connecting');
    run(1000);
    expect(latest(id).status).toBe('transferring');
    run(15000);
    const t = latest(id);
    expect(t.status).toBe('done');
    expect(t.doneBytes).toBe(t.totalBytes);
    expect(Object.values(t.fileBytes).reduce((a, b) => a + b, 0)).toBe(t.totalBytes);
  });

  it('waits for an untrusted device and respects a decline', () => {
    engine.nextSendOutcome = 'decline';
    const id = engine.sendNow('d-studio', sampleFiles('photos'));
    expect(latest(id).status).toBe('waiting');
    run(2500);
    expect(latest(id).status).toBe('declined');
  });

  it('marks a long wait after 60 seconds', () => {
    engine.nextSendOutcome = 'ignore';
    const id = engine.sendNow('d-studio', sampleFiles('photos'));
    run(61_000);
    expect(latest(id).status).toBe('waiting');
    expect(latest(id).waitingLong).toBe(true);
  });

  it('reconnects and rolls back a little, never below zero', () => {
    const id = engine.sendNow('d-aisyah', sampleFiles('video'));
    run(4000);
    const before = latest(id).doneBytes;
    engine.simDropConnection();
    expect(latest(id).status).toBe('reconnecting');
    run(2700);
    const after = latest(id);
    expect(after.status).toBe('transferring');
    expect(after.doneBytes).toBeGreaterThan(0);
    expect(after.doneBytes).toBeLessThan(before + after.totalBytes * 0.05);
  });

  it('pauses and resumes', () => {
    const id = engine.sendNow('d-aisyah', sampleFiles('video'));
    run(2000);
    engine.pause(id);
    expect(latest(id).status).toBe('paused');
    const stuck = latest(id).doneBytes;
    run(2000);
    expect(latest(id).doneBytes).toBe(stuck);
    engine.resume(id);
    run(1000);
    expect(latest(id).status).toBe('transferring');
  });

  it('retries a failed transfer from where it stopped', () => {
    const id = engine.sendNow('d-aisyah', sampleFiles('video'));
    run(3000);
    engine.simFail('network');
    const stopped = latest(id).doneBytes;
    expect(latest(id).status).toBe('failed');
    engine.retry(id);
    run(1000);
    expect(latest(id).status).toBe('transferring');
    expect(latest(id).doneBytes).toBeGreaterThanOrEqual(stopped);
  });

  it('auto-accepts incoming files from a trusted auto-accept device', () => {
    engine.simIncoming('photos', 'd-phone');
    expect(events.some((e) => e.type === 'incoming')).toBe(false);
    expect(transfers().some((t) => t.direction === 'receive' && t.deviceId === 'd-phone')).toBe(true);
  });

  it('raises a request for other devices and accepts it with trust', () => {
    engine.simIncoming('photos', 'd-studio');
    const req = events.find((e) => e.type === 'incoming') as Extract<EngineEvent, { type: 'incoming' }>;
    expect(req).toBeTruthy();
    const id = engine.acceptNow(req.request.id, { trust: true });
    expect(id).toBeTruthy();
    const trusted = events.filter((e) => e.type === 'device' && e.device.id === 'd-studio').pop() as Extract<EngineEvent, { type: 'device' }>;
    expect(trusted.device.trusted).toBe(true);
  });

  it('pauses with diskFull when the receiver runs out of space', () => {
    engine.simSetFreeSpace(10_000_000);
    engine.simIncoming('video', 'd-phone');
    run(3000);
    const t = transfers().filter((x) => x.direction === 'receive').pop()!;
    expect(t.status).toBe('paused');
    expect(t.error).toBe('diskFull');
  });

  it('redeems codes: unknown word, no match, then match', async () => {
    const bad = engine.redeemCode(['river', 'lanternx', 'seven']);
    run(300);
    expect(await bad).toEqual({ ok: false, reason: 'unknownWord', index: 1 });

    const wrong = engine.redeemCode(['river', 'lantern', 'acorn']);
    run(800);
    expect(await wrong).toEqual({ ok: false, reason: 'noMatch' });

    const good = engine.redeemCode(['River', 'lantern', 'seven']);
    run(1000);
    const r = await good;
    expect(r.ok).toBe(true);
    expect(events.some((e) => e.type === 'paired' && e.via === 'entered')).toBe(true);
  });

  it('creates a 10 minute code and pairs when the peer redeems it', () => {
    const code = engine.createCodeNow();
    expect(code.words).toHaveLength(3);
    expect(code.expiresAt - clock).toBe(600_000);
    engine.simPeerRedeemsCode();
    expect(events.some((e) => e.type === 'paired' && e.via === 'shown')).toBe(true);
  });

  it('checks and saves your own server', async () => {
    const own = { mode: 'own' as const, relay: 'http://192.168.1.20:3340', token: 'secret', lookup: 'http://192.168.1.20:3341' };
    expect(engine.getConnectionNow()).toEqual({ mode: 'public' });

    const ok = engine.checkServer(own);
    run(CHECK_MS);
    expect(await ok).toEqual({ relay: { status: 'ok', ms: 14 }, lookup: { status: 'ok', ms: 6 } });

    const wrong = engine.checkServer({ ...own, token: 'wrong' });
    run(CHECK_MS);
    expect((await wrong).relay).toEqual({ status: 'tokenRejected' });

    const off = engine.checkServer({ mode: 'own', relay: 'http://offline.lan:3340', token: 'x' });
    run(CHECK_MS);
    const offResult = await off;
    expect(offResult.relay.status).toBe('unreachable');
    expect(offResult.lookup).toBeNull();

    expect(() => engine.setConnectionNow({ mode: 'own', relay: '192.168.1.20' })).toThrow();
    expect(engine.getConnectionNow()).toEqual({ mode: 'public' });
    engine.setConnectionNow(own);
    expect(engine.getConnectionNow()).toEqual(own);
    expect(events.at(-1)).toEqual({ type: 'discovery', state: 'searching' });
  });
});
