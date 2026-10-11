import { describe, expect, it } from 'vitest';
import { fill, formatBytes, formatEta, formatSpeed, middleTruncate, percent, summarizeFiles } from './format';
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
    expect(formatEta(5, 1)).toBe('a few seconds');
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
