<script lang="ts">
  import { app } from '../lib/state/app.svelte';
  import { t } from '../lib/copy';
  import { formatBytes, isLink, textPreview } from '../lib/format';
  import { gesture } from '../lib/tokens';
  import { beginDrag, endDrag, moveDrag } from '../lib/drag';
  import { springIn, quickOut, springFlip } from '../lib/motion/transitions';
  import Banner from '../components/Banner.svelte';
  import Button from '../components/Button.svelte';
  import FileStack from '../components/FileStack.svelte';
  import Icon from '../components/Icon.svelte';
  import IconButton from '../components/IconButton.svelte';
  import HomePanel from '../components/HomePanel.svelte';
  import Room from '../components/Room.svelte';
  import { arrangeRoom } from '../lib/stage/room';
  import { prefs } from '../lib/state/prefs.svelte';
  import TransferCard from '../components/TransferCard.svelte';
  import { homeStatus } from '../lib/state/status';

  let fileInput: HTMLInputElement | undefined = $state();
  let folderInput: HTMLInputElement | undefined = $state();
  let pendingTarget: string | null = null;

  const expanded = $derived(app.layout === 'expanded');
  // The stage fills the space the column leaves under the actions; the camera fits the islands
  // to it (Radar.svelte), so it never pushes the page wider or taller.
  let wrapWidth = $state(0);
  let wrapHeight = $state(0);
  const layout = $derived(arrangeRoom(app.devices, prefs.roomPicked, wrapWidth, wrapHeight));
  let away = $state(0);
  const nothingNearby = $derived(app.discovery !== 'searching' && app.reachable.length === 0);
  const stagedBytes = $derived(app.staged.reduce((n, f) => n + f.size, 0));
  const staging = $derived(app.staged.length > 0 || app.stagedText !== null);
  const stagedTitle = $derived(app.stagedText !== null ? textPreview(app.stagedText) : app.stagedSummary);
  const stagedWhat = $derived(app.stagedText !== null ? t(isLink(app.stagedText) ? 'staged.link' : 'staged.text') : formatBytes(stagedBytes));
  const status = $derived(
    homeStatus({
      discovery: app.discovery,
      active: app.activeTransfers,
      reachable: app.reachable.length,
      staging,
      deviceName: (id) => app.deviceName(id),
    }),
  );
  const transfers = $derived(app.visibleTransfers);
  const compactTransfers = $derived(transfers.slice(0, 2));

  async function pickFiles(target: string | null = null) {
    if (app.native) {
      // The native dialog gives real paths; the browser input below doesn't.
      const files = await app.pickFiles();
      if (target && files.length) app.sendTo(target);
      return;
    }
    pendingTarget = target;
    fileInput?.click();
  }

  /** A folder, with everything inside it. The browser prototype uses a folder input. */
  async function pickFolders(target: string | null = null) {
    if (app.native) {
      const files = await app.pickFolders();
      if (target && files.length) app.sendTo(target);
      return;
    }
    pendingTarget = target;
    folderInput?.click();
  }

  // A device's sheet asked for files or a folder to send it.
  $effect(() => {
    const r = app.pickRequest;
    if (!r) return;
    app.pickRequest = null;
    if (r.kind === 'folder') void pickFolders(r.deviceId);
    else void pickFiles(r.deviceId);
  });

  /** Ctrl+V anywhere on Home (outside a field) starts a text with what was pasted. */
  function onPaste(e: ClipboardEvent) {
    const el = e.target instanceof Element ? e.target : null;
    if (app.sheet || app.screen !== 'home' || el?.closest('input, textarea, [contenteditable]')) return;
    const text = e.clipboardData?.getData('text/plain') ?? '';
    if (!text.trim()) return;
    e.preventDefault();
    app.sheet = { kind: 'compose', text };
  }

  function searchAgain() {
    if (app.mock) app.mock.simDiscovery('searching');
    else app.engine.rescan();
  }

  function onFiles(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    if (!input.files?.length) return;
    app.stageFromFileList(input.files);
    input.value = '';
    if (pendingTarget) app.sendTo(pendingTarget);
    pendingTarget = null;
  }

  function onPick(deviceId: string) {
    const d = app.device(deviceId);
    if (!d?.reachable && !d?.wakeable) return;
    if (staging) app.sendTo(deviceId);
    else pickFiles(deviceId);
  }

  // ---------- Dragging the staged stack onto a device ----------
  let stackEl: HTMLElement | undefined = $state();
  let draggingStack = $state(false);
  let down: { x: number; y: number; id: number } | null = null;

  function stackDown(e: PointerEvent) {
    down = { x: e.clientX, y: e.clientY, id: e.pointerId };
    // From the first press, so a quick flick that leaves the stack before the slop still drags it.
    stackEl?.setPointerCapture(e.pointerId);
  }

  function stackMove(e: PointerEvent) {
    if (!down || down.id !== e.pointerId) return;
    if (!draggingStack) {
      const slop = e.pointerType === 'touch' ? gesture.dragSlopTouch : gesture.dragSlopPointer;
      if (Math.hypot(e.clientX - down.x, e.clientY - down.y) < slop) return;
      draggingStack = true;
      beginDrag('stack', e.clientX, e.clientY, app.staged.length || 1);
      return;
    }
    moveDrag(e.clientX, e.clientY);
  }

  async function stackUp(e: PointerEvent) {
    if (!down || down.id !== e.pointerId) return;
    down = null;
    if (!draggingStack) return;
    const target = endDrag();
    const r = stackEl?.getBoundingClientRect();
    if (target) {
      app.sendTo(target);
      await app.dragLayer?.land(target);
    } else {
      await app.dragLayer?.cancel(r ? { x: r.left + r.width / 2, y: r.top + r.height / 2 } : undefined);
    }
    draggingStack = false;
  }
