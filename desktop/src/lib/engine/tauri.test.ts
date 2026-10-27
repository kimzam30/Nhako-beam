// The real engine's start and stop, with Tauri's invoke and listen stood in: a failed start or a
// stop while starting must never leave event listeners behind (each one would deliver every event
// again).
import { beforeEach, describe, expect, it, vi } from 'vitest';

const live = new Set<string>();
let seq = 0;
let startAnswer: () => Promise<unknown> = () => Promise.resolve({ snapshot: {}, me: {} });

vi.mock('@tauri-apps/api/event', () => ({
  listen: async (name: string) => {
    const id = `${name}#${++seq}`;
    live.add(id);
    return () => live.delete(id);
  },
}));
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string) => (cmd === 'engine_start' ? startAnswer() : Promise.resolve(null)),
}));

const { TauriEngine } = await import('./tauri');

describe('starting the real engine', () => {
  beforeEach(() => {
    live.clear();
    startAnswer = () => Promise.resolve({ snapshot: {}, me: {} });
  });

  it('listens once, and lets go when stopped', async () => {
    const e = new TauriEngine();
    await e.start();
    await e.start();
    expect(live.size).toBe(2);
    e.stop();
    expect(live.size).toBe(0);
  });

  it('lets go of its listeners when the start fails, so a retry has just one pair', async () => {
    const e = new TauriEngine();
    startAnswer = () => Promise.reject('The save folder is not writable.');
    await expect(e.start()).rejects.toBe('The save folder is not writable.');
    expect(live.size).toBe(0);
    startAnswer = () => Promise.resolve({ snapshot: {}, me: {} });
    await e.start();
    expect(live.size).toBe(2);
    expect(e.started).toBe(true);
  });

  it('a stop while starting wins over the start', async () => {
    const e = new TauriEngine();
    let answer!: (v: unknown) => void;
    startAnswer = () => new Promise((r) => (answer = r));
    const starting = e.start();
    await vi.waitFor(() => expect(answer).toBeDefined());
    e.stop();
    answer({ snapshot: {}, me: {} });
    await starting;
    expect(live.size).toBe(0);
    expect(e.started).toBe(false);
  });
});
