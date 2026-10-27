<script lang="ts">
  // The tray popover (desktop app only, 360 x 520): your devices, what's moving, requests waiting,
  // and "Open Nhako Beam". Click a device to pick files for it, or drop files on it. Files dropped
  // anywhere else go to the main window, which asks where to send them. src-tauri/src/popover.rs.
  import { invoke } from '@tauri-apps/api/core';
  import { message } from './lib/engine/tauri';
  import { app } from './lib/state/app.svelte';
  import { prefs } from './lib/state/prefs.svelte';
  import { listenForTaps } from './lib/sounds.svelte';
  import { onFileDrop } from './lib/shell';
  import { t } from './lib/copy';
  import { formatBytes, summarize } from './lib/format';
  import { springIn, quickOut, springFlip, stagger } from './lib/motion/transitions';
  import Button from './components/Button.svelte';
  import DeviceOrb from './components/DeviceOrb.svelte';
  import Icon from './components/Icon.svelte';
  import Toasts from './components/Toasts.svelte';
  import TransferCard from './components/TransferCard.svelte';

  prefs.init();
  app.init({ popover: true });
  $effect(() => listenForTaps());

  $effect(() => {
    const root = document.documentElement;
    root.dataset.theme = prefs.dark ? 'dark' : 'light';
    root.dataset.motion = prefs.reducedMotion ? 'reduced' : 'full';
    root.dataset.reduced = String(prefs.reducedMotion);
    root.dataset.platform = 'desktop';
  });

  const devices = $derived(app.radarDevices.filter((d) => d.reachable).slice(0, 8));
  const moving = $derived(app.activeTransfers.slice(0, 2));
  const status = $derived(
    app.discovery === 'offline' ? t('home.banner.offline') : app.discovery === 'searching' ? t('home.searching') : t('home.title'),
  );

  let dropOn = $state<string | null>(null);

  function deviceAt(x: number, y: number): string | null {
    const el = document.elementFromPoint(x, y)?.closest<HTMLElement>('[data-device]');
    return el?.dataset.device ?? null;
  }

  $effect(() => {
    let off: (() => void) | undefined;
    let gone = false;
    void onFileDrop({
      enter: (x, y) => (dropOn = deviceAt(x, y)),
      over: (x, y) => (dropOn = deviceAt(x, y)),
      leave: () => (dropOn = null),
      drop: async (x, y, paths) => {
        const target = deviceAt(x, y);
        dropOn = null;
        if (!target) return void invoke('hand_to_main', { paths }).catch((e) => app.toast(message(e), 'danger'));
        const files = await app.stagePaths(paths);
        if (files.length) app.sendTo(target, files);
      },
    }).then((u) => (gone ? u() : (off = u)));
    return () => {
      gone = true;
      off?.();
    };
  });

  // Each time the tray opens it, the popover drops in from the tray on the snappy spring.
  let pop: HTMLElement | undefined = $state();
  let lastDrop = 0;
  function drop() {
    const now = performance.now();
    if (!pop || now - lastDrop < 600) return;
    lastDrop = now;
    const keys = prefs.reducedMotion
      ? [{ opacity: 0.4 }, { opacity: 1 }]
      : [{ opacity: 0.4, transform: 'translateY(-10px) scale(0.98)' }, { opacity: 1, transform: 'none' }];
    pop.animate(keys, { duration: 220, easing: 'cubic-bezier(0.2, 0, 0, 1)' });
  }

  async function sendTo(deviceId: string) {
    const files = await app.pickFiles();
    if (files.length) app.sendTo(deviceId, files);
  }
</script>

<svelte:window onfocus={drop} />

