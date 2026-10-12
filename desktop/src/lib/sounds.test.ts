import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { SoundPlayer, type SoundName } from './sounds.svelte';
import { sounds as all } from '../../../design/sounds/build/sounds.js';

describe('sounds', () => {
  let played: SoundName[];
  let on: boolean;
  let player: SoundPlayer;

  beforeEach(() => {
    vi.useFakeTimers();
    played = [];
    on = true;
    player = new SoundPlayer((n) => played.push(n), () => on, () => 1);
  });
  afterEach(() => vi.useRealTimers());

  it('plays a tap once the click is handled', () => {
    player.play('tap');
    player.play('tap');
    expect(played).toEqual([]);
    vi.runAllTimers();
    expect(played).toEqual(['tap']);
  });

  it("lets a control's own sound replace the tap, so one press makes one sound", () => {
    player.play('tap');
    player.play('accept');
    vi.runAllTimers();
    expect(played).toEqual(['accept']);
  });

  it('plays the same sound once when a button and its code both ask for it', () => {
    player.play('accept');
    vi.advanceTimersByTime(380);
    player.play('accept');
    vi.advanceTimersByTime(500);
    player.play('accept');
    expect(played).toEqual(['accept', 'accept']);
  });

  it('plays the click that turns Sounds back on', () => {
    on = false;
    player.play('tap');
    on = true;
    vi.runAllTimers();
    expect(played).toEqual(['tap']);
  });

  it('stays quiet for the click that turns Sounds off', () => {
    player.play('tap');
    on = false;
    vi.runAllTimers();
    expect(played).toEqual([]);
  });

  it('stays quiet with Sounds off', () => {
    on = false;
    player.play('tap');
    player.play('done');
    vi.runAllTimers();
    expect(played).toEqual([]);
    expect(player.entries).toEqual([]);
  });

  it('passes the volume on', () => {
    const heard: number[] = [];
    const soft = new SoundPlayer((_, v) => heard.push(v), () => true, () => 0.4);
    soft.play('done');
    expect(heard).toEqual([0.4]);
  });

  it('has a file for every sound', () => {
    for (const [name, s] of Object.entries(all)) {
      expect(s.file, name).toMatch(/^snd_[a-z_]+\.wav$/);
      expect(s.use.length, name).toBeGreaterThan(3);
    }
  });
});
