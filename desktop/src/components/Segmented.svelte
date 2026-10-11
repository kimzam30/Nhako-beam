<script lang="ts" generics="T extends string">
  import Icon from './Icon.svelte';

  interface Option {
    value: T;
    label: string;
    icon?: string;
  }

  interface Props {
    options: Option[];
    value: T;
    label: string;
    testid?: string;
    onchange?: (v: T) => void;
  }

  let { options, value = $bindable(), label, testid, onchange }: Props = $props();
  const index = $derived(Math.max(0, options.findIndex((o) => o.value === value)));

  function pick(v: T) {
    value = v;
    onchange?.(v);
  }

  function key(e: KeyboardEvent) {
    const d = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0;
    if (!d) return;
    e.preventDefault();
    const next = options[(index + d + options.length) % options.length];
    if (next) pick(next.value);
  }
</script>

<div class="seg" role="radiogroup" aria-label={label} data-testid={testid} style:--count={options.length} style:--index={index} onkeydown={key} tabindex="-1">
  <span class="pill" aria-hidden="true"></span>
  {#each options as o (o.value)}
    <button role="radio" aria-checked={o.value === value} tabindex={o.value === value ? 0 : -1} class:active={o.value === value} onclick={() => pick(o.value)}>
      {#if o.icon}<Icon name={o.icon} size={16} weight={o.value === value ? 'fill' : 'duotone'} />{/if}
      {o.label}
    </button>
  {/each}
</div>

<style>
  .seg {
    position: relative;
    display: grid;
    grid-template-columns: repeat(var(--count), 1fr);
    padding: 3px;
    border-radius: var(--radius-sm);
    background: var(--color-fill);
  }
  .pill {
    position: absolute;
    top: 3px;
    bottom: 3px;
    left: 3px;
    width: calc((100% - 6px) / var(--count));
    border-radius: calc(var(--radius-sm) - 3px);
    background: var(--color-surface-raised);
    box-shadow: var(--elevation-1);
    transform: translateX(calc(100% * var(--index)));
    transition: transform var(--spring-snappy-duration) var(--spring-snappy);
  }
  button {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    min-height: 34px;
    padding: 0 var(--space-3);
    font-size: var(--type-callout-size);
    font-weight: 500;
    color: var(--color-text-secondary);
    border-radius: calc(var(--radius-sm) - 3px);
    transition: color var(--duration-fast) var(--ease-standard);
  }
  button.active {
    color: var(--color-text-primary);
  }
</style>
