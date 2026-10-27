<script lang="ts">
  import type { TransitionConfig } from 'svelte/transition';
  import { app } from './lib/state/app.svelte';
  import { prefs } from './lib/state/prefs.svelte';
  import { listenForTaps } from './lib/sounds.svelte';
  import { springEasing } from './lib/motion/physics';
  import { beginDrag, endDrag, moveDrag } from './lib/drag';
  import { onFileDrop } from './lib/shell';
  import Onboarding from './screens/Onboarding.svelte';
  import Home from './screens/Home.svelte';
  import Settings from './screens/Settings.svelte';
  import History from './screens/History.svelte';
  import Licenses from './screens/Licenses.svelte';
  import DragLayer from './components/DragLayer.svelte';
  import IncomingLayer from './components/IncomingLayer.svelte';
  import Sheets from './components/Sheets.svelte';
  import Toasts from './components/Toasts.svelte';
  import SplashIntro from './components/SplashIntro.svelte';

  prefs.init();
  app.init();
  $effect(() => listenForTaps());

  /** The launch intro plays once, over the first screen. */
  let intro = $state(true);

  let dragLayer: ReturnType<typeof DragLayer> | undefined = $state();
  $effect(() => {
    app.dragLayer = dragLayer ?? null;
  });

  // Theme and motion live on <html> so tokens.css and every component can react.
  $effect(() => {
    const root = document.documentElement;
    root.dataset.theme = prefs.dark ? 'dark' : 'light';
    root.dataset.motion = prefs.reducedMotion ? 'reduced' : 'full';
    root.dataset.reduced = String(prefs.reducedMotion);
    root.dataset.platform = app.isAndroid ? 'android' : 'desktop';
  });

  // Screen transitions: forward slides in 24 from the right on the gentle spring while the old
  // screen fades and shifts 8 the other way. Back is the mirror image.
  function screenIn(_n: Element): TransitionConfig {
    if (prefs.reducedMotion) return { duration: 150, css: (t) => `opacity:${t}` };
    const { duration, easing } = springEasing('gentle');
    const d = app.navDir;
    return { duration, css: (t) => `opacity:${Math.min(1, t * 2.2)};transform:translateX(${d * 24 * (1 - easing(t))}px)` };
  }
  function screenOut(_n: Element): TransitionConfig {
    if (prefs.reducedMotion) return { duration: 120, css: (t) => `opacity:${t}` };
    const d = app.navDir;
    return { duration: 150, css: (t, u) => `opacity:${t};transform:translateX(${-d * 8 * u}px)` };
  }

  // ---------- Desktop: files dragged in from the file manager ----------
  const canDrop = () => app.screen === 'home' && !app.sheet && !app.requests.length;

  // In the desktop app, Tauri reports the drag with the files' real paths (the browser drag events
  // below don't fire there). The drop animation starts right away; the files are read alongside.
  $effect(() => {
    if (!app.native) return;
    let off: (() => void) | undefined;
    let gone = false;
    void onFileDrop({
      enter: (x, y, count) => {
        if (canDrop() && !app.drag) beginDrag('os', x, y, count);
      },
      over: (x, y) => {
        if (app.drag?.source === 'os') moveDrag(x, y);
      },
      leave: () => {
        if (app.drag?.source !== 'os') return;
        endDrag();
        app.dragLayer?.cancel();
      },
      drop: async (x, y, paths) => {
        if (app.drag?.source !== 'os') return;
        moveDrag(x, y);
        const target = endDrag();
        const staging = app.stagePaths(paths);
        if (target) {
          void app.dragLayer?.land(target);
          if ((await staging).length) app.sendTo(target);
        } else {
          void app.dragLayer?.cancel();
          await staging;
          if (app.staged.length) app.sheet = { kind: 'send' };
        }
      },
    }).then((u) => (gone ? u() : (off = u)));
    return () => {
      gone = true;
      off?.();
    };
  });

  // ---------- Browser prototype: files dragged in, through the browser's own drag events ----------
  // The desktop app ignores these. WebKit sends them too, without the files (Tauri has those), and
  // its drop came before Tauri's and ended the drag, so dropped files went nowhere.
  let depth = 0;
  const hasFiles = (e: DragEvent) => !app.native && !!e.dataTransfer && [...e.dataTransfer.types].includes('Files');

  function dragenter(e: DragEvent) {
    if (!hasFiles(e) || !canDrop()) return;
    e.preventDefault();
    depth++;
    if (!app.drag) beginDrag('os', e.clientX, e.clientY, e.dataTransfer?.items.length || 1);
  }
  function dragover(e: DragEvent) {
    if (app.native || !app.drag || app.drag.source !== 'os') return;
    e.preventDefault();
    if (e.dataTransfer) e.dataTransfer.dropEffect = 'copy';
    moveDrag(e.clientX, e.clientY, e.dataTransfer?.items.length || undefined);
  }
  function dragleave(e: DragEvent) {
    if (app.native || !app.drag || app.drag.source !== 'os') return;
    depth = Math.max(0, depth - 1);
    if (depth === 0) {
      endDrag();
      app.dragLayer?.cancel();
    }
  }
  async function drop(e: DragEvent) {
    if (app.native || !app.drag || app.drag.source !== 'os') return;
    e.preventDefault();
    depth = 0;
    const target = endDrag();
    const files = e.dataTransfer?.files;
    if (files?.length) app.stageFromFileList(files);
    if (target) {
      app.sendTo(target);
      await app.dragLayer?.land(target);
    } else {
      app.dragLayer?.cancel();
      if (app.staged.length) app.sheet = { kind: 'send' };
    }
  }
</script>

<svelte:window ondragenter={dragenter} ondragover={dragover} ondragleave={dragleave} ondrop={drop} />

<div class="screens">
  {#key app.screen}
    <div class="screen" in:screenIn out:screenOut>
      {#if app.screen === 'onboarding'}
        <Onboarding />
      {:else if app.screen === 'settings'}
        <Settings />
      {:else if app.screen === 'history'}
        <History />
      {:else if app.screen === 'licenses'}
        <Licenses />
      {:else}
        <Home />
      {/if}
    </div>
  {/key}
</div>

<DragLayer bind:this={dragLayer} />
<IncomingLayer />
<Sheets />
<Toasts />
<!-- The prototype's scenario panel and Motion lab load only in the browser prototype, so the
     desktop app's bundle doesn't carry them. -->
{#if app.mock}
  {#await import('./components/DevPanel.svelte') then { default: DevPanel }}<DevPanel mock={app.mock} />{/await}
{/if}
{#if intro}<SplashIntro ondone={() => (intro = false)} />{/if}

<style>
  /* Window height rather than 100%: while two screens swap they share this grid cell, and WebKit
     (the Linux app) kept the taller one's height afterwards, leaving empty space to scroll into. */
  .screens {
    display: grid;
    min-height: 100vh;
    min-height: 100dvh;
  }
  .screen {
    grid-area: 1 / 1;
    min-width: 0;
  }
</style>
