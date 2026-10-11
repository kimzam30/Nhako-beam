<script lang="ts">
  // Three word slots. Autocomplete shows the rest of the word as ghost text (Tab, Enter or
  // right arrow accepts it; on touch, tap the suggestion chip). Pasting fills all three.
  // A correct word locks in with a small snappy pop; a wrong one shakes.
  import { isWord, splitCode, suggest } from '../lib/engine/words';
  import { shake } from '../lib/motion/springs';
  import { springIn } from '../lib/motion/transitions';
  import Icon from './Icon.svelte';

  interface Props {
    words: string[];
    disabled?: boolean;
    onsubmit: () => void;
  }

  let { words = $bindable(), disabled = false, onsubmit }: Props = $props();

  let inputs: HTMLInputElement[] = $state([]);
  let slots: HTMLElement[] = $state([]);
  let focusIndex = $state(0);

  const valid = $derived(words.map((w) => isWord(w)));
  const ghost = $derived(words.map((w) => (w && !isWord(w) ? (suggest(w, 1)[0] ?? '') : '')));
  const chips = $derived(suggest(words[focusIndex] ?? '', 3).filter((s) => s !== words[focusIndex]));

  function setWord(i: number, v: string) {
    const next = [...words];
    next[i] = v.toLowerCase().replace(/[^a-z]/g, '');
    words = next;
  }

  function onInput(i: number, e: Event) {
    const value = (e.currentTarget as HTMLInputElement).value;
    const parts = splitCode(value);
    if (parts.length > 1) {
      fill(parts, i);
      return;
    }
    setWord(i, value);
    if (value.endsWith(' ') && isWord(value.trim())) advance(i);
  }

  function fill(parts: string[], from = 0) {
    const next = [...words];
    parts.slice(0, 3 - from).forEach((p, k) => (next[from + k] = p));
    words = next;
    const last = Math.min(2, from + parts.length - 1);
    inputs[last]?.focus();
  }

  function complete(i: number): boolean {
    const g = ghost[i];
    if (!g) return false;
    setWord(i, g);
    return true;
  }

  function advance(i: number) {
    if (i < 2) inputs[i + 1]?.focus();
    else if (valid.every(Boolean)) onsubmit();
  }

  function onKey(i: number, e: KeyboardEvent) {
    const el = e.currentTarget as HTMLInputElement;
    if ((e.key === 'Tab' && !e.shiftKey) || e.key === 'Enter' || (e.key === 'ArrowRight' && el.selectionStart === el.value.length)) {
      if (complete(i)) {
        e.preventDefault();
        if (e.key !== 'ArrowRight') advance(i);
        return;
      }
      if (e.key === 'Enter') {
        e.preventDefault();
        advance(i);
      }
    } else if (e.key === 'Backspace' && !el.value && i > 0) {
      e.preventDefault();
      inputs[i - 1]?.focus();
    } else if (e.key === ' ') {
      e.preventDefault();
      complete(i);
      advance(i);
    }
  }

  function onPaste(i: number, e: ClipboardEvent) {
    const parts = splitCode(e.clipboardData?.getData('text') ?? '');
    if (parts.length > 1) {
      e.preventDefault();
      fill(parts, i);
    }
  }

  export function shakeSlot(i: number | null) {
    if (i === null) slots.forEach((s) => s && shake(s));
    else if (slots[i]) shake(slots[i]);
    if (i !== null) inputs[i]?.focus();
  }

  export function focusFirst() {
    inputs[0]?.focus();
  }
</script>

<div class="slots" role="group" aria-label="Code words">
  {#each [0, 1, 2] as i}
    <label class="slot" class:ok={valid[i]} bind:this={slots[i]}>
      <span class="sr-only">Word {i + 1}</span>
      <span class="field">
        <span class="ghost" aria-hidden="true"><span class="typed">{words[i]}</span>{(ghost[i] ?? '').slice((words[i] ?? '').length)}</span>
        <input
          bind:this={inputs[i]}
          value={words[i]}
          {disabled}
          autocomplete="off"
          autocapitalize="none"
          spellcheck="false"
          enterkeyhint={i < 2 ? 'next' : 'go'}
          data-testid="code-word-{i}"
          oninput={(e) => onInput(i, e)}
          onkeydown={(e) => onKey(i, e)}
          onpaste={(e) => onPaste(i, e)}
          onfocus={() => (focusIndex = i)}
        />
      </span>
      {#if valid[i]}
        <span class="tick" in:springIn={{ preset: 'snappy', scale: 0.4, y: 0 }}><Icon name="check-circle" size={18} weight="fill" /></span>
      {/if}
    </label>
  {/each}
</div>

<div class="chips" aria-live="polite">
  {#each chips as c (c)}
    <button class="chip" in:springIn={{ preset: 'snappy', y: 6 }} onclick={() => (setWord(focusIndex, c), advance(focusIndex))}>{c}</button>
  {/each}
</div>

<style>
  .slots {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: var(--space-3);
  }
  .slot {
    position: relative;
    display: flex;
    align-items: center;
    border-radius: var(--radius-sm);
    background: var(--color-surface-sunken);
    box-shadow: inset 0 0 0 1px var(--color-separator);
    transition:
      box-shadow var(--duration-fast) var(--ease-standard),
      background-color var(--duration-fast) var(--ease-standard),
      transform var(--spring-snappy-duration) var(--spring-snappy);
  }
  .slot:focus-within {
    box-shadow: inset 0 0 0 2px var(--color-accent);
  }
  .slot.ok {
    background: var(--color-accent-subtle);
  }
  .field {
    position: relative;
    flex: 1;
    min-width: 0;
  }
  input,
  .ghost {
    width: 100%;
    padding: var(--space-4) var(--space-3);
    font-size: var(--type-headline-size);
    font-weight: 600;
    text-align: left;
  }
  input {
    position: relative;
    border: 0;
    background: transparent;
    outline: none;
    color: var(--color-text-primary);
  }
  .ghost {
    position: absolute;
    inset: 0;
    color: var(--color-text-tertiary);
    white-space: pre;
    overflow: hidden;
    pointer-events: none;
  }
  .typed {
    color: transparent;
  }
  .tick {
    position: absolute;
    right: 6px;
    top: 6px;
    color: var(--color-accent-text);
    display: grid;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    min-height: 36px;
    margin-top: var(--space-3);
  }
  .chip {
    padding: var(--space-2) var(--space-4);
    border-radius: var(--radius-full);
    background: var(--color-fill);
    font-size: var(--type-callout-size);
    font-weight: 500;
    transition: transform var(--spring-press-duration) var(--spring-press);
  }
  .chip:active {
    transform: scale(var(--scale-pressed));
  }
  @media (max-width: 380px) {
    input,
    .ghost {
      font-size: var(--type-body-size);
      padding: var(--space-4) var(--space-2);
    }
  }
</style>
