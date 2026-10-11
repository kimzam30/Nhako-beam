<script lang="ts">
  import { playHaptic } from '../lib/haptics.svelte';

  interface Props {
    checked: boolean;
    label: string;
    hint?: string;
    testid?: string;
    onchange?: (v: boolean) => void;
  }

  let { checked = $bindable(), label, hint, testid, onchange }: Props = $props();
  const id = `t-${Math.random().toString(36).slice(2, 8)}`;

  function flip() {
    checked = !checked;
    playHaptic(checked ? 'toggleOn' : 'toggleOff');
    onchange?.(checked);
  }
</script>

<div class="row">
  <label for={id} class="text">
    <span class="label">{label}</span>
    {#if hint}<span class="hint">{hint}</span>{/if}
  </label>
  <button {id} role="switch" aria-checked={checked} aria-label={label} class="track" class:on={checked} data-testid={testid} onclick={flip}>
    <span class="thumb"></span>
  </button>
</div>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-5);
    min-height: var(--size-control-lg);
  }
  .text {
    flex: 1;
    display: grid;
    gap: var(--space-1);
    cursor: pointer;
  }
  .label {
    font-size: var(--type-body-size);
  }
  .hint {
    font-size: var(--type-footnote-size);
    line-height: var(--type-footnote-line);
    color: var(--color-text-secondary);
  }
  .track {
    position: relative;
    flex: none;
    width: 50px;
    height: 30px;
    border-radius: var(--radius-full);
    background: var(--color-fill-pressed);
    transition: background-color var(--duration-fast) var(--ease-standard);
  }
  .track.on {
    background: var(--color-accent);
  }
  .thumb {
    position: absolute;
    top: 3px;
    left: 3px;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background: #ffffff;
    box-shadow: var(--elevation-1);
    transition:
      transform var(--spring-snappy-duration) var(--spring-snappy),
      width var(--spring-snappy-duration) var(--spring-snappy);
  }
  .on .thumb {
    transform: translateX(20px);
  }
  .track:active .thumb {
    width: 28px;
  }
  .on.track:active .thumb {
    transform: translateX(16px);
  }
</style>
