<script lang="ts">
  // About > Open source licenses: every package the app ships, with its license text. The list
  // is made by scripts/licenses.mjs and only loaded when this screen opens.
  import { onMount } from 'svelte';
  import { app } from '../lib/state/app.svelte';
  import { openLink } from '../lib/shell';
  import { t } from '../lib/copy';
  import { springIn, quickOut } from '../lib/motion/transitions';
  import Icon from '../components/Icon.svelte';
  import IconButton from '../components/IconButton.svelte';

  interface Pkg {
    name: string;
    version: string;
    license: string;
    url: string;
    ecosystem: string;
    texts: string[];
  }
  interface Data {
    packages: Pkg[];
    texts: Record<string, string>;
  }

  let data = $state<Data | null>(null);
  let query = $state('');
  let open = $state<string | null>(null);

  onMount(() => {
    void import('../lib/licenses.json').then((m) => (data = m.default as Data));
  });

  const shown = $derived.by(() => {
    if (!data) return [];
    const q = query.trim().toLowerCase();
    return q ? data.packages.filter((p) => p.name.toLowerCase().includes(q) || p.license.toLowerCase().includes(q)) : data.packages;
  });

  const key = (p: Pkg) => `${p.ecosystem}:${p.name}@${p.version}`;

  function back() {
    app.navigate('settings');
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && back()} />

<section class="licenses" data-testid="licenses">
  <header class="bar">
    <IconButton icon="caret-left" label={t('settings.back')} size={24} testid="licenses-back" onclick={back} />
    <h1>{t('licenses.title')}</h1>
  </header>

  <p class="intro" in:springIn={{ y: 8 }}>{t('licenses.intro')}</p>

  {#if data}
    <div class="search" in:springIn={{ delay: 40, y: 8 }}>
      <Icon name="magnifying-glass" size={18} />
      <input bind:value={query} placeholder={t('licenses.search')} aria-label={t('licenses.search')} data-testid="licenses-search" />
      <span class="count tabular">{t('licenses.count', { count: shown.length })}</span>
    </div>

    <ul class="card" in:springIn={{ delay: 80, y: 12 }}>
      {#each shown as p (key(p))}
        {@const k = key(p)}
        <li class="pkg">
          <button class="row" aria-expanded={open === k} onclick={() => (open = open === k ? null : k)}>
            <span class="text">
              <strong>{p.name}</strong>
              <span class="sub tabular">{p.version} · {p.license}</span>
            </span>
            <span class="chev" class:turned={open === k}><Icon name="caret-right" size={16} /></span>
          </button>
          {#if open === k}
            <div class="body" in:springIn={{ y: -4 }} out:quickOut={{ y: -4 }}>
              <a
                href={p.url}
                target="_blank"
                rel="noreferrer"
                onclick={(e) => {
                  if (!app.native) return;
                  e.preventDefault();
                  openLink(p.url).catch(() => app.toast(t('transfer.openFailed'), 'danger'));
                }}>{p.url}</a
              >
              {#each p.texts as id (id)}
                <pre>{data.texts[id]}</pre>
              {/each}
            </div>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .licenses {
    max-width: 720px;
    margin: 0 auto;
    padding: var(--space-5) var(--space-5) calc(var(--space-9) + env(safe-area-inset-bottom));
  }
  .bar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0 0 var(--space-4) calc(var(--space-3) * -1);
  }
  h1 {
    font-size: var(--type-title1-size);
    line-height: var(--type-title1-line);
    font-weight: var(--type-title1-weight);
    letter-spacing: var(--type-title1-tracking);
  }
  .intro {
    margin: 0 0 var(--space-5);
    color: var(--color-text-secondary);
    font-size: var(--type-callout-size);
  }
  .search {
    position: sticky;
    top: var(--space-3);
    z-index: 1;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-bottom: var(--space-4);
    padding: 0 var(--space-4);
    min-height: var(--size-control-lg);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    box-shadow: var(--elevation-1);
    color: var(--color-text-tertiary);
  }
  .search input {
    flex: 1;
    min-width: 0;
    border: 0;
    background: none;
    color: var(--color-text-primary);
  }
  .search input:focus {
    outline: none;
  }
  .search:focus-within {
    box-shadow: inset 0 0 0 2px var(--color-accent);
  }
  .count {
    font-size: var(--type-footnote-size);
  }
  .card {
    list-style: none;
    margin: 0;
    padding: var(--space-1) 0;
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow: var(--elevation-1);
  }
  .pkg {
    /* Hundreds of rows: let the browser skip the ones off screen. */
    content-visibility: auto;
    contain-intrinsic-size: auto 56px;
  }
  .pkg + .pkg {
    border-top: 1px solid var(--color-separator);
  }
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    width: 100%;
    padding: var(--space-3) var(--space-5);
    text-align: left;
    transition: background-color var(--duration-fast) var(--ease-standard);
  }
  @media (hover: hover) {
    .row:hover {
      background: var(--color-fill);
    }
  }
  .text {
    flex: 1;
    display: grid;
    min-width: 0;
  }
  .text strong {
    font-weight: 500;
    overflow-wrap: anywhere;
  }
  .sub {
    font-size: var(--type-footnote-size);
    color: var(--color-text-secondary);
  }
  .chev {
    color: var(--color-text-tertiary);
    transition: transform var(--spring-snappy-duration) var(--spring-snappy);
  }
  .chev.turned {
    transform: rotate(90deg);
  }
  .body {
    display: grid;
    gap: var(--space-3);
    padding: 0 var(--space-5) var(--space-4);
  }
  .body a {
    font-size: var(--type-footnote-size);
    color: var(--color-accent-text);
    overflow-wrap: anywhere;
  }
  pre {
    margin: 0;
    padding: var(--space-4);
    max-height: 320px;
    overflow: auto;
    border-radius: var(--radius-sm);
    background: var(--color-surface-sunken);
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.5;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    color: var(--color-text-secondary);
  }
  :global([data-motion='reduced']) .chev {
    transition: none;
  }
</style>
