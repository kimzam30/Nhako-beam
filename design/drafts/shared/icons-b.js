// The full Nhako Beam icon set in style B ("Bolt solid"), chosen by Kim on 2026-11-09.
// 24 grid, 20 px live area (2..22); round shapes may reach 1..23 (`round: true`).
// Parts of an icon:
//   solid: filled paths (evenodd, so inner paths cut holes)
//   bold:  thick strokes for line-like icons, default 3 units, round caps and joins
//   knock: strokes cut out of solid and bold (detail and separation), default 2.2 units
//   top:   drawn after the cut-outs, never cut ({ d, w } stroke or { fill: d })
// Offset: the same drawing 1 device px down-right in navy, from 20 px up. Deeper navy #0B3E85
// whenever the icon or its surface is blue (accent buttons, accent-tinted and active icons),
// because in light theme the accent IS the brand navy (#1A6FE0) and the offset would vanish.
// Names are the app's own (desktop/src/lib/icons/icons.ts), so the set drops in by name.

const rr = (x, y, w, h, r) => `M${x + r} ${y}H${x + w - r}A${r} ${r} 0 0 1 ${x + w} ${y + r}V${y + h - r}A${r} ${r} 0 0 1 ${x + w - r} ${y + h}H${x + r}A${r} ${r} 0 0 1 ${x} ${y + h - r}V${y + r}A${r} ${r} 0 0 1 ${x + r} ${y}Z`;
const circ = (cx, cy, r) => `M${cx - r} ${cy}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0Z`;
const dot = (x, y) => `M${x} ${y}v.01`;

function gear(cx, cy, rOut, rIn, teeth) {
  const pts = [], step = 360 / teeth;
  for (let i = 0; i < teeth; i++) {
    const a = i * step - 90;
    for (const [d, r] of [[-step * 0.36, rIn], [-step * 0.2, rOut], [step * 0.2, rOut], [step * 0.36, rIn]]) {
      const t = ((a + d) * Math.PI) / 180;
      pts.push(`${(cx + r * Math.cos(t)).toFixed(2)} ${(cy + r * Math.sin(t)).toFixed(2)}`);
    }
  }
  return 'M' + pts.join('L') + 'Z';
}
// The app's own bolt (desktop/src/lib/motion/bolt.ts, 1024 box), fitted to the 24 grid.
const BOLT_1024 = [[676.16, 161.6], [829.76, 244.16], [608.96, 464.96], [743.36, 503.36], [194.24, 862.4], [436.16, 574.4], [301.76, 536]];
const BOLT = 'M' + BOLT_1024.map(([x, y]) => `${((x * 24 / 1024 - 12) * 1.2 + 12).toFixed(2)} ${((y * 24 / 1024 - 12) * 1.2 + 12).toFixed(2)}`).join('L') + 'Z';
const PAGE = 'M6.5 2.5H14L19 7.5V20A1.5 1.5 0 0 1 17.5 21.5H6.5A1.5 1.5 0 0 1 5 20V4A1.5 1.5 0 0 1 6.5 2.5Z';
const FOLD = 'M13.8 2.6V8H19';
const SPEAKER = 'M3 9.5A1.5 1.5 0 0 1 4.5 8H7.5L12 4.2A.8.8 0 0 1 13.3 4.8V19.2A.8.8 0 0 1 12 19.8L7.5 16H4.5A1.5 1.5 0 0 1 3 14.5Z';
const EYE = 'M12 5C17.5 5 21 9.6 22 12C21 14.4 17.5 19 12 19S3 14.4 2 12C3 9.6 6.5 5 12 5Z';
const WIFI = ['M3 9.5A13 13 0 0 1 21 9.5', 'M6.2 13A8.5 8.5 0 0 1 17.8 13', 'M9.4 16.3A4 4 0 0 1 14.6 16.3'];
const TRAY = 'M3 14H6V16.5H18V14H21V18A2.5 2.5 0 0 1 18.5 20.5H5.5A2.5 2.5 0 0 1 3 18Z';
const sunRays = Array.from({ length: 8 }, (_, i) => {
  const a = (i * 45 * Math.PI) / 180, c = Math.cos(a), s = Math.sin(a);
  return `M${(12 + 7.6 * c).toFixed(2)} ${(12 + 7.6 * s).toFixed(2)}L${(12 + 9.2 * c).toFixed(2)} ${(12 + 9.2 * s).toFixed(2)}`;
});

