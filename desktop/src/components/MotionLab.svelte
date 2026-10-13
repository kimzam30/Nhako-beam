<script lang="ts">
  // Prototype only: plays every piece of the bolt motion on one page, for review, with a switch
  // for reduced motion. Opened from Scenarios. Spec: docs/design/motion-spec.md, 4.15.
  import { app } from '../lib/state/app.svelte';
  import { prefs, type MotionPref } from '../lib/state/prefs.svelte';
  import { arcPath, spark, strike } from '../lib/motion/bolt';
  import { shake } from '../lib/motion/springs';
  import { springIn, quickOut } from '../lib/motion/transitions';
  import { brand } from '../lib/tokens';
  import Beam from './Beam.svelte';
  import Bolt from './Bolt.svelte';
  import Button from './Button.svelte';
  import DeviceOrb from './DeviceOrb.svelte';
  import IconButton from './IconButton.svelte';
  import RollingNumber from './RollingNumber.svelte';
  import Segmented from './Segmented.svelte';
  import Toggle from './Toggle.svelte';

  let { onclose }: { onclose: () => void } = $props();

  // Bumping a counter remounts a demo, so it plays its entrance again.
  let replay = $state({ done: 0, cracked: 0, asleep: 0, arc: 0 });
  let twitch = $state(0);
  let on = $state(true);
  let sound = $state<'loud' | 'soft' | 'off'>('soft');
  let progress = $state(0.42);
  let moving = $state(true);
  let count = $state(37);
  let shakeBox: HTMLElement | undefined = $state();
  let sparkBox: HTMLElement | undefined = $state();

  const device = $derived(app.devices[0]);

  // The beam and ring move on their own while "moving" is on, as a transfer would.
  $effect(() => {
    if (!moving) return;
    const id = setInterval(() => (progress = progress >= 0.98 ? 0.05 : progress + 0.04), 400);
    return () => clearInterval(id);
  });

  function arc(node: SVGGElement) {
    node.querySelectorAll('path').forEach((p) => void strike(p));
  }
</script>

