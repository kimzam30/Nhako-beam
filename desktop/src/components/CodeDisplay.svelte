<script lang="ts">
  // Shows this device's pairing code: QR (rounded dots, no images), three words and a
  // countdown ring around the QR that drains over the code's 10 minute life.
  import { encode } from 'uqr';
  import type { PairingCode } from '../lib/engine/types';
  import { t } from '../lib/copy';
  import { app } from '../lib/state/app.svelte';
  import { springIn, quickOut, stagger } from '../lib/motion/transitions';
  import Button from './Button.svelte';
  import Icon from './Icon.svelte';

  let { code }: { code: PairingCode } = $props();

  const LIFETIME = 10 * 60 * 1000;
  let now = $state(Date.now());
  $effect(() => {
    const id = setInterval(() => (now = Date.now()), 250);
    return () => clearInterval(id);
  });

  const left = $derived(Math.max(0, code.expiresAt - now));
  const expired = $derived(left <= 0);
  const warn = $derived(left > 0 && left <= 20_000);
  const fraction = $derived(Math.min(1, left / LIFETIME));
  const timeText = $derived(left >= 60_000 ? `${Math.ceil(left / 60_000)} min` : `${Math.ceil(left / 1000)}s`);

  const qr = $derived(encode(code.uri, { ecc: 'M', border: 0 }));
  const n = $derived(qr.size);
  const isFinder = (x: number, y: number, size: number) =>
    (x < 7 && y < 7) || (x >= size - 7 && y < 7) || (x < 7 && y >= size - 7);
  // The data modules as round dots, in a few diagonal bands (a handful of DOM nodes instead of
  // hundreds), so the code can draw itself in from the top left corner.
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

  // Ring geometry: a rounded square path around the QR.
  const S = 232;
  const R = 28;
  const perimeter = 4 * (S - 2 * R) + 2 * Math.PI * R;

  let copied = $state(false);
  async function copy() {
    try {
      await navigator.clipboard.writeText(code.words.join(' '));
    } catch {
      // Clipboard can be blocked; the toast still confirms the intent in the prototype.
    }
    copied = true;
    app.toast(t('code.show.copied'), 'success');
    setTimeout(() => (copied = false), 1600);
  }
</script>

<div class="code" class:expired data-testid="code-display">
  <div class="qrbox">
    <svg class="ring" viewBox="0 0 {S + 8} {S + 8}" aria-hidden="true">
      <rect x="4" y="4" width={S} height={S} rx={R} class="ring-track" />
      <rect
        x="4"
        y="4"
        width={S}
        height={S}
        rx={R}
        class="ring-left"
        class:warn
        stroke-dasharray={perimeter}
        stroke-dashoffset={perimeter * (1 - fraction)}
      />
    </svg>
    <svg class="qr" viewBox="-1 -1 {n + 2} {n + 2}" role="img" aria-label="QR code for pairing" in:springIn={{ y: 0, preset: 'smooth' }}>
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
    {#if expired}
      <div class="veil" in:springIn={{ y: 0, scale: 0.96 }}>
        <p>{t('code.show.expired')}</p>
        <Button size="sm" icon="arrows-clockwise" testid="new-code" onclick={() => app.newCode()}>{t('code.show.new')}</Button>
      </div>
    {/if}
  </div>

  <p class="words" aria-label="Code: {code.words.join(', ')}" data-testid="code-words">
    {#each code.words as w, i (w + i)}
      <span class="word" in:springIn={{ delay: 60 + stagger(i, 60), preset: 'snappy', y: 8 }} out:quickOut>{w}</span>
    {/each}
  </p>

  <div class="row">
    <span class="timer tabular" class:warn aria-live="off" data-testid="code-timer">
      <Icon name="timer" size={16} weight="bold" />
      {expired ? t('code.show.expired') : t('code.show.expiresIn', { time: timeText })}
    </span>
    <Button variant="secondary" size="sm" icon={copied ? 'check' : 'copy'} testid="copy-code" onclick={copy} disabled={expired}>
      {copied ? t('code.show.copied') : t('code.show.copy')}
    </Button>
  </div>
</div>

<style>
  .code {
    display: grid;
    justify-items: center;
    gap: var(--space-5);
  }
  .qrbox {
    position: relative;
    width: 240px;
    height: 240px;
    display: grid;
    place-items: center;
  }
  .ring {
    position: absolute;
    inset: 0;
    overflow: visible;
  }
  .ring rect {
    fill: none;
    stroke-width: 3;
  }
  .ring-track {
    stroke: var(--color-fill);
  }
  .ring-left {
    stroke: var(--color-accent);
    stroke-linecap: round;
    transition:
      stroke-dashoffset 250ms linear,
      stroke var(--duration-base) var(--ease-standard);
  }
  .ring-left.warn {
    stroke: var(--color-warning);
  }
  /* Always dark modules on white, in both themes: inverted codes scan badly on many cameras. */
  .qr {
    width: 204px;
    height: 204px;
    padding: 10px;
    border-radius: var(--radius-lg);
    background: #ffffff;
    fill: #121418;
    box-shadow: var(--elevation-1);
    transition: opacity var(--duration-base) var(--ease-standard);
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
  .expired .qr {
    opacity: 0.12;
  }
  .veil {
    position: absolute;
    inset: 0;
    display: grid;
    place-content: center;
    justify-items: center;
    gap: var(--space-3);
    text-align: center;
    font-weight: 600;
  }
  .words {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: var(--space-2) var(--space-4);
    font-size: var(--type-code-words-size);
    line-height: var(--type-code-words-line);
    font-weight: var(--type-code-words-weight);
    letter-spacing: var(--type-code-words-tracking);
  }
  .word {
    display: inline-block;
  }
  .expired .words {
    opacity: 0.35;
    text-decoration: line-through;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    width: 100%;
  }
  .timer {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--type-footnote-size);
    color: var(--color-text-secondary);
    transition: color var(--duration-base) var(--ease-standard);
  }
  .timer.warn {
    color: var(--color-warning);
  }
</style>
