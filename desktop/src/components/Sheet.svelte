<script lang="ts">
  // Bottom sheet on compact layouts, centered dialog on wider ones.
  // Drag the handle (or the header) down to dismiss; dragging up rubber bands.
  import type { Snippet } from 'svelte';
  import { gesture } from '../lib/tokens';
  import { app } from '../lib/state/app.svelte';
  import { rubberBand, VelocityTracker } from '../lib/motion/physics';
  import { SpringValue } from '../lib/motion/springs';
  import { fadeT, sheetIn, sheetOut } from '../lib/motion/transitions';
  import IconButton from './IconButton.svelte';

  interface Props {
    title: string;
    onclose: () => void;
    testid?: string;
    wide?: boolean;
    children: Snippet;
    footer?: Snippet;
  }

  let { title, onclose, testid, wide = false, children, footer }: Props = $props();

  const dialog = $derived(app.layout !== 'compact');
  let panel: HTMLElement | undefined = $state();
  const y = new SpringValue(0, (v) => panel && (panel.style.translate = `0 ${v}px`));
  const vt = new VelocityTracker();
  let startY = 0;
  let dragging = false;
  const titleId = `sheet-${Math.random().toString(36).slice(2, 8)}`;

  function down(e: PointerEvent) {
    if (dialog || (e.target as HTMLElement).closest('button:not([data-handle])')) return;
    dragging = true;
    startY = e.clientY - y.value;
    vt.reset();
    vt.add(e.timeStamp, 0, e.clientY);
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    y.stop();
  }

  function move(e: PointerEvent) {
    if (!dragging || !panel) return;
    const raw = e.clientY - startY;
    vt.add(e.timeStamp, 0, e.clientY);
    y.set(raw < 0 ? rubberBand(raw, panel.offsetHeight) : raw);
  }

  function up() {
    if (!dragging || !panel) return;
    dragging = false;
    const v = vt.velocity().y;
    const h = panel.offsetHeight;
    if (y.value > h * gesture.sheetDismissFraction || v > gesture.sheetDismissVelocity) {
      y.to(h, 'settle', Math.max(v, 600)).then(onclose);
    } else {
      y.to(0, 'settle', v);
    }
  }

  function key(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.stopPropagation();
      onclose();
    }
  }

  $effect(() => {
    if (!panel) return;
    const prev = document.activeElement as HTMLElement | null;
    const first = panel.querySelector<HTMLElement>('[autofocus], input, button:not([data-handle]), [tabindex="0"]');
    first?.focus({ preventScroll: true });
    return () => prev?.focus?.({ preventScroll: true });
  });
</script>

<svelte:window onkeydown={key} />

<div class="layer" class:dialog data-testid={testid}>
  <button class="scrim" aria-label="Close" tabindex="-1" onclick={onclose} transition:fadeT></button>
  <div
    class="panel"
    class:wide
    bind:this={panel}
    role="dialog"
    aria-modal="true"
    aria-labelledby={titleId}
    in:sheetIn={{ dialog }}
    out:sheetOut={{ dialog }}
  >
    <header role="presentation" onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={up}>
      {#if !dialog}<span class="handle" data-handle aria-hidden="true"></span>{/if}
      <div class="bar">
        <h2 id={titleId}>{title}</h2>
        <IconButton icon="x" label="Close" testid="sheet-close" onclick={onclose} />
      </div>
    </header>
    <div class="body">
      {@render children()}
    </div>
    {#if footer}<footer>{@render footer()}</footer>{/if}
  </div>
</div>

<style>
  .layer {
    position: fixed;
    inset: 0;
    z-index: var(--z-sheet);
    display: grid;
    align-items: end;
  }
  .layer.dialog {
    place-items: center;
    padding: var(--space-7);
  }
  .scrim {
    position: absolute;
    inset: 0;
    background: var(--color-scrim);
    cursor: default;
  }
  .panel {
    position: relative;
    display: flex;
    flex-direction: column;
    max-height: min(88vh, 760px);
    background: var(--color-surface-raised);
    border-radius: var(--radius-xl) var(--radius-xl) 0 0;
    box-shadow: var(--elevation-4);
    padding-bottom: env(safe-area-inset-bottom);
    will-change: transform;
  }
  .dialog .panel {
    width: min(100%, 460px);
    border-radius: var(--radius-xl);
    padding-bottom: 0;
  }
  .dialog .panel.wide {
    width: min(100%, 560px);
  }
  header {
    flex: none;
    padding: var(--space-3) var(--space-4) 0 var(--space-6);
    touch-action: none;
    cursor: grab;
  }
  .dialog header {
    cursor: default;
    padding-top: var(--space-4);
  }
  .handle {
    display: block;
    width: 36px;
    height: 5px;
    margin: 0 auto var(--space-2);
    border-radius: var(--radius-full);
    background: var(--color-fill-pressed);
  }
  .bar {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  h2 {
    flex: 1;
    font-size: var(--type-title2-size);
    line-height: var(--type-title2-line);
    font-weight: var(--type-title2-weight);
    letter-spacing: var(--type-title2-tracking);
  }
  .body {
    overflow-y: auto;
    padding: var(--space-3) var(--space-6) var(--space-6);
    overscroll-behavior: contain;
  }
  footer {
    flex: none;
    display: flex;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-6) var(--space-6);
    border-top: 1px solid var(--color-separator);
  }
</style>