<div class="lab" role="dialog" aria-label="Motion lab" data-testid="motion-lab" in:springIn={{ y: 16 }} out:quickOut>
  <header>
    <div>
      <h1>Motion lab</h1>
      <p class="muted">Every bit of the bolt motion. Prototype only.</p>
    </div>
    <IconButton icon="x" label="Close the motion lab" testid="lab-close" onclick={onclose} />
  </header>

  <div class="motion-pref">
    <Segmented
      label="Motion"
      testid="lab-motion"
      bind:value={prefs.motion}
      onchange={() => prefs.save()}
      options={[
        { value: 'system' as MotionPref, label: 'System' },
        { value: 'full' as MotionPref, label: 'Full' },
        { value: 'reduced' as MotionPref, label: 'Reduced' },
      ]}
    />
  </div>

  <div class="grid">
    <section>
      <h2>Spark</h2>
      <div class="stage" bind:this={sparkBox}><Bolt size={40} /></div>
      <Button variant="secondary" size="sm" testid="lab-spark" onclick={() => sparkBox && spark(sparkBox)}>Throw sparks</Button>
    </section>

    <section>
      <h2>Buttons</h2>
      <div class="stage col">
        <Button testid="lab-primary">Primary: rim and sparks</Button>
        <Button variant="secondary">Secondary: rim</Button>
      </div>
    </section>

    <section>
      <h2>Icon buttons</h2>
      <div class="stage">
        {#each ['gear-six', 'trash', 'x', 'caret-left', 'clock-counter-clockwise', 'files', 'pause'] as icon (icon)}
          <IconButton {icon} label={icon} />
        {/each}
      </div>
    </section>

    <section>
      <h2>Switch and choice</h2>
      <div class="stage col">
        <Toggle bind:checked={on} label="A switch with a bolt" testid="lab-toggle" />
        <Segmented
          label="Sounds"
          bind:value={sound}
          options={[
            { value: 'loud', label: 'Loud', icon: 'speaker-high' },
            { value: 'soft', label: 'Soft', icon: 'speaker-low' },
            { value: 'off', label: 'Off', icon: 'speaker-slash' },
          ]}
        />
      </div>
    </section>

    <section>
      <h2>Text field</h2>
      <div class="stage col">
        <label class="charge field"><input placeholder="Click to see the charge line" /></label>
        <div bind:this={shakeBox} class="shake-me">A wrong word shakes</div>
      </div>
      <Button variant="secondary" size="sm" onclick={() => shakeBox && shake(shakeBox)}>Shake</Button>
    </section>

    <section>
      <h2>Done, failed, empty</h2>
      <div class="stage">
        {#key replay.done}<span class="done"><Bolt size={40} mode="check" strike testid="lab-done" /></span>{/key}
        {#key replay.cracked}<span class="failed"><Bolt size={40} mode="cracked" tone="current" /></span>{/key}
        {#key replay.asleep}<Bolt size={40} mode="asleep" />{/key}
        <Bolt size={40} {twitch} />
      </div>
      <div class="row">
        <Button variant="secondary" size="sm" testid="lab-replay-done" onclick={() => replay.done++}>Done</Button>
        <Button variant="secondary" size="sm" onclick={() => replay.cracked++}>Failed</Button>
        <Button variant="secondary" size="sm" onclick={() => replay.asleep++}>Asleep</Button>
        <Button variant="secondary" size="sm" onclick={() => twitch++}>Twitch</Button>
      </div>
    </section>

    <section>
      <h2>Arc to a found device</h2>
      <div class="stage">
        <svg class="arc" viewBox="0 0 220 80" aria-hidden="true">
          <circle cx="24" cy="40" r="14" class="dot self" />
          <circle cx="196" cy="40" r="14" class="dot" />
          {#key replay.arc}
            <g use:arc>
              <path d={arcPath({ x: 44, y: 40 }, { x: 176, y: 40 }, replay.arc, 7)} transform="translate(1.5 2)" stroke={brand.navy} />
              <path d={arcPath({ x: 44, y: 40 }, { x: 176, y: 40 }, replay.arc, 7)} stroke={brand.yellow} />
            </g>
          {/key}
        </svg>
      </div>
      <Button variant="secondary" size="sm" onclick={() => replay.arc++}>Strike</Button>
    </section>

    <section>
      <h2>Beam and ring</h2>
      <div class="stage col">
        <Beam progress={progress} intensity={0.7} mode={moving ? 'moving' : 'paused'} />
        {#if device}<DeviceOrb {device} size={56} showName={false} progress={progress} />{/if}
      </div>
      <Toggle bind:checked={moving} label="Bytes moving" />
    </section>

    <section>
      <h2>Numbers</h2>
      <div class="stage"><span class="big tabular"><RollingNumber value={count} suffix="%" /></span></div>
      <div class="row">
        <Button variant="secondary" size="sm" testid="lab-count-up" onclick={() => (count = Math.min(100, count + 7))}>Up</Button>
        <Button variant="secondary" size="sm" onclick={() => (count = Math.max(0, count - 7))}>Down</Button>
      </div>
    </section>

    <section>
      <h2>Toast</h2>
      <div class="stage"><span class="muted">A thin bar drains as its time runs out.</span></div>
      <Button variant="secondary" size="sm" onclick={() => app.toast('Saved', 'success')}>Show a toast</Button>
    </section>
  </div>
</div>

<style>
  .lab {
    position: fixed;
    inset: 0;
    z-index: calc(var(--z-tooltip) - 1);
    overflow-y: auto;
    padding: var(--space-6) var(--space-5) var(--space-9);
    background: var(--color-canvas);
  }
  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: var(--space-4);
    max-width: 1080px;
    margin: 0 auto var(--space-5);
  }
  h1 {
    font-size: var(--type-title1-size);
    line-height: var(--type-title1-line);
    font-weight: var(--type-title1-weight);
  }
  .motion-pref {
    max-width: 1080px;
    margin: 0 auto var(--space-6);
  }
  .motion-pref :global(.seg) {
    max-width: 360px;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, 300px), 1fr));
    gap: var(--space-5);
    max-width: 1080px;
    margin: 0 auto;
  }
  section {
    display: grid;
    align-content: start;
    gap: var(--space-4);
    min-width: 0;
    padding: var(--space-5);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow: var(--elevation-1);
  }
  h2 {
    font-size: var(--type-headline-size);
    font-weight: 600;
  }
  .stage {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: center;
    gap: var(--space-5);
    min-height: 96px;
  }
  .stage.col {
    flex-direction: column;
    align-items: stretch;
  }
  .row {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .done {
    color: var(--color-success);
  }
  .failed {
    color: var(--color-danger);
  }
  .field input {
    width: 100%;
    padding: var(--space-3) var(--space-4);
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--color-surface-sunken);
  }
  .field input:focus {
    outline: none;
  }
  .shake-me {
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-sm);
    background: var(--color-danger-subtle);
    color: var(--color-danger);
    text-align: center;
  }
  .arc {
    width: 100%;
    max-width: 260px;
    overflow: visible;
  }
  .arc path {
    fill: none;
    stroke-width: 2.5;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .dot {
    fill: var(--color-surface-raised);
    stroke: var(--color-separator);
  }
  .dot.self {
    fill: var(--color-accent);
  }
  .big {
    font-size: var(--type-title1-size);
    font-weight: 700;
  }
</style>
