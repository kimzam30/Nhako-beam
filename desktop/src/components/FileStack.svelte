<script lang="ts">
  // Up to 3 stacked file cards with a count badge. Fans out on hover (or when `fanned`).
  import type { FileItem } from '../lib/engine/types';
  import { fileKind, isLink } from '../lib/format';
  import Icon from './Icon.svelte';

  interface Props {
    files: Pick<FileItem, 'name' | 'mime' | 'folder'>[];
    /** Text being sent: one card with a text (or link) glyph. */
    text?: string | null;
    count?: number;
    size?: number;
    fanned?: boolean;
    lifted?: boolean;
  }

  let { files, text = null, count, size = 56, fanned = false, lifted = false }: Props = $props();

  const kindIcon = { photo: 'image', video: 'film-strip', song: 'music-note', document: 'file-text', archive: 'file-zip', file: 'file' } as const;
  // A sent folder shows as a folder card on top of its files.
  const iconOf = (f: Pick<FileItem, 'name' | 'mime' | 'folder'>) => (f.folder ? 'folder-simple' : kindIcon[fileKind(f)]);
  const top = $derived(text !== null ? [{ name: 'text', mime: 'text/plain' }] : files.slice(0, 3));
  const total = $derived(text !== null ? 1 : (count ?? files.length));
  const textIcon = $derived(text !== null && isLink(text) ? 'link-simple' : 'text-aa');
</script>

<span class="stack" class:fanned class:lifted style:--s="{size}px" aria-hidden="true">
  {#each top.slice().reverse() as f, i (i)}
    {@const depth = top.length - 1 - i}
    <span class="card" style:--d={depth}>
      <Icon name={text !== null ? textIcon : iconOf(f)} size={Math.round(size * 0.42)} />
    </span>
  {/each}
  {#if total > 1}
    <span class="count tabular">{total > 99 ? '99+' : total}</span>
  {/if}
</span>

<style>
  .stack {
    position: relative;
    display: inline-block;
    width: var(--s);
    height: calc(var(--s) * 1.18);
    flex: none;
  }
  .card {
    position: absolute;
    inset: 0;
    display: grid;
    place-items: center;
    border-radius: var(--radius-md);
    background: var(--color-surface-raised);
    color: var(--color-accent-text);
    box-shadow: var(--elevation-2);
    transform: rotate(calc(var(--d) * -5deg)) translate(calc(var(--d) * -3px), calc(var(--d) * -2px));
    transition:
      transform var(--spring-smooth-duration) var(--spring-smooth),
      box-shadow var(--spring-smooth-duration) var(--spring-smooth);
  }
  .card:last-of-type {
    box-shadow: var(--elevation-2), 0 0 0 1px var(--color-separator);
  }
  .fanned .card,
  .stack:hover .card {
    transform: rotate(calc(var(--d) * -12deg)) translate(calc(var(--d) * -10px), calc(var(--d) * -4px));
  }
  .lifted .card {
    box-shadow: var(--elevation-4);
  }
  .count {
    position: absolute;
    top: -8px;
    right: -10px;
    min-width: 24px;
    height: 24px;
    padding: 0 6px;
    display: grid;
    place-items: center;
    border-radius: var(--radius-full);
    background: var(--color-accent);
    color: var(--color-on-accent);
    font-size: var(--type-caption-size);
    font-weight: 700;
    box-shadow: var(--elevation-1);
  }
</style>
