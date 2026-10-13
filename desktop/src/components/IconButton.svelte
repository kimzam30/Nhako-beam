<script lang="ts">
  import type { SoundName } from '../lib/sounds.svelte';
  import { prefs } from '../lib/state/prefs.svelte';
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

  let glyph: HTMLElement | undefined = $state();

  // Each icon answers a press in its own small way: the gear turns, the bin lid lifts, the x
  // spins a quarter (it looks the same after), the back caret leans back. Others pop.
  const nudges: Record<string, Keyframe[]> = {
    'gear-six': [{ transform: 'rotate(0deg)' }, { transform: 'rotate(70deg)', offset: 0.7 }, { transform: 'rotate(60deg)' }],
    x: [{ transform: 'rotate(0deg)' }, { transform: 'rotate(90deg)' }],
    trash: [{ transform: 'none' }, { transform: 'translateY(-3px) rotate(-10deg)', offset: 0.4 }, { transform: 'none' }],
    'caret-left': [{ transform: 'none' }, { transform: 'translateX(-4px)', offset: 0.4 }, { transform: 'none' }],
    'clock-counter-clockwise': [{ transform: 'rotate(0deg)' }, { transform: 'rotate(-40deg)', offset: 0.45 }, { transform: 'rotate(0deg)' }],
    files: [{ transform: 'none' }, { transform: 'translateY(-3px)', offset: 0.4 }, { transform: 'none' }],
  };
  const pop: Keyframe[] = [{ transform: 'scale(0.7)' }, { transform: 'scale(1.12)', offset: 0.55 }, { transform: 'scale(1)' }];

  function click(e: MouseEvent) {
    if (!prefs.reducedMotion) glyph?.animate(nudges[icon] ?? pop, { duration: 380, easing: 'cubic-bezier(0.2, 0, 0, 1)' });
    onclick?.(e);
  }
</script>

<button class="ib {tone}" aria-label={label} title={label} data-testid={testid} data-sound={sound} onclick={click}>
  <span class="glyph" bind:this={glyph}><Icon name={icon} weight={weight ?? (icon.startsWith('caret-') ? 'bold' : undefined)} {size} /></span>
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
  .glyph {
    display: grid;
    place-items: center;
  }
  .accent {
    color: var(--color-accent-text);
  }
  .danger {
    color: var(--color-danger);
  }
</style>
