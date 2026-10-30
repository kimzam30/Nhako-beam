<script lang="ts">
  // The launch intro: the window opens brand blue (tauri.conf.json backgroundColor), the voxel
  // bolt drops onto its spot, lands with a squash and blinks, then the blue fades away into the
  // app. Until the 3D bolt is ready (or without WebGL) the flat bolt stands in, so it's never
  // empty. The Svelte twin of android/.../ui/components/SplashIntro.kt. Motion:
  // docs/design/motion-spec.md, section 4.14.
  import { onMount } from 'svelte';
  import { brand } from '../lib/tokens';
  import { prefs } from '../lib/state/prefs.svelte';
  import { playSound } from '../lib/sounds.svelte';
  import { easeExit } from '../lib/motion/transitions';
  import Mascot3D from './Mascot3D.svelte';

  let { ondone }: { ondone: () => void } = $props();

  let cover = $state(1);
  /** The bolt's own fall and landing take this long; then the blue fades. */
  const LAND_MS = 900;
  const FADE_MS = 240;
  /** Waits no longer than this for the 3D bolt before going on with the flat one. */
  const WAIT_MS = 450;

  let begin: (() => void) | null = null;
  onMount(() => {
    const reduced = prefs.reducedMotion;
    let start = 0;
    let frame = 0;
    let begun = false;
    function tick(now: number) {
      if (!start) start = now;
      const t = now - start;
      const hold = reduced ? 0 : LAND_MS;
      const fade = reduced ? 200 : FADE_MS;
      cover = 1 - easeExit(Math.min(1, Math.max(0, (t - hold) / fade)));
      if (t < hold + fade) frame = requestAnimationFrame(tick);
      else ondone();
    }
    begin = () => {
      if (begun) return;
      begun = true;
      // The landing click.
      setTimeout(() => playSound('intro'), reduced ? 0 : 420);
      frame = requestAnimationFrame(tick);
    };
    const fallback = setTimeout(() => begin?.(), WAIT_MS);
    return () => {
      clearTimeout(fallback);
      cancelAnimationFrame(frame);
    };
  });
</script>

<div class="intro" style:opacity={cover} style:--blue={brand.blue} aria-hidden="true">
  <Mascot3D mood="idle" height={260} width={260} label="Nhako Beam" intro onready={() => begin?.()} testid="intro-mascot" />
</div>

<style>
  /* Decorative and short: clicks and keys go straight through to the app. */
  .intro {
    position: fixed;
    inset: 0;
    z-index: calc(var(--z-tooltip) + 1);
    display: grid;
    place-items: center;
    background: var(--blue);
    pointer-events: none;
  }
</style>
