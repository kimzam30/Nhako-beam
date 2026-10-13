// Pairing code words. Short, common, easy to say and type, no near-duplicates.
// One shared list for the app and the Rust core: core/src/words.txt (exactly 256 words, so each
// word is one byte). A code is four words: 4.3 billion combinations.
import list from '../../../../core/src/words.txt?raw';

export const WORDS: readonly string[] = list.split('\n').map((w) => w.trim()).filter(Boolean);

const SET = new Set<string>(WORDS);

export function isWord(w: string): boolean {
  return SET.has(w.trim().toLowerCase());
}

/** Up to `limit` words starting with `prefix` (for autocomplete). */
export function suggest(prefix: string, limit = 3): string[] {
  const p = prefix.trim().toLowerCase();
  if (!p) return [];
  const out: string[] = [];
  for (const w of SET) {
    if (w.startsWith(p)) out.push(w);
    if (out.length >= limit) break;
  }
  return out;
}

/** How many words a pairing code has. */
export const CODE_WORDS = 4;

export function randomWords(rand: () => number = Math.random): [string, string, string, string] {
  const list = [...SET];
  const pick = () => list[Math.floor(rand() * list.length)]!;
  return [pick(), pick(), pick(), pick()];
}

/** Splits pasted text like "river-lantern seven harbor" into words. */
export function splitCode(text: string): string[] {
  return text
    .toLowerCase()
    .split(/[^a-z]+/)
    .filter(Boolean);
}
