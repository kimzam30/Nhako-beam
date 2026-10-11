<script lang="ts">
  import { radarSlots } from '../lib/motion/physics';
  import { springIn, quickOut, stagger } from '../lib/motion/transitions';
  import { app } from '../lib/state/app.svelte';
  import { prefs } from '../lib/state/prefs.svelte';
  import { t } from '../lib/copy';
  import { percent } from '../lib/format';
  import DeviceOrb from './DeviceOrb.svelte';

  interface Props {
    size: number;
    onpick: (deviceId: string) => void;
    onmore: () => void;
  }

  let { size, onpick, onmore }: Props = $props();

  const MAX = 8;
  const shown = $derived(app.radarDevices.slice(0, MAX));
  const extra = $derived(Math.max(0, app.radarDevices.length - MAX));
  const slots = $derived(radarSlots(shown.length, size));
  const searching = $derived(app.discovery === 'searching');
  const orbSize = $derived(size < 380 ? 52 : 64);
  const selfSize = $derived(size < 380 ? 72 : 96);

  // Stagger only the first batch so later arrivals pop in immediately.
  let seen = 0;
  function delayFor(i: number) {
    const d = seen < 6 ? stagger(i) : 0;
    seen++;
    return d;
  }
</script>

<div class="radar" class:dragging={!!app.drag} style:--radar="{size}px" data-testid="radar" data-discovery={app.discovery}>
  <div class="rings" aria-hidden="true">
    <span class="guide g1"></span>
    <span class="guide g2"></span>
    {#if (searching || app.devices.length === 0) && !prefs.reducedMotion}
      <span class="sonar s1" class:slow={!searching}></span>
      <span class="sonar s2" class:slow={!searching}></span>
    {/if}
  </div>

  <div class="center">
    <DeviceOrb self selfName={app.selfName} size={selfSize} showName={false} />
  </div>

  {#each shown as device, i (device.id)}
    {@const slot = slots[i] ?? { x: 0, y: 0 }}
    {@const tr = app.transferFor(device.id)}
    <div class="slot" style:transform="translate({slot.x}px, {slot.y}px)" style:--half="{orbSize / 2}px">
      <div in:springIn={{ preset: 'bouncy', scale: 0.6, y: 0, opacity: 0.4, delay: delayFor(i) }} out:quickOut={{ scale: 0.9, y: 0 }}>
        <DeviceOrb
          {device}
          size={orbSize}
          register
          target={app.drag?.nearId === device.id && app.drag?.snappedId !== device.id}
          snapped={app.drag?.snappedId === device.id}
          waiting={tr?.status === 'waiting'}
          progress={tr && tr.status !== 'waiting' ? percent(tr.doneBytes, tr.totalBytes) / 100 : null}
          onclick={() => onpick(device.id)}
        />
      </div>
    </div>
  {/each}

  {#if extra > 0}
    <button class="more" onclick={onmore} in:springIn={{ preset: 'smooth' }} data-testid="more-devices">{t('home.moreDevices', { count: extra })}</button>
  {/if}

  <p class="status" aria-live="polite">
    {#if searching}{t('home.searching')}{/if}
  </p>
</div>

<style>
  .radar {
    position: relative;
    width: var(--radar);
    height: var(--radar);
    margin: 0 auto;
    flex: none;
  }
  .rings,
  .center {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
  }
  /* A zero-size point at the radar center, moved to its slot. Only the orb takes space. */
  .slot {
    position: absolute;
    left: 50%;
    top: 50%;
    width: 0;
    height: 0;
    pointer-events: none;
    transition: transform var(--spring-smooth-duration) var(--spring-smooth);
  }
  /* Anchor the orb so its circle center (not the whole button with its label) sits on the point.
     Uses the `translate` property so it stacks with the entrance `transform`. */
  .slot > :global(div) {
    position: absolute;
    left: 0;
    top: 0;
    translate: -50% calc(var(--half) * -1);
    pointer-events: auto;
  }
  .guide {
    position: absolute;
    border-radius: 50%;
    border: 1px solid var(--color-separator);
  }
  .g1 {
    width: 66%;
    height: 66%;
  }
  .g2 {
    width: 100%;
    height: 100%;
    opacity: 0.6;
  }
  .sonar {
    position: absolute;
    width: 34%;
    height: 34%;
    border-radius: 50%;
    border: 1.5px solid var(--color-accent);
    opacity: 0;
    animation: sonar var(--duration-radar-sweep) var(--ease-enter) infinite;
  }
  .s2 {
    animation-delay: calc(var(--duration-radar-sweep) / 2);
  }
  .sonar.slow {
    animation-duration: calc(var(--duration-radar-sweep) * 1.6);
  }
  @keyframes sonar {
    0% {
      transform: scale(0.9);
      opacity: 0.55;
    }
    100% {
      transform: scale(2.9);
      opacity: 0;
    }
  }
  .dragging .guide {
    border-color: var(--color-accent-subtle);
  }
  .more {
    position: absolute;
    left: 50%;
    bottom: -8px;
    translate: -50% 0;
    padding: var(--space-2) var(--space-4);
    border-radius: var(--radius-full);
    background: var(--color-fill);
    font-size: var(--type-footnote-size);
    font-weight: 500;
  }
  .status {
    position: absolute;
    left: 0;
    right: 0;
    bottom: calc(50% - 82px);
    text-align: center;
    font-size: var(--type-footnote-size);
    color: var(--color-text-secondary);
    pointer-events: none;
  }
</style>
