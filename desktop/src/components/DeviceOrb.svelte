<script lang="ts">
  // A device as a round "orb". States: idle, target (something is being dragged near it),
  // snapped (dragged files locked onto it), waiting (pulsing ring), busy (progress ring),
  // offline (dimmed), done (check pop). `pulse` bumps play the bouncy arrival pulse.
  import { onDestroy } from 'svelte';
  import type { Device, DeviceKind } from '../lib/engine/types';
  import { SpringValue } from '../lib/motion/springs';
  import { app } from '../lib/state/app.svelte';
  import Bolt from './Bolt.svelte';
  import Icon from './Icon.svelte';
  import { t } from '../lib/copy';

  interface Props {
    device?: Device;
    self?: boolean;
    selfName?: string;
    size?: number;
    showName?: boolean;
    target?: boolean;
    snapped?: boolean;
    waiting?: boolean;
    progress?: number | null;
    register?: boolean;
    /** The 3D stage draws this device under the button: the circle and its icon go clear. */
    ghost?: boolean;
    onclick?: () => void;
    /** Right-click, or a long press on touch: the device's own sheet. */
    onmenu?: () => void;
    testid?: string;
  }

  let { device, self = false, selfName = '', size = 64, showName = true, target = false, snapped = false, waiting = false, progress = null, register = false, ghost = false, onclick, onmenu, testid }: Props = $props();

  // A long press opens the menu, and the click that follows it doesn't also send.
  const LONG_PRESS_MS = 500;
  let pressTimer: ReturnType<typeof setTimeout> | null = null;
  let pressAt: { x: number; y: number } | null = null;
  let pressed = false;
  function pressDown(e: PointerEvent) {
    if (!onmenu || e.pointerType === 'mouse') return;
    pressed = false;
    pressAt = { x: e.clientX, y: e.clientY };
    pressTimer = setTimeout(() => {
      pressed = true;
      pressTimer = null;
      onmenu?.();
    }, LONG_PRESS_MS);
  }
  function pressMove(e: PointerEvent) {
    if (pressTimer && pressAt && Math.hypot(e.clientX - pressAt.x, e.clientY - pressAt.y) > 8) pressEnd();
  }
  function pressEnd() {
    if (pressTimer) clearTimeout(pressTimer);
    pressTimer = null;
  }
  // An orb that leaves mid-press (the device went away) doesn't open its menu afterwards.
  onDestroy(pressEnd);
  function click() {
    if (pressed) {
      pressed = false;
      return;
    }
    onclick?.();
  }
  function menu(e: MouseEvent) {
    if (!onmenu) return;
    e.preventDefault();
    onmenu();
  }

  const kindIcon: Record<DeviceKind, string> = { laptop: 'laptop', desktop: 'desktop-tower', phone: 'device-mobile', tablet: 'device-tablet' };
  const icon = $derived(self ? (app.isAndroid ? 'device-mobile' : 'laptop') : kindIcon[device?.kind ?? 'laptop']);
  const name = $derived(self ? selfName || 'This device' : (device?.name ?? ''));
  /** A phone Android put to sleep: it can still be sent to, it's woken up first. */
  const asleep = $derived(!self && !!device && !device.reachable && device.wakeable);
  const offline = $derived(!self && device ? !device.reachable && !device.wakeable : false);
  const ringR = $derived(size / 2 + 5);
  const ringC = $derived(2 * Math.PI * ringR);

  let circle: HTMLElement | undefined = $state();
  let scaleEl: HTMLElement | undefined = $state();

  const pulseSpring = new SpringValue(1, (v) => {
    if (scaleEl) scaleEl.style.transform = `scale(${v})`;
  });

  // Bouncy pulse whenever this device's pulse counter goes up.
  let lastPulse = -1;
  $effect(() => {
    const n = device ? (app.pulse[device.id] ?? 0) : 0;
    if (lastPulse >= 0 && n > lastPulse) {
      pulseSpring.set(1);
      pulseSpring.to(1, 'bouncy', 3.2);
    }
    lastPulse = n;
  });

  // Register as a magnet target for drag and drop.
  $effect(() => {
    if (!register || !device || !circle) return;
    const id = device.id;
    const el = circle;
    if (device.reachable || device.wakeable) app.orbEls.set(id, el);
    else app.orbEls.delete(id);
    return () => {
      if (app.orbEls.get(id) === el) app.orbEls.delete(id);
    };
  });

  const status = $derived(
    self ? 'This device' : offline ? 'offline' : asleep ? 'asleep, sending wakes it up' : progress !== null ? `${Math.round(progress * 100)} percent` : waiting ? 'waiting' : device?.trusted ? 'trusted device, ready' : 'ready',
  );
</script>

<button
  class="orb"
  class:target
  class:snapped
  class:offline
  class:asleep
  class:self
  class:ghost
  style:--size="{size}px"
  disabled={self || offline || !onclick}
  aria-label="{name}, {status}"
  data-testid={testid ?? (self ? 'orb-self' : `orb-${device?.id}`)}
  data-state={snapped ? 'snapped' : target ? 'target' : waiting ? 'waiting' : progress !== null ? 'busy' : offline ? 'offline' : 'idle'}
  onclick={click}
  oncontextmenu={menu}
  onpointerdown={pressDown}
  onpointermove={pressMove}
  onpointerup={pressEnd}
  onpointercancel={pressEnd}
