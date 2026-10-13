<script lang="ts">
  // History: every transfer that ended, grouped by day, newest first. The engine keeps the list
  // across restarts (core/src/history.rs). docs/design/screens-and-states.md, section 7.
  import { app } from '../lib/state/app.svelte';
  import type { HistoryEntry } from '../lib/engine/types';
  import { t } from '../lib/copy';
  import { dayLabel, dayStart, formatBytes, isLink, summarize, timeOfDay } from '../lib/format';
  import { springIn, quickOut, springFlip, stagger } from '../lib/motion/transitions';
  import Button from '../components/Button.svelte';
  import Bolt from '../components/Bolt.svelte';
  import Icon from '../components/Icon.svelte';
  import IconButton from '../components/IconButton.svelte';

  const now = Date.now();
  const groups = $derived.by(() => {
    const out: { day: number; label: string; entries: HistoryEntry[] }[] = [];
    for (const e of app.history) {
      const day = dayStart(e.finishedAt);
      let g = out.at(-1);
      if (!g || g.day !== day) {
        g = { day, label: dayLabel(e.finishedAt, now, t('history.today'), t('history.yesterday')), entries: [] };
        out.push(g);
      }
      g.entries.push(e);
    }
    return out;
  });

  /** Rows up to here come in one after another; the rest just appear. */
  let row = 0;
  const order = new Map<string, number>();
  $effect.pre(() => {
    for (const e of app.history) if (!order.has(e.id)) order.set(e.id, row++);
  });

  function status(e: HistoryEntry): string | null {
    if (e.status === 'failed') return t('history.status.failed');
    if (e.status === 'cancelled') return t('history.status.cancelled');
    if (e.status === 'declined') return t('history.status.declined');
    return null;
  }

  function back() {
    app.navigate('home');
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && !app.sheet && back()} />

<section class="history" data-testid="history">
  <header class="bar">
    <IconButton icon="caret-left" label={t('settings.back')} size={24} testid="history-back" onclick={back} />
    <h1>{t('history.title')}</h1>
    {#if app.history.length}
      <span class="clear" out:quickOut={{ y: 0 }}>
        <Button variant="plain" size="sm" testid="history-clear" onclick={() => (app.sheet = { kind: 'confirmClearHistory' })}>{t('history.clear')}</Button>
      </span>
    {/if}
  </header>

  {#each groups as g (g.day)}
    <section class="day" animate:springFlip out:quickOut={{ y: 0 }}>
      <h2 in:springIn={{ delay: stagger(order.get(g.entries[0]!.id) ?? 0), y: 8 }}>{g.label}</h2>
      <ul class="card">
        {#each g.entries as e, i (e.id)}
          {@const s = status(e)}
          <li animate:springFlip in:springIn={{ delay: stagger(order.get(e.id) ?? 0), y: 12 }} out:quickOut={{ x: -16, y: 0 }}>
            {#if i > 0}<div class="sep"></div>{/if}
            <button class="row" onclick={() => (app.sheet = e.text !== undefined && e.status === 'done' ? { kind: 'text', id: e.id } : { kind: 'historyItem', id: e.id })} data-testid="history-row-{e.id}">
              <span class="dir" class:receive={e.direction === 'receive'} class:muted-dir={!!s}>
                <Icon name={e.direction === 'send' ? 'upload-simple' : 'download-simple'} size={18} />
              </span>
              <span class="text">
                <strong>{summarize(e)}</strong>
                <span class="sub tabular">
                  {t(e.direction === 'send' ? 'history.sent' : 'history.received', { device: e.deviceName })} · {e.text !== undefined ? t(isLink(e.text) ? 'staged.link' : 'staged.text') : formatBytes(e.totalBytes)}
                </span>
              </span>
              <span class="side">
                <span class="time tabular">{timeOfDay(e.finishedAt)}</span>
                {#if s}<span class="status" class:failed={e.status === 'failed'}>{s}</span>{/if}
              </span>
            </button>
          </li>
        {/each}
      </ul>
    </section>
  {:else}
    <div class="empty" in:springIn={{ delay: 80 }} data-testid="history-empty">
      <!-- A sleeping bolt: it breathes once when you arrive, then rests. -->
      <span class="empty-icon"><Bolt size={34} mode="asleep" testid="sleeping-bolt" /></span>
      <p>{t('history.empty')}</p>
    </div>
  {/each}
</section>

<style>
  .history {
    max-width: 640px;
    margin: 0 auto;
    padding: var(--space-5) var(--space-5) calc(var(--space-9) + env(safe-area-inset-bottom));
  }
  .bar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0 0 var(--space-6) calc(var(--space-3) * -1);
  }
  h1 {
    flex: 1;
    font-size: var(--type-title1-size);
    line-height: var(--type-title1-line);
    font-weight: var(--type-title1-weight);
    letter-spacing: var(--type-title1-tracking);
  }
  .day + .day {
    margin-top: var(--space-6);
  }
  .day h2 {
    margin: 0 0 var(--space-3) var(--space-2);
    font-size: var(--type-footnote-size);
    font-weight: 600;
    color: var(--color-text-secondary);
  }
  .card {
    list-style: none;
    margin: 0;
    padding: var(--space-1) 0;
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow: var(--elevation-1);
  }
  .sep {
    height: 1px;
    margin-left: calc(var(--space-5) + 36px + var(--space-4));
    background: var(--color-separator);
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    width: 100%;
    padding: var(--space-3) var(--space-5);
    text-align: left;
    border-radius: var(--radius-md);
    transition:
      background-color var(--duration-fast) var(--ease-standard),
      transform var(--spring-press-duration) var(--spring-press);
  }
  .row:active {
    transform: scale(var(--scale-pressed));
  }
  @media (hover: hover) {
    .row:hover {
      background: var(--color-fill);
    }
  }
  .row:active {
    background: var(--color-fill-pressed);
  }
  .dir {
    display: grid;
    place-items: center;
    flex: none;
    width: 36px;
    height: 36px;
    border-radius: var(--radius-full);
    background: var(--color-accent-subtle);
    color: var(--color-accent-text);
  }
  .dir.receive {
    background: var(--color-success-subtle);
    color: var(--color-success);
  }
  .dir.muted-dir {
    background: var(--color-fill);
    color: var(--color-text-tertiary);
  }
  .text {
    flex: 1;
    display: grid;
    min-width: 0;
  }
  .text strong {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-weight: 500;
  }
  .sub {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--type-footnote-size);
    color: var(--color-text-secondary);
  }
  .side {
    display: grid;
    justify-items: end;
    gap: 2px;
    flex: none;
  }
  .time {
    font-size: var(--type-footnote-size);
    color: var(--color-text-secondary);
  }
  .status {
    font-size: var(--type-caption-size);
    font-weight: 600;
    color: var(--color-text-tertiary);
  }
  .status.failed {
    color: var(--color-danger);
  }
  .empty {
    display: grid;
    justify-items: center;
    gap: var(--space-4);
    padding: var(--space-9) var(--space-5);
    text-align: center;
    color: var(--color-text-secondary);
  }
  .empty-icon {
    display: grid;
    place-items: center;
    width: 64px;
    height: 64px;
    border-radius: var(--radius-full);
    background: var(--color-fill);
    color: var(--color-text-tertiary);
  }
</style>
