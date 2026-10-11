<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from './Icon.svelte';
  import { springIn, quickOut } from '../lib/motion/transitions';

  interface Props {
    tone?: 'warning' | 'danger' | 'success' | 'info';
    icon?: string;
    text: string;
    action?: Snippet;
    testid?: string;
  }

  let { tone = 'warning', icon = 'warning-circle', text, action, testid }: Props = $props();
</script>

<div class="banner {tone}" role="status" data-testid={testid} in:springIn={{ y: -12, scale: 1 }} out:quickOut={{ y: -8 }}>
  <Icon name={icon} size={20} />
  <span class="text">{text}</span>
  {#if action}<span class="action">{@render action()}</span>{/if}
</div>

<style>
  .banner {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-md);
    font-size: var(--type-callout-size);
    line-height: var(--type-callout-line);
  }
  .text {
    flex: 1;
  }
  .warning {
    background: var(--color-warning-subtle);
    color: var(--color-warning);
  }
  .danger {
    background: var(--color-danger-subtle);
    color: var(--color-danger);
  }
  .success {
    background: var(--color-success-subtle);
    color: var(--color-success);
  }
  .info {
    background: var(--color-accent-subtle);
    color: var(--color-accent-text);
  }
  .action :global(button) {
    color: inherit;
    font-weight: 600;
  }
</style>
