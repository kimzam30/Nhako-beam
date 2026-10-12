<script lang="ts">
  import type { SoundName } from '../lib/sounds.svelte';
  import Icon from './Icon.svelte';

  interface Props {
    icon: string;
    label: string;
    weight?: 'duotone' | 'fill' | 'bold';
    size?: number;
    tone?: 'neutral' | 'accent' | 'danger';
    testid?: string;
    /** Its own sound instead of the tap (lib/sounds.svelte.ts). */
    sound?: SoundName;
    onclick?: (e: MouseEvent) => void;
  }

  let { icon, label, weight, size = 20, tone = 'neutral', testid, sound, onclick }: Props = $props();
</script>

<button class="ib {tone}" aria-label={label} title={label} data-testid={testid} data-sound={sound} {onclick}>
  <Icon name={icon} weight={weight ?? (icon.startsWith('caret-') ? 'bold' : undefined)} {size} />
</button>

<style>
  .ib {
    display: inline-grid;
    place-items: center;
    min-width: var(--size-hit-min-touch);
    min-height: var(--size-hit-min-touch);
    border-radius: var(--radius-full);
    color: var(--color-text-secondary);
    transition:
      transform var(--spring-press-duration) var(--spring-press),
      background-color var(--duration-fast) var(--ease-standard),
      color var(--duration-fast) var(--ease-standard);
  }
  @media (pointer: fine) {
    .ib {
      min-width: 36px;
      min-height: 36px;
    }
  }
  .ib:active {
    transform: scale(0.92);
    background: var(--color-fill);
  }
  @media (hover: hover) {
    .ib:hover {
      background: var(--color-fill);
      color: var(--color-text-primary);
    }
  }
  .accent {
    color: var(--color-accent-text);
  }
  .danger {
    color: var(--color-danger);
  }
</style>
