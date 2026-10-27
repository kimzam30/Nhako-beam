# Sounds

Every sound in Nhako Beam, on the desktop and Android apps alike. They're made from scratch by
[build.mjs](build.mjs): no samples, nothing recorded or copied. The feel is a game console menu
(short, bright and clicky: soft clicks, woody marimba notes, small bell chimes, in a major key),
written as plain synthesis so it's ours to change.

To listen, play the WAVs in `build/res/raw/` (any music player opens them).

## Changing a sound

1. Edit its recipe in `build.mjs` (each one says what it's for, how long it is, and how loud its
   loudest moment is).
2. Run `npm run sounds` in `desktop/` (or `node design/sounds/build.mjs`).
3. Build the apps as usual. Both pick up `build/` by themselves; it's committed, like the tokens.

Adding a sound: add it to `build.mjs`, run the build, then play it where it belongs (below).

## The sounds

| Name | When | Length |
|---|---|---|
| `tap` | Any button, row or device you tap | 45 ms |
| `toggleOn` | A switch turned on | 130 ms |
| `toggleOff` | A switch turned off | 130 ms |
| `open` | A sheet slides up, or a screen opens | 150 ms |
| `close` | A sheet closes, or Back | 130 ms |
| `found` | A device shows up on the radar | 260 ms |
| `incoming` | Someone wants to send you files (also the notification) | 950 ms |
| `accept` | Accept | 420 ms |
| `decline` | Decline | 330 ms |
| `send` | Files start going (dropped on a device, or Send) | 320 ms |
| `pause` | Pause | 150 ms |
| `resume` | Resume, or Try again | 150 ms |
| `done` | A transfer finished (also the "Got files" notification) | 1150 ms |
| `failed` | A transfer couldn't finish | 620 ms |
| `cancel` | Cancel, or the other device cancelled | 190 ms |
| `matched` | A code connected two devices | 900 ms |
| `error` | Something went wrong (an error message) | 260 ms |
| `tick` | A swiped card crosses the line where letting go accepts or declines | 25 ms |
| `lift` | Picking up files to drag them | 100 ms |
| `snap` | Dragged files snap onto a device | 60 ms |
| `intro` | The launch intro: a click as the bolt presses in, then a rising chime | 1250 ms |

## How they play

- **One sound per press.** Anything you can press plays `tap`, unless it has its own sound (Accept,
  Pause, a switch, Back). The tap waits until the press has been handled, and any other sound in
  that moment replaces it. The same sound asked for twice within 400 ms plays once.
- **Loudness.** Taps peak at -27 dB, switches, sheets and other small things around -21 dB, and the
  sounds that ask for your attention (incoming, done, the launch) at -8 dB. Settings > Appearance >
  Sounds is Loud (as made), Soft (all at 40%, the default) or Off.
- **Devices showing up** make one sound, not one each: none in the first 2.5 s after launch, then
  at most one every 2 s.
- **Desktop:** the Rust side plays them (`desktop/src-tauri/src/sound.rs`, through rodio), because
  WebKit keeps a page silent until its first click. The window asks for each one by name
  (`desktop/src/lib/sounds.svelte.ts`). Notifications are silent; the app plays the sound.
- **Android:** SoundPool, as interface sounds (`platform/Sounds.kt`, the rules in
  `state/SoundPlayer.kt`). Quiet on silent or vibrate, and while the app is out of sight: then the
  notification channels carry `incoming` (requests) and `done` (files received) themselves.
- **Browser prototype:** plays the same files in the page, after the first click.

## Files

| File | What |
|---|---|
| `build.mjs` | The recipes and the build |
| `build/res/raw/snd_*.wav` | 16-bit mono WAVs, 44.1 kHz. Android reads this folder as resources; the desktop app embeds the same files |
| `build/sounds.js`, `build/sounds.d.ts` | The names, for the Svelte UI |
| `build/sounds.rs` | The names and files, for the desktop app |
| `build/kotlin/NhakoSounds.kt` | The names and resources, for the Android app (`NhakoSound`) |
