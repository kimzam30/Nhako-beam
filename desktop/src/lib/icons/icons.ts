// Phosphor icons (MIT). Only the icons the app uses are bundled, in three weights:
// duotone (default), fill (active state) and bold (16px and smaller).
const duotone = import.meta.glob(
  '/node_modules/@phosphor-icons/core/assets/duotone/{paper-plane-tilt,laptop,desktop-tower,device-mobile,device-tablet,check,check-circle,x,x-circle,pause,play,arrow-clockwise,qr-code,scan,copy,folder-open,file,files,image,film-strip,music-note,file-zip,file-text,gear-six,caret-left,caret-right,shield-check,wifi-high,wifi-slash,lightning,share-network,warning-circle,tray,info,trash,plus,download-simple,broadcast,hand-swipe-right,lock-simple,sun,moon,circle-half,wave-sine,bell,flask,hard-drives,keyboard,sparkle,arrows-clockwise,fingerprint-simple,timer,upload-simple,arrow-square-out,globe-hemisphere-west,eye,eye-slash,clock-counter-clockwise,magnifying-glass,speaker-high,speaker-low,speaker-slash,folder-simple,text-aa,link-simple,clipboard-text,handshake,cell-signal-full}-duotone.svg',
  { query: '?raw', import: 'default', eager: true },
) as Record<string, string>;

const fill = import.meta.glob(
  '/node_modules/@phosphor-icons/core/assets/fill/{paper-plane-tilt,laptop,desktop-tower,device-mobile,device-tablet,check-circle,x-circle,pause,play,shield-check,download-simple,gear-six,broadcast,qr-code,lightning,wifi-high,share-network,sparkle,bell,folder-open,globe-hemisphere-west,hard-drives,warning-circle,folder-simple,handshake,text-aa,link-simple}-fill.svg',
  { query: '?raw', import: 'default', eager: true },
) as Record<string, string>;

const bold = import.meta.glob(
  '/node_modules/@phosphor-icons/core/assets/bold/{check,x,caret-left,caret-right,plus,shield-check,lightning,wifi-high,share-network,warning-circle,lock-simple}-bold.svg',
  { query: '?raw', import: 'default', eager: true },
) as Record<string, string>;

export type IconWeight = 'duotone' | 'fill' | 'bold';

function index(map: Record<string, string>, suffix: string): Map<string, string> {
  const out = new Map<string, string>();
  for (const [path, svg] of Object.entries(map)) {
    const name = path.split('/').pop()!.replace(`-${suffix}.svg`, '');
    // Keep only the inner shapes; the component draws its own <svg>.
    out.set(name, svg.replace(/^<svg[^>]*>/, '').replace(/<\/svg>\s*$/, ''));
  }
  return out;
}

const sets: Record<IconWeight, Map<string, string>> = {
  duotone: index(duotone, 'duotone'),
  fill: index(fill, 'fill'),
  bold: index(bold, 'bold'),
};

/** Inner SVG markup for an icon, falling back to duotone if a weight is not bundled. */
export function iconMarkup(name: string, weight: IconWeight): string {
  return sets[weight].get(name) ?? sets.duotone.get(name) ?? '';
}