<main class="pop" data-testid="popover" bind:this={pop}>
  <header class="head" in:springIn={{ y: -8 }}>
    <svg class="mark" viewBox="0 0 1024 1024" aria-hidden="true">
      <rect width="1024" height="1024" rx="230" fill="var(--brand-blue)" />
      <path d="M666.8,242 L794.8,310.8 L610.8,494.8 L722.8,526.8 L265.2,826 L466.8,586 L354.8,554 Z" fill="var(--brand-navy)" />
      <path d="M648.8,220 L776.8,288.8 L592.8,472.8 L704.8,504.8 L247.2,804 L448.8,564 L336.8,532 Z" fill="var(--brand-yellow)" />
    </svg>
    <div class="who">
      <strong>{app.selfName || 'Nhako Beam'}</strong>
      <span class="muted">{status}</span>
    </div>
  </header>

  {#if !app.onboarded}
    <p class="note" in:springIn>{t('popover.finishSetup')}</p>
  {:else}
    {#each app.requests.slice(0, 2) as r (r.id)}
      <section class="request" animate:springFlip in:springIn={{ preset: 'bouncy', y: -12 }} out:quickOut={{ x: 24, y: 0 }}>
        <p>
          <strong>{t('incoming.title', { device: app.deviceName(r.deviceId) })}</strong>
          <span class="muted tabular">{summarize(r)}{r.text === undefined ? ` · ${formatBytes(r.totalBytes)}` : ''}</span>
        </p>
        <div class="request-actions">
          <Button variant="secondary" size="sm" sound="decline" onclick={() => app.decline(r.id)}>{t('incoming.decline')}</Button>
          <Button size="sm" sound="accept" onclick={() => app.accept(r.id)}>{t('incoming.accept')}</Button>
        </div>
      </section>
    {/each}

    <section class="block">
      <h2>{t('popover.devices')}</h2>
      {#if devices.length}
        <ul class="devices">
          {#each devices as d, i (d.id)}
            <li data-device={d.id} class:drop={dropOn === d.id} animate:springFlip in:springIn={{ delay: stagger(i), scale: 0.8, y: 0 }} out:quickOut>
              <DeviceOrb device={d} size={52} target={dropOn === d.id} onclick={() => sendTo(d.id)} testid="pop-device-{d.id}" />
            </li>
          {/each}
        </ul>
        <p class="hint muted">{t('popover.devices.hint')}</p>
      {:else}
        <p class="note muted" in:springIn>{t('home.empty.title')}</p>
      {/if}
    </section>

    <section class="block grow">
      <h2>{t('home.activity')}</h2>
      {#each moving as tr (tr.id)}
        <div animate:springFlip in:springIn={{ y: 16 }} out:quickOut={{ y: 8 }}>
          <TransferCard transfer={tr} />
        </div>
      {:else}
        <p class="note muted" in:springIn>{t('popover.quiet')}</p>
      {/each}
    </section>
  {/if}

  <footer>
    <Button full icon="arrow-square-out" testid="pop-open" onclick={() => invoke('open_main').catch((e) => console.warn('Could not open the main window:', message(e)))}>{t('popover.open')}</Button>
  </footer>
</main>

<Toasts />

<style>
  /* Only in the popover window: a bare body rule would also stop the main window scrolling. */
  :global(html[data-view='popover'] body) {
    overflow: hidden;
  }
  .pop {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
    height: 100vh;
    padding: var(--space-5);
    border: 1px solid var(--color-separator);
    background: var(--color-canvas);
  }
  .head {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  .mark {
    width: 32px;
    height: 32px;
    flex: none;
  }
  .who {
    display: grid;
    min-width: 0;
  }
  .who strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .who span {
    font-size: var(--type-footnote-size);
  }
  h2 {
    margin: 0 0 var(--space-3);
    font-size: var(--type-footnote-size);
    font-weight: 600;
    color: var(--color-text-secondary);
  }
  .block.grow {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
  }
  .devices {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: var(--space-3) var(--space-2);
  }
  .devices li {
    display: grid;
    justify-items: center;
    border-radius: var(--radius-md);
    transition: background-color var(--duration-fast) var(--ease-standard);
  }
  .devices li.drop {
    background: var(--color-accent-subtle);
  }
  .devices :global(.name) {
    font-size: var(--type-caption-size);
    max-width: 76px;
  }
  .hint {
    margin: var(--space-3) 0 0;
    font-size: var(--type-footnote-size);
  }
  .note {
    margin: 0;
    font-size: var(--type-callout-size);
  }
  .request {
    display: grid;
    gap: var(--space-3);
    padding: var(--space-4);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow: var(--elevation-2);
  }
  .request p {
    display: grid;
    margin: 0;
  }
  .request span {
    font-size: var(--type-footnote-size);
  }
  .request-actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-2);
  }
</style>
