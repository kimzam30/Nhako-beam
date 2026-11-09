<script lang="ts">
  // A send to several devices: one card, a row per device. Each device accepts or declines on its
  // own; a row opens that transfer's details. Android draws the same card (GroupCard.kt).
  import type { Transfer } from '../lib/engine/types';
  import { app } from '../lib/state/app.svelte';
  import { groupProgress } from '../lib/state/groups';
  import { t } from '../lib/copy';
  import { formatBytes, formatSpeed, percent, summarizeFiles, textPreview } from '../lib/format';
  import Button from './Button.svelte';
  import IconButton from './IconButton.svelte';

  interface Props {
    id: string;
    transfers: Transfer[];
  }

  let { id, transfers }: Props = $props();

  const ENDED = ['done', 'failed', 'cancelled', 'declined'];
  const first = $derived(transfers[0]!);
  const what = $derived(first.text !== undefined ? textPreview(first.text) : `${summarizeFiles(first.files)} · ${formatBytes(first.totalBytes)}`);
  const allEnded = $derived(transfers.every((x) => ENDED.includes(x.status)));
  const allDone = $derived(transfers.every((x) => x.status === 'done'));
  const pct = $derived(Math.floor(groupProgress(transfers) * 100));
  const title = $derived(
    t(allDone ? 'group.sent' : allEnded ? 'group.ended' : 'group.sending', { what: first.text !== undefined ? t('group.text') : summarizeFiles(first.files), count: transfers.length }),
  );

  function rowText(x: Transfer): string {
    switch (x.status) {
      case 'transferring':
        return t('group.row.moving', { percent: percent(x.doneBytes, x.totalBytes), speed: formatSpeed(x.speed) });
      case 'waiting':
      case 'connecting':
      case 'paused':
      case 'reconnecting':
      case 'done':
      case 'declined':
      case 'failed':
      case 'cancelled':
        return t(`group.row.${x.status}`);
      default:
        return '';
    }
  }
  const rowPct = (x: Transfer) => (x.status === 'done' ? 100 : percent(x.doneBytes, x.totalBytes));

  // All done: fold into history after 6 s, like a single transfer, unless you're looking at it.
  let hovered = $state(false);
  let focused = $state(false);
  $effect(() => {
    if (!allDone || hovered || focused) return;
    const timer = setTimeout(() => app.dismissGroup(id), 6000);
    return () => clearTimeout(timer);
  });
</script>

<article
  class="card"
  data-testid="group-{id}"
  aria-label={title}
  onpointerenter={() => (hovered = true)}
  onpointerleave={() => (hovered = false)}
  onfocusin={() => (focused = true)}
  onfocusout={() => (focused = false)}
>
  <header>
    <div class="titles">
      <h3 class="title" aria-live="polite">{title}</h3>
      <p class="detail">{what}</p>
    </div>
    {#if !allEnded}<span class="pct tabular">{pct}%</span>{/if}
    {#if allEnded}<IconButton icon="x" label={t('group.dismiss')} testid="group-dismiss" onclick={() => app.dismissGroup(id)} />{/if}
  </header>
  <ul class="rows">
    {#each transfers as x (x.id)}
      <li>
        <button class="row" onclick={() => (app.sheet = { kind: 'transfer', id: x.id })} data-testid="group-row-{x.id}" data-status={x.status}>
          <span class="line">
            <span class="name">{app.deviceName(x.deviceId)}</span>
            <span class="state {x.status}">{rowText(x)}</span>
          </span>
          <span class="track" aria-hidden="true"><span class="fill {x.status}" style:transform="scaleX({rowPct(x) / 100})"></span></span>
        </button>
      </li>
    {/each}
  </ul>
  {#if !allEnded}
    <p class="hint">{t('group.hint')}</p>
    <Button variant="plain" size="sm" testid="group-cancel" sound="cancel" onclick={() => app.cancelGroup(id)}>{t('group.cancelAll')}</Button>
  {/if}
</article>

<style>
  .card {
    display: grid;
    gap: var(--space-3);
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
    align-items: flex-start;
    gap: var(--space-3);
  }
  .titles {
    flex: 1;
    min-width: 0;
  }
  .title {
    margin: 0;
    font-size: var(--type-headline-size);
    font-weight: var(--type-headline-weight);
    line-height: var(--type-headline-line);
  }
  .detail,
  .hint {
    margin: 0;
    font-size: var(--type-footnote-size);
    line-height: var(--type-footnote-line);
    color: var(--color-text-secondary);
  }
  .detail {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .pct {
    font-size: var(--type-footnote-size);
    color: var(--color-text-secondary);
  }
  .rows {
    display: grid;
    gap: var(--space-1);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .row {
    display: grid;
    gap: 6px;
    width: 100%;
    padding: var(--space-2);
    border-radius: var(--radius-sm);
    text-align: left;
    transition: background-color var(--duration-fast) var(--ease-standard);
  }
  @media (hover: hover) {
    .row:hover {
      background: var(--color-fill);
    }
  }
  .row:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: 2px;
  }
  .line {
    display: flex;
    justify-content: space-between;
    gap: var(--space-3);
    font-size: var(--type-footnote-size);
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .state {
    flex: none;
    color: var(--color-text-secondary);
    font-variant-numeric: tabular-nums;
  }
  .state.done {
    color: var(--color-success);
  }
  .state.failed,
  .state.declined {
    color: var(--color-danger);
  }
  .track {
    height: 6px;
    border-radius: var(--radius-full);
    background: var(--color-beam-track);
    overflow: hidden;
  }
  .fill {
    display: block;
    height: 100%;
    background: var(--color-accent);
    transform-origin: left;
    transition: transform var(--duration-base) var(--ease-standard), background-color var(--duration-fast);
  }
  .fill.done {
    background: var(--color-success);
  }
  .fill.failed,
  .fill.declined,
  .fill.cancelled {
    background: var(--color-text-tertiary);
  }
  :global([data-reduced='true']) .fill {
    transition: none;
  }
</style>
