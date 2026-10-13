<script lang="ts">
  // A whole number whose digits roll up (or down) when they change, like a meter. Each place
  // keeps its slot, so with tabular figures nothing shifts sideways. Reduced motion: they swap.
  import type { TransitionConfig } from 'svelte/transition';
  import { prefs } from '../lib/state/prefs.svelte';
  import { springEasing } from '../lib/motion/physics';
  import { easeExit } from '../lib/motion/transitions';

  let { value, suffix = '' }: { value: number; suffix?: string } = $props();

  const whole = $derived(Math.round(value));
  const digits = $derived(String(whole).split(''));
  let dir = 1;
  let last: number | null = null;
  $effect.pre(() => {
    dir = last === null || whole >= last ? 1 : -1;
    last = whole;
  });

  function roll(_n: Element, { out }: { out: boolean }): TransitionConfig {
    if (prefs.reducedMotion) return { duration: 0 };
    const d = dir;
    if (out) return { duration: 150, easing: easeExit, css: (t, u) => `transform: translateY(${-d * 100 * u}%); opacity: ${t}` };
    const { duration, easing } = springEasing('snappy');
    return { duration, css: (t) => `transform: translateY(${d * 100 * (1 - easing(t))}%); opacity: ${Math.min(1, t * 2)}` };
  }
</script>

<span class="roll" aria-label="{whole}{suffix}" role="img">
  {#each digits as ch, i (digits.length - i)}
    <span class="cell" aria-hidden="true">
      {#key ch}<span class="d" in:roll={{ out: false }} out:roll={{ out: true }}>{ch}</span>{/key}
    </span>
  {/each}<span aria-hidden="true">{suffix}</span>
</span>

<style>
  .roll {
    display: inline-flex;
    font-variant-numeric: tabular-nums;
  }
  .cell {
    display: inline-grid;
    overflow: hidden;
  }
  .d {
    grid-area: 1 / 1;
  }
</style>
