# Nhako Beam design system

Version 0.1, Phase 1. The values live in [`design/tokens/tokens.json`](../../design/tokens/tokens.json). This document explains how to use them. If the two ever disagree, the JSON wins and this doc gets fixed.

Related docs:
- [Motion spec](motion-spec.md): every animation, gesture and haptic
- [Screens and states](screens-and-states.md): every screen and every state it can be in
- [UX copy](ux-copy.md): voice, tone and every string

---

## 1. Principles

1. **Calm by default.** A file transfer app is used when someone is in the middle of something else. Screens are quiet, with lots of space and one accent color. Nothing blinks for attention.
2. **Motion explains, it does not decorate.** Every animation answers a question: where did that come from, where did it go, is it working, did it finish? If an animation cannot answer one of those, it is cut.
3. **Direct manipulation.** Things follow your finger or cursor 1:1, keep their momentum when you let go, and can be grabbed again mid-animation. Nothing locks the UI while it animates.
4. **One brand, native manners.** Desktop and Android look like the same product (same colors, type and motion), but each respects its platform: Android back gesture, system share sheet and haptics; desktop keyboard shortcuts, tray and right-click menus.
5. **Honest status.** Progress, speed and connection type are always real numbers from the engine, never a fake animation. If something is stuck, we say so.
6. **No gradients.** Solid colors only. Depth comes from elevation (shadow), opacity and motion.

---

## 2. Color

Two themes, light and dark, following the system by default with a manual override in Settings. Every text and UI pair below is checked automatically by `node design/tokens/check-contrast.mjs` (currently 64 of 64 pass WCAG AA).

### Roles

| Role | Use it for | Do not use it for |
|---|---|---|
| `canvas` | App background behind everything | Cards |
| `surface` | Cards, lists, panels | The page background |
| `surfaceRaised` | Things floating above surfaces: sheets, menus, the incoming card | Static content |
| `surfaceSunken` | Wells: drop zone at rest, input fields | Buttons |
| `fill` / `fillPressed` | Secondary buttons, chips, segmented controls | Large areas |
| `separator` | Hairlines between rows | Borders around every card (use elevation instead) |
| `textPrimary` | Titles, body, values | |
| `textSecondary` | Supporting text, labels, metadata | Anything the user must read to act |
| `textTertiary` | Timestamps, hints | Key information |
| `textDisabled` | Disabled labels only (WCAG exempt) | Placeholder text |
| `accent` / `accentPressed` | Primary button, active device, selection, the beam | Body text |
| `accentText` | Links and text buttons on any surface | Filled backgrounds |
| `accentSubtle` | Selected row, focus wells, "trusted" badge background | |
| `onAccent` | Text and icons on `accent` | |
| `success`, `warning`, `danger` | Status text and icons | Decoration |
| `*Subtle` | Status banner backgrounds, paired with their color as text | |
| `onDanger` | Text on a filled `danger` button | |
| `focusRing` | Keyboard focus outline, 2px, offset 2px | |
| `scrim` | Behind sheets and dialogs | |
| `beamCore` / `beamGlow` / `beamTrack` | The transfer beam only | Anything else, so the beam stays special |

### Rules
- **One accent per screen region.** If two things on screen are accent colored, one of them is probably wrong.
- **Never rely on color alone.** Status always has an icon and words too ("Sent", "Failed", "Paused").
- **The glow is a shadow, not a gradient.** `beamGlow` is used as a `box-shadow` / Compose `shadow()` color only.

---

## 3. Typography

**Typeface: Inter** (SIL Open Font License, free), bundled inside the apps so it looks the same on Windows, Linux and Android. If it fails to load, it falls back to the system font listed in `font.sans`. Mono (`JetBrains Mono`, also OFL) is only for device fingerprints and technical details in Settings.

