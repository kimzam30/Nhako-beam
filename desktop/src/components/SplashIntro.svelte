<script lang="ts">
  // The launch intro: the window opens brand blue with the bolt (tauri.conf.json backgroundColor),
  // the bolt plays a short "charge", then the blue fades away into the app. The Svelte twin of
  // android/.../ui/components/SplashIntro.kt. Motion: docs/design/motion-spec.md, section 4.14.
  import { onMount } from 'svelte';
  import { brand, scale as scaleTokens } from '../lib/tokens';
  import { prefs } from '../lib/state/prefs.svelte';
  import { playSound } from '../lib/sounds.svelte';
  import { springEasing } from '../lib/motion/physics';
  import { easeEnter, easeExit, easeStandard } from '../lib/motion/transitions';

  let { ondone }: { ondone: () => void } = $props();

  /** The vector bolt from the first logo, in its 1024 unit box (the logo is now the pixel mascot). */
  const BOLT = 'M648.8,220 L776.8,288.8 L592.8,472.8 L704.8,504.8 L247.2,804 L448.8,564 L336.8,532 Z';

  // 1 = resting; lower = pressed in (the shadow tucks under the bolt).
  let lift = $state(1);
  let grow = $state(1);
  let ring = $state(0);
  let cover = $state(1);

  const clamp = (x: number) => Math.min(1, Math.max(0, x));
  const lerp = (a: number, b: number, p: number) => a + (b - a) * p;

  onMount(() => {
    // Its click lands as the bolt presses in (175 ms), so it plays with the first frame.
    playSound('intro');
    const bouncy = springEasing('bouncy');
    const reduced = prefs.reducedMotion;
    const end = reduced ? 200 : 820;
    const pressedTo = scaleTokens.enterFrom;
    let start = 0;
    let frame = 0;

    // One timeline, read off the clock, so a slow frame never stretches the intro.
    function tick(now: number) {
      if (!start) start = now;
      const t = now - start;
      if (reduced) {
        cover = 1 - easeExit(clamp(t / 200));
      } else {
        if (t < 180) {
          const p = easeStandard(t / 180);
          lift = 1 - p;
          grow = lerp(1, pressedTo, p);
        } else {
          const p = bouncy.easing(clamp((t - 180) / bouncy.duration));
          lift = p;
          grow = lerp(pressedTo, 1, p);
        }
        ring = easeEnter(clamp((t - 180) / 520));
        const out = easeExit(clamp((t - 580) / 240));
        if (out > 0) grow = lerp(grow, 0.82, out);
        cover = 1 - out;
      }
      if (t < end) frame = requestAnimationFrame(tick);
      else ondone();
    }
    frame = requestAnimationFrame(tick);
    return () => cancelAnimationFrame(frame);
  });
</script>

<div class="intro" style:opacity={cover} style:--blue={brand.blue} aria-hidden="true">
  <svg viewBox="0 0 288 288" width="240" height="240">
    {#if ring > 0 && ring < 1}
      <circle cx="144" cy="144" r={64 + 104 * ring} fill="none" stroke="#fff" stroke-opacity={0.45 * (1 - ring)} stroke-width="3" />
    {/if}
    <g transform="translate(144 144) scale({grow}) translate(-144 -144)">
      <g transform="translate(30 30) scale(0.2226)">
        <path d={BOLT} transform="translate({18 * lift} {22 * lift})" fill={brand.navy} stroke={brand.navy} stroke-width="12" stroke-linejoin="round" />
        <path d={BOLT} fill={brand.yellow} stroke={brand.yellow} stroke-width="12" stroke-linejoin="round" />
      </g>
    </g>
  </svg>
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
