<script lang="ts">
  // Small previews of the photos and videos in a request (or staged to send): up to 4 tiles, the
  // last counting the rest, and the whole list on demand. From a device you haven't trusted they
  // show blurred until you tap. Android draws the same (Previews.kt).
  import type { FileItem } from '../lib/engine/types';
  import { t } from '../lib/copy';
  import { fileKind, formatBytes } from '../lib/format';
  import { springIn } from '../lib/motion/transitions';
  import { stagger } from '../lib/motion/transitions';
  import Icon from './Icon.svelte';

  interface Props {
    files: FileItem[];
    /** Tile size, in px. */
    size?: number;
    /** From a stranger: blurred until tapped. */
    blurred?: boolean;
    /** The list of every file, under a disclosure. */
    list?: boolean;
  }

  let { files, size = 84, blurred = false, list = true }: Props = $props();

  const TILES = 4;
  /** The list never draws thousands of rows. */
  const LIST_MAX = 50;
  const kindIcon = { photo: 'image', video: 'film-strip', song: 'music-note', document: 'file-text', archive: 'file-zip', file: 'file' } as const;
  const withThumb = $derived(files.filter((f) => f.thumb));
  const tiles = $derived(withThumb.slice(0, TILES));
  /** Files not on a tile. The last tile says how many when there are any. */
  const rest = $derived(files.length - tiles.length);
  let open = $state(false);
  let revealed = $state(false);
  const hidden = $derived(blurred && !revealed);
</script>

{#if tiles.length}
  <div class="previews" data-testid="previews">
    <div class="tiles" class:hidden style:--s="{size}px">
      {#each tiles as f, i (f.id)}
        {@const last = i === tiles.length - 1 && rest > 0}
        <span class="tile" in:springIn={{ y: 6, scale: 1, delay: stagger(i) }}>
          <img src={f.thumb} alt={hidden ? '' : t('preview.label', { file: f.name })} draggable="false" decoding="async" />
          {#if fileKind(f) === 'video'}<span class="play" aria-hidden="true"><Icon name="play" size={14} weight="fill" /></span>{/if}
          {#if last && !hidden}<span class="more tabular" aria-label={t('preview.more', { count: rest })}>+{rest}</span>{/if}
        </span>
      {/each}
      {#if hidden}
        <button class="reveal" data-testid="previews-reveal" onclick={() => (revealed = true)}><span class="pill"><Icon name="eye" size={16} />{t('preview.blurred')}</span></button>
      {/if}
    </div>
    {#if list && files.length > 1}
      <button class="disclose" aria-expanded={open} data-testid="previews-list" onclick={() => (open = !open)}>
        {t(open ? 'preview.hide' : 'preview.show', { count: files.length })}
        <span class="chev" class:open><Icon name="caret-right" size={12} weight="bold" /></span>
      </button>
      {#if open}
        <ul class="files" in:springIn={{ y: -4 }}>
          {#each files.slice(0, LIST_MAX) as f (f.id)}
            <li>
              <span class="mini">
                {#if f.thumb && !hidden}<img src={f.thumb} alt="" draggable="false" />{:else}<Icon name={kindIcon[fileKind(f)]} size={16} />{/if}
              </span>
              <span class="name">{f.folder ? `${f.folder}/` : ''}{f.name}</span>
              <span class="size tabular">{formatBytes(f.size)}</span>
            </li>
          {/each}
          {#if files.length > LIST_MAX}<li class="muted">{t('preview.more', { count: files.length - LIST_MAX })}</li>{/if}
        </ul>
      {/if}
    {/if}
  </div>
{/if}

<style>
  .previews {
    display: grid;
    gap: var(--space-2);
  }
  .tiles {
    position: relative;
    display: flex;
    gap: var(--space-2);
    width: fit-content;
    max-width: 100%;
  }
  .tile {
    position: relative;
    width: var(--s);
    height: var(--s);
    flex: none;
    overflow: hidden;
    border-radius: var(--radius-md);
    background: var(--color-surface-sunken);
  }
  .tile img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
    transition: filter var(--duration-base) var(--ease-standard);
  }
  .hidden .tile img {
    filter: blur(10px);
  }
  .play {
    position: absolute;
    left: 6px;
    bottom: 6px;
    width: 24px;
    height: 24px;
    display: grid;
    place-items: center;
    border-radius: var(--radius-full);
    background: rgb(0 0 0 / 55%);
    color: #fff;
  }
  .more {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    background: rgb(0 0 0 / 64%);
    color: #fff;
    font-weight: 650;
    font-size: var(--type-headline-size);
  }
  .reveal {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    border-radius: var(--radius-md);
  }
  .pill {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    max-width: calc(100% - 16px);
    padding: 6px var(--space-3);
    border-radius: var(--radius-full);
    background: rgb(0 0 0 / 62%);
    color: #fff;
    font-weight: 600;
    font-size: var(--type-footnote-size);
    white-space: nowrap;
  }
  .reveal:focus-visible,
  .disclose:focus-visible {
    outline: 2px solid var(--color-focus-ring);
    outline-offset: 2px;
  }
  .disclose {
    justify-self: start;
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    min-height: 36px;
    padding: 0 var(--space-1);
    border-radius: var(--radius-sm);
    color: var(--color-accent-text);
    font-weight: 600;
    font-size: var(--type-footnote-size);
  }
  .chev {
    display: inline-grid;
    transition: transform var(--duration-fast) var(--ease-standard);
  }
  .chev.open {
    transform: rotate(90deg);
  }
  .files {
    display: grid;
    margin: 0;
    padding: 0;
    list-style: none;
    max-height: 220px;
    overflow-y: auto;
  }
  .files li {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-height: 40px;
    font-size: var(--type-footnote-size);
  }
  .mini {
    width: 32px;
    height: 32px;
    flex: none;
    display: grid;
    place-items: center;
    overflow: hidden;
    border-radius: var(--radius-sm);
    background: var(--color-surface-sunken);
    color: var(--color-text-secondary);
  }
  .mini img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .size,
  .muted {
    color: var(--color-text-secondary);
  }
  :global([data-reduced='true']) .tile img,
  :global([data-reduced='true']) .chev {
    transition: none;
  }
</style>
