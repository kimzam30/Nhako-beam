<script lang="ts">
  // Incoming request. Swipe right to accept, left to decline (or use the buttons / keys).
  // Follows the finger 1:1, tilts up to 6 degrees around the bottom center, crossing the
  // commit threshold fires a haptic and pops the edge label; release carries velocity.
  import type { IncomingRequest } from '../lib/engine/types';
  import { gesture } from '../lib/tokens';
  import { app } from '../lib/state/app.svelte';
  import { prefs } from '../lib/state/prefs.svelte';
  import { t } from '../lib/copy';
  import { formatBytes, summarizeFiles } from '../lib/format';
  import { pastSwipeThreshold, rubberBand, swipeDecision, VelocityTracker } from '../lib/motion/physics';
  import { SpringValue } from '../lib/motion/springs';
  import { playHaptic } from '../lib/haptics.svelte';
  import { playSound } from '../lib/sounds.svelte';
  import Button from './Button.svelte';
  import ConnectionBadge from './ConnectionBadge.svelte';
  import DeviceOrb from './DeviceOrb.svelte';
  import FileStack from './FileStack.svelte';
  import Icon from './Icon.svelte';
  import Toggle from './Toggle.svelte';

  interface Props {
    request: IncomingRequest;
    depth?: number;
  }

  let { request, depth = 0 }: Props = $props();

  const device = $derived(app.device(request.deviceId));
  const name = $derived(device?.name ?? 'Someone nearby');
  const noSpace = $derived(request.totalBytes > app.freeSpace);
  let always = $state(false);

  let card: HTMLElement | undefined = $state();
  let acceptLabel: HTMLElement | undefined = $state();
  let declineLabel: HTMLElement | undefined = $state();
  let past = $state(false);
  let leaving = false;

  let x = 0, y = 0, op = 1;
  const width = () => card?.offsetWidth ?? 320;
  const paint = () => {
    if (!card) return;
    const rot = Math.max(-1, Math.min(1, x / width())) * gesture.swipeMaxRotationDeg;
    card.style.transform = prefs.reducedMotion ? '' : `translate(${x}px, ${y}px) rotate(${rot}deg)`;
    card.style.opacity = String(op);
    const pr = Math.min(1, Math.abs(x) / (width() * gesture.swipeCommitFraction));
    if (acceptLabel) acceptLabel.style.opacity = String(x > 0 ? pr : 0);
    if (declineLabel) declineLabel.style.opacity = String(x < 0 ? pr : 0);
  };
  const sx = new SpringValue(0, (v) => ((x = v), paint()));
  const sy = new SpringValue(0, (v) => ((y = v), paint()));
  const so = new SpringValue(1, (v) => ((op = Math.max(0, Math.min(1, v))), paint()));

  const vt = new VelocityTracker();
  let start = { x: 0, y: 0 };
  let pointerId: number | null = null;
  let moved = false;

  function down(e: PointerEvent) {
    if (depth > 0 || leaving || noSpace) return;
    if ((e.target as HTMLElement).closest('button, [role="switch"], input')) return;
    pointerId = e.pointerId;
    moved = false;
    start = { x: e.clientX - x, y: e.clientY - y };
    vt.reset();
    vt.add(e.timeStamp, e.clientX, e.clientY);
    sx.stop();
    sy.stop();
  }

  function move(e: PointerEvent) {
    if (pointerId !== e.pointerId) return;
    const dx = e.clientX - start.x;
    const dy = e.clientY - start.y;
    if (!moved) {
      const slop = e.pointerType === 'touch' ? gesture.dragSlopTouch : gesture.dragSlopPointer;
      if (Math.hypot(dx, dy) < slop) return;
      moved = true;
      card?.setPointerCapture(e.pointerId);
    }
    vt.add(e.timeStamp, e.clientX, e.clientY);
    sx.set(dx);
    sy.set(rubberBand(dy, 400, 0.35));
    const nowPast = pastSwipeThreshold(dx, width());
    if (nowPast !== past) {
      past = nowPast;
      playHaptic(nowPast ? 'thresholdActivate' : 'thresholdDeactivate');
      if (nowPast) playSound('tick');
    }
  }

  function up(e: PointerEvent) {
    if (pointerId !== e.pointerId) return;
    pointerId = null;
    if (!moved) return;
    const v = vt.velocity();
    const outcome = swipeDecision(x, v.x, width());
    past = false;
    if (outcome === 'cancel') {
      sx.to(0, 'settle', v.x);
      sy.to(0, 'settle', v.y);
    } else {
      commit(outcome, v.x);
    }
  }

  async function commit(outcome: 'accept' | 'decline', vx = 0) {
    if (leaving) return;
    leaving = true;
    playHaptic(outcome);
    playSound(outcome);
    const dir = outcome === 'accept' ? 1 : -1;
    if (prefs.reducedMotion) {
      await so.to(0, 'smooth');
    } else {
      const distance = window.innerWidth + width();
      const speed = Math.max(Math.abs(vx), gesture.swipeFlyOutVelocityMin);
      sy.to(y - 40, 'settle');
      await Promise.race([sx.to(dir * distance, 'settle', dir * speed), new Promise((r) => setTimeout(r, 380))]);
    }
    if (outcome === 'accept') app.accept(request.id, { autoAccept: always });
    else app.decline(request.id);
  }

  function key(e: KeyboardEvent) {
    if (depth > 0 || (e.target as HTMLElement).closest('input, textarea')) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      commit('decline');
    } else if (e.key === 'Enter' && !noSpace && !(e.target as HTMLElement).closest('button')) {
      e.preventDefault();
      commit('accept');
    }
  }

  $effect(() => {
    if (card) paint();
  });

  export function focusCard() {
    card?.focus();
  }
