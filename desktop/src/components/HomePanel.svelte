<script lang="ts">
  // The buttons over the room. It follows the pointer exactly, throws with momentum (where the throw
  // would land decides where it stops), rubber-bands past its ends, and a touch catches it
  // mid-flight: one continuous position, so nothing ever has to finish first. The Compose twin is
  // HomePanel in android/.../ui/components/Room.kt.
  import { onMount, type Snippet } from 'svelte';
  import { t } from '../lib/copy';
  import type { Device } from '../lib/engine/types';
  import { app } from '../lib/state/app.svelte';
  import { prefs } from '../lib/state/prefs.svelte';
  import Button from './Button.svelte';
  import Icon from './Icon.svelte';
  import IconButton from './IconButton.svelte';

  interface Props {
    staging: boolean;
    /** Devices not in the room, shown as bubbles (a tall window; a wide one has them on the floor). */
    bubbles: Device[];
    /** How far the buttons have gone, 0 to 1. */
    away: number;
    /** How much of the panel shows above the window's bottom edge, in px. */
    shown?: number;
    onfiles: () => void;
    onfolder: () => void;
    onpick: (deviceId: string) => void;
    above?: Snippet;
  }

  let { staging, bubbles, away = $bindable(0), shown = $bindable(0), onfiles, onfolder, onpick, above }: Props = $props();

  const kindIcon: Record<string, string> = { laptop: 'laptop', desktop: 'desktop-tower', phone: 'device-mobile', tablet: 'device-tablet' };
  let tray: HTMLElement | undefined = $state();
  let btns: HTMLElement | undefined = $state();
  let row: HTMLElement | undefined = $state();
  let rvs: HTMLElement[] = [];

  // A spring with a response of .42 s and a little under critical damping, the feel of an iOS sheet.
  const RESPONSE = 0.42;
  const DAMPING = 0.84;
  const K = ((2 * Math.PI) / RESPONSE) ** 2;
  const B = (4 * Math.PI * DAMPING) / RESPONSE;
  let travel = 0;
  let y = 0;
  let v = 0;
  let goal = 0;
  let last = 0;
  let raf = 0;
  let running = false;
  let open = $state(true);

  const clamp01 = (n: number) => Math.max(0, Math.min(1, n));
  function draw() {
    if (!tray) return;
    tray.style.transform = `translate3d(0, ${y.toFixed(2)}px, 0)`;
    const p = travel ? 1 - clamp01(y / travel) : 1;
    away = 1 - p;
    shown = Math.max(0, tray.offsetHeight - y);
    rvs = btns ? [...btns.querySelectorAll<HTMLElement>('.rv')] : [];
    const n = rvs.length, has = bubbles.length > 0;
    // Each button has its own stretch of the travel, so they arrive one after another, even dragged slowly.
    rvs.forEach((el, i) => {
      const base = has ? clamp01((p - 0.35) / 0.65) : p;
      const k = clamp01((base - (n - 1 - i) * 0.055) / (1 - (n - 1) * 0.055)), e = k * k * (3 - 2 * k);
      el.style.opacity = e.toFixed(3);
      el.style.transform = `translateY(${((1 - e) * 16).toFixed(1)}px) scale(${(0.82 + 0.18 * e).toFixed(3)})`;
    });
    row?.querySelectorAll<HTMLElement>('.bub').forEach((b, i) => {
      const k = clamp01(((0.35 - p) / 0.35 - Math.min(i, 5) * 0.08) / 0.6), e = k * k * (3 - 2 * k);
      b.style.opacity = e.toFixed(3);
      b.style.transform = `translateY(${((1 - e) * 14).toFixed(1)}px) scale(${(0.8 + 0.2 * e).toFixed(3)})`;
    });
    if (row) row.style.pointerEvents = p > 0.3 ? 'none' : 'auto';
  }
  function run(now: number) {
    let dt = Math.min(0.05, (now - last) / 1000);
    last = now;
    while (dt > 0) {
      const h = Math.min(dt, 1 / 240);
      v += (-K * (y - goal) - B * v) * h;
      y += v * h;
      dt -= h;
    }
    if (Math.abs(y - goal) < 0.3 && Math.abs(v) < 4) {
      y = goal;
      v = 0;
      running = false;
      draw();
      return;
    }
    draw();
    raf = requestAnimationFrame(run);
  }
  function fling(toOpen: boolean, speed = 0) {
    open = toOpen;
    goal = toOpen ? 0 : travel;
    v = speed;
    if (prefs.reducedMotion) {
      y = goal;
      v = 0;
      draw();
      return;
    }
    if (!running) {
      running = true;
      last = performance.now();
      raf = requestAnimationFrame(run);
    }
  }
  function measure() {
    if (!btns) return;
    const rowH = row && bubbles.length ? row.offsetHeight + 4 : 0;
    travel = bubbles.length ? Math.max(0, btns.offsetHeight - rowH) : btns.offsetHeight + 16;
    if (!open) {
      goal = travel;
      y = travel;
    }
    draw();
  }

  const rubber = (x: number, d: number) => (1 - 1 / ((x * 0.55) / d + 1)) * d;
  let drag: { id: number; y0: number; x0: number; from: number; on: boolean; caught: boolean; lastY: number; t: number; speed: number; moved: boolean; row: boolean; left: number; sideways: boolean } | null = null;
  let scrolled = false;

  function down(e: PointerEvent) {
    const el = e.target as HTMLElement;
    if (e.pointerType === 'mouse' && e.button !== 0) return;
    // The lower part of the room counts too, so a closed panel is easy to bring back.
    const onTray = !!el.closest('[data-panel]');
    const inRoom = !!el.closest('[data-testid="radar"]') && !el.closest('button') && e.clientY > innerHeight * 0.55;
    if ((!onTray && !inRoom) || app.sheet || app.drag || el.closest('[data-testid="staged-stack"]')) return;
    const caught = running;
    cancelAnimationFrame(raf);
    running = false;
    v = 0;
    drag = { id: e.pointerId, y0: e.clientY, x0: e.clientX, from: y, on: !!el.closest('[data-handle]'), caught, lastY: e.clientY, t: performance.now(), speed: 0, moved: false, row: !!el.closest('[data-bubbles]'), left: row?.scrollLeft ?? 0, sideways: false };
  }
  function move(e: PointerEvent) {
    if (!drag || e.pointerId !== drag.id) return;
    const dy = e.clientY - drag.y0, dx = e.clientX - drag.x0;
    // Sideways on the bubbles scrolls them (touch does this by itself; this is for a mouse).
    if (drag.row) {
      if (drag.sideways) {
        if (row) row.scrollLeft = drag.left - dx;
        scrolled = true;
        return;
      }
      if (!drag.moved && Math.abs(dx) > 6 && Math.abs(dx) > Math.abs(dy)) {
        drag.sideways = true;
        return;
      }
    }
    if (!drag.moved) {
      if (Math.abs(dy) < 6) return;
      drag.moved = true;
      drag.y0 = e.clientY;
      suppressClick();
    }
    const raw = drag.from + (e.clientY - drag.y0);
    y = raw < 0 ? -rubber(-raw, 90) : raw > travel ? travel + rubber(raw - travel, 60) : raw;
    const now = performance.now();
    drag.speed = 0.7 * ((e.clientY - drag.lastY) / Math.max(1, now - drag.t)) * 1000 + 0.3 * drag.speed;
    drag.lastY = e.clientY;
    drag.t = now;
    draw();
  }
  function up(e: PointerEvent) {
    if (!drag || e.pointerId !== drag.id) return;
    const d = drag;
    drag = null;
    if (!d.moved) {
      if (d.on && !d.caught) fling(!open);
      else if (y !== goal) fling(open);
      return;
    }
    // Where the throw would carry it decides where it lands (the way iOS projects momentum).
    const speed = performance.now() - d.t > 90 ? 0 : d.speed;
    fling(y + speed * 0.18 <= travel / 2, speed);
  }
  /** A drag that began on a button mustn't also press it when the pointer lifts. */
  function suppressClick() {
    const eat = (ev: Event) => {
      ev.stopPropagation();
      ev.preventDefault();
    };
    addEventListener('click', eat, { capture: true, once: true });
    setTimeout(() => removeEventListener('click', eat, { capture: true }), 700);
  }
  function key(e: KeyboardEvent) {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      fling(false);
    }
    if (e.key === 'ArrowUp') {
      e.preventDefault();
      fling(true);
    }
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      fling(!open);
    }
  }
  function bubbleClick(d: Device) {
    if (scrolled) {
      scrolled = false;
      return;
    }
    onpick(d.id);
  }

  onMount(() => {
    measure();
    const ro = new ResizeObserver(() => (measure(), draw()));
    if (btns) ro.observe(btns);
    if (tray) ro.observe(tray);
    return () => {
      ro.disconnect();
      cancelAnimationFrame(raf);
    };
  });
  // Bubbles coming and going change how far it travels; staged files bring the buttons back.
  $effect(() => {
    void bubbles.length;
    queueMicrotask(measure);
  });
  $effect(() => {
    if (staging && !open) fling(true);
  });
