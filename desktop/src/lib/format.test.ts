import { describe, expect, it } from 'vitest';
import { dayLabel, fill, formatBytes, formatDuration, formatEta, formatSpeed, middleTruncate, percent, summarizeFiles } from './format';
import { isWord, splitCode, suggest, WORDS } from './engine/words';

describe('formatBytes', () => {
  it('uses at most one decimal and drops trailing .0', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(999)).toBe('999 B');
    expect(formatBytes(1_000)).toBe('1 KB');
    expect(formatBytes(1_250_000)).toBe('1.3 MB');
    expect(formatBytes(1_400_000_000)).toBe('1.4 GB');
    expect(formatBytes(240_000_000)).toBe('240 MB');
  });
  it('formats speed', () => expect(formatSpeed(48_000_000)).toBe('48 MB/s'));
});

describe('formatEta', () => {
  it('hides when there is nothing to estimate', () => {
    expect(formatEta(100, 0)).toBeNull();
    expect(formatEta(0, 100)).toBeNull();
  });
  it('rounds to friendly units', () => {
    expect(formatEta(5, 1)).toBe('5 sec');
    expect(formatEta(2, 1)).toBe('5 sec');
    expect(formatEta(42, 1)).toBe('45 sec');
    expect(formatEta(130, 1)).toBe('2 min');
    expect(formatEta(3600 * 2 + 60 * 15, 1)).toBe('2 hr 15 min');
  });
});

describe('percent', () => {
  it('floors and clamps', () => {
    expect(percent(999, 1000)).toBe(99);
    expect(percent(2000, 1000)).toBe(100);
    expect(percent(1, 0)).toBe(0);
  });
});

describe('summarizeFiles', () => {
  const img = { name: 'a.jpg', mime: 'image/jpeg' };
  const vid = { name: 'b.mp4', mime: 'video/mp4' };
  const doc = { name: 'c.pdf', mime: 'application/pdf' };
  it('names a single file', () => expect(summarizeFiles([img])).toBe('a.jpg'));
  it('groups up to two kinds', () => expect(summarizeFiles([img, img, img, vid])).toBe('3 photos and 1 video'));
  it('falls back to a file count for mixed sets', () => expect(summarizeFiles([img, vid, doc])).toBe('3 files'));
});

describe('middleTruncate', () => {
  it('keeps the extension', () => {
    const t = middleTruncate('holiday-video-from-langkawi-final-final.mp4', 24);
    expect(t.endsWith('.mp4')).toBe(true);
    expect(t.length).toBeLessThanOrEqual(24);
    expect(t).toContain('…');
  });
  it('leaves short names alone', () => expect(middleTruncate('a.txt')).toBe('a.txt'));
});

describe('fill', () => {
  it('replaces placeholders', () => expect(fill('Send to {device}', { device: 'Studio PC' })).toBe('Send to Studio PC'));
});

describe('words', () => {
  it('has no duplicates', () => {
    expect(new Set(WORDS).size).toBe(WORDS.length);
    expect(WORDS.length).toBe(256);
  });
  it('recognises words case-insensitively', () => {
    expect(isWord('River')).toBe(true);
    expect(isWord('notaword')).toBe(false);
  });
  it('suggests by prefix', () => expect(suggest('lan')).toContain('lantern'));
  it('splits pasted codes', () => expect(splitCode('River-lantern  seven')).toEqual(['river', 'lantern', 'seven']));
});

describe('History formatting', () => {
  it('formatDuration reads naturally', () => {
    expect(formatDuration(200)).toBe('under a second');
    expect(formatDuration(12_000)).toBe('12 sec');
    expect(formatDuration(245_000)).toBe('4 min 5 sec');
    expect(formatDuration(180_000)).toBe('3 min');
    expect(formatDuration(4_800_000)).toBe('1 hr 20 min');
  });

  it('dayLabel says Today and Yesterday, then dates', () => {
    const now = new Date(2026, 9, 12, 9, 30).getTime();
    expect(dayLabel(new Date(2026, 9, 12, 0, 5).getTime(), now, 'Today', 'Yesterday')).toBe('Today');
    expect(dayLabel(new Date(2026, 9, 11, 23, 59).getTime(), now, 'Today', 'Yesterday')).toBe('Yesterday');
    const older = dayLabel(new Date(2026, 9, 9, 12).getTime(), now, 'Today', 'Yesterday');
    expect(older).not.toBe('Yesterday');
    expect(older).toMatch(/9/);
    expect(dayLabel(new Date(2025, 9, 9, 12).getTime(), now, 'Today', 'Yesterday')).toMatch(/2025/);
  });
});