</script>

<svelte:window onkeydown={key} />

<div
  class="card"
  class:past
  class:behind={depth > 0}
  style:--depth={depth}
  bind:this={card}
  role="alertdialog"
  aria-modal={depth === 0 ? 'true' : undefined}
  aria-label="Incoming files from {name}: {summarizeFiles(request.files)}, {formatBytes(request.totalBytes)}. Actions: accept, decline."
  tabindex={depth === 0 ? 0 : -1}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={up}
  data-testid={depth === 0 ? 'incoming-card' : undefined}
>
  <span class="edge accept" bind:this={acceptLabel} aria-hidden="true"><Icon name="check-circle" size={20} weight="fill" />{t('incoming.accept')}</span>
  <span class="edge decline" bind:this={declineLabel} aria-hidden="true">{t('incoming.decline')}<Icon name="x-circle" size={20} weight="fill" /></span>

  <div class="who">
    {#if device}<DeviceOrb {device} size={56} showName={false} />{/if}
    <div>
      <p class="from">{t('incoming.title', { device: name })}</p>
      <h2 class="what">{summarizeFiles(request.files)}</h2>
      <p class="meta"><span class="tabular">{formatBytes(request.totalBytes)}</span>{#if device?.route}<ConnectionBadge route={device.route} />{/if}</p>
    </div>
    <span class="files"><FileStack files={request.files} size={48} /></span>
  </div>

  {#if noSpace}
    <div class="nospace" role="status">
      <Icon name="hard-drives" size={20} />
      <span>{t('incoming.noSpace', { needed: formatBytes(request.totalBytes - app.freeSpace) })}</span>
      <button class="link" onclick={() => app.changeSaveLocation()}>{t('incoming.noSpace.fix')}</button>
    </div>
  {:else if device}
    <Toggle bind:checked={always} label={t('incoming.alwaysAccept', { device: name })} testid="always-accept" />
  {/if}

  <div class="buttons">
    <Button variant="secondary" size="lg" full testid="decline" onclick={() => commit('decline')}>{t('incoming.decline')}</Button>
    <Button variant="primary" size="lg" full testid="accept" disabled={noSpace} onclick={() => commit('accept')}>{t('incoming.accept')}</Button>
  </div>

  {#if app.isAndroid && !noSpace}
    <p class="gesture"><Icon name="hand-swipe-right" size={16} weight="bold" />{t('incoming.hint.gesture')}</p>
  {/if}
</div>

<style>
  .card {
    position: relative;
    display: grid;
    /* One column that can't grow past the card: a long file name would widen it otherwise. */
    grid-template-columns: minmax(0, 1fr);
    gap: var(--space-5);
    width: min(calc(100vw - 32px), 420px);
    padding: var(--space-6);
    border-radius: var(--radius-xl);
    background: var(--color-surface-raised);
    box-shadow: var(--elevation-4);
    transform-origin: 50% 100%;
    touch-action: none;
    user-select: none;
    cursor: grab;
    will-change: transform;
    transition: scale var(--spring-snappy-duration) var(--spring-snappy);
  }
  .card:active {
    cursor: grabbing;
  }
  .card.past {
    scale: var(--scale-lifted);
  }
  .card.behind {
    position: absolute;
    inset: auto 0 0 0;
    margin: 0 auto;
    translate: 0 calc(var(--depth) * -10px);
    scale: calc(1 - var(--depth) * 0.04);
    opacity: calc(1 - var(--depth) * 0.3);
    pointer-events: none;
    z-index: -1;
  }
  .behind > * {
    visibility: hidden;
  }
  .edge {
    position: absolute;
    top: var(--space-5);
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    border-radius: var(--radius-full);
    font-weight: 700;
    opacity: 0;
    pointer-events: none;
    transition: transform var(--spring-snappy-duration) var(--spring-snappy);
    z-index: 1;
  }
  .edge.accept {
    left: var(--space-5);
    background: var(--color-success-subtle);
    color: var(--color-success);
  }
  .edge.decline {
    right: var(--space-5);
    background: var(--color-danger-subtle);
    color: var(--color-danger);
  }
  .past .edge {
    transform: scale(1.08);
  }
  .who {
    display: flex;
    align-items: center;
    gap: var(--space-4);
  }
  .who > div {
    flex: 1;
    min-width: 0;
  }
  .from {
    font-size: var(--type-footnote-size);
    color: var(--color-text-secondary);
  }
  .what {
    font-size: var(--type-title2-size);
    line-height: var(--type-title2-line);
    font-weight: var(--type-title2-weight);
    letter-spacing: var(--type-title2-tracking);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-top: var(--space-1);
    font-size: var(--type-footnote-size);
    color: var(--color-text-secondary);
  }
  .files {
    flex: none;
    padding: var(--space-3) var(--space-3) 0 0;
  }
  @media (max-width: 440px) {
    .files {
      display: none;
    }
  }
  .nospace {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-md);
    background: var(--color-warning-subtle);
    color: var(--color-warning);
    font-size: var(--type-callout-size);
  }
  .nospace span {
    flex: 1;
  }
  .link {
    font-weight: 600;
    color: inherit;
    text-decoration: underline;
  }
  .buttons {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-3);
  }
  .gesture {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    font-size: var(--type-footnote-size);
    color: var(--color-text-secondary);
    margin-top: calc(var(--space-2) * -1);
  }
</style>
