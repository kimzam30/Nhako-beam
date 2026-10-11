<script lang="ts">
  // Phosphor icon. Changing `name` or `weight` crossfades with a small snappy scale,
  // so state changes (pause to play, copy to check) never just blink.
  import { iconMarkup, type IconWeight } from '../lib/icons/icons';
  import { springIn, quickOut } from '../lib/motion/transitions';

  interface Props {
    name: string;
    weight?: IconWeight;
    size?: number;
    label?: string;
    class?: string;
  }

  let { name, weight, size = 20, label, class: klass = '' }: Props = $props();
  const resolved = $derived<IconWeight>(weight ?? (size <= 16 ? 'bold' : 'duotone'));
  const key = $derived(`${name}:${resolved}`);
</script>

<span class="icon {klass}" style:width="{size}px" style:height="{size}px" role={label ? 'img' : undefined} aria-label={label} aria-hidden={label ? undefined : 'true'}>
  {#key key}
    <svg
      viewBox="0 0 256 256"
      fill="currentColor"
      width={size}
      height={size}
      in:springIn={{ scale: 0.6, y: 0, preset: 'snappy' }}
      out:quickOut={{ scale: 0.6, y: 0, duration: 90 }}
    >
      {@html iconMarkup(name, resolved)}
    </svg>
  {/key}
</span>

<style>
  .icon {
    position: relative;
    display: inline-grid;
    flex: none;
    place-items: center;
  }
  .icon > :global(svg) {
    grid-area: 1 / 1;
    display: block;
  }
  /* The duotone layer uses the same color at 20% (set in the SVG), so it follows currentColor. */
</style>
