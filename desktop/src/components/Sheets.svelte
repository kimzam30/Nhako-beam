<script lang="ts">
  import { app } from '../lib/state/app.svelte';
  import { message } from '../lib/engine/tauri';
  import type { RedeemResult } from '../lib/engine/types';
  import { t } from '../lib/copy';
  import { formatBytes, formatDuration, formatSpeed, middleTruncate, percent, summarizeFiles, timeOfDay, dayLabel } from '../lib/format';
  import { playHaptic } from '../lib/haptics.svelte';
  import { playSound } from '../lib/sounds.svelte';
  import { SpringValue, wait } from '../lib/motion/springs';
  import { springIn, stagger } from '../lib/motion/transitions';
  import Button from './Button.svelte';
  import CodeDisplay from './CodeDisplay.svelte';
  import CodeInput from './CodeInput.svelte';
  import ConnectionBadge from './ConnectionBadge.svelte';
  import DeviceOrb from './DeviceOrb.svelte';
  import FileStack from './FileStack.svelte';
  import Icon from './Icon.svelte';
  import QrScanner from './QrScanner.svelte';
  import Sheet from './Sheet.svelte';
  import Toggle from './Toggle.svelte';

  // A sheet sliding up or away has a sound. The paired sheet has its own (`matched`, app state).
  let shownKind: string | null = null;
  $effect(() => {
    const kind = app.sheet?.kind ?? null;
    if (kind === shownKind) return;
    if (kind && kind !== 'paired') playSound('open');
    else if (!kind && shownKind) playSound('close');
    shownKind = kind;
  });

  const close = () => {
    if (app.sheet?.kind === 'showCode') app.engine.cancelCode();
    app.sheet = null;
  };

  // ---------- Send to ----------
  const trusted = $derived(app.devices.filter((d) => d.trusted));
  const nearby = $derived(app.devices.filter((d) => !d.trusted && d.reachable));

  // ---------- Show code ----------
  $effect(() => {
    if (app.sheet?.kind === 'showCode' && !app.code) app.newCode();
  });

  // ---------- Enter code ----------
  let words = $state(['', '', '']);
  let checking = $state(false);
  let codeError = $state('');
  let codeInput: ReturnType<typeof CodeInput> | undefined = $state();

  $effect(() => {
    if (app.sheet?.kind === 'enterCode') {
      words = ['', '', ''];
      codeError = '';
      checking = false;
    }
  });

  async function redeem() {
    if (checking) return;
    checking = true;
    codeError = '';
    let r: RedeemResult;
    try {
      r = await app.engine.redeemCode(words);
    } catch (e) {
      checking = false;
      codeError = message(e);
      return;
    }
    checking = false;
    if (r.ok) return; // the engine's 'paired' event opens the paired sheet
    playHaptic('decline');
    playSound('error');
    if (r.reason === 'unknownWord') {
      codeError = t('code.enter.unknownWord');
      codeInput?.shakeSlot(r.index);
    } else if (r.reason === 'tokenRejected') {
      codeError = t('code.enter.tokenRejected');
      codeInput?.shakeSlot(null);
    } else {
      codeError = t('code.enter.wrong');
      codeInput?.shakeSlot(null);
    }
  }

  // ---------- Scan ----------
  async function scanned() {
    // Only the prototype pretends to scan; the real scanner comes with the Android app.
    if (app.mock) await app.engine.redeemCode(app.mock.peerCode);
  }

  // ---------- Paired ----------
  let trustNew = $state(true);
  let leftOrb: HTMLElement | undefined = $state();
  let rightOrb: HTMLElement | undefined = $state();
  const pairedDevice = $derived(app.sheet?.kind === 'paired' ? app.device(app.sheet.deviceId) : undefined);

  $effect(() => {
    if (app.sheet?.kind !== 'paired' || !leftOrb || !rightOrb) return;
    trustNew = true;
    const l = leftOrb, r = rightOrb;
    const gap = new SpringValue(70, (v) => {
      l.style.transform = `translateX(${-v}px)`;
      r.style.transform = `translateX(${v}px)`;
    });
    gap.set(70);
    (async () => {
      await wait(120);
      await gap.to(6, 'smooth');
      if (pairedDevice) app.bump(pairedDevice.id);
    })();
    return () => gap.stop();
  });

  function finishPairing(send: boolean) {
    const s = app.sheet;
    if (s?.kind !== 'paired') return;
    if (trustNew) app.engine.setTrusted(s.deviceId, true);
    app.sheet = null;
    if (send && app.staged.length) app.sendTo(s.deviceId);
  }

  // ---------- Transfer details ----------
  const detail = $derived(app.sheet?.kind === 'transfer' ? app.transfers.find((x) => x.id === (app.sheet as { id: string }).id) : undefined);

  // ---------- History item ----------
  const past = $derived(app.sheet?.kind === 'historyItem' ? app.historyEntry(app.sheet.id) : undefined);
  const pastStatus = $derived.by(() => {
    if (!past) return '';
    if (past.status === 'done') return t(past.direction === 'send' ? 'transfer.done.sent' : 'transfer.done.received', { device: past.deviceName });
    if (past.status === 'declined') return t('send.declined', { device: past.deviceName });
    if (past.status === 'cancelled') {
      return past.cancelledBy === 'peer' ? t('transfer.cancelled.byOther', { device: past.deviceName }) : t('transfer.cancelled.byYou');
    }
    return t('history.status.failed');
  });

  // ---------- Confirm remove ----------
  const removing = $derived(app.sheet?.kind === 'confirmRemove' ? app.device(app.sheet.deviceId) : undefined);
