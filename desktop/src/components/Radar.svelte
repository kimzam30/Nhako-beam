<script lang="ts">
  // Home's stage: you (the voxel bolt) in the middle of a 3D floor, your devices standing around
  // you as small voxel devices, ripples while it searches, an arc to each device that turns up,
  // and sparks while files move (lib/stage/scene.ts). Each device keeps a real button on top of
  // its drawing (DeviceOrb, see-through once the 3D is there), placed with the same camera
  // (lib/stage/view.ts), so clicking, dropping files, the keyboard and screen readers work as
  // before. Until the 3D has loaded, or without WebGL, it's the flat radar.
  import { onMount } from 'svelte';
  import { radarSlots } from '../lib/motion/physics';
  import { springIn, quickOut, stagger } from '../lib/motion/transitions';
  import { app } from '../lib/state/app.svelte';
  import { prefs } from '../lib/state/prefs.svelte';
  import { t } from '../lib/copy';
  import { percent } from '../lib/format';
  import { homeCamera, project, slotOnFloor, STAGE } from '../lib/stage/view';
  import { mascotMood, nextChange } from '../lib/stage/mood';
  import type { Stage } from '../lib/stage/scene';
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

  // ---------- The 3D stage ----------

  const cam = homeCamera();
  /** The stage is a little wider than tall: the floor, seen from above at an angle, is. */
  const height = $derived(Math.round(size * STAGE.heightRatio));
  /** The middle of a device, above the floor, where its button goes. */
  const DEVICE_MID = 0.55;
  let canvas: HTMLCanvasElement | undefined = $state();
  let stage = $state<Stage | null>(null);

  onMount(() => {
    let gone = false;
    let made: Stage | null = null;
    void import('../lib/stage/scene')
      .then((m) => m.createStage(canvas!, { camera: cam, home: true, mascotScale: STAGE.mascotScale, reduced: prefs.reducedMotion }))
      .then((s) => {
        if (gone) return s.destroy();
        made = s;
        s.resize(size, height);
        stage = s;
      })
      .catch((e) => console.warn('The 3D stage is not available; the flat radar stays:', e));
    return () => {
      gone = true;
      made?.destroy();
    };
  });

  /** Each device: where it stands on the floor, and where its button goes on screen. */
  const places = $derived(
    shown.map((d, i) => {
      const at = slotOnFloor(slots[i] ?? { x: 0, y: 0 }, size);
      const p = project(cam, [at[0], DEVICE_MID, at[2]], size, height);
      return { id: d.id, at, x: stage ? p.x - size / 2 : (slots[i]?.x ?? 0), y: stage ? p.y - height / 2 : (slots[i]?.y ?? 0) };
    }),
  );
  const self = $derived(project(cam, [0, 3 * STAGE.mascotScale * 0.45, 0], size, height));

  let hovered = $state<string | null>(null);
  let pointer = $state<{ x: number; y: number } | null>(null);
  const lookAt = $derived(app.drag?.nearId ?? hovered);

  $effect(() => stage?.resize(size, height));
  $effect(() => stage?.setReduced(prefs.reducedMotion));
  // A sheet covers the stage: it holds still underneath until the sheet goes.
  $effect(() => stage?.setPaused(!!app.sheet));
  $effect(() => {
    void prefs.dark;
    stage?.retheme();
  });
  $effect(() => {
    stage?.setDevices(
      shown.map((d, i) => ({
        id: d.id,
        kind: d.kind,
        at: places[i]!.at,
        reachable: d.reachable || d.wakeable,
        lifted: lookAt === d.id,
      })),
    );
  });

  // Where the bolt looks: the device under the pointer (or the files), else the pointer.
  $effect(() => {
    if (!stage) return;
    const i = shown.findIndex((d) => d.id === lookAt);
    const place = i >= 0 ? places[i] : undefined;
    if (place) stage.setLook({ x: Math.max(-1, Math.min(1, place.x / (size / 2))), y: Math.max(-1, Math.min(1, place.y / (height / 2))) });
    else stage.setLook(pointer);
  });

  // The bolt's mood. Moments (found, done, failed) run out by themselves, so check again then.
  let now = $state(Date.now());
  const moving = $derived(app.activeTransfers.find((x) => x.status === 'transferring' || x.status === 'connecting' || x.status === 'reconnecting'));
  const moodInput = $derived({
    now,
    started: app.engineReady,
    offline: app.discovery === 'offline',
    searching,
    devices: shown.length,
    sending: moving?.direction === 'send',
    receiving: moving?.direction === 'receive',
    looking: !!lookAt,
    moments: app.moments,
  });
  const mood = $derived(mascotMood(moodInput));
  $effect(() => {
    const at = nextChange(moodInput);
    if (at === null) return;
    const timer = setTimeout(() => (now = Date.now()), at - Date.now() + 20);
    return () => clearTimeout(timer);
  });
  $effect(() => {
    // A fresh moment: look at the clock again now.
    void app.moments.found;
    void app.moments.done;
    void app.moments.failed;
    now = Date.now();
  });
  $effect(() => stage?.setMood(mood));
  $effect(() => stage?.setTransferDevice(moving?.deviceId ?? null));

  // A device that turns up while you watch gets a lightning arc from you to it (not the ones
  // already there when Home opens: the stage isn't there yet for those).
  let arcs = $state(0);
  const known = new Set<string>();
  $effect(() => {
    const ids = shown.map((d) => d.id);
    for (const device of shown) {
      if (known.has(device.id)) continue;
      known.add(device.id);
      if (!stage || prefs.reducedMotion || !(device.reachable || device.wakeable)) continue;
      const s = stage;
      setTimeout(() => s.arc(device.id), 120);
      arcs++;
    }
    for (const id of [...known]) if (!ids.includes(id)) known.delete(id);
  });

  // Stagger only the first batch so later arrivals pop in immediately.
  let seen = 0;
  function delayFor(i: number) {
    const d = seen < 6 ? stagger(i) : 0;
    seen++;
    return d;
  }

  /** Which way is "out" for a slot, so a device that leaves drifts away from you. */
  const outward = (x: number, y: number, by = 14) => {
    const len = Math.hypot(x, y) || 1;
    return { x: (x / len) * by, y: (y / len) * by };
  };

  function track(e: PointerEvent) {
    const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
    pointer = { x: ((e.clientX - r.left) / r.width) * 2 - 1, y: ((e.clientY - r.top) / r.height) * 2 - 1 };
  }

  const MOOD_WORDS: Record<string, string> = {
    idle: 'ready',
    looking: 'looking around',
    happy: 'happy',
    sending: 'sending',
    receiving: 'receiving',
    done: 'done',
    failed: 'something went wrong',
    asleep: 'asleep',
    offline: 'offline',
    loading: 'starting',
  };