>
  <span class="scale" bind:this={scaleEl}>
    <span class="circle" bind:this={circle}>
      <Icon name={icon} size={Math.round(size * 0.42)} weight={snapped || target ? 'fill' : 'duotone'} />
      {#if device?.trusted && !self}
        <span class="trusted" title="Trusted"><Icon name="shield-check" size={12} weight="fill" /></span>
      {/if}
    </span>
    <svg class="rings" width={size + 16} height={size + 16} viewBox="0 0 {size + 16} {size + 16}" aria-hidden="true">
      {#if waiting}
        <circle class="wait" cx={size / 2 + 8} cy={size / 2 + 8} r={ringR} />
      {/if}
      {#if progress !== null}
        <circle class="track" cx={size / 2 + 8} cy={size / 2 + 8} r={ringR} />
        <circle
          class="progress"
          cx={size / 2 + 8}
          cy={size / 2 + 8}
          r={ringR}
          stroke-dasharray={ringC}
          stroke-dashoffset={ringC * (1 - Math.min(1, Math.max(0, progress)))}
        />
      {/if}
      <circle class="snapring" cx={size / 2 + 8} cy={size / 2 + 8} r={ringR} />
    </svg>
    {#if progress !== null && progress > 0.01 && progress < 0.995}
      <!-- A bolt rides the head of the progress ring. -->
      <span class="ringhead" style:transform="rotate({Math.min(1, progress) * 360}deg)" aria-hidden="true">
        <span><Bolt size={12} /></span>
      </span>
    {/if}
  </span>
  {#if showName}
    <span class="name">{name}</span>
    {#if offline}<span class="sub">{t('home.orb.offline')}</span>{:else if asleep}<span class="sub">{t('home.orb.asleep')}</span>{/if}
  {/if}
</button>

<style>
  .ghost .circle {
    background: transparent;
    box-shadow: none;
  }
  .ghost .circle > :global(:not(.trusted)) {
    opacity: 0;
  }
  .ghost .name {
    text-shadow: 0 1px 2px var(--color-canvas);
  }
  .orb {
    display: grid;
    justify-items: center;
    gap: var(--space-3);
    width: calc(var(--size) + 56px);
    max-width: 120px;
    color: var(--color-accent-text);
    -webkit-tap-highlight-color: transparent;
  }
  .orb:disabled {
    cursor: default;
  }
  .scale {
    position: relative;
    display: grid;
    place-items: center;
    width: var(--size);
    height: var(--size);
    will-change: transform;
  }
  .circle {
    position: relative;
    display: grid;
    place-items: center;
    width: var(--size);
    height: var(--size);
    border-radius: 50%;
    background: var(--color-surface-raised);
    box-shadow: var(--elevation-2);
    transition:
      transform var(--spring-snappy-duration) var(--spring-snappy),
      box-shadow var(--spring-smooth-duration) var(--spring-smooth),
      background-color var(--duration-fast) var(--ease-standard),
      color var(--duration-fast) var(--ease-standard);
  }
  .self .circle {
    background: var(--color-accent);
    color: var(--color-on-accent);
    box-shadow: var(--elevation-3);
  }
  .orb:not(:disabled):active .circle {
    transform: scale(var(--scale-pressed));
  }
  @media (hover: hover) {
    .orb:not(:disabled):hover .circle {
      transform: scale(var(--scale-lifted));
      box-shadow: var(--elevation-3);
    }
  }
  .target .circle {
    transform: scale(var(--scale-drop-target-hover));
    box-shadow: var(--elevation-3);
  }
  .snapped .circle {
    transform: scale(var(--scale-drop-target-hover));
    background: var(--color-accent);
    color: var(--color-on-accent);
    box-shadow: var(--elevation-4);
  }
  .offline {
    opacity: 0.4;
  }
  .asleep .circle {
    opacity: 0.7;
  }
  .trusted {
    position: absolute;
    right: -2px;
    bottom: -2px;
    display: grid;
    place-items: center;
    width: 22px;
    height: 22px;
    border-radius: 50%;
    background: var(--color-surface-raised);
    color: var(--color-success);
    box-shadow: var(--elevation-1);
  }
  .rings {
    position: absolute;
    inset: -8px;
    pointer-events: none;
    overflow: visible;
  }
  .rings circle {
    fill: none;
    stroke-width: 3;
    transform-origin: center;
  }
  .track {
    stroke: var(--color-beam-track);
  }
  .progress {
    stroke: var(--color-beam-core);
    stroke-linecap: round;
    transform: rotate(-90deg);
    transition: stroke-dashoffset var(--spring-smooth-duration) var(--spring-smooth);
  }
  .ringhead {
    position: absolute;
    inset: -8px;
    pointer-events: none;
    transition: transform var(--spring-smooth-duration) var(--spring-smooth);
  }
  .ringhead > span {
    position: absolute;
    left: 50%;
    top: 3px;
    translate: -50% -50%;
  }
  .wait {
    stroke: var(--color-accent);
    animation: breathe var(--duration-searching-pulse) var(--ease-standard) infinite;
  }
  .snapring {
    stroke: var(--color-accent);
    opacity: 0;
    transform: scale(0.9);
    transition:
      opacity var(--duration-fast) var(--ease-standard),
      transform var(--spring-snappy-duration) var(--spring-snappy);
  }
  .snapped .snapring {
    opacity: 1;
    transform: scale(1.12);
  }
  @keyframes breathe {
    0%,
    100% {
      opacity: 0.25;
    }
    50% {
      opacity: 1;
    }
  }
  :global([data-reduced='true']) .wait {
    animation: none;
    opacity: 0.8;
  }
  .name {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--type-caption-size);
    line-height: var(--type-caption-line);
    font-weight: var(--type-caption-weight);
    color: var(--color-text-primary);
  }
  .sub {
    margin-top: calc(var(--space-3) * -1);
    font-size: var(--type-caption-size);
    line-height: var(--type-caption-line);
    color: var(--color-text-secondary);
  }
</style>