</script>

{#snippet openPast()}
  {#if past}
    {@const id = past.id}
    <Button variant="secondary" full icon="folder-open" testid="history-reveal" onclick={() => app.openReceived(id, true)}>{t('transfer.done.showInFolder')}</Button>
    <Button full icon="arrow-square-out" testid="history-open" onclick={() => app.openReceived(id, false)}>{t('transfer.done.open')}</Button>
  {/if}
{/snippet}

{#if app.sheet?.kind === 'send'}
  <Sheet title={t('send.choose.title')} onclose={close} testid="sheet-send">
    {#if app.staged.length}
      <div class="staged">
        <FileStack files={app.staged} size={40} />
        <span>{app.stagedSummary} · {formatBytes(app.staged.reduce((n, f) => n + f.size, 0))}</span>
      </div>
    {/if}
    {#each [{ label: t('send.choose.trusted'), list: trusted }, { label: t('send.choose.nearby'), list: nearby }] as group}
      {#if group.list.length}
        <h3 class="group">{group.label}</h3>
        <ul class="list">
          {#each group.list as d, i (d.id)}
            <li in:springIn={{ delay: stagger(i), y: 8 }}>
              <button class="row" disabled={!d.reachable && !d.wakeable} onclick={() => app.sendTo(d.id)} data-testid="send-row-{d.id}">
                <DeviceOrb device={d} size={40} showName={false} />
                <span class="name">{d.name}</span>
                {#if d.route}<ConnectionBadge route={d.route} />{:else}<span class="muted small">{t(d.wakeable ? 'home.orb.asleep' : 'home.orb.offline')}</span>{/if}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    {/each}
    <Button variant="ghost" icon="qr-code" full testid="use-code" onclick={() => (app.sheet = { kind: 'showCode' })}>{t('send.choose.useCode')}</Button>
  </Sheet>
{:else if app.sheet?.kind === 'showCode'}
  <Sheet title={t('code.show.title')} onclose={close} testid="sheet-show-code">
    <p class="lede">{t('code.show.body')}</p>
    {#if app.code}<CodeDisplay code={app.code} />{/if}
  </Sheet>
{:else if app.sheet?.kind === 'enterCode'}
  <Sheet title={t('code.enter.title')} onclose={close} testid="sheet-enter-code">
    <p class="lede">{t('code.enter.body')}</p>
    <CodeInput bind:this={codeInput} bind:words disabled={checking} onsubmit={redeem} />
    {#if codeError}<p class="error" role="alert" in:springIn={{ y: -6, preset: 'snappy' }}><Icon name="warning-circle" size={16} weight="bold" />{codeError}</p>{/if}
    {#if app.mock}<p class="demo">Prototype: the other device is showing <b>{app.mock.peerCode.join(' ')}</b></p>{/if}
    {#snippet footer()}
      {#if app.isAndroid}<Button variant="secondary" icon="scan" onclick={() => (app.sheet = { kind: 'scan' })}>{t('code.enter.scan')}</Button>{/if}
      <Button full loading={checking} disabled={!words.every((w) => w.length > 1)} testid="connect" onclick={redeem}>{t('code.enter.connect')}</Button>
    {/snippet}
  </Sheet>
{:else if app.sheet?.kind === 'scan'}
  <Sheet title={t('code.scan.title')} onclose={close} testid="sheet-scan">
    <QrScanner onfound={scanned} />
    {#snippet footer()}
      <Button variant="secondary" full icon="keyboard" onclick={() => (app.sheet = { kind: 'enterCode' })}>{t('code.scan.type')}</Button>
    {/snippet}
  </Sheet>
{:else if app.sheet?.kind === 'paired' && pairedDevice}
  <Sheet title={t('code.matched', { device: pairedDevice.name })} onclose={() => finishPairing(false)} testid="sheet-paired">
    <div class="meet">
      <span bind:this={leftOrb}><DeviceOrb self selfName={app.selfName} size={72} showName={false} /></span>
      <span bind:this={rightOrb}><DeviceOrb device={pairedDevice} size={72} showName={false} /></span>
    </div>
    <div class="verify">
      <span class="muted">{t('code.matched.verify')}</span>
      <span class="fp tabular" data-testid="pair-check"><Icon name="fingerprint-simple" size={18} />{app.sheet.check}</span>
    </div>
    <Toggle bind:checked={trustNew} label={t('code.matched.trust')} hint={t('code.matched.trust.body')} testid="trust-new" />
    {#snippet footer()}
      {#if app.staged.length}
        <Button full size="lg" icon="paper-plane-tilt" iconWeight="fill" testid="paired-send" onclick={() => finishPairing(true)}>{t('code.matched.send', { summary: app.stagedSummary })}</Button>
      {:else}
        <Button full size="lg" testid="paired-continue" onclick={() => finishPairing(false)}>{t('code.matched.continue')}</Button>
      {/if}
    {/snippet}
  </Sheet>
{:else if app.sheet?.kind === 'transfer' && detail}
  <Sheet title={t('transfer.files')} onclose={close} testid="sheet-transfer" wide>
    <ul class="files">
      {#each detail.files as f, i (f.id)}
        {@const done = detail.fileBytes[f.id] ?? 0}
        {@const p = percent(done, f.size)}
        <li in:springIn={{ delay: stagger(i), y: 8 }}>
          <span class="fname" title={f.name}>{middleTruncate(f.name, 34)}</span>
          <span class="fsize tabular">{p === 100 ? formatBytes(f.size) : `${p}%`}</span>
          {#if p === 100}
            <span class="fdone" in:springIn={{ preset: 'snappy', scale: 0.4, y: 0 }}><Icon name="check-circle" size={18} weight="fill" /></span>
          {:else}
            <span class="fbar"><span style:transform="scaleX({p / 100})"></span></span>
          {/if}
        </li>
      {/each}
    </ul>
    <p class="muted small">{summarizeFiles(detail.files)} · {formatBytes(detail.totalBytes)}</p>
  </Sheet>
{:else if app.sheet?.kind === 'historyItem' && past}
  <Sheet
    title={summarizeFiles(past.files)}
    onclose={close}
    testid="sheet-history"
    wide
    footer={past.direction === 'receive' && past.status === 'done' ? openPast : undefined}
  >
    <p class="past-status" class:bad={past.status !== 'done'}>
      <Icon name={past.status === 'done' ? 'check-circle' : past.status === 'failed' ? 'warning-circle' : 'x-circle'} size={18} weight="fill" />
      {pastStatus}
    </p>
    <dl class="facts">
      <div in:springIn={{ delay: stagger(0), y: 8 }}>
        <dt>{t('history.detail.when')}</dt>
        <dd class="tabular">{dayLabel(past.finishedAt, Date.now(), t('history.today'), t('history.yesterday'))}, {timeOfDay(past.finishedAt)}</dd>
      </div>
      <div in:springIn={{ delay: stagger(1), y: 8 }}>
        <dt>{t('history.detail.device')}</dt>
        <dd>{past.deviceName}</dd>
      </div>
      <div in:springIn={{ delay: stagger(2), y: 8 }}>
        <dt>{t('history.detail.route')}</dt>
        <dd><ConnectionBadge route={past.route} /></dd>
      </div>
      <div in:springIn={{ delay: stagger(3), y: 8 }}>
        <dt>{t('history.detail.size')}</dt>
        <dd class="tabular">
          {past.doneBytes < past.totalBytes
            ? t('history.detail.partial', { done: formatBytes(past.doneBytes), total: formatBytes(past.totalBytes) })
            : formatBytes(past.totalBytes)}
        </dd>
      </div>
      {#if past.doneBytes > 0}
        <div in:springIn={{ delay: stagger(4), y: 8 }}>
          <dt>{t('history.detail.duration')}</dt>
          <dd class="tabular">{formatDuration(past.finishedAt - past.startedAt)}</dd>
        </div>
      {/if}
      {#if past.averageSpeed > 0}
        <div in:springIn={{ delay: stagger(5), y: 8 }}>
          <dt>{t('history.detail.speed')}</dt>
          <dd class="tabular">{formatSpeed(past.averageSpeed)}</dd>
        </div>
      {/if}
    </dl>
    <h3 class="group">{t('transfer.files')}</h3>
    <ul class="files">
      {#each past.files as f, i (f.id)}
        <li in:springIn={{ delay: stagger(i), y: 8 }}>
          <span class="fname" title={f.name}>{middleTruncate(f.name, 34)}</span>
          <span class="fsize tabular">{formatBytes(f.size)}</span>
        </li>
      {/each}
    </ul>
  </Sheet>
{:else if app.sheet?.kind === 'confirmClearHistory'}
  <Sheet title={t('history.clear')} onclose={close} testid="sheet-clear-history">
    <p class="lede">{t('history.clear.confirm')}</p>
    {#snippet footer()}
      <Button variant="secondary" full onclick={close}>{t('common.cancel')}</Button>
      <Button variant="danger" full testid="confirm-clear-history" onclick={() => app.clearHistory()}>{t('history.clear')}</Button>
    {/snippet}
  </Sheet>
{:else if app.sheet?.kind === 'confirmRemove' && removing}
  <Sheet title={t('settings.trusted.remove')} onclose={close} testid="sheet-remove">
    <p class="lede">{t('settings.trusted.remove.confirm', { device: removing.name })}</p>
    {#snippet footer()}
      <Button variant="secondary" full onclick={close}>{t('common.cancel')}</Button>
      <Button
        variant="danger"
        full
        testid="confirm-remove"
        onclick={() => {
          app.engine.setTrusted(removing.id, false);
          app.sheet = null;
        }}>{t('settings.trusted.remove')}</Button
      >
    {/snippet}
  </Sheet>
{/if}

<style>
  .past-status {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-5);
    font-weight: 500;
    color: var(--color-success);
  }
  .past-status.bad {
    color: var(--color-text-secondary);
  }
  .facts {
    display: grid;
    margin: 0 0 var(--space-5);
  }
  .facts div {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    min-height: var(--size-control-md);
    border-bottom: 1px solid var(--color-separator);
  }
  .facts dt {
    color: var(--color-text-secondary);
  }
  .facts dd {
    margin: 0;
    text-align: right;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .lede {
    color: var(--color-text-secondary);
    margin-bottom: var(--space-6);
  }
  .staged {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-3) 0 var(--space-4);
    font-weight: 500;
  }
  .group {
    margin: var(--space-4) 0 var(--space-2);
    font-size: var(--type-footnote-size);
    font-weight: 600;
    color: var(--color-text-secondary);
  }
  .list,
  .files {
    list-style: none;
    margin: 0 0 var(--space-4);
    padding: 0;
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    width: 100%;
    padding: var(--space-2);
    border-radius: var(--radius-md);
    text-align: left;
    transition:
      background-color var(--duration-fast) var(--ease-standard),
      transform var(--spring-press-duration) var(--spring-press);
  }
  .row :global(.orb) {
    width: auto;
  }
  .row:active:not(:disabled) {
    transform: scale(0.98);
    background: var(--color-fill);
  }
  @media (hover: hover) {
    .row:hover:not(:disabled) {
      background: var(--color-fill);
    }
  }
  .row:disabled {
    opacity: 0.5;
    cursor: default;
  }
  .name {
    flex: 1;
    font-weight: 500;
  }
  .small {
    font-size: var(--type-footnote-size);
  }
  .error {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-top: var(--space-2);
    color: var(--color-danger);
    font-size: var(--type-callout-size);
  }
  .demo {
    margin-top: var(--space-5);
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-sm);
    background: var(--color-surface-sunken);
    font-size: var(--type-footnote-size);
    color: var(--color-text-secondary);
  }
  .meet {
    display: flex;
    justify-content: center;
    padding: var(--space-6) 0;
  }
  .meet > span {
    display: inline-block;
    will-change: transform;
  }
  .meet :global(.orb) {
    width: auto;
  }
  .verify {
    display: grid;
    justify-items: center;
    gap: var(--space-2);
    margin-bottom: var(--space-6);
    font-size: var(--type-footnote-size);
  }
  .fp {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4);
    border-radius: var(--radius-sm);
    background: var(--color-fill);
    font-family: var(--font-mono);
    font-size: var(--type-headline-size);
    font-weight: 600;
    letter-spacing: 0.08em;
  }
  .files li {
    display: grid;
    grid-template-columns: 1fr auto 22px;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) 0;
    border-bottom: 1px solid var(--color-separator);
  }
  .fname {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .fsize {
    font-size: var(--type-footnote-size);
    color: var(--color-text-secondary);
  }
  .fdone {
    color: var(--color-success);
    display: grid;
  }
  .fbar {
    height: 4px;
    border-radius: var(--radius-full);
    background: var(--color-beam-track);
    overflow: hidden;
  }
  .fbar span {
    display: block;
    height: 100%;
    background: var(--color-beam-core);
    transform-origin: left;
    transition: transform var(--spring-smooth-duration) var(--spring-smooth);
  }
</style>
