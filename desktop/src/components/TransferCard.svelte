<script lang="ts">
  import type { Transfer } from '../lib/engine/types';
  import { app } from '../lib/state/app.svelte';
  import { t } from '../lib/copy';
  import { formatBytes, formatEta, formatSpeed, percent, summarizeFiles } from '../lib/format';
  import { shake } from '../lib/motion/springs';
  import { springIn } from '../lib/motion/transitions';
  import Beam from './Beam.svelte';
  import Button from './Button.svelte';
  import ConnectionBadge from './ConnectionBadge.svelte';
  import Icon from './Icon.svelte';
  import IconButton from './IconButton.svelte';

  interface Props {
    transfer: Transfer;
    ondetails?: () => void;
  }

  let { transfer: tr, ondetails }: Props = $props();

  let card: HTMLElement | undefined = $state();
  let hovered = $state(false);
  let focused = $state(false);

  const device = $derived(app.device(tr.deviceId));
  const name = $derived(device?.name ?? 'The other device');
  const pct = $derived(percent(tr.doneBytes, tr.totalBytes));
  const moving = $derived(tr.status === 'transferring');
  const ended = $derived(['done', 'failed', 'cancelled', 'declined'].includes(tr.status));
  const showEta = $derived(moving && tr.movingSince !== null && Date.now() - tr.movingSince > 3000);
  const eta = $derived(formatEta(tr.totalBytes - tr.doneBytes, tr.speed));
  const mode = $derived(
    tr.status === 'transferring' ? 'moving' : tr.status === 'paused' ? 'paused' : tr.status === 'reconnecting' ? 'reconnecting' : tr.status === 'done' ? 'done' : tr.status === 'failed' ? 'failed' : 'idle',
  );
  const kindIcon = { laptop: 'laptop', desktop: 'desktop-tower', phone: 'device-mobile', tablet: 'device-tablet' } as const;

  const title = $derived.by(() => {
    switch (tr.status) {
      case 'waiting':
        return tr.waitingLong ? t('send.waiting.long', { device: name }) : t('send.waiting', { device: name });
      case 'connecting':
        return tr.waking ? t('transfer.waking', { device: name }) : t('transfer.connecting');
      case 'done':
        return tr.direction === 'send' ? t('transfer.done.sent', { device: name }) : t('transfer.done.received', { device: name });
      case 'declined':
        return t('send.declined', { device: name });
      case 'failed':
        return tr.error === 'peerGone' && tr.doneBytes === 0 ? t('send.unreachable', { device: name }) : t('transfer.failed');
      case 'cancelled':
        return tr.cancelledBy === 'peer' ? t('transfer.cancelled.byOther', { device: name }) : t('transfer.cancelled.byYou');
      default:
        return tr.direction === 'send' ? t('transfer.sending', { device: name }) : t('transfer.receiving', { device: name });
    }
  });

  const detail = $derived.by(() => {
    if (tr.status === 'paused') {
      if (tr.error === 'diskFull') return t('transfer.diskFull');
      if (tr.error === 'diskFullPeer') return t('transfer.diskFull.other', { device: name });
      if (tr.pausedBy === 'peer') return t('transfer.paused.byOther', { device: name });
      return t('transfer.paused');
    }
    if (tr.status === 'reconnecting') return t('transfer.reconnecting');
    if (tr.status === 'failed' && tr.error && !(tr.error === 'peerGone' && tr.doneBytes === 0)) {
      const key = `error.${tr.error}` as const;
      return t(key as Parameters<typeof t>[0], { device: name, fileName: tr.files[0]?.name ?? 'A file' });
    }
    return `${summarizeFiles(tr.files)} · ${formatBytes(tr.totalBytes)}`;
  });

  // Failure: one small shake.
  let prevStatus: string | null = null;
  $effect(() => {
    const s = tr.status;
    if (prevStatus !== null && s !== prevStatus && s === 'failed' && card) shake(card);
    prevStatus = s;
  });

  // Done: collapse into history after 6 s unless hovered or focused.
  $effect(() => {
    if (tr.status !== 'done' || hovered || focused) return;
    const id = tr.id;
    const timer = setTimeout(() => app.dismissTransfer(id), 6000);
    return () => clearTimeout(timer);
  });
