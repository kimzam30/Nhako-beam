<script lang="ts">
  // The logo bolt as a living icon. `mode`: bolt, check (the bolt morphs into a check), cracked
  // (it splits along its zig and dims), asleep (dim, breathes once when it appears).
  // `strike` drops it in with sparks on mount; bumping `twitch` gives it one small shake.
  // Twin: android/.../ui/components/Bolt.kt. Spec: docs/design/motion-spec.md, 4.15.
  import { onMount, untrack } from 'svelte';
  import { brand } from '../lib/tokens';
  import { prefs } from '../lib/state/prefs.svelte';
  import { CRACK_LINE, shapePath, spark } from '../lib/motion/bolt';
  import { SpringValue, wait } from '../lib/motion/springs';

  interface Props {
    size?: number;
    mode?: 'bolt' | 'check' | 'cracked' | 'asleep';
    /** brand: yellow with the dark blue offset. current: one flat color from CSS `color`. */
    tone?: 'brand' | 'current';
    strike?: boolean;
    twitch?: number;
    testid?: string;
  }

  let { size = 24, mode = 'bolt', tone = 'brand', strike = false, twitch = 0, testid }: Props = $props();

  const uid = `b${Math.random().toString(36).slice(2, 8)}`;
  let svg: SVGSVGElement | undefined = $state();
  let morph = $state(0);
  let split = $state(0);
  const startAt = untrack(() => (strike && !prefs.reducedMotion ? 0 : 1));
  let drop = $state(startAt);
  let wiggle = $state(0);
  let mounted = $state(false);

  const m = new SpringValue(0, (v) => (morph = v));
  const s = new SpringValue(0, (v) => (split = v));
  const d = new SpringValue(startAt, (v) => (drop = v));
  const w = new SpringValue(0, (v) => (wiggle = v));

  // Morph and crack follow the mode, starting from wherever they are now.
  $effect(() => {
    const target = mode === 'check' ? 1 : 0;
    if (strike && !mounted) return; // the strike plays first, then morphs
    void m.to(target, 'snappy');
  });
  $effect(() => {
    void s.to(mode === 'cracked' ? 1 : 0, 'snappy');
  });

  let lastTwitch: number | null = null;
  $effect(() => {
    const n = twitch;
    const was = lastTwitch;
    lastTwitch = n;
    if (was === null || n === was || prefs.reducedMotion) return;
    void (async () => {
      await Promise.race([w.to(1, 'snappy'), wait(70)]);
      await Promise.race([w.to(-0.7, 'snappy'), wait(70)]);
      await w.to(0, 'snappy');
    })();
  });

  onMount(() => {
    if (!strike || prefs.reducedMotion) {
      mounted = true;
      return;
    }
    void (async () => {
      await d.to(1, 'bouncy');
      if (svg) spark(svg, { count: 4 });
      if (mode === 'check') await wait(120);
      mounted = true;
    })();
    return () => (d.stop(), m.stop(), s.stop(), w.stop());
  });

  const path = $derived(shapePath(morph));
  // A brand bolt turning into a check takes on the surrounding color (green when done).
  const fill = $derived(tone === 'brand' && morph < 0.35 ? brand.yellow : 'currentColor');
  // The offset shadow belongs to the bolt, so it fades as the bolt turns into a check.
  const shadowOpacity = $derived(tone === 'brand' && mode !== 'asleep' ? 1 - morph : 0);
  const [[ax, ay], [bx, by]] = CRACK_LINE;
  const transform = $derived(
    `translate(512 512) rotate(${wiggle * 9}) translate(0 ${(1 - drop) * -260}) scale(${0.55 + 0.45 * drop}) translate(-512 -512)`,
  );
</script>

<svg
  bind:this={svg}
  class="bolt"
  class:asleep={mode === 'asleep'}
  class:cracked={mode === 'cracked'}
  width={size}
  height={size}
  viewBox="120 120 784 784"
  aria-hidden="true"
  data-testid={testid}
  data-mode={mode}
  style:opacity={Math.min(1, drop * 1.6)}
>
  <defs>
    <clipPath id="{uid}-top"><path d="M0,0 L1024,0 L{bx},{by} L{ax},{ay} Z" /></clipPath>
    <clipPath id="{uid}-bottom"><path d="M{ax},{ay} L{bx},{by} L1024,1024 L0,1024 Z" /></clipPath>
  </defs>
  <g {transform}>
    {#if split > 0.002}
      <!-- Cracked: two halves, cut along the zig, pushed apart and turned a little. -->
      {#each ['top', 'bottom'] as half (half)}
        {@const k = half === 'top' ? -1 : 1}
        <g clip-path="url(#{uid}-{half})" transform="translate({k * 30 * split} {k * 44 * split}) rotate({k * 9 * split} 512 512)">
          {@render shape()}
        </g>
      {/each}
    {:else}
      {@render shape()}
    {/if}
  </g>
</svg>

{#snippet shape()}
  {#if shadowOpacity > 0}
    <path d={path} transform="translate(26 32)" fill={brand.navy} stroke={brand.navy} stroke-width="22" stroke-linejoin="round" opacity={shadowOpacity} />
  {/if}
  <path class="main" d={path} {fill} stroke={fill} stroke-width="22" stroke-linejoin="round" />
{/snippet}

<style>
  .bolt {
    display: block;
    overflow: visible;
    flex: none;
  }
  .main {
    transition:
      fill var(--duration-base) var(--ease-standard),
      stroke var(--duration-base) var(--ease-standard);
  }
  .cracked .main {
    opacity: 0.85;
  }
  .asleep .main {
    fill: var(--color-text-disabled);
    stroke: var(--color-text-disabled);
  }
  .asleep {
    animation: breathe-once 1.6s var(--ease-standard) 0.2s 1 both;
  }
  @keyframes breathe-once {
    0% {
      transform: scale(0.94);
      opacity: 0.4;
    }
    45% {
      transform: scale(1.06);
      opacity: 1;
    }
    100% {
      transform: scale(1);
      opacity: 0.8;
    }
  }
  :global([data-reduced='true']) .asleep {
    animation: none;
    opacity: 0.8;
  }
</style>
