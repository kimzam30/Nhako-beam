<script lang="ts">
  // The voxel mascot on its own (the splash, empty History, pairing). The 3D stage loads after
  // the first paint; until then, and where WebGL isn't there, the flat bolt stands in.
  // motion-spec.md section 4.18.
  import { onMount } from 'svelte';
  import { prefs } from '../lib/state/prefs.svelte';
  import type { MascotState } from '../lib/stage/pose';
  import type { Stage } from '../lib/stage/scene';
  import Bolt from './Bolt.svelte';

  interface Props {
    mood: MascotState;
    /** Pixels. */
    height: number;
    width?: number;
    label: string;
    /** Two bolts meeting (pairing). */
    pair?: boolean;
    /** Drop in and land (the launch intro). */
    intro?: boolean;
    testid?: string;
    /** The 3D bolt is drawn (it isn't, without WebGL). */
    onready?: () => void;
  }

  let { mood, height, width = height, label, pair = false, intro = false, testid, onready }: Props = $props();

  let canvas: HTMLCanvasElement | undefined = $state();
  let stage = $state<Stage | null>(null);

  onMount(() => {
    let gone = false;
    let made: Stage | null = null;
    void import('../lib/stage/scene')
      .then((m) =>
        m.createStage(canvas!, {
          camera: { eye: [0, 2.6, pair ? 10.5 : 8.6], target: [0, 1.45, 0], fovDeg: 30 },
          home: false,
          mascotScale: 1,
          pair,
          intro,
          reduced: prefs.reducedMotion,
        }),
      )
      .then((s) => {
        if (gone) return s.destroy();
        made = s;
        s.resize(width, height);
        s.setMood(mood);
        stage = s;
        onready?.();
      })
      .catch((e) => console.warn('The 3D mascot is not available:', e));
    return () => {
      gone = true;
      made?.destroy();
    };
  });

  $effect(() => stage?.setMood(mood));
  $effect(() => stage?.setReduced(prefs.reducedMotion));
  $effect(() => stage?.resize(width, height));
  $effect(() => {
    void prefs.dark;
    stage?.retheme();
  });

  const flat = $derived(mood === 'asleep' ? 'asleep' : mood === 'failed' ? 'cracked' : mood === 'done' ? 'check' : 'bolt');
</script>

<span class="mascot" style:width="{width}px" style:height="{height}px" role="img" aria-label={label} data-testid={testid} data-state={mood}>
  <canvas bind:this={canvas} class:ready={!!stage} style:width="{width}px" style:height="{height}px"></canvas>
  {#if !stage}
    <span class="flat"><Bolt size={Math.round(height * 0.42)} mode={flat} /></span>
  {/if}
</span>

<style>
  .mascot {
    position: relative;
    display: inline-grid;
    place-items: center;
    flex: none;
  }
  canvas {
    position: absolute;
    inset: 0;
    opacity: 0;
    transition: opacity var(--duration-base) var(--ease-standard);
  }
  canvas.ready {
    opacity: 1;
  }
  .flat {
    display: grid;
    place-items: center;
  }
</style>
