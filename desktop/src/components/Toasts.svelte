<script lang="ts">
  // Max 2 toasts at the bottom. Enter on the smooth spring; swipe sideways to dismiss.
  import { app, type Toast } from '../lib/state/app.svelte';
  import { springIn, quickOut, springFlip } from '../lib/motion/transitions';
  import { VelocityTracker } from '../lib/motion/physics';
  import { SpringValue } from '../lib/motion/springs';
  import Icon from './Icon.svelte';

  const icons: Record<Toast['tone'], string> = { neutral: 'info', success: 'check-circle', warning: 'warning-circle', danger: 'x-circle' };

  function swipe(node: HTMLElement, id: number) {
    const x = new SpringValue(0, (v) => {
      node.style.translate = `${v}px 0`;
      node.style.opacity = String(Math.max(0, 1 - Math.abs(v) / 240));
    });
    const vt = new VelocityTracker();
    let start = 0;
    let active = false;
    const down = (e: PointerEvent) => {
      if ((e.target as HTMLElement).closest('button')) return;
      active = true;
      start = e.clientX;
      vt.reset();
      vt.add(e.timeStamp, e.clientX, 0);
      node.setPointerCapture(e.pointerId);
      x.stop();
    };
    const move = (e: PointerEvent) => {
      if (!active) return;
      vt.add(e.timeStamp, e.clientX, 0);
      x.set(e.clientX - start);
    };
    const up = () => {
      if (!active) return;
      active = false;
      const v = vt.velocity().x;
      if (Math.abs(x.value) > 80 || Math.abs(v) > 650) {
        const dir = Math.sign(x.value || v);
        x.to(dir * 420, 'settle', dir * Math.max(Math.abs(v), 1200)).then(() => app.dismissToast(id));
      } else x.to(0, 'settle', v);
    };
    node.addEventListener('pointerdown', down);
    node.addEventListener('pointermove', move);
    node.addEventListener('pointerup', up);
    node.addEventListener('pointercancel', up);
    return {
      destroy() {
        node.removeEventListener('pointerdown', down);
        node.removeEventListener('pointermove', move);
        node.removeEventListener('pointerup', up);
        node.removeEventListener('pointercancel', up);
      },
    };
  }
</script>

<div class="toasts" aria-live="polite">
  {#each app.toasts as toast (toast.id)}
    <div class="toast {toast.tone}" use:swipe={toast.id} in:springIn={{ y: 24, scale: 0.96 }} out:quickOut={{ y: 12 }} animate:springFlip data-testid="toast">
      <Icon name={icons[toast.tone]} size={20} weight={toast.tone === 'neutral' ? 'duotone' : 'fill'} />
      <span class="text">{toast.text}</span>
      {#if toast.action}
        {@const action = toast.action}
        <button class="action" onclick={() => (action.run(), app.dismissToast(toast.id))}>{action.label}</button>
      {/if}
      <span class="left" style:animation-duration="{toast.ms}ms" aria-hidden="true"></span>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    left: 50%;
    bottom: calc(var(--space-7) + env(safe-area-inset-bottom));
    translate: -50% 0;
    z-index: var(--z-toast);
    display: grid;
    gap: var(--space-3);
    width: min(calc(100% - 32px), 420px);
    pointer-events: none;
  }
  .toast {
    position: relative;
    overflow: hidden;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-5);
    border-radius: var(--radius-lg);
    background: var(--color-surface-raised);
    box-shadow: var(--elevation-3);
    font-size: var(--type-callout-size);
    line-height: var(--type-callout-line);
    pointer-events: auto;
    touch-action: pan-y;
    user-select: none;
  }
  .text {
    flex: 1;
  }
  /* A thin bar along the bottom drains as the toast's time runs out. */
  .left {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 0;
    height: 2px;
    background: currentColor;
    opacity: 0.35;
    transform-origin: left center;
    animation: drain linear both;
  }
  :global([data-reduced='true']) .left {
    display: none;
  }
  @keyframes drain {
    from {
      transform: scaleX(1);
    }
    to {
      transform: scaleX(0);
    }
  }
  .success {
    color: var(--color-success);
  }
  .warning {
    color: var(--color-warning);
  }
  .danger {
    color: var(--color-danger);
  }
  .success .text,
  .warning .text,
  .danger .text {
    color: var(--color-text-primary);
  }
  .action {
    font-weight: 600;
    color: var(--color-accent-text);
  }
</style>
