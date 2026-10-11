<script lang="ts">
  // The transfer beam. Fill follows real progress on the smooth spring (retargeted on each
  // update), glow follows throughput, and a solid highlight travels along the filled part
  // at a speed tied to throughput. Everything animates transform and opacity only.
  import { SpringValue } from '../lib/motion/springs';
  import { prefs } from '../lib/state/prefs.svelte';

  type Mode = 'moving' | 'idle' | 'paused' | 'reconnecting' | 'done' | 'failed';

  interface Props {
    progress: number;
    /** 0..1 how fast relative to a fast nearby transfer; drives glow and shimmer speed. */
    intensity?: number;
    mode: Mode;
    collapsed?: boolean;
  }

  let { progress, intensity = 0, mode, collapsed = false }: Props = $props();

  let fill: HTMLElement | undefined = $state();
  let glow: HTMLElement | undefined = $state();
  let wrap: HTMLElement | undefined = $state();

  const p = new SpringValue(0, (v) => {
    const c = Math.max(0, Math.min(1, v));
    if (fill) fill.style.transform = `translateX(${(c - 1) * 100}%)`;
    if (glow) glow.style.transform = `scaleX(${c})`;
  });
  const shrink = new SpringValue(1, (v) => {
    if (wrap) wrap.style.transform = `scaleX(${v})`;
    if (wrap) wrap.style.opacity = String(Math.max(0, Math.min(1, v * 1.4)));
  });

  let first = true;
  $effect(() => {
    const target = Math.max(0, Math.min(1, progress));
    if (!fill) return;
    if (first) {
      first = false;
      p.set(target);
      return;
    }
    if (prefs.reducedMotion) p.set(target);
    else p.to(target, 'smooth');
  });

  $effect(() => {
    if (!wrap) return;
    if (collapsed) shrink.to(0, 'smooth');
    else shrink.set(1);
  });

  const shimmerMs = $derived(Math.round(2600 - 1700 * Math.min(1, intensity)));
  const glowOpacity = $derived(mode === 'moving' ? 0.35 + 0.65 * Math.min(1, intensity) : 0);
</script>

<div class="beam {mode}" bind:this={wrap} role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={Math.floor(progress * 100)} data-testid="beam" data-mode={mode}>
  <div class="glow" bind:this={glow} style:opacity={glowOpacity} aria-hidden="true"></div>
  <div class="track">
    <div class="fill" bind:this={fill}>
      {#if mode === 'moving' && !prefs.reducedMotion}
        <span class="shimmer" style:animation-duration="{shimmerMs}ms"></span>
      {/if}
    </div>
  </div>
</div>

<style>
  .beam {
    position: relative;
    height: 8px;
    transform-origin: left center;
  }
  .track {
    position: relative;
    height: 100%;
    border-radius: var(--radius-full);
    background: var(--color-beam-track);
    overflow: hidden;
    /* Keeps the rounded clip on the GPU in WebKit. */
    isolation: isolate;
  }
  .fill {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: var(--color-beam-core);
    transform: translateX(-100%);
    will-change: transform;
    transition: background-color var(--duration-base) var(--ease-standard);
  }
  .glow {
    position: absolute;
    inset: 0;
    border-radius: var(--radius-full);
    box-shadow: 0 0 14px 3px var(--color-beam-glow);
    transform-origin: left center;
    transform: scaleX(0);
    transition: opacity var(--duration-slow) var(--ease-standard);
    pointer-events: none;
  }
  .shimmer {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    width: 22%;
    border-radius: inherit;
    background: #ffffff;
    opacity: 0.32;
    animation: travel 1.4s linear infinite;
  }
  @keyframes travel {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(460%);
    }
  }
  .paused .fill {
    background: var(--color-text-tertiary);
  }
  .reconnecting .fill {
    animation: breathe var(--duration-searching-pulse) var(--ease-standard) infinite;
  }
  .failed .fill {
    background: var(--color-danger);
  }
  .done .fill {
    background: var(--color-success);
  }
  @keyframes breathe {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.45;
    }
  }
  :global([data-reduced='true']) .reconnecting .fill {
    animation: none;
    opacity: 0.7;
  }
</style>
