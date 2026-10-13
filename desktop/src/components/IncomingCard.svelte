<script lang="ts">
  // Incoming request. Swipe right to accept, left to decline (or use the buttons / keys).
  // Follows the finger 1:1, tilts up to 6 degrees around the bottom center, crossing the
  // commit threshold fires a haptic and pops the edge label; release carries velocity.
  import type { IncomingRequest } from '../lib/engine/types';
  import { gesture } from '../lib/tokens';
  import { app } from '../lib/state/app.svelte';
  import { prefs } from '../lib/state/prefs.svelte';
  import { t } from '../lib/copy';
  import { formatBytes, isLink, singleFolder, summarizeFiles, textPreview } from '../lib/format';
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
  import { springIn } from '../lib/motion/transitions';
  import { spark } from '../lib/motion/bolt';
  import Bolt from './Bolt.svelte';

  interface Props {
    request: IncomingRequest;
    depth?: number;
  }

  let { request, depth = 0 }: Props = $props();

  const device = $derived(app.device(request.deviceId));
  const name = $derived(device?.name ?? 'Someone nearby');
  /** Text rather than files: nothing goes to disk, so space doesn't matter. */
  const isText = $derived(request.text !== undefined);
  const link = $derived(isText && isLink(request.text ?? ''));
  const noSpace = $derived(!isText && request.totalBytes > app.freeSpace);
  /** One folder: its name as the title, the count with the size. */
  const folder = $derived(isText ? null : singleFolder(request.files));
  let always = $state(false);

  let card: HTMLElement | undefined = $state();
  let acceptLabel: HTMLElement | undefined = $state();
  let declineLabel: HTMLElement | undefined = $state();
  let meter: HTMLElement | undefined = $state();
  let side = $state<'accept' | 'decline'>('accept');
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
    // The charge meter fills toward the side you're swiping to and is full at the commit point.
    if (meter) {
      meter.style.transformOrigin = x < 0 ? 'right center' : 'left center';
      meter.style.transform = `scaleX(${pr})`;
      if ((x < 0) !== (side === 'decline')) side = x < 0 ? 'decline' : 'accept';
    }
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

  /** `then`, for text: what accepting does with it (Copy is what a swipe or Enter does). */
  async function commit(outcome: 'accept' | 'decline', vx = 0, then: 'copy' | 'open' = 'copy') {
    if (leaving) return;
    leaving = true;
    playHaptic(outcome);
    playSound(outcome);
    const dir = outcome === 'accept' ? 1 : -1;
    // Zap: sparks thrown the way the card is about to fly.
    if (card) {
      const r = card.getBoundingClientRect();
      spark({ x: dir > 0 ? r.right - 24 : r.left + 24, y: r.top + r.height / 2 }, { toward: dir > 0 ? 0 : Math.PI, count: 5, reach: 36 });
    }
    if (prefs.reducedMotion) {
      await so.to(0, 'smooth');
    } else {
      const distance = window.innerWidth + width();
      const speed = Math.max(Math.abs(vx), gesture.swipeFlyOutVelocityMin);
      sy.to(y - 40, 'settle');
      await Promise.race([sx.to(dir * distance, 'settle', dir * speed), new Promise((r) => setTimeout(r, 380))]);
    }
    if (outcome === 'decline') return app.decline(request.id);
    const text = request.text;
    app.accept(request.id, { autoAccept: always });
    if (isText && text) void (then === 'open' ? app.openLink(text) : app.copyText(text));
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

  // The bolt badge on the sender gives one small twitch once the card has landed.
  let twitch = $state(0);
  $effect(() => {
    if (depth > 0) return;
    const id = setTimeout(() => twitch++, 420);
    return () => clearTimeout(id);
  });
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
  aria-label={isText ? `Text from ${name}: ${textPreview(request.text ?? '', 120)}. Actions: copy, decline.` : `Incoming files from ${name}: ${summarizeFiles(request.files)}, ${formatBytes(request.totalBytes)}. Actions: accept, decline.`}
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
    {#if device}
      <span class="sender">
        <DeviceOrb {device} size={56} showName={false} />
        <span class="badge"><Bolt size={18} {twitch} testid="incoming-bolt" /></span>
      </span>
    {/if}
    <div>
      <p class="from">{t('incoming.title', { device: name })}</p>
      {#if isText}
        <h2 class="what">{t(link ? 'staged.link' : 'staged.text')}</h2>
        <p class="meta">{#if device?.route}<ConnectionBadge route={device.route} />{/if}</p>
      {:else}
        <h2 class="what" title={folder?.name}>{folder ? folder.name : summarizeFiles(request.files)}</h2>
        <p class="meta"><span class="tabular">{folder ? `${folder.files} · ` : ''}{formatBytes(request.totalBytes)}</span>{#if device?.route}<ConnectionBadge route={device.route} />{/if}</p>
      {/if}
      {#if !device?.trusted}
        <!-- A stranger can copy a name, so say plainly when this isn't one of your saved devices. -->
        <p class="not-yours" data-testid="not-yours" in:springIn={{ preset: 'bouncy', scale: 0.85, y: 4, delay: 120 }}>
          <Icon name="warning-circle" size={14} weight="fill" />{t('incoming.notYours')}
        </p>
      {/if}
    </div>
    <span class="files"><FileStack files={request.files} text={request.text ?? null} size={48} /></span>
  </div>

  {#if isText}
    <!-- Read it before deciding: the whole point of text arriving in the request. -->
    <blockquote class="textbody" data-testid="incoming-text">{request.text}</blockquote>
  {/if}

  {#if noSpace}
    <div class="nospace" role="status">
      <Icon name="hard-drives" size={20} />
      <span>{t('incoming.noSpace', { needed: formatBytes(request.totalBytes - app.freeSpace) })}</span>
      <button class="link" onclick={() => app.changeSaveLocation()}>{t('incoming.noSpace.fix')}</button>
    </div>
  {:else if device}
    <Toggle bind:checked={always} label={t('incoming.alwaysAccept', { device: name })} testid="always-accept" />
  {/if}

  <div class="buttons" class:three={isText && link}>
    <Button variant="secondary" size="lg" full testid="decline" onclick={() => commit('decline')}>{t('incoming.decline')}</Button>
    {#if isText}
      <Button variant={link ? 'secondary' : 'primary'} size="lg" full testid="accept" sparks={false} onclick={() => commit('accept', 0, 'copy')}>{t('text.copy')}</Button>
      {#if link}
        <Button variant="primary" size="lg" full testid="accept-open" sparks={false} onclick={() => commit('accept', 0, 'open')}>{t('text.open')}</Button>
      {/if}
    {:else}
      <Button variant="primary" size="lg" full testid="accept" sparks={false} disabled={noSpace} onclick={() => commit('accept')}>{t('incoming.accept')}</Button>
    {/if}
  </div>

  <span class="meter-track" aria-hidden="true"><span class="meter" data-side={side} bind:this={meter}></span></span>

  {#if app.isAndroid && !noSpace}
    <p class="gesture"><Icon name="hand-swipe-right" size={16} weight="bold" />{t('incoming.hint.gesture')}</p>
  {/if}
</div>

<style>
  .textbody {
    margin: 0;
    padding: var(--space-3) var(--space-4);
    border-left: 3px solid var(--brand-yellow);
    border-radius: var(--radius-sm);
    background: var(--color-surface-sunken);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    display: -webkit-box;
    -webkit-line-clamp: 5;
    line-clamp: 5;
    -webkit-box-orient: vertical;
    overflow: hidden;
    line-height: 1.45;
  }
  .buttons.three {
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: var(--space-2);
  }
  .buttons.three :global(.btn) {
    min-width: 0;
    padding: 0 var(--space-2);
  }
  .not-yours {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    margin: var(--space-2) 0 0;
    padding: 2px var(--space-3) 2px var(--space-2);
    border-radius: var(--radius-full);
    background: var(--color-warning-subtle);
    color: var(--color-warning);
    font-size: var(--type-caption-size);
    line-height: var(--type-caption-line);
    font-weight: 600;
  }
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
  .sender {
    position: relative;
    flex: none;
  }
  .badge {
    position: absolute;
    left: -4px;
    top: -6px;
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: 50%;
    background: var(--color-surface-raised);
    box-shadow: var(--elevation-1);
  }
  /* The charge meter along the bottom edge, inside the card's rounded corners. */
  .meter-track {
    position: absolute;
    left: var(--space-6);
    right: var(--space-6);
    bottom: 10px;
    height: 3px;
    border-radius: 2px;
    overflow: hidden;
    pointer-events: none;
  }
  .meter {
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: var(--color-success);
    transform: scaleX(0);
  }
  .meter[data-side='decline'] {
    background: var(--color-danger);
  }
  .past .meter[data-side='accept'] {
    background: var(--brand-yellow);
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
