<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';

  interface Props {
    variant?: 'primary' | 'secondary' | 'ghost' | 'danger' | 'plain';
    size?: 'sm' | 'md' | 'lg';
    icon?: string;
    iconWeight?: 'duotone' | 'fill' | 'bold';
    loading?: boolean;
    disabled?: boolean;
    full?: boolean;
    type?: 'button' | 'submit';
    label?: string;
    testid?: string;
    onclick?: (e: MouseEvent) => void;
    children?: Snippet;
  }

  let {
    variant = 'primary',
    size = 'md',
    icon,
    iconWeight,
    loading = false,
    disabled = false,
    full = false,
    type = 'button',
    label,
    testid,
    onclick,
    children,
  }: Props = $props();
</script>

<button
  {type}
  class="btn {variant} {size}"
  class:full
  class:loading
  disabled={disabled || loading}
  aria-busy={loading || undefined}
  aria-label={label}
  data-testid={testid}
  {onclick}
>
  <span class="inner">
    {#if icon}<Icon name={icon} weight={iconWeight} size={size === 'sm' ? 16 : 20} />{/if}
    {#if children}<span class="text">{@render children()}</span>{/if}
  </span>
  {#if loading}<span class="spinner" aria-hidden="true"></span>{/if}
</button>

<style>
  .btn {
    position: relative;
    display: inline-grid;
    place-items: center;
    border-radius: var(--radius-full);
    font-weight: 600;
    white-space: nowrap;
    user-select: none;
    touch-action: manipulation;
    transition:
      transform var(--spring-press-duration) var(--spring-press),
      background-color var(--duration-instant) var(--ease-standard),
      opacity var(--duration-fast) var(--ease-standard),
      box-shadow var(--duration-fast) var(--ease-standard);
  }
  .btn:active:not(:disabled) {
    transform: scale(var(--scale-pressed));
  }
  .btn:disabled {
    cursor: default;
    opacity: var(--opacity-disabled);
  }
  .btn.loading {
    opacity: 1;
  }
  .inner {
    grid-area: 1 / 1;
    display: inline-flex;
    align-items: center;
    gap: var(--space-3);
    transition: opacity var(--duration-fast) var(--ease-standard);
  }
  .loading .inner {
    opacity: 0;
  }
  .spinner {
    grid-area: 1 / 1;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 2px solid currentColor;
    border-right-color: transparent;
    animation: spin 0.8s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(1turn);
    }
  }

  .sm {
    min-height: var(--size-control-sm);
    padding: 0 var(--space-4);
    font-size: var(--type-callout-size);
  }
  .md {
    min-height: var(--size-control-md);
    padding: 0 var(--space-6);
    font-size: var(--type-body-size);
  }
  .lg {
    min-height: var(--size-control-lg);
    padding: 0 var(--space-7);
    font-size: var(--type-headline-size);
  }
  .full {
    width: 100%;
  }

  .primary {
    background: var(--color-accent);
    color: var(--color-on-accent);
    box-shadow: var(--elevation-1);
  }
  .primary:active:not(:disabled) {
    background: var(--color-accent-pressed);
  }
  .secondary {
    background: var(--color-fill);
    color: var(--color-text-primary);
  }
  .secondary:active:not(:disabled) {
    background: var(--color-fill-pressed);
  }
  .ghost {
    color: var(--color-accent-text);
  }
  .ghost:active:not(:disabled) {
    background: var(--color-accent-subtle);
  }
  .danger {
    background: var(--color-danger);
    color: var(--color-on-danger);
  }
  .plain {
    color: var(--color-text-secondary);
  }
  .plain:active:not(:disabled) {
    background: var(--color-fill);
  }

  @media (hover: hover) {
    .secondary:hover:not(:disabled) {
      background: var(--color-fill-pressed);
    }
    .ghost:hover:not(:disabled),
    .plain:hover:not(:disabled) {
      background: var(--color-fill);
    }
    .primary:hover:not(:disabled) {
      box-shadow: var(--elevation-2);
    }
  }
</style>
