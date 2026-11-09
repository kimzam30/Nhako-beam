// Sample icons for the style choice, drawn on a 24 grid with a 20 px live area (2 px padding).
// Three styles share one skeleton so only the treatment differs:
//   A "Beam duotone": outline plus a 20% fill of the same color.
//   B "Bolt solid":   solid shapes plus the poster's navy offset (shown from 20 px up).
//   C "Clean line":   outline only.
// Stroke is optically corrected per size, so it lands on whole device pixels:
//   16 px -> 1.5 px, 20 px -> 1.75 px, 24 px and up -> 2 px per 24 grid.

const circle = (cx, cy, r) => `M${cx - r} ${cy}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0Z`;

function gearPath(cx, cy, rOut, rIn, teeth) {
  const pts = [];
  const step = 360 / teeth;
  for (let i = 0; i < teeth; i++) {
    const a = i * step - 90;
    for (const [d, r] of [[-step * 0.36, rIn], [-step * 0.2, rOut], [step * 0.2, rOut], [step * 0.36, rIn]]) {
      const t = ((a + d) * Math.PI) / 180;
      pts.push(`${(cx + r * Math.cos(t)).toFixed(2)} ${(cy + r * Math.sin(t)).toFixed(2)}`);
    }
  }
  return 'M' + pts.join('L') + 'Z';
}
const GEAR = gearPath(12, 12, 9, 7, 6);

// line: stroked paths. fill: the 20% duotone back. solid: filled paths (evenodd). knock: strokes cut out of solid.
export const ICONS = {
  send: {
    label: 'Send',
    line: ['M20.5 3.5L3.5 10.8L10.6 13.4L13.2 20.5Z', 'M10.6 13.4L20.5 3.5'],
    fill: ['M20.5 3.5L3.5 10.8L10.6 13.4L13.2 20.5Z'],
    solid: ['M21 3L2.8 10.8L10.2 13.8L13.2 21.2Z'],
    knock: ['M10.2 13.8L18 6'],
  },
  receive: {
    label: 'Receive',
    line: ['M12 3.5V14', 'M7.5 9.5L12 14L16.5 9.5', 'M4 14.5V17.5A2 2 0 0 0 6 19.5H18A2 2 0 0 0 20 17.5V14.5'],
    fill: ['M4 14.5H20V17.5A2 2 0 0 1 18 19.5H6A2 2 0 0 1 4 17.5Z'],
    solid: ['M10.75 3H13.25V10.5H16.8L12 15.3L7.2 10.5H10.75Z',
      'M3 14H6V16.5H18V14H21V18A2.5 2.5 0 0 1 18.5 20.5H5.5A2.5 2.5 0 0 1 3 18Z'],
  },
  laptop: {
    label: 'Laptop',
    line: ['M5 6A1 1 0 0 1 6 5H18A1 1 0 0 1 19 6V15H5Z', 'M3 18H21'],
    fill: ['M5 6A1 1 0 0 1 6 5H18A1 1 0 0 1 19 6V15H5Z'],
    solid: ['M4 5.5A1.5 1.5 0 0 1 5.5 4H18.5A1.5 1.5 0 0 1 20 5.5V15.5H4ZM6 6V13.5H18V6Z',
      'M2 17H22V18A2 2 0 0 1 20 20H4A2 2 0 0 1 2 18Z'],
  },
  phone: {
    label: 'Phone',
    line: ['M9 3H15A2 2 0 0 1 17 5V19A2 2 0 0 1 15 21H9A2 2 0 0 1 7 19V5A2 2 0 0 1 9 3Z', 'M11 6H13'],
    fill: ['M9 3H15A2 2 0 0 1 17 5V19A2 2 0 0 1 15 21H9A2 2 0 0 1 7 19V5A2 2 0 0 1 9 3Z'],
    solid: ['M9 2H15A3 3 0 0 1 18 5V19A3 3 0 0 1 15 22H9A3 3 0 0 1 6 19V5A3 3 0 0 1 9 2ZM8 6.5V17.5H16V6.5Z'],
  },
  check: {
    label: 'Done',
    lineC: ['M4.5 12.5L9.5 17.5L19.5 6.5'],
    line: [circle(12, 12, 9), 'M8 12.3L10.8 15L16 9.3'],
    fill: [circle(12, 12, 9)],
    solid: [circle(12, 12, 10)],
    knock: ['M7.6 12.4L10.6 15.3L16.4 9.2'],
  },
  close: {
    label: 'Close',
    lineC: ['M6 6L18 18', 'M18 6L6 18'],
    line: [circle(12, 12, 9), 'M9 9L15 15', 'M15 9L9 15'],
    fill: [circle(12, 12, 9)],
    solid: [circle(12, 12, 10)],
    knock: ['M8.8 8.8L15.2 15.2', 'M15.2 8.8L8.8 15.2'],
  },
  pause: {
    label: 'Pause',
    line: ['M7 6A1 1 0 0 1 8 5H9.5A1 1 0 0 1 10.5 6V18A1 1 0 0 1 9.5 19H8A1 1 0 0 1 7 18Z',
      'M13.5 6A1 1 0 0 1 14.5 5H16A1 1 0 0 1 17 6V18A1 1 0 0 1 16 19H14.5A1 1 0 0 1 13.5 18Z'],
    fill: ['M7 6A1 1 0 0 1 8 5H9.5A1 1 0 0 1 10.5 6V18A1 1 0 0 1 9.5 19H8A1 1 0 0 1 7 18Z',
      'M13.5 6A1 1 0 0 1 14.5 5H16A1 1 0 0 1 17 6V18A1 1 0 0 1 16 19H14.5A1 1 0 0 1 13.5 18Z'],
    solid: ['M6 5.5A1.5 1.5 0 0 1 7.5 4H9.5A1.5 1.5 0 0 1 11 5.5V18.5A1.5 1.5 0 0 1 9.5 20H7.5A1.5 1.5 0 0 1 6 18.5Z',
      'M13 5.5A1.5 1.5 0 0 1 14.5 4H16.5A1.5 1.5 0 0 1 18 5.5V18.5A1.5 1.5 0 0 1 16.5 20H14.5A1.5 1.5 0 0 1 13 18.5Z'],
  },
  settings: {
    label: 'Settings',
    line: [GEAR, circle(12, 12, 3)],
    fill: [GEAR],
    solid: [GEAR + circle(12, 12, 3.2)],
  },
};