</script>

<svelte:window onpointerdown={down} onpointermove={move} onpointerup={up} onpointercancel={up} />

<div class="panel" bind:this={tray} data-panel data-testid="home-panel" data-open={open}>
  {@render above?.()}
  <button class="handle" data-handle aria-expanded={open} aria-label={t('home.panel')} data-testid="panel-handle" onkeydown={key}><span></span></button>
  <div class="stack">
    <nav class="btns" bind:this={btns} aria-label="Actions" inert={!open}>
      <div class="row1">
        <div class="rv main">
          {#if staging}
            <Button size="lg" icon="paper-plane-tilt" iconWeight="fill" testid="choose-device" onclick={() => (app.sheet = { kind: 'send' })}>{t('send.choose.title')}…</Button>
          {:else}
            <Button size="lg" icon="paper-plane-tilt" iconWeight="fill" testid="send-files" onclick={onfiles}>
              {app.isAndroid ? t('home.action.sendFiles') : t('home.action.chooseFiles')}
            </Button>
          {/if}
        </div>
        {#if !staging}
          <div class="rv round"><IconButton icon="folder-simple" label={t('home.action.folder')} size={22} testid="send-folder" onclick={onfolder} /></div>
          <div class="rv round"><IconButton icon="text-aa" label={t('home.action.text')} size={22} testid="send-text" onclick={() => (app.sheet = { kind: 'compose' })} /></div>
          <div class="rv round"><IconButton icon="clipboard-text" label={t('home.action.clipboard')} size={22} testid="send-clipboard" onclick={() => void app.stageClipboard()} /></div>
        {/if}
      </div>
      <div class="row2">
        <div class="rv"><Button variant="secondary" icon="qr-code" testid="send-code" onclick={() => (app.sheet = { kind: 'showCode' })}>{t('home.action.yourCode')}</Button></div>
        <div class="rv"><Button variant="secondary" icon="download-simple" testid="receive-code" onclick={() => (app.sheet = { kind: app.isAndroid ? 'scan' : 'enterCode' })}>{t(app.isAndroid ? 'home.action.scanCode' : 'home.action.enterCode')}</Button></div>
      </div>
    </nav>
    {#if bubbles.length}
      <div class="bubbles" bind:this={row} data-bubbles aria-label={t('home.devices')} inert={open}>
        {#each bubbles as d (d.id)}
          {@const usable = d.reachable || d.wakeable}
          <button class="bub" disabled={!usable} class:go={app.activeTransfers.some((x) => x.deviceId === d.id)} data-testid="bubble-{d.id}" aria-label={usable ? d.name : `${d.name}, ${t('home.orb.offline')}`} onclick={() => bubbleClick(d)}>
            <i><Icon name={kindIcon[d.kind] ?? 'laptop'} size={23} /></i>
            <small>{d.name}</small>
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .panel {
    position: absolute;
    left: 50%;
    bottom: 0;
    width: min(520px, 100%);
    margin-left: calc(min(520px, 100%) / -2);
    padding: 0 var(--space-4) calc(var(--space-4) + env(safe-area-inset-bottom));
    will-change: transform;
    touch-action: none;
    user-select: none;
    -webkit-user-select: none;
    z-index: 2;
  }
  .handle {
    display: block;
    width: 100%;
    height: 32px;
    cursor: grab;
    touch-action: none;
  }
  .handle:active {
    cursor: grabbing;
  }
  .handle span {
    display: block;
    width: 44px;
    height: 5px;
    margin: auto;
    border-radius: 3px;
    background: #ffffffa8;
    box-shadow: 0 1px 4px #000a;
    transition: width var(--duration-fast), background-color var(--duration-fast);
  }
  .handle:hover span {
    width: 56px;
    background: #fff;
  }
  .stack {
    position: relative;
  }
  .btns {
    display: grid;
    gap: var(--space-3);
  }
  .row1 {
    display: flex;
    gap: var(--space-3);
    align-items: center;
  }
  .row2 {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-3);
  }
  .rv {
    display: grid;
    min-width: 0;
    will-change: transform, opacity;
  }
  .main {
    flex: 1 1 0;
  }
  .round :global(button) {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    background: var(--color-surface-raised);
    color: var(--color-text-primary);
    box-shadow: var(--elevation-2);
  }
  .btns :global(.btn) {
    box-shadow: var(--elevation-2);
  }
  @media (max-width: 380px) {
    .row2 {
      grid-template-columns: 1fr;
    }
  }
  .bubbles {
    position: absolute;
    left: calc(var(--space-4) * -1);
    right: calc(var(--space-4) * -1);
    top: 0;
    display: flex;
    gap: var(--space-2);
    padding: 0 var(--space-4) 6px;
    overflow-x: auto;
    scroll-snap-type: x proximity;
    scroll-padding: var(--space-4);
    scrollbar-width: none;
    touch-action: pan-x;
    overscroll-behavior-x: contain;
  }
  .bubbles::-webkit-scrollbar {
    display: none;
  }
  .bub {
    flex: none;
    width: 70px;
    display: grid;
    justify-items: center;
    gap: 5px;
    scroll-snap-align: start;
    opacity: 0;
    color: #f2f4f7;
    will-change: transform, opacity;
  }
  .bub i {
    width: 48px;
    height: 48px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    background: var(--color-surface-raised);
    color: var(--color-text-primary);
    box-shadow: var(--elevation-2);
    transition: transform var(--spring-press-duration) var(--spring-press), background-color var(--duration-fast);
  }
  .bub:active i {
    transform: scale(0.9);
  }
  .bub.go i {
    background: var(--color-accent);
    color: var(--color-on-accent);
  }
  .bub small {
    max-width: 70px;
    font-size: var(--type-caption-size);
    font-weight: 600;
    line-height: 15px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    text-shadow: 0 1px 3px #000;
  }
  .bub:disabled {
    cursor: default;
    color: #aab2c7;
  }
  .bub:disabled i {
    color: var(--color-text-tertiary);
  }
</style>