| Token | Size / line | Weight | Use |
|---|---|---|---|
| `largeTitle` | 32 / 38 | 700 | One per screen at most: "Ready to beam" |
| `title1` | 26 / 32 | 700 | Screen titles on desktop |
| `title2` | 21 / 27 | 650 | Section titles, device name on the transfer screen |
| `headline` | 17 / 22 | 600 | Card titles, file names, primary button labels |
| `body` | 15 / 22 | 400 | Default text |
| `callout` | 14 / 20 | 400 | Banners, secondary descriptions |
| `footnote` | 13 / 18 | 400 | Metadata: size, time, connection type |
| `caption` | 12 / 16 | 500 | Badges, labels under device orbs |
| `codeWords` | 24 / 30 | 600 | The pairing words, for example "river lantern seven" |

Rules:
- Sentence case everywhere. No ALL CAPS labels.
- **Tabular numbers** (`.tabular` / `fontFeatureSettings = "tnum"`) for anything that changes live: percent, speed, size, ETA, countdown. Otherwise digits jitter as they change.
- Android uses the same numbers in `sp`, so text grows with the user's font size setting. Layouts must survive 200% font scale.
- Truncate file names in the middle, not the end: `holiday-vid…final.mp4`, so the extension stays visible.

---

## 4. Spacing, radius, size

**Spacing** is a 4 point scale: `space.1` = 2, `2` = 4, `3` = 8, `4` = 12, `5` = 16, `6` = 20, `7` = 24, `8` = 32, `9` = 40, `10` = 48, `11` = 64.
- Screen side padding: 16 on compact, 24 on medium, 32 on expanded.
- Gap inside a card: 12. Gap between cards: 12 to 16. Gap between sections: 32.

**Radius:** `xs` 6 (badges), `sm` 10 (chips, inputs), `md` 14 (list cards, swatches), `lg` 20 (main cards, incoming card), `xl` 28 (sheets, large panels), `full` (pills, orbs, primary buttons).
- Nested corners: inner radius = outer radius minus the padding between them, so curves stay parallel.
- Android uses Compose `RoundedCornerShape`; where we want the smoother Apple-like corner we can switch to a squircle shape later without changing tokens.

**Hit targets:** at least 48dp on Android (`hitMinTouch`) and 32px on desktop with a pointer (`hitMinPointer`). Small icons get invisible padding to reach this.

**Content width:** reading content never goes wider than 560 (`contentMaxWidth`), even on a big desktop window.

---

## 5. Elevation

Five levels. Light theme uses soft layered shadows. Dark theme uses a faint 1px top light edge plus a deeper shadow, because shadows are hard to see on dark backgrounds.

| Level | Use |
|---|---|
| 0 | Flat content on canvas |
| 1 | Cards and list groups at rest |
| 2 | Hovered card, dragged file stack, device orb under the cursor |
| 3 | Menus, popovers, toasts, the incoming request card |
| 4 | Sheets and dialogs |

Elevation changes are animated with the `smooth` spring together with a `lifted` scale (1.02), so lifting something looks physical.

---

### Depth (Phase 22)

Home is a real 3D stage (the voxel mascot and your devices on a floor, `design/mascot/`), lit by one key light from the upper left and a soft sky fill, with a soft shadow under everything that stands on the floor. Colors stay flat: no gradients, no textures. Around it the flat UI gets a little depth from the `depth` tokens: dialog sheets lean up to 2.5 degrees towards the pointer (`tiltMaxDeg`, `perspective` 1100). Reduced motion keeps them flat.

## 6. Layout and breakpoints

| Class | Width | Layout |
|---|---|---|
| Compact | under 600 | Single column. Bottom actions within thumb reach. Android phones and very narrow desktop windows. |
| Medium | 600 to 839 | Single column with wider margins. Small desktop windows, tablets in portrait. |
| Expanded | 840 and up | Two panes: devices on the left, transfer activity on the right. Default desktop window. |

The desktop window opens at 960 x 640 and can shrink to 380 x 560 (compact layout). The tray popover is 360 x 520 and always uses the compact layout.

