<script lang="ts">
  // Home's room: a person asleep at a desk, your devices on the furniture, the bolt as a trophy on
  // the shelf. Pre-rendered art (design/room/build.mjs) in layers, so only what moves redraws. Each
  // device keeps a real button over its drawing (DeviceOrb), so clicking, dropping files, the
  // keyboard and screen readers work. The Compose twin is android/.../ui/components/Room.kt.
  import { springIn, quickOut, stagger } from '../lib/motion/transitions';
  import { app } from '../lib/state/app.svelte';
  import { prefs } from '../lib/state/prefs.svelte';
  import { percent } from '../lib/format';
  import { mascotMood, nextChange } from '../lib/stage/mood';
  import { deviceArt, fitRoom, ROOM, roomImage, roomTags, type RoomLayout } from '../lib/stage/room';
  import DeviceOrb from './DeviceOrb.svelte';

  interface Props {
    width: number;
    height: number;
    layout: RoomLayout;
    onpick: (deviceId: string) => void;
  }

  let { width, height, layout, onpick }: Props = $props();

  const fit = $derived(fitRoom(width, height));
  const view = $derived(ROOM.views[fit.view]);
  const who = $derived(prefs.roomPerson);
  const sx = (x: number) => fit.ox + x * fit.k;
  const sy = (y: number) => fit.oy + y * fit.k;
  const box = (x: number, y: number, w: number, h: number) => `left:${sx(x)}px;top:${sy(y)}px;width:${w * fit.k}px;height:${h * fit.k}px;`;
  const P = ROOM.person;
  const boltU = ROOM.bolt.size / ROOM.boltLit.unit;

  const shown = $derived(layout.room);
  // Farthest first, so a nearer device stands in front.
  const byDepth = $derived([...shown].sort((a, b) => b.z - a.z));
  const tags = $derived(roomTags(shown, fit, width));

  // ---------- Who's moving, what the sleeper does ----------

  const moving = $derived(app.activeTransfers.find((x) => x.status === 'transferring' || x.status === 'connecting' || x.status === 'reconnecting'));
  const target = $derived(moving ? shown.find((q) => q.device.id === moving.deviceId) : undefined);
  const searching = $derived(app.discovery === 'searching');
  const thread = $derived.by(() => {
    if (!target) return '';
    const ax = sx(ROOM.bolt.threadX), ay = sy(ROOM.bolt.threadY), ex = sx((target.box.x0 + target.box.x1) / 2), ey = sy((target.box.y0 + target.box.y1) / 2);
    return `M${ax} ${ay} Q${(ax + ex) / 2} ${Math.min(ay, ey) - 90 * fit.k} ${ex} ${ey}`;
  });

  let hovered = $state<string | null>(null);
  const lookAt = $derived(app.drag?.nearId ?? hovered);
  let now = $state(Date.now());
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
    void app.moments.found;
    void app.moments.done;
    void app.moments.failed;
    now = Date.now();
  });

  // A device that turns up while you watch: the trophy flashes and the device springs in.
  let arcs = $state(0);
  let flash = $state(false);
  const known = new Set<string>();
  $effect(() => {
    const ids = shown.map((q) => q.device.id);
    for (const { device } of shown) {
      if (known.has(device.id)) continue;
      known.add(device.id);
      if (prefs.reducedMotion || !(device.reachable || device.wakeable)) continue;
      arcs++;
      flash = true;
      setTimeout(() => (flash = false), 500);
    }
    for (const id of [...known]) if (!ids.includes(id)) known.delete(id);
  });
  let seen = 0;
  function delayFor(i: number) {
    const d = seen < 6 ? stagger(i) : 0;
    seen++;
    return d;
  }

  const dust = Array.from({ length: 22 }, (_, i) => ({ u: ((i * 37) % 100) / 100, v: ((i * 61) % 100) / 100, d: 7 + ((i * 13) % 80) / 10, delay: -((i * 29) % 120) / 10 }));
  const rain = Array.from({ length: 22 }, (_, i) => ({ u: ((i * 43) % 100) / 100, d: 0.8 + ((i * 7) % 6) / 10, delay: -((i * 19) % 20) / 10 }));
  const [c0, c1, c2, c3] = ROOM.cone as [number[], number[], number[], number[]];
  const dustAt = (u: number, v: number) => {
    const lx = c0[0]! + (c3[0]! - c0[0]!) * v, rx = c1[0]! + (c2[0]! - c1[0]!) * v;
    return [lx + (rx - lx) * u, c0[1]! + (c3[1]! - c0[1]!) * v] as const;
  };

  const MOOD_WORDS: Record<string, string> = { idle: 'ready', looking: 'looking around', happy: 'happy', sending: 'sending', receiving: 'receiving', done: 'done', failed: 'something went wrong', asleep: 'asleep', offline: 'offline', loading: 'starting' };
