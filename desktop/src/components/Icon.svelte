<script lang="ts">
  // An icon from the Nhako Beam set (style B, design/icons/icons.mjs). Changing `name` crossfades
  // with a small snappy scale, so state changes (pause to play, copy to check) never just blink.
  import { iconMarkup, wantsDeepOffset, type IconWeight } from '../lib/icons/icons';
  import { springIn, quickOut } from '../lib/motion/transitions';

  interface Props {
    name: string;
    /** Accepted for older callers; style B has one drawing per icon. Active is shown by color. */
    weight?: IconWeight;
    size?: number;
    label?: string;
    class?: string;
  }

  let { name, size = 20, label, class: klass = '' }: Props = $props();
  let el: HTMLSpanElement | undefined = $state();
  // Blue or white icons get the deeper navy offset; read from the real color once it's on screen.
  let deep = $state(false);
  $effect(() => {
    void name;
    if (el && size >= 20) deep = wantsDeepOffset(getComputedStyle(el).color);
  });
  const markup = $derived(iconMarkup(name, size, deep));
</script>

<span bind:this={el} class="icon {klass}" style:width="{size}px" style:height="{size}px" role={label ? 'img' : undefined} aria-label={label} aria-hidden={label ? undefined : 'true'}>
  {#key name}
    <svg
      viewBox="0 0 24 24"
      width={size}
      height={size}
      overflow="visible"
      in:springIn={{ scale: 0.6, y: 0, preset: 'snappy' }}
      out:quickOut={{ scale: 0.6, y: 0, duration: 90 }}
    >
      {@html markup}
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
</style>
