<script lang="ts">
  // Simulated camera for the prototype (Android only in the real app). A code drifts in the
  // view; when "recognised", the viewfinder corners snap onto its bounds with the magnet spring.
  import { SpringValue, wait } from '../lib/motion/springs';
  import { prefs } from '../lib/state/prefs.svelte';
  import Icon from './Icon.svelte';

  let { onfound }: { onfound: () => void } = $props();

  let frame: HTMLElement | undefined = $state();
  let found = $state(false);

  // Viewfinder box (percent of the view): starts large, snaps to the code.
  const box = { x: 12, y: 12, w: 76, h: 76 };
  const paint = () => {
    if (!frame) return;
    frame.style.left = `${box.x}%`;
    frame.style.top = `${box.y}%`;
    frame.style.width = `${box.w}%`;
    frame.style.height = `${box.h}%`;
  };
  const springs = (['x', 'y', 'w', 'h'] as const).map((k) => new SpringValue(box[k], (v) => ((box[k] = v), paint())));

  $effect(() => {
    if (!frame) return;
    paint();
    let alive = true;
    (async () => {
      await wait(1500);
      if (!alive) return;
      found = true;
      const target = { x: 30, y: 27, w: 40, h: 40 };
      await Promise.all(springs.map((s, i) => s.to([target.x, target.y, target.w, target.h][i]!, 'magnet')));
      await wait(prefs.reducedMotion ? 300 : 450);
      if (alive) onfound();
    })();
    return () => {
      alive = false;
      springs.forEach((s) => s.stop());
    };
  });
</script>

<div class="view" data-testid="qr-scanner" data-found={found}>
  <div class="scene" class:found aria-hidden="true">
    <div class="paper">
      <svg viewBox="0 0 21 21" class="fakeqr">
        <rect x="0.5" y="0.5" width="6" height="6" rx="1.6" fill="none" stroke="currentColor" />
        <rect x="14.5" y="0.5" width="6" height="6" rx="1.6" fill="none" stroke="currentColor" />
        <rect x="0.5" y="14.5" width="6" height="6" rx="1.6" fill="none" stroke="currentColor" />
        {#each Array.from({ length: 60 }, (_, i) => i) as i}
          <circle cx={8 + ((i * 7) % 13) + 0.5} cy={1 + ((i * 11) % 19) + 0.5} r="0.42" fill="currentColor" />
        {/each}
      </svg>
    </div>
  </div>
  <div class="frame" class:found bind:this={frame} aria-hidden="true">
    <span class="c tl"></span><span class="c tr"></span><span class="c bl"></span><span class="c br"></span>
  </div>
  <p class="label" role="status">
    {#if found}<Icon name="check-circle" size={18} weight="fill" /> Code found{:else}Point at the code on the other device{/if}
  </p>
</div>

<style>
  .view {
    position: relative;
    aspect-ratio: 3 / 4;
    max-height: 52vh;
    margin: 0 auto;
    width: 100%;
    border-radius: var(--radius-lg);
    overflow: hidden;
    background: #0a0b0d;
    color: #f1f2f4;
  }
  .scene {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
  }
  .paper {
    width: 46%;
    aspect-ratio: 1;
    padding: 4%;
    border-radius: 10px;
    background: #f4f5f7;
    color: #121418;
    animation: drift 3.2s ease-in-out infinite alternate;
    transition: transform var(--spring-magnet-duration) var(--spring-magnet);
  }
  .found .paper {
    animation: none;
    transform: none;
  }
  .fakeqr {
    width: 100%;
    height: 100%;
  }
  @keyframes drift {
    from {
      transform: translate(-6%, 4%) rotate(-5deg) scale(0.94);
    }
    to {
      transform: translate(5%, -3%) rotate(4deg) scale(1.02);
    }
  }
  :global([data-reduced='true']) .paper {
    animation: none;
  }
  .frame {
    position: absolute;
  }
  .c {
    position: absolute;
    width: 22%;
    height: 22%;
    border: 3px solid #ffffff;
    transition: border-color var(--duration-fast) var(--ease-standard);
  }
  .found .c {
    border-color: var(--color-success);
  }
  .tl {
    left: 0;
    top: 0;
    border-right: 0;
    border-bottom: 0;
    border-top-left-radius: 14px;
  }
  .tr {
    right: 0;
    top: 0;
    border-left: 0;
    border-bottom: 0;
    border-top-right-radius: 14px;
  }
  .bl {
    left: 0;
    bottom: 0;
    border-right: 0;
    border-top: 0;
    border-bottom-left-radius: 14px;
  }
  .br {
    right: 0;
    bottom: 0;
    border-left: 0;
    border-top: 0;
    border-bottom-right-radius: 14px;
  }
  .label {
    position: absolute;
    left: 0;
    right: 0;
    bottom: var(--space-5);
    display: flex;
    justify-content: center;
    align-items: center;
    gap: var(--space-2);
    font-size: var(--type-callout-size);
    font-weight: 600;
  }
</style>
