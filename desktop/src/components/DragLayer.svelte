<script lang="ts">
  // The dragged file stack. Follows the pointer 1:1 while free, glides onto a device with the
  // magnet spring when it snaps, and shrinks into the device on drop.
  import { app } from '../lib/state/app.svelte';
  import { drawnPosition, orbCenter } from '../lib/drag';
  import { SpringValue } from '../lib/motion/springs';
  import { prefs } from '../lib/state/prefs.svelte';
  import { t } from '../lib/copy';
  import { fadeT } from '../lib/motion/transitions';
  import FileStack from './FileStack.svelte';

  let ghost: HTMLElement | undefined = $state();
  let visible = $state(false);
  let label = $state('');
  let files = $state<{ name: string; mime: string }[]>([]);
  let count = $state(1);

  let x = 0, y = 0, s = 1, o = 1;
  const paint = () => {
    if (ghost) {
      ghost.style.transform = `translate(${x}px, ${y}px) translate(-50%, -50%) scale(${s})`;
      ghost.style.opacity = String(o);
    }
  };
  const sx = new SpringValue(0, (v) => ((x = v), paint()));
  const sy = new SpringValue(0, (v) => ((y = v), paint()));
  const ss = new SpringValue(1, (v) => ((s = v), paint()));
  const so = new SpringValue(1, (v) => ((o = Math.max(0, Math.min(1, v))), paint()));

  let lastSnap: string | null = null;
  let landing = false;

  // Paint once the element exists so the first frame is already in the right place.
  $effect(() => {
    if (ghost) paint();
  });

  $effect(() => {
    const d = app.drag;
    if (!d) return;
    if (!visible) {
      visible = true;
      landing = false;
      lastSnap = null;
      files = app.staged.length && d.source === 'stack' ? app.staged : [{ name: 'file', mime: '' }];
      count = d.count;
      const p = drawnPosition() ?? { x: d.x, y: d.y };
      sx.set(p.x);
      sy.set(p.y);
      ss.set(prefs.reducedMotion ? 1 : 0.85);
      so.set(1);
      ss.to(1, 'magnet');
    }
    count = d.count;
    const deviceName = d.snappedId ? app.deviceName(d.snappedId) : '';
    label = d.snappedId ? t('home.drop.onDevice', { device: deviceName }) : d.nearId ? t('home.drop.hint') : t('home.drop.noTarget');

    let target = drawnPosition() ?? { x: d.x, y: d.y };
    if (d.snappedId) {
      // Sit on the top-right of the device like a badge, so the device stays visible.
      const c = orbCenter(d.snappedId);
      if (c) target = { x: c.x + 26, y: c.y - 26 };
    }
    const snapChanged = d.snappedId !== lastSnap;
    lastSnap = d.snappedId;
    if (snapChanged || (d.snappedId && sx.animating)) {
      sx.to(target.x, 'magnet');
      sy.to(target.y, 'magnet');
      ss.to(d.snappedId ? 0.72 : 1, 'snappy');
    } else if (!d.snappedId) {
      sx.set(target.x);
      sy.set(target.y);
    }
  });

  /** Drop on a device: shrink into it. */
  export async function land(deviceId: string): Promise<void> {
    if (!visible) return;
    landing = true;
    const c = orbCenter(deviceId);
    if (c && !prefs.reducedMotion) {
      sx.to(c.x, 'smooth');
      sy.to(c.y, 'smooth');
      so.to(0, 'smooth');
      await ss.to(0.3, 'smooth');
    }
    finish();
  }

  /** Released on nothing: spring back to where it came from, or just fade. */
  export async function cancel(to?: { x: number; y: number }): Promise<void> {
    if (!visible) return;
    landing = true;
    if (to && !prefs.reducedMotion) {
      sx.to(to.x, 'settle');
      ss.to(1, 'settle');
      await sy.to(to.y, 'settle');
    } else {
      await so.to(0, 'smooth');
    }
    finish();
  }

  function finish() {
    if (app.drag) return; // a new drag started meanwhile
    visible = false;
    landing = false;
  }

  // If the drag ended without land/cancel being called, hide on the next frame.
  $effect(() => {
    if (!app.drag && visible && !landing) {
      requestAnimationFrame(() => {
        if (!app.drag && !landing) visible = false;
      });
    }
  });
</script>

{#if app.drag?.source === 'os'}
  <div class="scrim" transition:fadeT={{ duration: 150 }} aria-hidden="true"></div>
{/if}

{#if app.drag}
  <div class="hint" role="status" transition:fadeT={{ duration: 150 }} data-testid="drop-hint">{label}</div>
{/if}

{#if visible}
  <div class="ghost" bind:this={ghost} aria-hidden="true" data-testid="drag-ghost">
    <FileStack {files} {count} size={60} fanned lifted />
  </div>
{/if}

<style>
  .scrim {
    position: fixed;
    inset: 0;
    background: var(--color-scrim);
    opacity: 0.45;
    pointer-events: none;
    z-index: var(--z-drop-overlay);
  }
  .ghost {
    position: fixed;
    left: 0;
    top: 0;
    pointer-events: none;
    z-index: calc(var(--z-drop-overlay) + 2);
    will-change: transform, opacity;
  }
  .hint {
    position: fixed;
    left: 50%;
    bottom: calc(var(--space-9) + env(safe-area-inset-bottom));
    translate: -50% 0;
    padding: var(--space-3) var(--space-6);
    border-radius: var(--radius-full);
    background: var(--color-surface-raised);
    box-shadow: var(--elevation-3);
    font-weight: 600;
    pointer-events: none;
    z-index: calc(var(--z-drop-overlay) + 1);
    white-space: nowrap;
  }
</style>