</script>

<div
  class="radar room"
  class:awake={!!moving}
  class:dragging={!!app.drag}
  class:recede={!!app.sheet}
  style:--k={fit.k}
  data-testid="radar"
  data-discovery={app.discovery}
  data-mood={mood}
  data-arcs={arcs}
  role="presentation"
>
  <img class="art" alt="" src={roomImage(`room-${who}-${fit.view}`)} style={box(view.x, view.y, view.w, view.h)} />
  <span class="me" style:left="{sx(745)}px" style:top="{sy(633)}px" role="img" aria-label="{app.selfName || 'This device'}, this device, {MOOD_WORDS[mood]}" data-testid="mascot" data-state={mood}></span>

  <!-- Rain falls behind the window's frame, curtains and sill, so it only shows through the glass. It stops with reduced motion. -->
  <div class="window" aria-hidden="true" style={box(ROOM.window.x, ROOM.window.y, ROOM.window.w, ROOM.window.h)}>
    {#each rain as r}<i class="rain" style:left="{r.u * 100}%" style:--d="{r.d}s" style:animation-delay="{r.delay}s"></i>{/each}
  </div>
  <img class="art" alt="" src={roomImage(`front-${who}`)} style={box(ROOM.front.x, ROOM.front.y, ROOM.front.w, ROOM.front.h)} />

  <!-- Devices stand on furniture behind the person, so they're drawn first. -->
  {#each byDepth as spot, i (spot.device.id)}
    <img class="art device" alt="" src={roomImage(deviceArt(spot))} in:springIn={{ preset: 'bouncy', scale: 0.8, y: 0, delay: delayFor(i) }} style={box(spot.art.x, spot.art.y, spot.art.w, spot.art.h)} />
  {/each}

  <img class="art" alt="" src={roomImage(`${who}-under`)} style={box(P.x, P.y, P.w, P.h)} />
  <div class="breath"><img class="art" alt="" src={roomImage(`${who}-torso`)} style={box(P.x, P.y, P.w, P.h)} /></div>
  <div class="head-move"><img class="art" alt="" src={roomImage(`${who}-head`)} style={box(P.x, P.y, P.w, P.h)} /></div>
  <div class="breath"><img class="art" alt="" src={roomImage(`${who}-over`)} style={box(P.x, P.y, P.w, P.h)} /></div>
  <img class="art" alt="" src={roomImage(`${who}-chair`)} style={box(P.x, P.y, P.w, P.h)} />

  <img class="art bolt" class:on={!!moving || flash} alt="" src={roomImage('bolt-lit')} style={box(ROOM.bolt.x + ROOM.boltLit.x * boltU, ROOM.bolt.y + ROOM.boltLit.y * boltU, ROOM.boltLit.w * boltU, ROOM.boltLit.w * boltU)} />

  <!-- What moves by itself. All of it stops with reduced motion. -->
  {#each dust as m}
    {@const [x, y] = dustAt(m.u, m.v)}
    <i class="mote" aria-hidden="true" style:left="{sx(x)}px" style:top="{sy(y)}px" style:--d="{m.d}s" style:animation-delay="{m.delay}s"></i>
  {/each}
  {#each [0, 1, 2] as s}<i class="steam" aria-hidden="true" style:left="{sx(ROOM.steam.x + s * 6)}px" style:top="{sy(ROOM.steam.y - 30)}px" style:animation-delay="{-s * 1.2}s"></i>{/each}
  {#each [0, 1, 2] as s}<i class="zee" aria-hidden="true" style:left="{sx(ROOM.z.x)}px" style:top="{sy(ROOM.z.y)}px" style:animation-delay="{-s * 1.3}s">z</i>{/each}
  {#if thread}
    <svg class="thread" aria-hidden="true" width={width} height={height}><path d={thread} class="glow" /><path d={thread} /></svg>
  {/if}

  {#each shown as spot (spot.device.id)}
    {@const device = spot.device}
    {@const tr = app.transferFor(device.id)}
    {@const orb = Math.round(Math.max(52, (spot.box.y1 - spot.box.y0) * fit.k))}
    <div class="slot" style:left="{sx((spot.box.x0 + spot.box.x1) / 2)}px" style:top="{sy((spot.box.y0 + spot.box.y1) / 2)}px">
      <div
        in:springIn={{ preset: 'bouncy', scale: 0.8, y: 0, opacity: 0.4 }}
        out:quickOut={{ scale: 0.9, duration: 200 }}
        onpointerenter={() => (hovered = device.id)}
        onpointerleave={() => hovered === device.id && (hovered = null)}
        role="presentation"
      >
        <DeviceOrb {device} size={orb} register ghost showName={false} target={app.drag?.nearId === device.id && app.drag?.snappedId !== device.id} snapped={app.drag?.snappedId === device.id} waiting={tr?.status === 'waiting'} progress={tr && tr.status !== 'waiting' ? percent(tr.doneBytes, tr.totalBytes) / 100 : null} onclick={() => onpick(device.id)} onmenu={() => (app.sheet = { kind: 'device', deviceId: device.id })} />
      </div>
    </div>
  {/each}

  <!-- Names last, so no device ever covers one. -->
  {#each tags as tag (tag.id)}
    {@const spot = shown.find((q) => q.device.id === tag.id)!}
    {@const tr = app.transferFor(tag.id)}
    <span class="tag" aria-hidden="true" data-testid="tag-{tag.id}" style:left="{tag.x}px" style:top="{tag.y}px" style:width="{ROOM.tag.w}px">
      <span class:off={!spot.device.reachable && !spot.device.wakeable}>{spot.device.name}{tr && tr.status !== 'waiting' ? ` ${percent(tr.doneBytes, tr.totalBytes)}%` : ''}</span>
    </span>
  {/each}
</div>

<style>
  .room {
    position: absolute;
    inset: 0;
    overflow: hidden;
    background: #060a1e;
    transition: transform var(--spring-smooth-duration) var(--spring-smooth);
  }
  /* Like a sheet on iOS: whatever opens over the room pushes it back a little. */
  .room.recede {
    transform: scale(0.955);
  }
  :global([data-reduced='true']) .room.recede {
    transform: none;
  }
  .art {
    position: absolute;
    display: block;
    max-width: none;
    pointer-events: none;
    user-select: none;
    -webkit-user-drag: none;
  }
  .me {
    position: absolute;
    width: 150px;
    height: 200px;
    translate: -50% -50%;
    pointer-events: none;
  }
  .breath,
  .head-move {
    position: absolute;
    inset: 0;
    pointer-events: none;
    transition: transform 750ms cubic-bezier(0.3, 0.9, 0.3, 1.1);
  }
  .breath {
    animation: room-breath 4.2s ease-in-out infinite;
  }
  .head-move {
    animation: room-nod 4.2s ease-in-out infinite;
  }
  .awake .breath {
    animation: none;
    transform: translateY(calc(-10px * var(--k)));
  }
  .awake .head-move {
    animation: none;
    transform: translateY(calc(-36px * var(--k)));
  }
  @keyframes room-breath {
    50% {
      transform: translateY(calc(-2px * var(--k)));
    }
  }
  @keyframes room-nod {
    50% {
      transform: translateY(calc(-5px * var(--k)));
    }
  }
  .bolt {
    opacity: 0;
    transition: opacity var(--duration-slow) var(--ease-standard);
  }
  .bolt.on {
    opacity: 1;
  }
  .window {
    position: absolute;
    overflow: hidden;
    pointer-events: none;
  }
  .rain {
    position: absolute;
    top: -20%;
    width: 1px;
    height: 7%;
    background: #8fb2f0;
    opacity: 0.6;
    rotate: 6deg;
    animation: room-rain var(--d, 1s) linear infinite;
  }
  @keyframes room-rain {
    to {
      transform: translate(-30px, 1700%);
    }
  }
  .mote {
    position: absolute;
    width: 2px;
    height: 2px;
    border-radius: 50%;
    background: #ffe9ad;
    opacity: 0;
    pointer-events: none;
    animation: room-dust var(--d, 8s) ease-in-out infinite;
  }
  @keyframes room-dust {
    25% {
      opacity: 0.9;
    }
    75% {
      opacity: 0.7;
    }
    100% {
      transform: translate(14px, -70px);
      opacity: 0;
    }
  }
  .steam {
    position: absolute;
    width: 2px;
    height: calc(30px * var(--k));
    border-radius: 2px;
    background: #fff3d2;
    opacity: 0;
    pointer-events: none;
    animation: room-steam 3.6s ease-out infinite;
  }
  @keyframes room-steam {
    30% {
      opacity: 0.45;
    }
    100% {
      transform: translateY(calc(-30px * var(--k))) scaleX(1.6);
      opacity: 0;
    }
  }
  .zee {
    position: absolute;
    font: 700 calc(15px * var(--k)) / 1 var(--font-sans, system-ui);
    font-style: normal;
    color: #dbe7ff;
    opacity: 0;
    pointer-events: none;
    animation: room-z 4s ease-out infinite;
  }
  .awake .zee {
    display: none;
  }
  @keyframes room-z {
    30% {
      opacity: 1;
    }
    100% {
      transform: translate(22px, -42px) scale(1.15);
      opacity: 0;
    }
  }
  .thread {
    position: absolute;
    inset: 0;
    pointer-events: none;
  }
  .thread path {
    fill: none;
    stroke: #fff1a8;
    stroke-width: 3;
    stroke-linecap: round;
    stroke-dasharray: 3 15;
    animation: room-thread 0.7s linear infinite;
  }
  .thread .glow {
    stroke: #ffd23f;
    stroke-width: 8;
    opacity: 0.3;
  }
  @keyframes room-thread {
    to {
      stroke-dashoffset: -18;
    }
  }
  :global([data-reduced='true']) .room * {
    animation: none !important;
  }
  :global([data-reduced='true']) .mote {
    opacity: 0.6;
  }
  :global([data-reduced='true']) .rain,
  :global([data-reduced='true']) .steam,
  :global([data-reduced='true']) .zee {
    display: none;
  }
  .tag {
    position: absolute;
    display: flex;
    justify-content: center;
    pointer-events: none;
  }
  .tag > span {
    max-width: 100%;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    padding: 3px var(--space-2);
    border-radius: var(--radius-full);
    background: #10132a;
    color: #f2f5ff;
    box-shadow: inset 0 0 0 0.7px #6e7ba3;
    font-size: var(--type-caption-size);
    line-height: var(--type-caption-line);
    font-weight: 600;
  }
  .tag > span.off {
    color: #aab2c7;
  }
  /* A zero-size point in the middle of the device. Only its button takes space. */
  .slot {
    position: absolute;
    width: 0;
    height: 0;
    pointer-events: none;
  }
  .slot > :global(div) {
    position: absolute;
    left: 0;
    top: 0;
    translate: -50% -50%;
    pointer-events: auto;
  }
</style>