export const SET = {
  // Devices
  'paper-plane-tilt': { group: 'Send and receive', solid: ['M21 3L2.8 10.8L10.2 13.8L13.2 21.2Z'], knock: ['M10.2 13.8L18 6'] },
  'download-simple': { group: 'Send and receive', solid: ['M10.75 3H13.25V10.5H16.8L12 15.3L7.2 10.5H10.75Z', TRAY] },
  'upload-simple': { group: 'Send and receive', solid: ['M10.75 15.3H13.25V7.8H16.8L12 3L7.2 7.8H10.75Z', TRAY] },
  'share-network': { group: 'Send and receive', solid: [circ(17.5, 5.5, 3), circ(6.5, 12, 3), circ(17.5, 18.5, 3)], bold: [{ d: 'M8.8 10.6L15.2 6.9', w: 2.4 }, { d: 'M8.8 13.4L15.2 17.1', w: 2.4 }] },
  'link-simple': { group: 'Send and receive', bold: [{ d: 'M10 7.5L11.6 5.9A4.2 4.2 0 0 1 17.6 11.9L16 13.5', w: 2.8 }, { d: 'M14 16.5L12.4 18.1A4.2 4.2 0 0 1 6.4 12.1L8 10.5', w: 2.8 }, { d: 'M9.6 14.4L14.4 9.6', w: 2.8 }] },
  'qr-code': { group: 'Send and receive', solid: [rr(3, 3, 7, 7, 1.5) + rr(5.2, 5.2, 2.6, 2.6, .4), rr(14, 3, 7, 7, 1.5) + rr(16.2, 5.2, 2.6, 2.6, .4), rr(3, 14, 7, 7, 1.5) + rr(5.2, 16.2, 2.6, 2.6, .4), rr(14, 14, 3, 3, .6), rr(18, 18, 3, 3, .6), rr(14, 18.5, 2.5, 2.5, .6), rr(18.5, 14, 2.5, 2.5, .6)] },
  scan: { group: 'Send and receive', bold: [{ d: 'M3.5 8V5.5A2 2 0 0 1 5.5 3.5H8' }, { d: 'M16 3.5H18.5A2 2 0 0 1 20.5 5.5V8' }, { d: 'M20.5 16V18.5A2 2 0 0 1 18.5 20.5H16' }, { d: 'M8 20.5H5.5A2 2 0 0 1 3.5 18.5V16' }, { d: 'M7.5 12H16.5', w: 2.6 }] },
  handshake: { group: 'Send and receive', solid: [rr(2, 7, 3.2, 9, 1), rr(18.8, 7, 3.2, 9, 1), 'M6.3 8.6L9.6 6.6H13.6L17.7 8.4V14.2L12.6 19A1.4 1.4 0 0 1 10.6 18.9L10 18.3A1.4 1.4 0 0 1 8.1 18.1L7.7 17.6A1.4 1.4 0 0 1 6.3 16.6Z'], knock: [{ d: 'M14.2 6.9L10.6 10.2A1.3 1.3 0 0 0 12.4 12.1L14.7 10.6', w: 1.6 }, { d: 'M12.4 15.1L14.1 16.8', w: 1.4 }, { d: 'M10.3 16.5L11.6 17.8', w: 1.4 }] },
  'fingerprint-simple': { group: 'Send and receive', bold: [{ d: 'M3.5 13V12A8.5 8.5 0 0 1 20.5 12V13.5', w: 2 }, { d: 'M7 19.5C6.3 17.8 6 16.3 6 14.8V12A6 6 0 0 1 18 12V15C18 17 17.6 18.8 16.8 20.5', w: 2 }, { d: 'M12 11.5V15C12 17 12.5 18.8 13.5 20.5', w: 2 }, { d: 'M9 18C8.6 17 8.5 16 8.5 15V12A3.5 3.5 0 0 1 15.5 12V15', w: 2 }] },

  // Devices
  laptop: { group: 'Devices', solid: ['M4 5.5A1.5 1.5 0 0 1 5.5 4H18.5A1.5 1.5 0 0 1 20 5.5V15.5H4ZM6 6V13.5H18V6Z', 'M2 17H22V18A2 2 0 0 1 20 20H4A2 2 0 0 1 2 18Z'] },
  'desktop-tower': { group: 'Devices', solid: [rr(2, 4, 13.5, 10.5, 1.5) + rr(4, 6, 9.5, 6.5, .4), 'M7.2 14.5H10.3V17.5H12.3V19.5H5.2V17.5H7.2Z', rr(17.2, 4, 4.8, 15.5, 1.5)], knock: [{ d: 'M18.6 7.2H20.6', w: 1.4 }, { d: dot(19.6, 16), w: 1.8 }] },
  'device-mobile': { group: 'Devices', solid: ['M9 2H15A3 3 0 0 1 18 5V19A3 3 0 0 1 15 22H9A3 3 0 0 1 6 19V5A3 3 0 0 1 9 2ZM8 6.5V17.5H16V6.5Z'] },
  'device-tablet': { group: 'Devices', solid: [rr(4, 2, 16, 20, 2.5) + rr(6, 4.5, 12, 13.5, .5)] },
  devices: { group: 'Devices', solid: [rr(2, 5, 12.5, 9.5, 1.5) + rr(4, 7, 8.5, 5.5, .3), rr(2, 15.8, 12.5, 2.2, 1.1), rr(15.8, 7.5, 6.2, 13, 1.6) + rr(17.3, 9.5, 3.2, 8.5, .3)] },
  'hard-drives': { group: 'Devices', solid: [rr(3, 3.5, 18, 7.5, 2), rr(3, 13, 18, 7.5, 2)], knock: [{ d: dot(17, 7.25), w: 2.4 }, { d: dot(17, 16.75), w: 2.4 }, { d: 'M6.5 7.25H11.5', w: 1.8 }, { d: 'M6.5 16.75H11.5', w: 1.8 }] },
  keyboard: { group: 'Devices', solid: [rr(2, 5.5, 20, 13, 2.5)], knock: [...[6, 9.3, 12.6, 15.9, 18.2].map((x) => ({ d: dot(x, 9.5), w: 2 })), ...[6, 9.3, 12.6, 15.9].map((x) => ({ d: dot(x + 1.6, 12.6), w: 2 })), { d: 'M8 15.5H16', w: 2 }] },
  'battery-charging': { group: 'Devices', solid: [rr(2, 7, 16.8, 10, 2.5), rr(19.6, 10, 2.4, 4, 1)], knock: [{ d: 'M11.6 9.4L8.8 12.4H12.2L9.6 15', w: 1.8 }] },
  'cell-signal-full': { group: 'Devices', solid: [rr(3, 16, 3, 5, 1), rr(8, 12.5, 3, 8.5, 1), rr(13, 8.5, 3, 12.5, 1), rr(18, 4.5, 3, 16.5, 1)] },

  // Transfer and states
  check: { group: 'States', bold: [{ d: 'M4.5 12.5L9.5 17.5L19.5 6.5' }] },
  'check-circle': { group: 'States', round: true, solid: [circ(12, 12, 10)], knock: ['M7.6 12.4L10.6 15.3L16.4 9.2'] },
  x: { group: 'States', bold: [{ d: 'M6 6L18 18' }, { d: 'M18 6L6 18' }] },
  'x-circle': { group: 'States', round: true, solid: [circ(12, 12, 10)], knock: ['M8.8 8.8L15.2 15.2', 'M15.2 8.8L8.8 15.2'] },
  'warning-circle': { group: 'States', round: true, solid: [circ(12, 12, 10)], knock: [{ d: 'M12 7V12.6', w: 2.4 }, { d: dot(12, 16.4), w: 2.8 }] },
  info: { group: 'States', round: true, solid: [circ(12, 12, 10)], knock: [{ d: 'M12 11V16.4', w: 2.4 }, { d: dot(12, 7.6), w: 2.8 }] },
  'shield-check': { group: 'States', solid: ['M12 2.5L20 5.5V11.5C20 16.5 16.6 19.9 12 21.5C7.4 19.9 4 16.5 4 11.5V5.5Z'], knock: ['M8.5 12.2L11 14.7L15.8 9.7'] },
  'lock-simple': { group: 'States', solid: [rr(4.5, 10, 15, 11.5, 2.5)], bold: [{ d: 'M8 10V7.5A4 4 0 0 1 16 7.5V10', w: 2.6 }], knock: [{ d: 'M12 14.5V17', w: 2.4 }] },
  pause: { group: 'States', solid: ['M6 5.5A1.5 1.5 0 0 1 7.5 4H9.5A1.5 1.5 0 0 1 11 5.5V18.5A1.5 1.5 0 0 1 9.5 20H7.5A1.5 1.5 0 0 1 6 18.5Z', 'M13 5.5A1.5 1.5 0 0 1 14.5 4H16.5A1.5 1.5 0 0 1 18 5.5V18.5A1.5 1.5 0 0 1 16.5 20H14.5A1.5 1.5 0 0 1 13 18.5Z'] },
  play: { group: 'States', solid: ['M7 4.8A1.5 1.5 0 0 1 9.3 3.5L20.3 10.7A1.5 1.5 0 0 1 20.3 13.3L9.3 20.5A1.5 1.5 0 0 1 7 19.2Z'] },
  'arrow-clockwise': { group: 'States', round: true, bold: [{ d: 'M19.5 12A7.5 7.5 0 1 1 17.3 6.7' }], solid: ['M20.5 3.5V10.5H13.5Z'] },
  'arrows-clockwise': { group: 'States', round: true, bold: [{ d: 'M4.5 10.5A7.5 7.5 0 0 1 17.6 7.2', w: 2.8 }, { d: 'M19.5 13.5A7.5 7.5 0 0 1 6.4 16.8', w: 2.8 }], solid: ['M20.5 3V9.6H13.9Z', 'M3.5 21V14.4H10.1Z'] },
  timer: { group: 'States', solid: [circ(12, 13.5, 8)], bold: [{ d: 'M9.5 3H14.5', w: 2.4 }], knock: [{ d: 'M12 13.5V9.5', w: 2.2 }] },
  'clock-counter-clockwise': { group: 'States', round: true, bold: [{ d: 'M5.3 8.5A8 8 0 1 1 4.5 15', w: 2.6 }, { d: 'M12 7.8V12.5L15 14.5', w: 2.2 }], solid: ['M2.5 4.5V10.5H8.5Z'] },
  lightning: { group: 'States', solid: [BOLT], top: [] },
  sparkle: { group: 'States', solid: ['M12 2.5C12.9 8 16 11.1 21.5 12C16 12.9 12.9 16 12 21.5C11.1 16 8 12.9 2.5 12C8 11.1 11.1 8 12 2.5Z'] },
  broadcast: { group: 'States', round: true, solid: [circ(12, 12, 2.6)], bold: [{ d: 'M8.3 8.3A5.2 5.2 0 0 0 8.3 15.7', w: 2.4 }, { d: 'M15.7 8.3A5.2 5.2 0 0 1 15.7 15.7', w: 2.4 }, { d: 'M5.2 5.2A9.6 9.6 0 0 0 5.2 18.8', w: 2.4 }, { d: 'M18.8 5.2A9.6 9.6 0 0 1 18.8 18.8', w: 2.4 }] },
  'wifi-high': { group: 'States', bold: WIFI.map((d) => ({ d, w: 2.8 })), solid: [circ(12, 19.5, 1.7)] },
  'wifi-slash': { group: 'States', bold: WIFI.map((d) => ({ d, w: 2.8 })), solid: [circ(12, 19.5, 1.7)], knock: [{ d: 'M4 4L20 20', w: 5 }], top: [{ d: 'M4 4L20 20', w: 2.6 }] },
  'globe-hemisphere-west': { group: 'States', round: true, solid: [circ(12, 12, 10)], knock: [{ d: 'M2.5 12H21.5', w: 1.6 }, { d: 'M12 2A14 14 0 0 1 12 22', w: 1.6 }, { d: 'M12 2A14 14 0 0 0 12 22', w: 1.6 }] },

  // Files
  file: { group: 'Files', solid: [PAGE], knock: [{ d: FOLD, w: 1.6 }] },
  files: { group: 'Files', solid: ['M8.5 2H15L20 7V17A1.5 1.5 0 0 1 18.5 18.5H8.5A1.5 1.5 0 0 1 7 17V3.5A1.5 1.5 0 0 1 8.5 2Z', 'M3 7.5A1.5 1.5 0 0 1 4.5 6H5V20.2H16V20.5A1.5 1.5 0 0 1 14.5 22H4.5A1.5 1.5 0 0 1 3 20.5Z'], knock: [{ d: 'M14.6 2.1V7.4H19.9', w: 1.6 }] },
  image: { group: 'Files', solid: [rr(2.5, 4, 19, 16, 2.5)], knock: [{ d: 'M5 17L9.5 12L13 15.5L15.5 13L19.5 17', w: 2 }, { d: circ(15.5, 8.6, .9), w: 2.2 }] },
  'film-strip': { group: 'Files', solid: [rr(3, 3, 18, 18, 2.5)], knock: [{ d: 'M7.5 3V21', w: 1.8 }, { d: 'M16.5 3V21', w: 1.8 }, ...[7, 12, 17].flatMap((y) => [{ d: dot(5.25, y), w: 2 }, { d: dot(18.75, y), w: 2 }])] },
  'music-note': { group: 'Files', solid: [circ(7.8, 17.5, 3.6), 'M11.4 3.2L18.2 5.4A1 1 0 0 1 18.9 6.4V9.3A1 1 0 0 1 17.6 10.3L11.4 8.4Z'], bold: [{ d: 'M10 17V4', w: 2.8 }] },
  'file-zip': { group: 'Files', solid: [PAGE], knock: [{ d: FOLD, w: 1.6 }, { d: 'M10 3.5V4.6', w: 1.6 }, { d: 'M12 5.6V6.7', w: 1.6 }, { d: 'M10 7.7V8.8', w: 1.6 }, { d: 'M12 9.8V10.9', w: 1.6 }, { d: rr(9.6, 13, 2.8, 3.4, .6), w: 1.5 }] },
  'file-text': { group: 'Files', solid: [PAGE], knock: [{ d: FOLD, w: 1.6 }, { d: 'M8.5 12.5H15.5', w: 1.8 }, { d: 'M8.5 16H15.5', w: 1.8 }, { d: 'M8.5 9H11', w: 1.8 }] },
  'folder-simple': { group: 'Files', solid: ['M3 6A2 2 0 0 1 5 4H9.6L11.8 6.5H19A2 2 0 0 1 21 8.5V18A2 2 0 0 1 19 20H5A2 2 0 0 1 3 18Z'] },
  'folder-open': { group: 'Files', solid: ['M3 6A2 2 0 0 1 5 4H9.5L11.5 6.5H18A2 2 0 0 1 20 8.5V10H7.2A2 2 0 0 0 5.3 11.4L3 18Z', 'M7 11.8H21A1 1 0 0 1 21.95 13.1L19.7 19.6A2 2 0 0 1 17.8 21H3.4L6.1 12.5A1 1 0 0 1 7 11.8Z'], knock: [{ d: 'M6.6 10.9H21', w: 1.4 }] },
  copy: { group: 'Files', solid: ['M5 3H14A2 2 0 0 1 16 5V6.8H8.8A2 2 0 0 0 6.8 8.8V16H5A2 2 0 0 1 3 14V5A2 2 0 0 1 5 3Z', rr(8.5, 8.5, 12.5, 12.5, 2)] },
  'clipboard-text': { group: 'Files', solid: [rr(4.5, 4, 15, 18, 2.5)], knock: [{ d: rr(8, 1.6, 8, 4.8, 1.8), w: 2.2 }, { d: 'M8 11H16', w: 1.8 }, { d: 'M8 14.5H16', w: 1.8 }, { d: 'M8 18H13', w: 1.8 }], top: [{ fill: rr(8.6, 2.2, 6.8, 3.6, 1.2) }] },
  tray: { group: 'Files', solid: ['M3 13.5L5.4 5.5A2 2 0 0 1 7.3 4H16.7A2 2 0 0 1 18.6 5.5L21 13.5V18A2 2 0 0 1 19 20H5A2 2 0 0 1 3 18Z'], knock: [{ d: 'M3.5 13.5H8L9.5 16H14.5L16 13.5H20.5', w: 1.8 }] },
  trash: { group: 'Files', solid: ['M5.8 8.5H18.2L17.3 19.6A2 2 0 0 1 15.3 21.5H8.7A2 2 0 0 1 6.7 19.6Z'], bold: [{ d: 'M4 6H20', w: 2.6 }, { d: 'M9.5 5.5V4.2A1.2 1.2 0 0 1 10.7 3H13.3A1.2 1.2 0 0 1 14.5 4.2V5.5', w: 2 }], knock: [{ d: 'M10 12V17.5', w: 1.8 }, { d: 'M14 12V17.5', w: 1.8 }] },

  // App and settings
  'gear-six': { group: 'App and settings', round: true, solid: [gear(12, 12, 9, 7, 6) + circ(12, 12, 3.2)] },
  'caret-left': { group: 'App and settings', bold: [{ d: 'M15 5L8 12L15 19' }] },
  'caret-right': { group: 'App and settings', bold: [{ d: 'M9 5L16 12L9 19' }] },
  plus: { group: 'App and settings', bold: [{ d: 'M12 4.5V19.5' }, { d: 'M4.5 12H19.5' }] },
  'arrow-square-out': { group: 'App and settings', bold: [{ d: 'M18 13V18A2.5 2.5 0 0 1 15.5 20.5H6A2.5 2.5 0 0 1 3.5 18V8.5A2.5 2.5 0 0 1 6 6H11', w: 2.6 }, { d: 'M11 13L18.5 5.5', w: 2.6 }], solid: ['M13.5 3.5H20.5V10.5Z'] },
  'magnifying-glass': { group: 'App and settings', bold: [{ d: circ(10.5, 10.5, 6.5) }, { d: 'M15.5 15.5L20.5 20.5', w: 3.4 }] },
  bell: { group: 'App and settings', solid: ['M12 2A1.5 1.5 0 0 1 13.5 3.5V4.3A6 6 0 0 1 18 10.1V14L19.8 16.4A1 1 0 0 1 19 18H5A1 1 0 0 1 4.2 16.4L6 14V10.1A6 6 0 0 1 10.5 4.3V3.5A1.5 1.5 0 0 1 12 2Z', 'M9.6 19.3H14.4A2.4 2.4 0 0 1 9.6 19.3Z'] },
  sun: { group: 'App and settings', round: true, solid: [circ(12, 12, 4.6)], bold: sunRays.map((d) => ({ d, w: 2.4 })) },
  moon: { group: 'App and settings', solid: ['M20.5 14.5A8.5 8.5 0 1 1 9.5 3.5A7 7 0 0 0 20.5 14.5Z'] },
  'circle-half': { group: 'App and settings', round: true, bold: [{ d: circ(12, 12, 9), w: 2.4 }], solid: ['M12 3A9 9 0 0 0 12 21Z'] },
  'speaker-high': { group: 'App and settings', solid: [SPEAKER], bold: [{ d: 'M16 9A4.2 4.2 0 0 1 16 15', w: 2.2 }, { d: 'M18.6 7A7.5 7.5 0 0 1 18.6 17', w: 2.2 }] },
  'speaker-low': { group: 'App and settings', solid: [SPEAKER], bold: [{ d: 'M16 9A4.2 4.2 0 0 1 16 15', w: 2.2 }] },
  'speaker-slash': { group: 'App and settings', solid: [SPEAKER], bold: [{ d: 'M16.5 9.5L21.5 14.5', w: 2.2 }, { d: 'M21.5 9.5L16.5 14.5', w: 2.2 }] },
  eye: { group: 'App and settings', solid: [EYE], knock: [{ d: circ(12, 12, 3.6), w: 2.2 }] },
  'eye-slash': { group: 'App and settings', solid: [EYE], knock: [{ d: circ(12, 12, 3.6), w: 2.2 }, { d: 'M4 4L20 20', w: 5 }], top: [{ d: 'M4 4L20 20', w: 2.6 }] },
  'text-aa': { group: 'App and settings', bold: [{ d: 'M3.5 19L9 5L14.5 19', w: 2.8 }, { d: 'M5.8 14H12.2', w: 2.4 }, { d: circ(17.8, 16.2, 2.6), w: 2 }, { d: 'M20.6 13V19', w: 2 }] },
  'hand-swipe-right': { group: 'App and settings', bold: [{ d: 'M3.5 6H14', w: 2.6 }], solid: ['M14 2.8L18.5 6L14 9.2Z', 'M9 21V15.8L7.4 14.1A1.4 1.4 0 0 1 9.4 12.1L10.5 13.1V10A1.5 1.5 0 0 1 13.5 10V13.2H15.8A3.2 3.2 0 0 1 19 16.4V18A3 3 0 0 1 16 21Z'] },
  flask: { group: 'App and settings', solid: ['M9 2.5H15V4.3H14.2V8.9L19.3 18A2 2 0 0 1 17.6 21H6.4A2 2 0 0 1 4.7 18L9.8 8.9V4.3H9Z'], knock: [{ d: 'M7.2 15H16.8', w: 1.6 }] },
  'wave-sine': { group: 'App and settings', bold: [{ d: 'M3 12C5.2 6 7.6 6 9.8 12S14.4 18 16.6 12S19.4 7.5 21 8.5', w: 2.8 }] },
};

