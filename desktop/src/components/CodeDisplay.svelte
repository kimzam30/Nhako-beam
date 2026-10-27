<script lang="ts">
  // Shows this device's pairing code: QR (rounded dots, no images), the words and a
  // countdown ring around the QR that drains over the code's 10 minute life.
  import { onDestroy } from 'svelte';
  import type { PairingCode } from '../lib/engine/types';
  import { t } from '../lib/copy';
  import { app } from '../lib/state/app.svelte';
  import { springIn, quickOut, stagger } from '../lib/motion/transitions';
  import Button from './Button.svelte';
  import Icon from './Icon.svelte';
  import QrCode from './QrCode.svelte';

  let { code }: { code: PairingCode } = $props();

  const LIFETIME = 10 * 60 * 1000;
  let now = $state(Date.now());
  // Ticks until the code runs out, then stops (a new code restarts it).
  $effect(() => {
    const until = code.expiresAt;
    const id = setInterval(() => {
      now = Date.now();
      if (now >= until) clearInterval(id);
    }, 250);
    return () => clearInterval(id);
  });

  const left = $derived(Math.max(0, code.expiresAt - now));
  const expired = $derived(left <= 0);
  const warn = $derived(left > 0 && left <= 20_000);
  const fraction = $derived(Math.min(1, left / LIFETIME));
  const timeText = $derived(left >= 60_000 ? `${Math.ceil(left / 60_000)} min` : `${Math.ceil(left / 1000)}s`);

  // Ring geometry: a rounded square path around the QR.
  const S = 232;
  const R = 28;
  const perimeter = 4 * (S - 2 * R) + 2 * Math.PI * R;

  let copied = $state(false);
  async function copy() {
    if (!(await app.copyText(code.words.join(' '), t('code.show.copied')))) return;
    copied = true;
    clearTimeout(copiedTimer);
    copiedTimer = setTimeout(() => (copied = false), 1600);
  }
  let copiedTimer: ReturnType<typeof setTimeout> | undefined;
  onDestroy(() => clearTimeout(copiedTimer));
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
    <QrCode value={code.uri} label="QR code for pairing" faded={expired} />
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
