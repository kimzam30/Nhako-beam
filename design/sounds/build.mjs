// Makes every Nhako Beam sound from scratch: no samples, nothing recorded or copied. The feel is
// a game console menu (short, bright, clicky: soft clicks, woody marimba notes, small bell chimes,
// in a major key), written as plain synthesis so it's ours to tweak.
//
//   node design/sounds/build.mjs        (or `npm run sounds` in desktop/)
//
// Writes, into design/sounds/build/:
//   res/raw/snd_<name>.wav   16-bit mono WAVs. Android reads this folder as resources; the desktop
//                            app embeds the same files.
//   sounds.js, sounds.d.ts   the names, for the Svelte UI
//   sounds.rs                the names and files, for the desktop app (Rust)
//   kotlin/NhakoSounds.kt    the names and resources, for the Android app
// What each sound is for: design/sounds/README.md.

import { mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = dirname(fileURLToPath(import.meta.url));
const OUT = join(HERE, 'build');
const SR = 44100;

// Notes (Hz).
const N = {
  B4: 493.88, C5: 523.25, D5: 587.33, E5: 659.25, G5: 783.99, A5: 880.0, B5: 987.77,
  C6: 1046.5, D6: 1174.66, E6: 1318.51, G6: 1567.98, A6: 1760.0, C7: 2093.0, E7: 2637.02, G7: 3135.96,
};

// Timbres: [frequency ratio, level, how fast it fades relative to the note].
const SINE = [[1, 1, 1]];
const MARIMBA = [[1, 1, 1], [4, 0.22, 0.28], [9.9, 0.05, 0.12]];
const BELL = [[1, 1, 1], [2, 0.32, 0.7], [2.76, 0.18, 0.5], [5.4, 0.07, 0.3]];
const SOFT = [[1, 1, 1], [2, 0.12, 0.6]];

const ms = (x) => Math.round((x * SR) / 1000);

/** A struck note: partials that fade exponentially, with an optional pitch glide into it. */
function note(out, atMs, { f, dur = 200, gain = 1, attack = 1.5, decay = 60, timbre = MARIMBA, glide = 1, glideMs = 10 }) {
  const start = ms(atMs);
  const len = Math.min(ms(dur), out.length - start);
  for (const [ratio, level, fade] of timbre) {
    let phase = 0;
    const tau = (decay * fade * SR) / 1000;
    for (let i = 0; i < len; i++) {
      const t = i / SR;
      const bend = 1 + (glide - 1) * Math.exp(-(t * 1000) / glideMs);
      phase += (2 * Math.PI * f * ratio * bend) / SR;
      const a = Math.min(1, i / Math.max(1, ms(attack)));
      const tail = Math.min(1, (len - i) / ms(4));
      out[start + i] += Math.sin(phase) * level * gain * a * tail * Math.exp(-i / tau);
    }
  }
}

/** Filtered noise, for clicks and the whoosh of sending. The cutoff moves from `from` to `to` Hz. */
function noise(out, atMs, { dur = 20, gain = 0.3, from = 6000, to = from, peakAt = 0.15, seed = 7 }) {
  const start = ms(atMs);
  const len = Math.min(ms(dur), out.length - start);
  let s = seed;
  let lp = 0;
  let prev = 0;
  for (let i = 0; i < len; i++) {
    s = (s * 1103515245 + 12345) & 0x7fffffff;
    const white = (s / 0x3fffffff) - 1;
    const p = i / len;
    const cut = from + (to - from) * p;
    const k = 1 - Math.exp((-2 * Math.PI * cut) / SR);
    lp += k * (white - lp);
    const hp = lp - prev;
    prev = lp;
    const env = p < peakAt ? p / peakAt : Math.pow(1 - (p - peakAt) / (1 - peakAt), 2);
    out[start + i] += (lp * 0.6 + hp * 0.4) * env * gain;
  }
}

/** A soft, high click with a little body: the sound of pressing something. */
function click(out, atMs, { f = 2300, gain = 1, body = 300 } = {}) {
  note(out, atMs, { f, dur: 40, gain, decay: 7, timbre: SINE, glide: 1.5, glideMs: 3, attack: 0.3 });
  note(out, atMs, { f: body, dur: 30, gain: gain * 0.35, decay: 6, timbre: SINE, attack: 0.5 });
  noise(out, atMs, { dur: 5, gain: gain * 0.12, from: 9000 });
}

// Every sound: what it's for, how long it is, how loud its loudest moment is (dB below full
// scale), and how it's made. Taps are the quietest; things that ask for attention are loudest.
const SOUNDS = {
  tap: {
    use: 'Any button, row or device you tap',
    dur: 45, peak: -27,
    // Rounder than the other clicks: you hear it all the time.
    make: (o) => {
      note(o, 0, { f: 1500, dur: 40, decay: 6, timbre: SINE, glide: 1.25, glideMs: 3, attack: 0.8 });
      note(o, 0, { f: 240, dur: 30, gain: 0.25, decay: 6, timbre: SINE, attack: 0.8 });
      noise(o, 0, { dur: 4, gain: 0.04, from: 6000 });
    },
  },
  toggleOn: {
    use: 'A switch turned on',
    dur: 130, peak: -21,
    make: (o) => {
      note(o, 0, { f: N.E6, dur: 80, decay: 14 });
      note(o, 45, { f: N.A6, dur: 85, decay: 18 });
    },
  },
  toggleOff: {
    use: 'A switch turned off',
    dur: 130, peak: -22,
    make: (o) => {
      note(o, 0, { f: N.E6, dur: 80, decay: 14 });
      note(o, 45, { f: N.B5, dur: 85, decay: 18 });
    },
  },
  open: {
    use: 'A sheet slides up, or a screen opens',
    dur: 150, peak: -20,
    make: (o) => {
      note(o, 0, { f: 640, dur: 140, decay: 34, timbre: SOFT, glide: 0.62, glideMs: 28 });
      note(o, 32, { f: N.G6, dur: 110, gain: 0.22, decay: 30 });
    },
  },
  close: {
    use: 'A sheet closes, or Back',
    dur: 130, peak: -21,
    make: (o) => note(o, 0, { f: 600, dur: 125, decay: 30, timbre: SOFT, glide: 1.45, glideMs: 26 }),
  },
  select: {
    use: 'Picking files, or picking where they go',
    dur: 170, peak: -19,
    make: (o) => note(o, 0, { f: N.C6, dur: 170, decay: 34, timbre: [...MARIMBA, [2, 0.18, 0.5]] }),
  },
  found: {
    use: 'A device shows up on the radar',
    dur: 260, peak: -15,
    make: (o) => {
      note(o, 0, { f: N.D6, dur: 150, decay: 38, glide: 0.82, glideMs: 12 });
      note(o, 70, { f: N.G6, dur: 190, decay: 46, glide: 0.82, glideMs: 12 });
    },
  },
  incoming: {
    use: 'Someone wants to send you files (also the notification)',
    dur: 950, peak: -8,
    make: (o) => {
      note(o, 0, { f: N.G5, dur: 600, decay: 150, timbre: BELL });
      note(o, 95, { f: N.C6, dur: 650, decay: 170, timbre: BELL });
      note(o, 190, { f: N.E6, dur: 760, decay: 230, timbre: BELL });
      note(o, 190, { f: N.C7, dur: 600, gain: 0.14, decay: 160, timbre: SINE });
    },
  },
  accept: {
    use: 'Accept',
    dur: 420, peak: -10,
    make: (o) => {
      note(o, 0, { f: N.E6, dur: 260, decay: 60, timbre: BELL });
      note(o, 70, { f: N.A6, dur: 350, decay: 100, timbre: BELL });
    },
  },
  decline: {
    use: 'Decline',
    dur: 330, peak: -13,
    make: (o) => {
      note(o, 0, { f: N.A5, dur: 200, decay: 45 });
      note(o, 85, { f: N.E5, dur: 240, decay: 60 });
    },
  },
  send: {
    use: 'Files start going (dropped on a device, or Send)',
    dur: 320, peak: -12,
    make: (o) => {
      noise(o, 0, { dur: 260, gain: 0.55, from: 700, to: 6500, peakAt: 0.45, seed: 11 });
      note(o, 30, { f: 1400, dur: 250, gain: 0.6, decay: 70, timbre: SOFT, glide: 0.36, glideMs: 70 });
    },
  },
  pause: {
    use: 'Pause',
    dur: 150, peak: -15,
    make: (o) => {
      note(o, 0, { f: N.G6, dur: 80, decay: 22 });
      note(o, 50, { f: N.D6, dur: 100, decay: 26 });
    },
  },
  resume: {
    use: 'Resume, or Try again',
    dur: 150, peak: -15,
    make: (o) => {
      note(o, 0, { f: N.D6, dur: 80, decay: 22 });
      note(o, 50, { f: N.G6, dur: 100, decay: 26 });
    },
  },
  done: {
    use: 'A transfer finished (also the "Got files" notification)',
    dur: 1150, peak: -8,
    make: (o) => {
      note(o, 0, { f: N.C6, dur: 260, decay: 60, timbre: BELL });
      note(o, 80, { f: N.E6, dur: 260, decay: 60, timbre: BELL });
      note(o, 160, { f: N.G6, dur: 300, decay: 70, timbre: BELL });
      note(o, 255, { f: N.C7, dur: 880, decay: 270, timbre: BELL });
      note(o, 255, { f: N.E7, dur: 700, gain: 0.35, decay: 200, timbre: SINE });
    },
  },
  failed: {
    use: "A transfer couldn't finish",
    dur: 620, peak: -11,
    make: (o) => {
      const detuned = [[1, 1, 1], [1.012, 0.45, 0.9], [4, 0.12, 0.25]];
      note(o, 0, { f: N.E5, dur: 300, decay: 80, timbre: detuned });
      note(o, 130, { f: N.C5, dur: 480, decay: 140, timbre: detuned });
    },
  },
  cancel: {
    use: 'Cancel, or the other device cancelled',
    dur: 190, peak: -14,
    make: (o) => note(o, 0, { f: 430, dur: 185, decay: 46, timbre: SOFT, glide: 2.1, glideMs: 45 }),
  },
  matched: {
    use: 'A code connected two devices',
    dur: 900, peak: -10,
    make: (o) => {
      [N.C6, N.E6, N.G6, N.C7, N.E7].forEach((f, i) => note(o, i * 42, { f, dur: 220, decay: 60, timbre: BELL, gain: 0.8 }));
      note(o, 5 * 42, { f: N.G7, dur: 650, decay: 220, timbre: BELL, gain: 0.7 });
    },
  },
  error: {
    use: 'Something went wrong (an error message)',
    dur: 260, peak: -13,
    make: (o) => {
      const bonk = [[1, 1, 1], [2, 0.3, 0.5], [3, 0.08, 0.3]];
      note(o, 0, { f: 330, dur: 110, decay: 34, timbre: bonk });
      note(o, 115, { f: 311, dur: 140, decay: 40, timbre: bonk });
      noise(o, 0, { dur: 8, gain: 0.08, from: 3000 });
    },
  },
  tick: {
    use: 'A swiped card crosses the line where letting go accepts or declines',
    dur: 25, peak: -28,
    make: (o) => note(o, 0, { f: 3000, dur: 22, decay: 3.5, timbre: SINE, attack: 0.3 }),
  },
  lift: {
    use: 'Picking up files to drag them',
    dur: 100, peak: -22,
    make: (o) => note(o, 0, { f: 520, dur: 95, decay: 24, timbre: SOFT, glide: 0.7, glideMs: 22 }),
  },
  snap: {
    use: 'Dragged files snap onto a device',
    dur: 60, peak: -21,
    make: (o) => {
      click(o, 0, { f: 1500, body: 420 });
    },
  },
  intro: {
    use: 'The launch intro: a click as the bolt presses in, then a rising chime',
    dur: 1250, peak: -8,
    make: (o) => {
      noise(o, 0, { dur: 170, gain: 0.12, from: 2500, to: 900, peakAt: 0.7, seed: 3 });
      click(o, 175, { f: 2600, gain: 1, body: 160 });
      note(o, 175, { f: 130, dur: 120, gain: 0.5, decay: 30, timbre: SINE, glide: 1.6, glideMs: 15 });
      note(o, 230, { f: N.C6, dur: 500, decay: 120, timbre: BELL, gain: 0.7 });
      note(o, 300, { f: N.G6, dur: 600, decay: 150, timbre: BELL, gain: 0.7 });
      note(o, 370, { f: N.C7, dur: 880, decay: 280, timbre: BELL, gain: 0.8 });
    },
  },
};

/** Scales a sound so its loudest moment is exactly `peak` dB (all well below clipping). */
function finish(buf, peak) {
  let max = 0;
  for (const v of buf) max = Math.max(max, Math.abs(v));
  const k = max > 0 ? Math.pow(10, peak / 20) / max : 0;
  return buf.map((v) => v * k);
}

function wav(samples) {
  const data = Buffer.alloc(samples.length * 2);
  samples.forEach((v, i) => data.writeInt16LE(Math.max(-32768, Math.min(32767, Math.round(v * 32767))), i * 2));
  const h = Buffer.alloc(44);
  h.write('RIFF', 0);
  h.writeUInt32LE(36 + data.length, 4);
  h.write('WAVE', 8);
  h.write('fmt ', 12);
  h.writeUInt32LE(16, 16);
  h.writeUInt16LE(1, 20); // PCM
  h.writeUInt16LE(1, 22); // mono
  h.writeUInt32LE(SR, 24);
  h.writeUInt32LE(SR * 2, 28);
  h.writeUInt16LE(2, 32);
  h.writeUInt16LE(16, 34);
  h.write('data', 36);
  h.writeUInt32LE(data.length, 40);
  return Buffer.concat([h, data]);
}

const snake = (name) => name.replace(/[A-Z]/g, (c) => `_${c.toLowerCase()}`);
const pascal = (name) => name[0].toUpperCase() + name.slice(1);
const file = (name) => `snd_${snake(name)}.wav`;
const names = Object.keys(SOUNDS);

rmSync(OUT, { recursive: true, force: true });
mkdirSync(join(OUT, 'res/raw'), { recursive: true });
mkdirSync(join(OUT, 'kotlin'), { recursive: true });

const header = 'Generated by design/sounds/build.mjs. Do not edit.';
for (const name of names) {
  const s = SOUNDS[name];
  const buf = new Float32Array(ms(s.dur));
  s.make(buf);
  writeFileSync(join(OUT, 'res/raw', file(name)), wav(finish(buf, s.peak)));
}

writeFileSync(
  join(OUT, 'sounds.js'),
  `// ${header}\nexport const sounds = ${JSON.stringify(Object.fromEntries(names.map((n) => [n, { file: file(n), use: SOUNDS[n].use }])), null, 2)};\n`,
);
writeFileSync(
  join(OUT, 'sounds.d.ts'),
  `// ${header}\nexport type SoundName = ${names.map((n) => `'${n}'`).join(' | ')};\nexport declare const sounds: Record<SoundName, { file: string; use: string }>;\n`,
);
writeFileSync(
  join(OUT, 'sounds.rs'),
  `// ${header}\n/// Every sound by the name the UI uses, with its WAV file.\npub const SOUNDS: &[(&str, &[u8])] = &[\n${names
    .map((n) => `    ("${n}", include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../design/sounds/build/res/raw/${file(n)}"))),`)
    .join('\n')}\n];\n`,
);
writeFileSync(
  join(OUT, 'kotlin/NhakoSounds.kt'),
  `// ${header}\npackage io.github.kimzam30.nhakobeam.design\n\nimport io.github.kimzam30.nhakobeam.R\n\n/** Every sound, with its resource (design/sounds/README.md says what each is for). */\nenum class NhakoSound(val res: Int) {\n${names
    .map((n) => `    /** ${SOUNDS[n].use}. */\n    ${pascal(n)}(R.raw.${file(n).replace('.wav', '')}),`)
    .join('\n')}\n}\n`,
);
console.log(`${names.length} sounds in design/sounds/build/`);