export const NAMES = Object.keys(SET);

/** Offset color for where the icon sits. */
export const OFFSET = { navy: '#1A6FE0', deep: '#0B3E85' };

let uid = 0;
/** SVG for one icon. `deep` uses the deeper navy offset (blue surfaces, accent and active icons). */
export function iconB(name, size = 24, { deep = false, offset, label } = {}) {
  const d = SET[name];
  if (!d) throw new Error('No icon ' + name);
  const id = `nb${++uid}`;
  const knocks = (d.knock ?? []).map((k) => (typeof k === 'string' ? { d: k, w: 2.2 } : { w: 2.2, ...k }));
  const mask = knocks.length
    ? `<mask id="${id}" maskUnits="userSpaceOnUse" x="-2" y="-2" width="28" height="28"><rect x="-2" y="-2" width="28" height="28" fill="#fff"/>${knocks.map((k) => `<path d="${k.d}" fill="none" stroke="#000" stroke-width="${k.w}" stroke-linecap="round" stroke-linejoin="round"/>`).join('')}</mask>` : '';
  const m = knocks.length ? ` mask="url(#${id})"` : '';
  const body = (color) => {
    const solid = (d.solid ?? []).map((p) => `<path d="${p}" fill="${color}" fill-rule="evenodd"/>`).join('');
    const bold = (d.bold ?? []).map((b) => `<path d="${b.d}" fill="none" stroke="${color}" stroke-width="${b.w ?? 3}" stroke-linecap="round" stroke-linejoin="round"/>`).join('');
    const top = (d.top ?? []).map((t) => t.fill ? `<path d="${t.fill}" fill="${color}"/>` : `<path d="${t.d}" fill="none" stroke="${color}" stroke-width="${t.w ?? 3}" stroke-linecap="round"/>`).join('');
    // The bolt gets a thin same-color stroke so its corners round like the app's.
    const join = name === 'lightning' ? `<path d="${BOLT}" fill="none" stroke="${color}" stroke-width=".9" stroke-linejoin="round"/>` : '';
    return `<g${m}>${solid}${bold}${join}</g>${top}`;
  };
  const shift = +(24 / size).toFixed(4);
  const off = offset ?? (deep ? OFFSET.deep : OFFSET.navy);
  const shadow = size >= 20 ? `<g transform="translate(${shift} ${shift})" data-offset>${body(off)}</g>` : '';
  const a11y = label ? `role="img" aria-label="${label}"` : 'aria-hidden="true"';
  return `<svg ${a11y} width="${size}" height="${size}" viewBox="0 0 24 24" data-icon="${name}" data-style="B" data-size="${size}"${d.round ? ' data-round' : ''} style="display:block;flex:none;overflow:visible"><defs>${mask}</defs>${shadow}${body('currentColor')}</svg>`;
}
