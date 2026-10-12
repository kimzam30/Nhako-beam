<script lang="ts">
  // Prototype only: triggers every state in docs/design/screens-and-states.md without real
  // devices, and shows which haptic would fire. Not part of the real app.
  import { app, type PlatformMode } from '../lib/state/app.svelte';
  import { haptics } from '../lib/haptics.svelte';
  import { sampleFiles, type MockEngine, type SampleKind, type SendOutcome } from '../lib/engine/mock';
  import { springIn, quickOut } from '../lib/motion/transitions';
  import Icon from './Icon.svelte';
  import Segmented from './Segmented.svelte';
  import IconButton from './IconButton.svelte';

  let { mock }: { mock: MockEngine } = $props();

  let outcome = $state<SendOutcome>('accept');
  $effect(() => {
    mock.nextSendOutcome = outcome;
  });

  const e = $derived(mock);
  const stage = (k: SampleKind) => app.stage(sampleFiles(k));

  type Action = [label: string, run: () => void, id: string];
  const groups: [string, Action[]][] = $derived([
    [
      'Files to send',
      [
        ['4 photos', () => stage('photos'), 'stage-photos'],
        ['1.4 GB video', () => stage('video'), 'stage-video'],
        ['Photos and video', () => stage('mixed'), 'stage-mixed'],
        ['48 GB backup', () => stage('big'), 'stage-big'],
      ],
    ],
    [
      'Incoming',
      [
        ["From Aisyah's laptop", () => e.simIncoming('mixed', 'd-aisyah'), 'in-aisyah'],
        ['From Studio PC', () => e.simIncoming('photos', 'd-studio'), 'in-studio'],
        ['One video, long name', () => e.simIncoming('longname', 'd-studio'), 'in-longname'],
        ['From Work phone (auto)', () => e.simIncoming('photos', 'd-phone'), 'in-auto'],
        ['48 GB (no space)', () => e.simIncoming('big', 'd-studio'), 'in-big'],
        ['Sender cancels', () => e.simWithdraw(), 'in-withdraw'],
      ],
    ],
    [
      'Active transfer',
      [
        ['Drop connection', () => e.simDropConnection(), 'tr-drop'],
        ['Switch to direct', () => e.simSwitchRoute('direct'), 'tr-direct'],
        ['Switch to relayed', () => e.simSwitchRoute('relayed'), 'tr-relayed'],
        ['Fail', () => e.simFail('network'), 'tr-fail'],
        ['Disk full', () => e.simDiskFull(), 'tr-disk'],
        ['Other side pauses', () => e.simPeerPause(), 'tr-peer-pause'],
        ['Other side resumes', () => e.simPeerResume(), 'tr-peer-resume'],
        ['Other side cancels', () => e.simPeerCancel(), 'tr-peer-cancel'],
        ['Skip to 60 s wait', () => e.simWaitLong(), 'tr-wait-long'],
      ],
    ],
    [
      'Codes',
      [
        ['Other device enters my code', () => e.simPeerRedeemsCode(), 'code-peer'],
        ['Expire my code soon', () => e.simExpireCodeSoon(25), 'code-expire-soon'],
        ['Expire my code now', () => e.simExpireCodeSoon(0), 'code-expire'],
      ],
    ],
    [
      'Discovery',
      [
        ['Search again', () => e.simDiscovery('searching'), 'disc-search'],
        ['Go offline', () => e.simDiscovery('offline'), 'disc-offline'],
        ['Discovery blocked', () => e.simDiscovery('blocked'), 'disc-blocked'],
        ['Add a device', () => e.simAddDevice(), 'disc-add'],
        ['Remove a device', () => e.simRemoveDevice(), 'disc-remove'],
        ['Add 6 devices', () => Array.from({ length: 6 }, () => e.simAddDevice()), 'disc-many'],
      ],
    ],
    [
      'Your own server',
      [
        ['Turns the token down', () => e.simServer('tokenRejected'), 'server-token'],
        ["Doesn't answer", () => e.simServer('unreachable'), 'server-down'],
        ['Fine again', () => e.simServer(null), 'server-ok'],
      ],
    ],
  ]);
</script>

<button class="fab" class:open={app.devPanel} aria-expanded={app.devPanel} aria-label="Prototype scenarios" data-testid="dev-toggle" onclick={() => (app.devPanel = !app.devPanel)}>
  <Icon name="flask" size={20} weight={app.devPanel ? 'fill' : 'duotone'} />
  <span>Scenarios</span>
</button>

