<script lang="ts">
  // A QR code with rounded dots and no images, drawn in from the top left corner (in a few
  // diagonal bands, a handful of DOM nodes instead of hundreds). Pairing codes and shared links.
  import { encode } from 'uqr';
  import { springIn } from '../lib/motion/transitions';

  let { value, label, size = 204, faded = false }: { value: string; label: string; size?: number; faded?: boolean } = $props();

  const qr = $derived(encode(value, { ecc: 'M', border: 0 }));
  const n = $derived(qr.size);
  const isFinder = (x: number, y: number, size: number) => (x < 7 && y < 7) || (x >= size - 7 && y < 7) || (x < 7 && y >= size - 7);
  const BANDS = 7;
  const dotBands = $derived.by(() => {
    const r = 0.42;
    const bands = Array.from({ length: BANDS }, () => '');
    qr.data.forEach((row, y) =>
      row.forEach((on, x) => {
        if (!on || isFinder(x, y, n)) return;
        const b = Math.min(BANDS - 1, Math.floor(((x + y) / (2 * n)) * BANDS));
        bands[b] += `M${x + 0.5 - r} ${y + 0.5}a${r} ${r} 0 1 0 ${2 * r} 0a${r} ${r} 0 1 0 ${-2 * r} 0`;
      }),
    );
    return bands;
  });
  const finders = $derived([
    [0, 0],
    [n - 7, 0],
    [0, n - 7],
  ]);
</script>

<svg class="qr" class:faded viewBox="-1 -1 {n + 2} {n + 2}" role="img" aria-label={label} style:width="{size}px" style:height="{size}px" in:springIn={{ y: 0, preset: 'smooth' }}>
  {#each finders as [fx, fy], i}
    <g class="draw" style:animation-delay="{[0, 2, 4][i]! * 45}ms">
      <rect x={fx! + 0.5} y={fy! + 0.5} width="6" height="6" rx="1.8" class="finder-outer" />
      <rect x={fx! + 2} y={fy! + 2} width="3" height="3" rx="0.9" class="finder-inner" />
    </g>
  {/each}
  {#each dotBands as d, i}
    <path class="draw" {d} style:animation-delay="{40 + i * 45}ms" />
  {/each}
</svg>

<style>
  /* Always dark modules on white, in both themes: inverted codes scan badly on many cameras. */
  .qr {
    padding: 10px;
    border-radius: var(--radius-lg);
    background: #ffffff;
    fill: #121418;
    box-shadow: var(--elevation-1);
    transition: opacity var(--duration-base) var(--ease-standard);
  }
  .faded {
    opacity: 0.12;
  }
  .draw {
    transform-box: fill-box;
    transform-origin: center;
    animation: draw-in var(--spring-smooth-duration) var(--spring-smooth) both;
  }
  @keyframes draw-in {
    from {
      opacity: 0;
      transform: scale(0.82);
    }
  }
  :global([data-reduced='true']) .draw {
    animation: none;
  }
  .finder-outer {
    fill: none;
    stroke: #121418;
    stroke-width: 1;
  }
  .finder-inner {
    fill: #1a6fe0;
  }
</style>
