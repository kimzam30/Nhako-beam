<script lang="ts">
  import { app } from '../lib/state/app.svelte';
  import { message } from '../lib/engine/tauri';
  import type { RedeemResult } from '../lib/engine/types';
  import { TEXT_MAX } from '../lib/engine/types';
  import { untrack } from 'svelte';
  import { t } from '../lib/copy';
  import { CODE_WORDS } from '../lib/engine/words';
  import { formatBytes, formatDuration, formatSpeed, isLink, middleTruncate, percent, summarizeFiles, textPreview, timeOfDay, dayLabel, utf8Bytes } from '../lib/format';
  import { playHaptic } from '../lib/haptics.svelte';
  import { playSound } from '../lib/sounds.svelte';
  import { SpringValue, wait } from '../lib/motion/springs';
  import { springIn, stagger } from '../lib/motion/transitions';
  import { spark } from '../lib/motion/bolt';
  import Bolt from './Bolt.svelte';
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

  // ---------- Compose ----------
  let draft = $state('');
  let composeBox: HTMLTextAreaElement | undefined = $state();
  $effect(() => {
    if (app.sheet?.kind !== 'compose') return;
    const start = app.sheet.text ?? untrack(() => app.stagedText) ?? '';
    draft = start;
    queueMicrotask(() => composeBox?.focus());
  });
  const composeTo = $derived(app.sheet?.kind === 'compose' && app.sheet.deviceId ? app.device(app.sheet.deviceId) : undefined);
  const over = $derived(Math.max(0, utf8Bytes(draft) - TEXT_MAX));

  async function pasteInto() {
    const text = await app.readClipboard();
    if (!text) {
      app.toast(t('send.clipboard.empty'), 'warning');
      return;
    }
    draft = draft.trim() ? `${draft}${draft.endsWith('\n') ? '' : '\n'}${text}` : text;
    composeBox?.focus();
  }

  function composeDone() {
    if (!draft.trim() || over) return;
    if (composeTo) return app.sendTextTo(composeTo.id, draft);
    if (app.stageText(draft)) app.sheet = { kind: 'send' };
  }

  // ---------- Device ----------
  const sheetDevice = $derived(app.sheet?.kind === 'device' ? app.device(app.sheet.deviceId) : undefined);
  /** Sends what's waiting, or asks Home to pick files or a folder for this device. */
  function sendFromDevice(kind: 'files' | 'folder' | 'text') {
    const d = sheetDevice;
    if (!d) return;
    if (kind === 'text') {
      app.sheet = { kind: 'compose', deviceId: d.id };
      return;
    }
    app.sheet = null;
    if (app.hasStaged && kind === 'files') app.sendTo(d.id);
    else app.requestPick(kind, d.id);
  }

  // ---------- Trust ----------
  const trustDevice = $derived(app.sheet && (app.sheet.kind === 'trustWait' || app.sheet.kind === 'trustAsk') ? app.device(app.sheet.deviceId) : undefined);

  // ---------- Text ----------
  const textItem = $derived.by(() => {
    const sh = app.sheet;
    if (sh?.kind !== 'text') return undefined;
    const live = app.transfers.find((x) => x.id === sh.id);
    if (live) return { text: live.text ?? '', direction: live.direction, device: app.deviceName(live.deviceId) };
    const past = app.historyEntry(sh.id);
    return past ? { text: past.text ?? '', direction: past.direction, device: past.deviceName } : undefined;
  });
  const textTitle = $derived.by(() => {
    if (!textItem) return '';
    const link = isLink(textItem.text);
    const key = textItem.direction === 'send' ? (link ? 'text.link.to' : 'text.to') : link ? 'text.link.from' : 'text.from';
    return t(key, { device: textItem.device });
  });

  // ---------- Show code ----------
  $effect(() => {
    if (app.sheet?.kind === 'showCode' && !app.code) app.newCode();
  });

  // ---------- Enter code ----------
  let words = $state<string[]>(Array(CODE_WORDS).fill(''));
  let checking = $state(false);
  let codeError = $state('');
  let codeInput: ReturnType<typeof CodeInput> | undefined = $state();

  $effect(() => {
    if (app.sheet?.kind === 'enterCode') {
      words = Array(CODE_WORDS).fill('');
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
  let met = $state(false);
  const pairedDevice = $derived(app.sheet?.kind === 'paired' ? app.device(app.sheet.deviceId) : undefined);

  $effect(() => {
    if (app.sheet?.kind !== 'paired' || !leftOrb || !rightOrb) return;
    trustNew = true;
    met = false;
    const l = leftOrb, r = rightOrb;
    const gap = new SpringValue(70, (v) => {
      l.style.transform = `translateX(${-v}px)`;
      r.style.transform = `translateX(${v}px)`;
    });
    gap.set(70);
    (async () => {
      await wait(120);
      await gap.to(6, 'smooth');
      met = true;
      if (pairedDevice) app.bump(pairedDevice.id);
    })();
    return () => gap.stop();
  });

  /** Throws sparks from an element once it has landed in place. */
  function zapHere(node: HTMLElement) {
    const id = setTimeout(() => spark(node, { count: 6, reach: 34 }), 60);
    return { destroy: () => clearTimeout(id) };
  }

  function finishPairing(send: boolean) {
    const s = app.sheet;
    if (s?.kind !== 'paired') return;
    // From Nearby, both have already saved each other.
    if (trustNew && s.via !== 'nearby') app.engine.setTrusted(s.deviceId, true);
    app.sheet = null;
    if (send && app.staged.length) app.sendTo(s.deviceId);
  }

  // ---------- Transfer details ----------
  const detail = $derived(app.sheet?.kind === 'transfer' ? app.transfers.find((x) => x.id === (app.sheet as { id: string }).id) : undefined);

  // ---------- History item ----------
  const past = $derived(app.sheet?.kind === 'historyItem' ? app.historyEntry(app.sheet.id) : undefined);
  // Long file lists show a page at a time, so a transfer of thousands of files opens instantly.
  const FILES_PAGE = 100;
  let filesShown = $state(FILES_PAGE);
  $effect(() => {
    void (app.sheet && 'id' in app.sheet ? app.sheet.id : null);
    filesShown = FILES_PAGE;
  });
  const pastStatus = $derived.by(() => {
    if (!past) return '';
    if (past.status === 'done') return t(past.direction === 'send' ? 'transfer.done.sent' : 'transfer.done.received', { device: past.deviceName });
    if (past.status === 'declined') return t(past.error === 'notAllowed' ? 'send.notAllowed' : 'send.declined', { device: past.deviceName });
    if (past.status === 'cancelled') {
      return past.cancelledBy === 'peer' ? t('transfer.cancelled.byOther', { device: past.deviceName }) : t('transfer.cancelled.byYou');
    }
    return t('history.status.failed');
  });

  // ---------- Confirm remove ----------
  const removing = $derived(app.sheet?.kind === 'confirmRemove' ? app.device(app.sheet.deviceId) : undefined);
</script>

{#snippet moreFiles(total: number)}
  {#if total > filesShown}
    <div class="more-files" in:springIn={{ y: 6 }}>
      <Button variant="ghost" size="sm" icon="plus" testid="files-more" onclick={() => (filesShown += FILES_PAGE)}>
        {t('transfer.files.more', { count: Math.min(FILES_PAGE, total - filesShown) })}
      </Button>
    </div>
  {/if}
{/snippet}

{#snippet openPast()}
  {#if past}
    {@const id = past.id}
    <Button variant="secondary" full icon="folder-open" testid="history-reveal" onclick={() => app.openReceived(id, true)}>{t('transfer.done.showInFolder')}</Button>
    <Button full icon="arrow-square-out" testid="history-open" onclick={() => app.openReceived(id, false)}>{t('transfer.done.open')}</Button>
  {/if}
{/snippet}

{#if app.sheet?.kind === 'send'}
  <Sheet title={t('send.choose.title')} onclose={close} testid="sheet-send">
    {#if app.stagedText !== null}
      <div class="staged">
        <FileStack files={[]} text={app.stagedText} size={40} />
        <span class="staged-text">{textPreview(app.stagedText)}</span>
      </div>
    {:else if app.staged.length}
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
            <li class="send-li" in:springIn={{ delay: stagger(i), y: 8 }}>
              <button class="row" disabled={!d.reachable && !d.wakeable} onclick={() => app.sendTo(d.id)} data-testid="send-row-{d.id}">
                <DeviceOrb device={d} size={40} showName={false} />
                <span class="name">{d.name}</span>
                {#if d.route}<ConnectionBadge route={d.route} />{:else}<span class="muted small">{t(d.wakeable ? 'home.orb.asleep' : 'home.orb.offline')}</span>{/if}
              </button>
              {#if !d.trusted && d.route === 'nearby'}
                <Button variant="ghost" size="sm" icon="handshake" testid="send-trust-{d.id}" onclick={() => app.askTrust(d.id)}>{t('send.choose.trust')}</Button>
              {/if}
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
  <Sheet title={t(app.sheet.via === 'nearby' ? 'code.matched.nearby' : 'code.matched', { device: pairedDevice.name })} onclose={() => finishPairing(false)} testid="sheet-paired">
    <div class="meet">
      <span bind:this={leftOrb}><DeviceOrb self selfName={app.selfName} size={72} showName={false} /></span>
      <span bind:this={rightOrb}><DeviceOrb device={pairedDevice} size={72} showName={false} /></span>
      {#if met}
        <!-- Where they meet, a bolt joins them. -->
        <span class="meet-bolt" in:springIn={{ preset: 'bouncy', scale: 0.3, y: 0 }} use:zapHere><Bolt size={30} testid="paired-bolt" /></span>
      {/if}
    </div>
    <div class="verify">
      <span class="muted">{t('code.matched.verify')}</span>
      <span class="fp tabular scan" data-testid="pair-check"><Icon name="fingerprint-simple" size={18} />{app.sheet.check}</span>
    </div>
    {#if app.sheet.via === 'nearby'}
      <p class="lede center" data-testid="paired-nearby">{t('code.matched.nearby.body')}</p>
    {:else}
      <Toggle bind:checked={trustNew} label={t('code.matched.trust')} hint={t('code.matched.trust.body')} testid="trust-new" />
    {/if}
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
      {#each detail.files.slice(0, filesShown) as f, i (f.id)}
        {@const done = detail.fileBytes[f.id] ?? 0}
        {@const p = percent(done, f.size)}
        <li in:springIn={{ delay: stagger(i), y: 8 }}>
          <span class="fname" title={f.folder ? `${f.folder}/${f.name}` : f.name}>{#if f.folder}<span class="fdir">{middleTruncate(f.folder, 24)}/</span>{/if}{middleTruncate(f.name, 34)}</span>
          <span class="fsize tabular">{p === 100 ? formatBytes(f.size) : `${p}%`}</span>
          {#if p === 100}
            <span class="fdone" in:springIn={{ preset: 'snappy', scale: 0.4, y: 0 }}><Icon name="check-circle" size={18} weight="fill" /></span>
          {:else}
            <span class="fbar"><span style:transform="scaleX({p / 100})"></span></span>
          {/if}
        </li>
      {/each}
    </ul>
    {@render moreFiles(detail.files.length)}
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
      {#each past.files.slice(0, filesShown) as f, i (f.id)}
        <li in:springIn={{ delay: stagger(i), y: 8 }}>
          <span class="fname" title={f.folder ? `${f.folder}/${f.name}` : f.name}>{#if f.folder}<span class="fdir">{middleTruncate(f.folder, 24)}/</span>{/if}{middleTruncate(f.name, 34)}</span>
          <span class="fsize tabular">{formatBytes(f.size)}</span>
        </li>
      {/each}
    </ul>
    {@render moreFiles(past.files.length)}
  </Sheet>
{:else if app.sheet?.kind === 'compose'}
  <Sheet title={composeTo ? t('compose.sendTo', { device: composeTo.name }) : t('compose.title')} onclose={close} testid="sheet-compose">
    <div class="compose" class:over={over > 0}>
      <textarea
        bind:this={composeBox}
        bind:value={draft}
        rows="5"
        placeholder={t('compose.placeholder')}
        aria-label={t('compose.title')}
        data-testid="compose-text"
        onkeydown={(e) => {
          if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) composeDone();
        }}
      ></textarea>
      <span class="compose-line" aria-hidden="true"></span>
    </div>
    {#if over > 0}
      <p class="error" role="alert" in:springIn={{ y: -6, preset: 'snappy' }}><Icon name="warning-circle" size={16} weight="bold" />{t('compose.tooLong', { over: formatBytes(over) })}</p>
    {/if}
    {#snippet footer()}
      <Button variant="secondary" icon="clipboard-text" testid="compose-paste" onclick={pasteInto}>{t('compose.paste')}</Button>
      <Button full icon="paper-plane-tilt" iconWeight="fill" disabled={!draft.trim() || over > 0} testid="compose-next" onclick={composeDone}>
        {composeTo ? t('compose.sendTo', { device: composeTo.name }) : t('compose.next')}
      </Button>
    {/snippet}
  </Sheet>
{:else if app.sheet?.kind === 'device' && sheetDevice}
  {@const d = sheetDevice}
  <Sheet title={d.name} onclose={close} testid="sheet-device">
    <div class="device-head">
      <span in:springIn={{ preset: 'bouncy', scale: 0.6, y: 0 }}><DeviceOrb device={d} size={64} showName={false} /></span>
      <div class="device-facts">
        <span class="muted">{t(d.trusted ? 'device.yours' : 'device.notYours')}</span>
        <span class="device-meta">
          {#if d.route}<ConnectionBadge route={d.route} />{/if}
          <span class="fp-small tabular" title={t('device.fingerprint')}><Icon name="fingerprint-simple" size={16} />{d.fingerprint}</span>
        </span>
      </div>
    </div>
    <ul class="list">
      {#each [{ kind: 'files', icon: 'files', label: t('device.send.files') }, { kind: 'folder', icon: 'folder-simple', label: t('device.send.folder') }, { kind: 'text', icon: 'text-aa', label: t('device.send.text') }] as a, i (a.kind)}
        <li in:springIn={{ delay: stagger(i), y: 8 }}>
          <button class="row" disabled={!d.reachable && !d.wakeable} onclick={() => sendFromDevice(a.kind as 'files' | 'folder' | 'text')} data-testid="device-send-{a.kind}">
            <span class="row-icon"><Icon name={a.icon} size={22} /></span>
            <span class="name">{a.label}</span>
            <Icon name="caret-right" size={16} />
          </button>
        </li>
      {/each}
    </ul>
    {#if !d.trusted && d.reachable && d.route === 'nearby'}
      <div class="trust-box" in:springIn={{ delay: stagger(3), y: 8 }}>
        <p class="muted small">{t('device.trust.hint')}</p>
        <Button full icon="handshake" testid="device-trust" onclick={() => app.askTrust(d.id)}>{t('device.trust')}</Button>
      </div>
    {/if}
  </Sheet>
{:else if app.sheet?.kind === 'trustWait'}
  {@const ask = app.sheet}
  {@const name = trustDevice?.name ?? app.deviceName(ask.deviceId)}
  <Sheet title={t('trust.wait.title', { device: name })} onclose={() => app.cancelTrust(ask.deviceId)} testid="sheet-trust-wait">
    <div class="meet apart">
      <span><DeviceOrb self selfName={app.selfName} size={64} showName={false} /></span>
      <span class="charge-gap" class:live={!!ask.check} aria-hidden="true"><span></span></span>
      <span>{#if trustDevice}<DeviceOrb device={trustDevice} size={64} showName={false} waiting />{/if}</span>
    </div>
    {#if ask.check}
      <div class="verify" in:springIn={{ y: 8 }}>
        <span class="muted">{t('trust.wait.body', { device: name })}</span>
        <span class="fp tabular scan" data-testid="trust-check"><Icon name="fingerprint-simple" size={18} />{ask.check}</span>
      </div>
    {:else}
      <p class="lede center" data-testid="trust-reaching">{t('trust.wait.reaching', { device: name })}</p>
    {/if}
    {#snippet footer()}
      <Button variant="secondary" full testid="trust-cancel" onclick={() => app.cancelTrust(ask.deviceId)}>{t('common.cancel')}</Button>
    {/snippet}
  </Sheet>
{:else if app.sheet?.kind === 'trustAsk'}
  {@const ask = app.sheet}
  {@const name = trustDevice?.name ?? app.deviceName(ask.deviceId)}
  <Sheet title={t('trust.ask.title', { device: name })} onclose={() => app.answerTrust(ask.deviceId, false)} testid="sheet-trust-ask">
    <div class="meet apart">
      <span><DeviceOrb self selfName={app.selfName} size={64} showName={false} /></span>
      <span class="charge-gap live" aria-hidden="true"><span></span></span>
      <span>{#if trustDevice}<DeviceOrb device={trustDevice} size={64} showName={false} />{/if}</span>
    </div>
    <p class="lede">{t('trust.ask.body', { device: name })}</p>
    <div class="verify">
      <span class="muted">{t('trust.ask.check')}</span>
      <span class="fp tabular scan" data-testid="trust-ask-check"><Icon name="fingerprint-simple" size={18} />{ask.check}</span>
    </div>
    {#snippet footer()}
      <Button variant="secondary" full testid="trust-no" onclick={() => app.answerTrust(ask.deviceId, false)}>{t('trust.ask.no')}</Button>
      <Button full icon="handshake" testid="trust-yes" onclick={() => app.answerTrust(ask.deviceId, true)}>{t('trust.ask.yes')}</Button>
    {/snippet}
  </Sheet>
{:else if app.sheet?.kind === 'text' && textItem}
  {@const item = textItem}
  <Sheet title={textTitle} onclose={close} testid="sheet-text" wide>
    <pre class="text-body" data-testid="text-body">{item.text}</pre>
    {#snippet footer()}
      <Button variant={isLink(item.text) ? 'secondary' : 'primary'} full icon="copy" testid="text-copy" onclick={() => void app.copyText(item.text)}>{t('text.copy')}</Button>
      {#if isLink(item.text)}
        <Button full icon="arrow-square-out" testid="text-open" onclick={() => void app.openLink(item.text)}>{t('text.open')}</Button>
      {/if}
    {/snippet}
  </Sheet>
{:else if app.sheet?.kind === 'confirmClearHistory'}
  <Sheet title={t('history.clear')} onclose={close} testid="sheet-clear-history">
    <p class="lede">{t('history.clear.confirm')}</p>
    {#snippet footer()}
      <Button variant="secondary" full onclick={close}>{t('common.cancel')}</Button>
      <Button variant="danger" full testid="confirm-clear-history" onclick={() => app.clearHistory()}>{t('history.clear')}</Button>
    {/snippet}
  </Sheet>
{:else if app.sheet?.kind === 'confirmOpen'}
  {@const ask = app.sheet}
  <Sheet title={t('open.risky.title', { file: middleTruncate(ask.fileName, 28) })} onclose={close} testid="sheet-confirm-open">
    <div class="risky">
      <span class="risky-icon" in:springIn={{ preset: 'bouncy', scale: 0.4, y: 0, delay: 80 }}><Icon name="warning-circle" size={28} weight="fill" /></span>
      <p class="lede">{t('open.risky.body', { device: ask.deviceName })}</p>
    </div>
    {#snippet footer()}
      <Button variant="secondary" full testid="cancel-open" onclick={close}>{t('common.cancel')}</Button>
      <Button variant="danger" full testid="confirm-open" onclick={() => app.openReceived(ask.transferId, false, true)}>{t('open.risky.confirm')}</Button>
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
  .center {
    text-align: center;
  }
  .staged-text {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .send-li {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }
  .send-li .row {
    flex: 1;
    min-width: 0;
  }
  .compose {
    position: relative;
  }
  .compose textarea {
    display: block;
    width: 100%;
    min-height: 132px;
    max-height: 40vh;
    resize: vertical;
    padding: var(--space-3) var(--space-4);
    border: 1px solid var(--color-separator);
    border-radius: var(--radius-md);
    background: var(--color-surface-sunken);
    color: var(--color-text);
    font: inherit;
    line-height: 1.45;
    outline: none;
  }
  /* The charge line: focus draws it along the bottom, like the other fields. */
  .compose-line {
    position: absolute;
    left: var(--space-3);
    right: var(--space-3);
    bottom: 0;
    height: 2px;
    border-radius: var(--radius-full);
    background: var(--color-accent);
    transform: scaleX(0);
    transform-origin: left;
    transition: transform var(--spring-smooth-duration) var(--spring-smooth);
  }
  .compose:focus-within .compose-line {
    transform: scaleX(1);
  }
  .compose.over .compose-line {
    background: var(--color-danger);
  }
  :global([data-reduced='true']) .compose-line {
    transform: none;
    opacity: 0;
    transition: opacity var(--duration-base) var(--ease-standard);
  }
  :global([data-reduced='true']) .compose:focus-within .compose-line {
    opacity: 1;
  }
  .device-head {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    margin-bottom: var(--space-4);
  }
  .device-head :global(.orb) {
    width: auto;
  }
  .device-facts {
    display: grid;
    gap: var(--space-2);
    min-width: 0;
    font-size: var(--type-footnote-size);
  }
  .device-meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
  }
  .fp-small {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    font-family: var(--font-mono);
    color: var(--color-text-secondary);
  }
  .row-icon {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: var(--radius-full);
    background: var(--color-fill);
    color: var(--color-accent-text);
  }
  .trust-box {
    display: grid;
    gap: var(--space-3);
    padding: var(--space-4);
    border-radius: var(--radius-md);
    background: var(--color-surface-sunken);
  }
  .meet.apart {
    align-items: center;
    gap: var(--space-2);
  }
  /* Between two devices that are about to trust each other: a line that charges while waiting. */
  .charge-gap {
    width: 72px;
    height: 2px;
    border-radius: var(--radius-full);
    background: var(--color-beam-track);
    overflow: hidden;
  }
  .charge-gap span {
    display: block;
    height: 100%;
    width: 40%;
    background: var(--brand-yellow);
    transform: translateX(-100%);
  }
  .charge-gap.live span {
    animation: charge-run 1100ms var(--ease-standard) infinite;
  }
  @keyframes charge-run {
    to {
      transform: translateX(260%);
    }
  }
  :global([data-reduced='true']) .charge-gap span {
    animation: none;
    width: 100%;
    transform: none;
    opacity: 0.5;
  }
  .text-body {
    margin: 0;
    max-height: 50vh;
    overflow: auto;
    padding: var(--space-4);
    border-radius: var(--radius-md);
    background: var(--color-surface-sunken);
    font: inherit;
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    user-select: text;
  }
  .fdir {
    color: var(--color-text-tertiary);
  }
  .risky {
    display: flex;
    gap: var(--space-4);
    align-items: flex-start;
  }
  .risky-icon {
    flex: none;
    display: grid;
    place-items: center;
    width: 48px;
    height: 48px;
    border-radius: var(--radius-full);
    background: var(--color-warning-subtle);
    color: var(--color-warning);
  }
  .risky .lede {
    margin: 0;
  }
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
  .more-files {
    display: flex;
    justify-content: center;
    margin: calc(-1 * var(--space-2)) 0 var(--space-4);
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
  .meet {
    position: relative;
  }
  .meet > span {
    display: inline-block;
    will-change: transform;
  }
  .meet > .meet-bolt {
    position: absolute;
    left: 50%;
    top: 50%;
    margin: -15px 0 0 -15px;
  }
  /* The check code is read off left to right by a thin line, once. */
  .scan {
    position: relative;
    overflow: hidden;
  }
  .scan::after {
    content: '';
    position: absolute;
    inset: 0;
    background: var(--color-fill);
    border-left: 2px solid var(--color-accent);
    transform: translateX(0);
    animation: scan-off 640ms var(--ease-standard) 260ms both;
  }
  @keyframes scan-off {
    to {
      transform: translateX(101%);
    }
  }
  :global([data-reduced='true']) .scan::after {
    animation: fade-off var(--duration-base) var(--ease-standard) 120ms both;
  }
  @keyframes fade-off {
    to {
      opacity: 0;
    }
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
    text-align: center;
    text-wrap: balance;
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