{#if app.devPanel}
  <aside class="panel" in:springIn={{ x: 24, y: 0, scale: 0.98 }} out:quickOut={{ x: 16, y: 0 }} data-testid="dev-panel" aria-label="Prototype scenarios">
    <header>
      <div>
        <strong>Prototype scenarios</strong>
        <span class="muted">Not part of the app</span>
      </div>
      <IconButton icon="x" label="Close scenarios" testid="dev-close" onclick={() => (app.devPanel = false)} />
    </header>

    <div class="block">
      <span class="label">Platform</span>
      <Segmented
        label="Platform"
        testid="platform"
        bind:value={app.platformMode}
        options={[
          { value: 'auto' as PlatformMode, label: 'Auto' },
          { value: 'desktop' as PlatformMode, label: 'Desktop' },
          { value: 'android' as PlatformMode, label: 'Android' },
        ]}
      />
    </div>

    <div class="block">
      <span class="label">Next send to an untrusted device</span>
      <Segmented
        label="Next send outcome"
        testid="outcome"
        bind:value={outcome}
        options={[
          { value: 'accept' as SendOutcome, label: 'Accept' },
          { value: 'decline' as SendOutcome, label: 'Decline' },
          { value: 'ignore' as SendOutcome, label: 'Ignore' },
          { value: 'unreachable' as SendOutcome, label: 'Gone' },
        ]}
      />
    </div>

    {#each groups as [title, actions]}
      <div class="block">
        <span class="label">{title}</span>
        <div class="chips">
          {#each actions as [label, run, id]}
            <button class="chip" data-testid="sim-{id}" onclick={run}>{label}</button>
          {/each}
        </div>
      </div>
    {/each}

    <div class="block">
      <span class="label">Haptics (Android)</span>
      <ol class="log" data-testid="haptic-log">
        {#each haptics.entries as h (h.id)}
          <li in:springIn={{ y: -6, preset: 'snappy' }}><Icon name="wave-sine" size={14} weight="bold" />{h.name}</li>
        {:else}
          <li class="muted">None yet</li>
        {/each}
      </ol>
    </div>

    <button class="chip reset" data-testid="sim-reset" onclick={() => app.resetOnboarding()}>Restart onboarding</button>
  </aside>
{/if}

<style>
  .fab {
    position: fixed;
    right: var(--space-4);
    bottom: calc(var(--space-4) + env(safe-area-inset-bottom));
    z-index: var(--z-tooltip);
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-4) var(--space-2) var(--space-3);
    border-radius: var(--radius-full);
    background: var(--color-surface-raised);
    color: var(--color-text-secondary);
    box-shadow: var(--elevation-3);
    font-size: var(--type-footnote-size);
    font-weight: 600;
    transition: transform var(--spring-press-duration) var(--spring-press);
  }
  .fab:active {
    transform: scale(0.95);
  }
  .fab.open {
    color: var(--color-accent-text);
  }
  .panel {
    position: fixed;
    right: var(--space-4);
    bottom: calc(var(--space-4) + 52px + env(safe-area-inset-bottom));
    z-index: var(--z-tooltip);
    width: min(calc(100vw - 32px), 360px);
    max-height: min(72vh, 680px);
    overflow-y: auto;
    padding: var(--space-5);
    border-radius: var(--radius-lg);
    background: var(--color-surface-raised);
    box-shadow: var(--elevation-4);
    display: grid;
    gap: var(--space-5);
    font-size: var(--type-footnote-size);
  }
  header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    margin: calc(var(--space-2) * -1) calc(var(--space-3) * -1) 0 0;
  }
  header > div {
    display: grid;
    padding-top: var(--space-2);
  }
  .block {
    display: grid;
    gap: var(--space-2);
  }
  .label {
    font-weight: 600;
    color: var(--color-text-secondary);
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .chip {
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-sm);
    background: var(--color-fill);
    font-weight: 500;
    transition:
      transform var(--spring-press-duration) var(--spring-press),
      background-color var(--duration-fast) var(--ease-standard);
  }
  .chip:active {
    transform: scale(var(--scale-pressed));
    background: var(--color-fill-pressed);
  }
  .reset {
    justify-self: start;
    color: var(--color-danger);
  }
  .log {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: var(--space-1);
    font-family: var(--font-mono);
  }
  .log li {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }
  .panel :global(.seg button) {
    font-size: var(--type-footnote-size);
    padding: 0 var(--space-2);
  }
  @media (max-width: 599px) {
    .fab {
      right: 0;
      bottom: auto;
      top: 42%;
      padding: var(--space-3) var(--space-2) var(--space-3) var(--space-3);
      border-radius: var(--radius-md) 0 0 var(--radius-md);
    }
    .fab span {
      display: none;
    }
    .panel {
      top: 50%;
      bottom: auto;
      translate: 0 -50%;
      right: 44px;
      width: min(calc(100vw - 56px), 360px);
    }
  }
</style>