</script>

<article
  class="card {tr.status}"
  bind:this={card}
  onpointerenter={() => (hovered = true)}
  onpointerleave={() => (hovered = false)}
  onfocusin={() => (focused = true)}
  onfocusout={() => (focused = false)}
  data-testid="transfer-{tr.id}"
  data-status={tr.status}
  aria-label={title}
>
  <header>
    <span class="avatar" class:done={tr.status === 'done'}>
      {#if tr.status === 'done'}
        <span class="check" in:springIn={{ preset: 'bouncy', scale: 0.3, y: 0 }}><Icon name="check-circle" size={22} weight="fill" /></span>
      {:else}
        <Icon name={tr.direction === 'send' ? kindIcon[device?.kind ?? 'laptop'] : 'download-simple'} size={20} />
      {/if}
    </span>
    <div class="titles">
      <h3 class="title">{title}</h3>
      <p class="detail" class:warn={tr.status === 'reconnecting' || tr.error === 'diskFull' || tr.error === 'diskFullPeer'} class:bad={tr.status === 'failed'}>{detail}</p>
    </div>
  </header>

  {#if tr.status !== 'waiting' && tr.status !== 'declined' && !(tr.status === 'failed' && tr.doneBytes === 0)}
    <Beam progress={tr.doneBytes / tr.totalBytes} intensity={tr.speed / 92e6} {mode} collapsed={tr.status === 'done'} />
  {/if}

  <footer>
    {#if tr.status === 'transferring' || tr.status === 'paused' || tr.status === 'reconnecting' || tr.status === 'connecting'}
      <p class="numbers tabular" aria-live="off">
        <span class="pct">{pct}%</span>
        {#if moving && tr.speed > 0}<span>{formatSpeed(tr.speed)}</span>{/if}
        {#if moving}<span class="eta">{showEta && eta ? t('transfer.eta', { time: eta }) : t('transfer.eta.calculating')}</span>{/if}
        <ConnectionBadge route={tr.route} />
      </p>
      <div class="actions">
        {#if tr.status === 'paused' && tr.pausedBy !== 'peer'}
          <IconButton icon="play" weight="fill" label={t('transfer.resume')} tone="accent" testid="resume" sound="resume" onclick={() => app.engine.resume(tr.id)} />
        {:else if tr.status === 'transferring'}
          <IconButton icon="pause" weight="fill" label={t('transfer.pause')} testid="pause" sound="pause" onclick={() => app.engine.pause(tr.id)} />
        {/if}
        {#if ondetails}<IconButton icon="files" label={t('transfer.details')} testid="details" onclick={ondetails} />{/if}
        <IconButton icon="x" label={t('send.cancel')} testid="cancel" sound="cancel" onclick={() => app.engine.cancel(tr.id)} />
      </div>
    {:else if tr.status === 'waiting'}
      <span class="pulse-dot" aria-hidden="true"></span>
      <div class="actions grow">
        {#if tr.waitingLong}
          <Button variant="secondary" size="sm" testid="keep-waiting" onclick={() => app.toast(t('send.waiting', { device: name }))}>{t('send.waiting.keep')}</Button>
        {/if}
        <Button variant="plain" size="sm" testid="cancel" sound="cancel" onclick={() => app.engine.cancel(tr.id)}>{t('send.cancel')}</Button>
      </div>
    {:else if tr.status === 'done'}
      <div class="actions grow">
        <!-- Only received files are saved here, so there's nothing to open after sending. -->
        {#if tr.direction === 'receive'}
          <Button variant="secondary" size="sm" icon="arrow-square-out" testid="open" onclick={() => app.openReceived(tr.id, false)}>{t('transfer.done.open')}</Button>
          {#if app.isAndroid}
            <Button variant="plain" size="sm" icon="share-network" onclick={() => app.toast('Prototype: would open the share sheet')}>{t('transfer.done.share')}</Button>
          {:else}
            <Button variant="plain" size="sm" icon="folder-open" testid="show-in-folder" onclick={() => app.openReceived(tr.id, true)}>{t('transfer.done.showInFolder')}</Button>
          {/if}
        {/if}
        <span class="spacer"></span>
        <IconButton icon="x" label={t('transfer.dismiss')} testid="dismiss" onclick={() => app.dismissTransfer(tr.id)} />
      </div>
    {:else}
      <div class="actions grow">
        {#if tr.status === 'failed' || tr.status === 'declined' || (tr.status === 'cancelled' && tr.cancelledBy === 'peer')}
          <Button variant="secondary" size="sm" icon="arrow-clockwise" testid="retry" sound="resume" onclick={() => app.engine.retry(tr.id)}>{t('transfer.failed.retry')}</Button>
          {#if tr.status === 'failed' && tr.doneBytes > 0}<span class="hint">{t('transfer.failed.retry.hint')}</span>{/if}
        {/if}
        <span class="spacer"></span>
        <IconButton icon="x" label={t('transfer.dismiss')} testid="dismiss" onclick={() => app.dismissTransfer(tr.id)} />
      </div>
    {/if}
  </footer>
</article>

<style>
  .card {
    display: grid;
    gap: var(--space-4);
    padding: var(--space-5);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow: var(--elevation-1);
    transition: box-shadow var(--spring-smooth-duration) var(--spring-smooth);
  }
  @media (hover: hover) {
    .card:hover {
      box-shadow: var(--elevation-2);
    }
  }
  header {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    min-width: 0;
  }
  .avatar {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    flex: none;
    border-radius: 50%;
    background: var(--color-accent-subtle);
    color: var(--color-accent-text);
    transition:
      background-color var(--duration-base) var(--ease-standard),
      color var(--duration-base) var(--ease-standard);
  }
  .avatar.done {
    background: var(--color-success-subtle);
    color: var(--color-success);
  }
  .failed .avatar {
    background: var(--color-danger-subtle);
    color: var(--color-danger);
  }
  .titles {
    flex: 1;
    min-width: 0;
  }
  .title {
    font-size: var(--type-headline-size);
    line-height: var(--type-headline-line);
    font-weight: var(--type-headline-weight);
    letter-spacing: var(--type-headline-tracking);
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  .detail {
    font-size: var(--type-footnote-size);
    line-height: var(--type-footnote-line);
    color: var(--color-text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    display: -webkit-box;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    -webkit-box-orient: vertical;
  }
  .detail.warn {
    color: var(--color-warning);
  }
  .detail.bad {
    color: var(--color-danger);
  }
  footer {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-height: 36px;
    margin: calc(var(--space-2) * -1) 0;
  }
  .numbers {
    flex: 1;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-1) var(--space-4);
    font-size: var(--type-footnote-size);
    color: var(--color-text-secondary);
    font-variant-numeric: tabular-nums;
  }
  .pct {
    color: var(--color-text-primary);
    font-weight: 600;
    min-width: 3.2ch;
  }
  .actions {
    display: flex;
    align-items: center;
    gap: var(--space-1);
  }
  .actions.grow {
    flex: 1;
    gap: var(--space-2);
  }
  .spacer {
    flex: 1;
  }
  .hint {
    font-size: var(--type-footnote-size);
    color: var(--color-text-secondary);
  }
  .pulse-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--color-accent);
    animation: breathe var(--duration-searching-pulse) var(--ease-standard) infinite;
  }
  @keyframes breathe {
    0%,
    100% {
      opacity: 0.3;
      transform: scale(0.8);
    }
    50% {
      opacity: 1;
      transform: scale(1);
    }
  }
  .check {
    display: grid;
  }
</style>
