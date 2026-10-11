<script lang="ts">
  import type { Route } from '../lib/engine/types';
  import Icon from './Icon.svelte';
  import { t } from '../lib/copy';

  let { route }: { route: Route } = $props();

  const meta = $derived(
    {
      nearby: { icon: 'wifi-high', label: t('transfer.route.nearby') },
      direct: { icon: 'lightning', label: t('transfer.route.direct') },
      relayed: { icon: 'share-network', label: t('transfer.route.relayed') },
    }[route],
  );
</script>

<span class="badge {route}" title={route === 'relayed' ? t('transfer.route.relayed.info') : undefined} data-testid="route-badge" data-route={route}>
  <Icon name={meta.icon} size={14} weight="bold" />
  {#key route}<span class="label">{meta.label}</span>{/key}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: 2px var(--space-3) 2px var(--space-2);
    border-radius: var(--radius-xs);
    font-size: var(--type-caption-size);
    line-height: var(--type-caption-line);
    font-weight: var(--type-caption-weight);
    background: var(--color-fill);
    color: var(--color-text-secondary);
    transition:
      background-color var(--duration-base) var(--ease-standard),
      color var(--duration-base) var(--ease-standard);
  }
  .nearby,
  .direct {
    background: var(--color-accent-subtle);
    color: var(--color-accent-text);
  }
</style>