</script>

<div
  class="radar"
  class:dragging={!!app.drag}
  class:stage={!!stage}
  style:--radar="{size}px"
  style:--radar-h="{stage ? height : size}px"
  data-testid="radar"
  data-discovery={app.discovery}
  data-mood={mood}
  data-arcs={arcs}
  data-quality={stage?.quality}
  role="presentation"
  onpointermove={track}
  onpointerleave={() => (pointer = null)}
>
  <canvas bind:this={canvas} class="scene" style:width="{size}px" style:height="{height}px" aria-hidden="true"></canvas>

  {#if !stage}
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
  {:else}
    <span
      class="me"
      style:left="{self.x}px"
      style:top="{self.y}px"
      role="img"
      aria-label="{app.selfName || 'This device'}, this device, {MOOD_WORDS[mood]}"
      data-testid="mascot"
      data-state={mood}
    ></span>
  {/if}

  {#each shown as device, i (device.id)}
    {@const place = places[i] ?? { x: 0, y: 0 }}
    {@const tr = app.transferFor(device.id)}
    <div class="slot" style:transform="translate({place.x}px, {place.y}px)" style:--half="{orbSize / 2}px">
      <div
        in:springIn={{ preset: 'bouncy', scale: 0.6, y: 0, opacity: 0.4, delay: delayFor(i) }}
        out:quickOut={{ scale: 0.9, ...outward(place.x, place.y), duration: 220 }}
        onpointerenter={() => (hovered = device.id)}
        onpointerleave={() => hovered === device.id && (hovered = null)}
        role="presentation"
      >
        <DeviceOrb
          {device}
          size={orbSize}
          register
          ghost={!!stage}
          target={app.drag?.nearId === device.id && app.drag?.snappedId !== device.id}
          snapped={app.drag?.snappedId === device.id}
          waiting={tr?.status === 'waiting'}
          progress={tr && tr.status !== 'waiting' ? percent(tr.doneBytes, tr.totalBytes) / 100 : null}
          onclick={() => onpick(device.id)}
          onmenu={() => (app.sheet = { kind: 'device', deviceId: device.id })}
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
    height: var(--radar-h);
    margin: 0 auto;
    flex: none;
  }
  .scene {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .rings,
  .center {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
  }
  /* Where the bolt stands, for screen readers (the drawing is the canvas under it). */
  .me {
    position: absolute;
    width: 30%;
    height: 40%;
    translate: -50% -50%;
    pointer-events: none;
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
  .stage .status {
    bottom: 4px;
  }
</style>