</script>

<svelte:window onpaste={onPaste} />
<input bind:this={fileInput} type="file" multiple hidden onchange={onFiles} data-testid="file-input" />
<input bind:this={folderInput} type="file" webkitdirectory multiple hidden onchange={onFiles} data-testid="folder-input" />

<div class="home" class:expanded data-testid="home">
  <main class="main" bind:clientWidth={wrapWidth} bind:clientHeight={wrapHeight}>
    {#if wrapWidth && wrapHeight}
      <Room width={wrapWidth} height={wrapHeight} {layout} onpick={onPick} />
    {/if}
    <div class="over">
    <header class="top">
      <div>
        <!-- What the app is doing now. A new line rolls up into place; the percent ticks on its own. -->
        <h1 class="title" data-testid="home-status" data-tone={status.tone}>
          <span class="dot" data-tone={status.tone} aria-hidden="true"></span>
          <span class="line" aria-live="polite">
            {#key status.key}
              <span class="text" in:springIn={{ y: 18, scale: 1, preset: 'smooth' }} out:quickOut={{ y: -14, scale: 1 }}>{status.text}</span>
            {/key}
          </span>
          {#if status.percent !== null}
            <span class="pct" aria-hidden="true" in:springIn={{ scale: 0.8, y: 0, preset: 'bouncy' }} out:quickOut={{ y: 0 }}>{status.percent}%</span>
          {/if}
        </h1>
        <p class="muted sub">{app.selfName || 'This device'}</p>
      </div>
      <div class="top-actions">
        <IconButton icon="devices" label={t('home.devices')} size={24} testid="open-devices" onclick={() => (app.sheet = { kind: 'devices' })} />
        <IconButton icon="clock-counter-clockwise" label={t('history.title')} size={24} testid="open-history" onclick={() => app.navigate('history')} />
        <IconButton icon="gear-six" label={t('settings.title')} size={24} testid="open-settings" onclick={() => app.navigate('settings')} />
      </div>
    </header>

    <div class="banners">
      {#if app.discovery === 'offline'}
        <Banner icon="wifi-slash" text={t('home.banner.offline')} testid="banner-offline" />
      {:else if !app.onLan}
        <Banner icon="cell-signal-full" text={t('home.banner.noLan')} testid="banner-no-lan" />
      {:else if app.discovery === 'blocked'}
        <Banner icon="warning-circle" text={t('home.banner.discoveryBlocked')} testid="banner-blocked">
          {#snippet action()}
            <button onclick={searchAgain}>{t('home.banner.discoveryBlocked.fix')}</button>
          {/snippet}
        </Banner>
      {/if}
      {#if app.firewallBlocked && app.discovery !== 'offline' && app.onLan}
        <Banner icon="shield-check" text={t('firewall.banner')} testid="banner-firewall">
          {#snippet action()}
            <button onclick={() => app.fixFirewall()} disabled={app.fixingFirewall} data-testid="firewall-fix">
              {t(app.fixingFirewall ? 'firewall.fixing' : 'firewall.fix')}
            </button>
          {/snippet}
        </Banner>
      {/if}
      {#if app.linkShare && app.sheet?.kind !== 'link'}
        {@const asking = app.linkShare.visitors.filter((v) => v.state === 'asking').length}
        <Banner tone="info" icon="link-simple" text={asking ? `${t('link.active')} · ${t('link.active.asking', { n: asking })}` : t('link.active')} testid="banner-link">
          {#snippet action()}
            <button onclick={() => (app.sheet = { kind: 'link' })}>{t('text.show')}</button>
          {/snippet}
        </Banner>
      {/if}
      {#if app.update && app.updateSeen !== app.update.version}
        <Banner tone="info" icon="sparkle" text={t('update.available', { version: app.update.version })} testid="banner-update">
          {#snippet action()}
            <button onclick={() => app.hideUpdate()} data-testid="update-later">{t('update.later')}</button>
            <button onclick={() => app.openUpdate()} data-testid="update-download">{t('update.download')}</button>
          {/snippet}
        </Banner>
      {/if}
      {#if app.serverProblem && app.connection.mode === 'own'}
        <Banner
          icon={app.serverProblem === 'tokenRejected' ? 'warning-circle' : 'hard-drives'}
          text={t(app.serverProblem === 'tokenRejected' ? 'home.banner.server.tokenRejected' : 'home.banner.server.unreachable')}
          testid="banner-server"
        >
          {#snippet action()}
            <button onclick={() => app.navigate('settings')}>{t('home.banner.server.fix')}</button>
          {/snippet}
        </Banner>
      {/if}
    </div>

    {#if staging}
      <div class="tray" in:springIn={{ preset: 'bouncy', y: -16 }} out:quickOut={{ y: -8 }} data-testid="staged-tray">
        <span
          class="grab"
          class:hidden={draggingStack}
          bind:this={stackEl}
          role="button"
          tabindex="0"
          aria-label={app.stagedText !== null ? 'Drag the text onto a device' : 'Drag files onto a device'}
          onpointerdown={stackDown}
          onpointermove={stackMove}
          onpointerup={stackUp}
          onpointercancel={stackUp}
          data-testid="staged-stack"
        >
          <FileStack files={app.staged} text={app.stagedText} size={44} />
        </span>
        <div class="tray-text">
          <strong data-testid="staged-title">{stagedTitle}</strong>
          <span class="muted tabular">{stagedWhat} · {app.isAndroid ? t('home.staged.hint') : t('home.staged.hint.pointer')}</span>
        </div>
        <Button variant="plain" size="sm" testid="clear-staged" onclick={() => app.clearStaged()}>{t('send.clear')}</Button>
      </div>
    {/if}


    </div>
    {#if !app.isAndroid && !staging && !nothingNearby && away > 0.5}
      <p class="drop-hint" data-testid="home-drop-hint"><Icon name="download-simple" size={16} />{t('home.dropHint')}</p>
    {/if}
    <HomePanel {staging} bubbles={layout.rest} bind:away onfiles={() => pickFiles()} onfolder={() => pickFolders()} onpick={onPick}>
      {#snippet above()}
        {#if nothingNearby && !app.onLan && app.discovery !== 'offline'}
          <!-- On mobile data: nearby can't work, so say why and offer the way that does. -->
          <div class="empty" in:springIn={{ delay: 120 }} data-testid="empty-no-lan">
            <h2>{t('home.empty.noLan.title')}</h2>
            <p class="muted">{t('home.empty.noLan.body')}</p>
            <div class="empty-action">
              <Button variant="secondary" size="sm" icon="qr-code" testid="no-lan-code" onclick={() => (app.sheet = { kind: 'showCode' })}>{t('home.empty.noLan.action')}</Button>
            </div>
          </div>
        {:else if nothingNearby}
          <div class="empty" in:springIn={{ delay: 120 }} data-testid="empty-state">
            <h2>{t('home.empty.title')}</h2>
            <p class="muted">{t('home.empty.body')}</p>
          </div>
        {/if}
    {#if !expanded && compactTransfers.length}
      <div class="compact-activity">
        {#each compactTransfers as tr (tr.id)}
          <div animate:springFlip in:springIn={{ y: 24 }} out:quickOut={{ y: 12 }}>
            <TransferCard transfer={tr} ondetails={() => (app.sheet = { kind: 'transfer', id: tr.id })} />
          </div>
        {/each}
      </div>
    {/if}
      {/snippet}
    </HomePanel>
  </main>

  {#if expanded && transfers.length}
    <aside class="activity" aria-label={t('home.activity')}>
      {#each transfers as tr (tr.id)}
        <div animate:springFlip in:springIn={{ y: 16 }} out:quickOut={{ x: 24, y: 0 }}>
          <TransferCard transfer={tr} ondetails={() => (app.sheet = { kind: 'transfer', id: tr.id })} />
        </div>
      {/each}
    </aside>
  {/if}
</div>

<style>
  .home {
    /* The window's height, not 100% of the screen container: that container briefly holds the
       taller Settings screen while they swap, and WebKit (Linux app) kept that height afterwards. */
    position: relative;
    height: 100vh;
    height: 100dvh;
    overflow: hidden;
    background: #060a1e;
  }
  .main {
    position: absolute;
    inset: 0;
    min-width: 0;
    overflow: hidden;
  }
  /* What sits on the room's top edge: status, the round buttons, banners, staged files. */
  .over {
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    z-index: 1;
    display: grid;
    gap: var(--space-3);
    padding: var(--space-4) var(--space-5) 0;
    pointer-events: none;
  }
  .over > :global(*) {
    pointer-events: auto;
  }
  .top {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-4);
  }
  .top-actions {
    display: flex;
    gap: var(--space-1);
  }
  .top-actions :global(button) {
    width: 42px;
    height: 42px;
    border-radius: 50%;
    background: var(--color-surface-raised);
    color: var(--color-text-primary);
    box-shadow: var(--elevation-1);
  }
  .title {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    min-width: 0;
    font-size: var(--type-title2-size);
    line-height: var(--type-title2-line);
    font-weight: var(--type-title2-weight);
    letter-spacing: var(--type-title2-tracking);
    color: #fff;
    text-shadow: 0 1px 4px #000c;
  }
  .top > div:first-child {
    min-width: 0;
  }
  /* The old and new lines share one cell while they swap, and the cell clips them so the roll
     reads as one line turning over. */
  .line {
    display: grid;
    min-width: 0;
    overflow: clip;
    overflow-clip-margin: 4px;
  }
  /* Up to two lines; a very long device name ends in an ellipsis. */
  .text {
    grid-area: 1 / 1;
    min-width: 0;
    display: -webkit-box;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    overflow: hidden;
  }
  .pct {
    flex: none;
    font-variant-numeric: tabular-nums;
    color: #9acbff;
  }
  /* Steady when ready, a sonar ring while looking, a heartbeat while working. */
  .dot {
    position: relative;
    flex: none;
    width: 10px;
    height: 10px;
    margin-top: calc((var(--type-title2-line) - 10px) / 2);
    border-radius: var(--radius-full);
    background: var(--color-success);
    transition: background-color var(--duration-base) ease;
  }
  .dot[data-tone='searching'],
  .dot[data-tone='busy'] {
    background: var(--color-accent);
  }
  .dot[data-tone='warn'] {
    background: var(--color-warning);
  }
  .dot::after {
    content: '';
    position: absolute;
    inset: 0;
    border-radius: inherit;
    background: inherit;
    opacity: 0;
  }
  .dot[data-tone='searching']::after {
    animation: status-sonar var(--duration-searching-pulse) ease-out infinite;
  }
  .dot[data-tone='busy'] {
    animation: status-beat 1.1s ease-in-out infinite;
  }
  @keyframes status-sonar {
    from {
      opacity: 0.6;
      transform: scale(1);
    }
    to {
      opacity: 0;
      transform: scale(2.6);
    }
  }
  @keyframes status-beat {
    0%,
    100% {
      transform: scale(1);
    }
    40% {
      transform: scale(1.3);
    }
  }
  :global([data-reduced='true']) .dot,
  :global([data-reduced='true']) .dot::after {
    animation: none;
  }
  .sub {
    padding-left: 18px;
    font-size: var(--type-callout-size);
    color: #c9d0de;
    text-shadow: 0 1px 4px #000c;
  }
  .banners:empty {
    display: none;
  }
  .tray {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-4) var(--space-4) var(--space-4) var(--space-5);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow: var(--elevation-2);
  }
  .grab {
    padding: var(--space-2) var(--space-3) 0 0;
    cursor: grab;
    touch-action: none;
    transition: opacity var(--duration-fast) var(--ease-standard);
  }
  .grab:active {
    cursor: grabbing;
  }
  .grab.hidden {
    opacity: 0;
  }
  .tray-text {
    flex: 1;
    display: grid;
    min-width: 0;
  }
  .tray-text strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .tray-text span {
    font-size: var(--type-footnote-size);
  }
  .empty {
    text-align: center;
    display: grid;
    gap: var(--space-2);
    max-width: 40ch;
    margin: 0 auto;
  }
  .empty h2 {
    font-size: var(--type-headline-size);
    font-weight: 600;
  }
  .empty p {
    font-size: var(--type-callout-size);
  }
  .empty-action {
    display: flex;
    justify-content: center;
    margin-top: var(--space-2);
  }
  .compact-activity {
    display: grid;
    gap: var(--space-3);
  }
  .drop-hint {
    position: absolute;
    left: 0;
    right: 0;
    bottom: 56px;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    font-size: var(--type-footnote-size);
    color: #c9d0de;
    text-shadow: 0 1px 3px #000;
    text-align: center;
    pointer-events: none;
  }
  .empty {
    padding: var(--space-4);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    margin-bottom: var(--space-2);
  }
  .compact-activity {
    margin-bottom: var(--space-2);
  }
  /* Transfers float on the room's right side; nothing there when there aren't any. */
  .activity {
    position: absolute;
    right: var(--space-5);
    bottom: var(--space-5);
    z-index: 1;
    width: 340px;
    max-height: calc(100vh - 120px);
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    overflow-y: auto;
  }
</style>
