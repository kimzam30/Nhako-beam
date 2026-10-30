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
  import Radar from '../components/Radar.svelte';
  import TransferCard from '../components/TransferCard.svelte';

  let fileInput: HTMLInputElement | undefined = $state();
  let folderInput: HTMLInputElement | undefined = $state();
  let pendingTarget: string | null = null;

  const expanded = $derived(app.layout === 'expanded');
  // Sized from the space it actually has (not the window), so it can never push the page wider.
  // The radar's area takes the height the rest of the column leaves, so short windows (a laptop
  // with nobody nearby, which also shows the empty hint) never scroll.
  let wrapWidth = $state(0);
  let wrapHeight = $state(0);
  const RADAR_PAD = 40; // the area's top and bottom padding
  const radarSize = $derived(
    Math.max(
      260,
      Math.min(expanded ? 460 : app.layout === 'medium' ? 440 : 400, (wrapWidth || app.width - 40) - 24, wrapHeight ? wrapHeight - RADAR_PAD : Infinity),
    ),
  );
  const nothingNearby = $derived(app.discovery !== 'searching' && app.reachable.length === 0);
  const stagedBytes = $derived(app.staged.reduce((n, f) => n + f.size, 0));
  const staging = $derived(app.staged.length > 0 || app.stagedText !== null);
  const stagedTitle = $derived(app.stagedText !== null ? textPreview(app.stagedText) : app.stagedSummary);
  const stagedWhat = $derived(app.stagedText !== null ? t(isLink(app.stagedText) ? 'staged.link' : 'staged.text') : formatBytes(stagedBytes));
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
  }

  function stackMove(e: PointerEvent) {
    if (!down || down.id !== e.pointerId) return;
    if (!draggingStack) {
      const slop = e.pointerType === 'touch' ? gesture.dragSlopTouch : gesture.dragSlopPointer;
      if (Math.hypot(e.clientX - down.x, e.clientY - down.y) < slop) return;
      draggingStack = true;
      stackEl?.setPointerCapture(e.pointerId);
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
  <main class="main">
    <header class="top">
      <div>
        <h1 class="title">{t('home.title')}</h1>
        <p class="muted sub">{app.selfName || 'This device'}</p>
      </div>
      <div class="top-actions">
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

    <div class="radar-wrap" class:staged={staging} bind:clientWidth={wrapWidth} bind:clientHeight={wrapHeight}>
      <div class="radar-box">
        <Radar size={radarSize} onpick={onPick} onmore={() => (app.sheet = { kind: 'send' })} />
      </div>
    </div>

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

    <nav class="actions" aria-label="Actions">
      {#if staging}
        <Button size="lg" icon="paper-plane-tilt" iconWeight="fill" testid="choose-device" onclick={() => (app.sheet = { kind: 'send' })}>{t('send.choose.title')}…</Button>
      {:else}
        <Button size="lg" icon="paper-plane-tilt" iconWeight="fill" testid="send-files" onclick={() => pickFiles()}>
          {app.isAndroid ? t('home.action.sendFiles') : t('home.action.chooseFiles')}
        </Button>
        <div class="kinds" in:springIn={{ y: 6, delay: 60 }}>
          <Button variant="ghost" size="sm" icon="folder-simple" testid="send-folder" onclick={() => pickFolders()}>{t('home.action.folder')}</Button>
          <Button variant="ghost" size="sm" icon="text-aa" testid="send-text" onclick={() => (app.sheet = { kind: 'compose' })}>{t('home.action.text')}</Button>
          <Button variant="ghost" size="sm" icon="clipboard-text" testid="send-clipboard" onclick={() => void app.stageClipboard()}>{t('home.action.clipboard')}</Button>
        </div>
      {/if}
      <div class="code-actions">
        <Button variant="secondary" icon="qr-code" testid="send-code" onclick={() => (app.sheet = { kind: 'showCode' })}>{t('home.action.sendCode')}</Button>
        <Button variant="secondary" icon="download-simple" testid="receive-code" onclick={() => (app.sheet = { kind: app.isAndroid ? 'scan' : 'enterCode' })}>{t('home.action.receiveCode')}</Button>
      </div>
      {#if !app.isAndroid && !staging}
        <p class="drop-hint" data-testid="home-drop-hint"><Icon name="download-simple" size={16} />{t('home.dropHint')}</p>
      {/if}
    </nav>
  </main>

  {#if expanded}
    <aside class="activity" aria-label={t('home.activity')}>
      <h2 class="activity-title">{t('home.activity')}</h2>
      {#each transfers as tr (tr.id)}
        <div animate:springFlip in:springIn={{ y: 16 }} out:quickOut={{ x: 24, y: 0 }}>
          <TransferCard transfer={tr} ondetails={() => (app.sheet = { kind: 'transfer', id: tr.id })} />
        </div>
      {:else}
        <p class="muted activity-empty" in:springIn>{t('home.activity.empty')}</p>
      {/each}
    </aside>
  {/if}
</div>

<style>
  .home {
    /* The window's height, not 100% of the screen container: that container briefly holds the
       taller Settings screen while they swap, and WebKit (Linux app) kept that height afterwards. */
    min-height: 100vh;
    min-height: 100dvh;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
  }
  .home.expanded {
    grid-template-columns: minmax(0, 1fr) 380px;
  }
  .main {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
    padding: var(--space-6) var(--space-5) calc(var(--space-6) + env(safe-area-inset-bottom));
    min-width: 0;
  }
  .expanded .main {
    padding: var(--space-7) var(--space-8);
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
  .title {
    font-size: var(--type-large-title-size);
    line-height: var(--type-large-title-line);
    font-weight: var(--type-large-title-weight);
    letter-spacing: var(--type-large-title-tracking);
  }
  .sub {
    font-size: var(--type-callout-size);
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
  .radar-wrap {
    /* Takes the height left over, never its content's (the radar sits in an absolute box), so the
       radar can size itself to fit. If the radar decided this height, any size would hold itself up. */
    position: relative;
    flex: 1 1 0;
    min-width: 0;
    overflow-x: clip;
    overflow-y: visible;
    /* The smallest radar (260; smaller crowds the orb names) plus the box's top and bottom gap (RADAR_PAD). */
    min-height: 300px;
  }
  .radar-box {
    position: absolute;
    inset: var(--space-6) 0 var(--space-4);
    display: grid;
    place-items: center;
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
  .kinds {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--space-2);
    margin-top: calc(var(--space-1) * -1);
  }
  .kinds :global(.btn) {
    padding: 0 var(--space-2);
  }
  .compact-activity {
    display: grid;
    gap: var(--space-3);
  }
  .actions {
    display: grid;
    gap: var(--space-3);
    width: 100%;
    max-width: var(--size-content-max-width);
    margin: 0 auto;
  }
  .drop-hint {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    font-size: var(--type-footnote-size);
    color: var(--color-text-tertiary);
    text-align: center;
  }
  .code-actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--space-3);
  }
  .code-actions :global(.btn) {
    padding: 0 var(--space-3);
  }
  @media (max-width: 380px) {
    .code-actions {
      grid-template-columns: 1fr;
    }
  }
  .activity {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-7) var(--space-6);
    background: var(--color-surface-sunken);
    border-left: 1px solid var(--color-separator);
    overflow-y: auto;
    max-height: 100vh;
  }
  .activity-title {
    font-size: var(--type-title2-size);
    line-height: var(--type-title2-line);
    font-weight: var(--type-title2-weight);
    margin-bottom: var(--space-3);
  }
  .activity-empty {
    font-size: var(--type-callout-size);
  }
</style>