export const STYLES = {
  A: { name: 'Beam duotone', note: 'Outline plus a 20% fill. Closest to today, in our own shapes.' },
  B: { name: 'Bolt solid', note: 'Solid shapes with the poster bolt\'s navy offset. Loudest, most on-brand.' },
  C: { name: 'Clean line', note: 'Outline only. Quietest, most neutral.' },
};

/** Device-pixel stroke for an icon size, then converted into 24-grid units. */
export function strokeFor(size) {
  const device = size <= 16 ? 1.5 : size <= 20 ? 1.75 : 2 * (size / 24);
  return +(device * 24 / size).toFixed(3);
}

let uid = 0;
/** SVG markup for one icon. `offset` is the navy shadow color for style B. */
export function icon(name, style, size = 24, { offset = 'var(--brand-navy)', label } = {}) {
  const d = ICONS[name];
  const sw = strokeFor(size);
  const a11y = label ? `role="img" aria-label="${label}"` : 'aria-hidden="true"';
  const open = `<svg ${a11y} width="${size}" height="${size}" viewBox="0 0 24 24" data-icon="${name}" data-style="${style}" data-size="${size}" style="display:block;flex:none;overflow:visible">`;
  const strokes = (paths) => paths.map((p) => `<path d="${p}"/>`).join('');
  if (style === 'C') {
    return `${open}<g fill="none" stroke="currentColor" stroke-width="${sw}" stroke-linecap="round" stroke-linejoin="round">${strokes(d.lineC ?? d.line)}</g></svg>`;
  }
  if (style === 'A') {
    return `${open}<g fill="currentColor" opacity=".2">${strokes(d.fill)}</g><g fill="none" stroke="currentColor" stroke-width="${sw}" stroke-linecap="round" stroke-linejoin="round">${strokes(d.line)}</g></svg>`;
  }
  // B: solid with optional knock-out strokes, plus a navy offset copy behind it from 20 px up.
  const id = `nbk${++uid}`;
  const knockW = Math.max(sw, 2.2);
  const mask = d.knock
    ? `<mask id="${id}" maskUnits="userSpaceOnUse" x="-2" y="-2" width="28" height="28"><rect x="-2" y="-2" width="28" height="28" fill="#fff"/><g fill="none" stroke="#000" stroke-width="${knockW}" stroke-linecap="round" stroke-linejoin="round">${strokes(d.knock)}</g></mask>`
    : '';
  const m = d.knock ? ` mask="url(#${id})"` : '';
  const solids = d.solid.map((p) => `<path d="${p}" fill-rule="evenodd"/>`).join('');
  const shift = +(24 / size).toFixed(3); // exactly 1 device px
  const shadow = size >= 20 ? `<g fill="${offset}" transform="translate(${shift} ${shift})"${m}>${solids}</g>` : '';
  return `${open}<defs>${mask}</defs>${shadow}<g fill="currentColor"${m}>${solids}</g></svg>`;
}