---

## 7. Iconography

**Phosphor Icons** (MIT license, free). One icon set on both platforms; the SVGs convert to Android vector drawables at build time. No emoji anywhere in the UI.

| Weight | Use |
|---|---|
| **Duotone** (default) | Every icon at rest. The second tone is the same color at 20% opacity, so icons feel richer without gradients. |
| **Fill** | Active or selected state: current tab, toggled on, the device you are dropping onto. Swapping duotone to fill is animated (crossfade plus a small `snappy` scale). |
| **Bold** | Tiny sizes (16px and under) where the duotone layer would turn to mush. |

Sizes: 16, 20 (default), 24, and 32 to 40 for device orbs. Platform glyphs for device orbs: laptop, desktop tower, phone, tablet.

Icons that change meaning animate between states instead of swapping instantly: pause to play, copy to check, send to check, and the radar icon spins briefly while a manual scan runs.

---

## 8. Component inventory

Components the prototype (Phase 2) will build. Each one must define: rest, hover (desktop), pressed, focused, disabled, loading where it applies, and its reduced-motion variant.

| Component | Notes |
|---|---|
| **Button** | Primary (accent, pill), secondary (fill), ghost (accentText), destructive (danger). Press scale 0.97 with `press` spring. Loading state swaps label for a small spinner without changing width. |
| **IconButton** | 32px pointer / 48dp touch target. Tooltip on desktop after 600ms hover. |
| **DeviceOrb** | 64 (list) or 96 (radar focus). Circle with initials or platform glyph. States: idle, nearby (soft breathing ring), hovered or drag-target (scales 1.08, ring tightens), busy (beam ring), offline (40% opacity), trusted (small check badge). |
| **DeviceRadar** | Home screen centerpiece. Your device in the middle, nearby devices placed around it, settling in with `bouncy`. Sweep ring every 2.4s while searching only. |
| **DropZone** | Desktop: the whole window becomes the drop target when files enter it. Shows the file stack under the cursor. |
| **FileStack** | Up to 3 stacked thumbnails with a count badge ("+12"). Fans out on hover; follows the cursor with `magnet` while dragging. |
| **IncomingCard** | Swipeable request card: sender, file summary, total size. Swipe right accept, left decline, or use the buttons. Includes "Always accept from this device" toggle. |
| **BeamProgress** | The transfer bar. Fill tracks real bytes, glow pulses with throughput. Shows percent, speed, time left in tabular numbers. |
| **ConnectionBadge** | "Nearby" (LAN), "Direct" (internet P2P), "Relayed". Icon plus word, never color alone. |
| **CodeDisplay** | QR code plus three words plus a countdown ring. Copy button. |
| **CodeInput** | Three word slots with autocomplete from the word list; paste fills all three; wrong word shakes gently. |
| **Sheet** | Bottom sheet on compact, centered dialog on expanded. Drag down to dismiss (rubber band past the top). |
| **Toast** | Bottom, max 2 stacked, swipe sideways to dismiss, auto-hide after 4s unless it has an action. |
| **ListRow** | History and settings rows. Swipe left on Android for quick actions. |
| **Toggle**, **SegmentedControl** | `snappy` spring for the thumb and selection pill. |
| **Banner** | Inline status (offline, reconnecting, low space) using the `*Subtle` colors. |
| **EmptyState** | Illustration-free: one sentence, one action. |

---

## 9. Accessibility baseline

- WCAG 2.1 AA contrast for all text and UI parts (automated check).
- Full keyboard use on desktop: every action reachable, visible focus ring, `Esc` closes sheets, `Enter` accepts focused request.
- Screen readers: TalkBack and NVDA/Orca labels on every control; progress announced at 25% steps, not continuously.
- Reduced motion respected on both platforms, plus an in-app override (see motion spec).
- Gestures always have a button alternative (swipe to accept also has Accept and Decline buttons).
- Never time out a decision without saying so on screen.
