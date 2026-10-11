<script lang="ts">
  import { app } from '../lib/state/app.svelte';
  import { fadeT, springIn } from '../lib/motion/transitions';
  import IncomingCard from './IncomingCard.svelte';

  const stack = $derived(app.requests.slice(0, 3));
</script>

{#if stack.length}
  <div class="layer" class:center={app.layout !== 'compact'}>
    <div class="scrim" transition:fadeT aria-hidden="true"></div>
    <div class="stack">
      {#each stack as request, i (request.id)}
        <div class="slot" class:top={i === 0} in:springIn={{ preset: 'bouncy', y: 24, scale: 0.94 }}>
          <IncomingCard {request} depth={i} />
        </div>
      {/each}
    </div>
  </div>
{/if}

<style>
  .layer {
    position: fixed;
    inset: 0;
    z-index: calc(var(--z-sheet) + 1);
    display: grid;
    align-items: end;
    justify-items: center;
    padding: var(--space-5) var(--space-5) calc(var(--space-7) + env(safe-area-inset-bottom));
    overflow: hidden;
  }
  .layer.center {
    align-items: center;
  }
  .scrim {
    position: absolute;
    inset: 0;
    background: var(--color-scrim);
  }
  .stack {
    position: relative;
    display: grid;
  }
  .slot {
    grid-area: 1 / 1;
    display: grid;
    align-items: end;
  }
  .slot.top {
    z-index: 2;
  }
</style>
