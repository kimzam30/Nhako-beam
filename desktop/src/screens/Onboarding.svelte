<script lang="ts">
  import { app, prettyPath } from '../lib/state/app.svelte';
  import { t } from '../lib/copy';
  import { springEasing } from '../lib/motion/physics';
  import { prefs } from '../lib/state/prefs.svelte';
  import { springIn, stagger } from '../lib/motion/transitions';
  import type { TransitionConfig } from 'svelte/transition';
  import Button from '../components/Button.svelte';
  import DeviceOrb from '../components/DeviceOrb.svelte';
  import Icon from '../components/Icon.svelte';
  import Toggle from '../components/Toggle.svelte';
  import { fileMenu, startAtLogin } from '../lib/shell';

  type Step = 'welcome' | 'name' | 'nearby' | 'notify' | 'save';
  const steps = $derived<Step[]>(app.isAndroid ? ['welcome', 'name', 'nearby', 'notify', 'save'] : ['welcome', 'name', 'save']);
  let index = $state(0);
  let dir = $state(1);
  const step = $derived(steps[Math.min(index, steps.length - 1)]!);

  if (!app.selfName) app.selfName = app.isAndroid ? 'My phone' : 'My laptop';

  // Desktop: start at login, on unless they turn it off here (ready to receive, like the phone's
  // always ready). Not offered in a Flatpak, which can't reach outside its sandbox.
  let loginOn = $state(true);
  let canStartAtLogin = $state(!app.isAndroid);
  $effect(() => {
    if (app.native) fileMenu.get().then((m) => (canStartAtLogin = !m.sandboxed), () => {});
  });

  function go(d: 1 | -1) {
    dir = d;
    if (index + d >= steps.length) {
      if (app.native && canStartAtLogin && loginOn) startAtLogin.set(true).catch((e) => console.warn('Could not turn on start at login:', e));
      app.finishOnboarding();
      return;
    }
    index = Math.max(0, index + d);
  }

  // Steps slide 24px and fade on the gentle spring; the old one leaves 8px the other way.
  function slide(_n: Element, { out }: { out: boolean }): TransitionConfig {
    if (prefs.reducedMotion) return { duration: 150, css: (t) => `opacity:${t}` };
    if (out) return { duration: 150, css: (t, u) => `opacity:${t};transform:translateX(${-dir * 8 * u}px)` };
    const { duration, easing } = springEasing('gentle');
    return { duration, css: (t) => `opacity:${Math.min(1, t * 2.5)};transform:translateX(${dir * 24 * (1 - easing(t))}px)` };
  }
</script>

<section class="onboarding" data-testid="onboarding" data-step={step}>
  <div class="dots" aria-hidden="true">
    {#each steps as s, i (s)}
      <span class:on={i === index} class:past={i < index}></span>
    {/each}
  </div>

  <div class="stage">
    {#key step}
      <div class="step" in:slide={{ out: false }} out:slide={{ out: true }}>
        {#if step === 'welcome'}
          <div class="hero" aria-hidden="true">
            <span class="hero-orb a" in:springIn={{ preset: 'bouncy', scale: 0.5, y: 0, delay: 80 }}><Icon name="laptop" size={34} /></span>
            <span class="hero-beam" in:springIn={{ preset: 'smooth', scale: 0.2, y: 0, delay: 220 }}></span>
            <span class="hero-orb b" in:springIn={{ preset: 'bouncy', scale: 0.5, y: 0, delay: 160 }}><Icon name="device-mobile" size={34} /></span>
          </div>
          <h1 class="title" in:springIn={{ delay: stagger(2) }}>{t('onboarding.welcome.title')}</h1>
          <p class="body" in:springIn={{ delay: stagger(3) }}>{t('onboarding.welcome.body')}</p>
          <ul class="points">
            {#each [['lock-simple', t('onboarding.welcome.point.encrypted')], ['lightning', t('onboarding.welcome.point.fast')], ['share-network', t('onboarding.welcome.point.anywhere')]] as [icon, text], i}
              <li in:springIn={{ delay: stagger(4 + i) }}><Icon name={icon!} size={20} />{text}</li>
            {/each}
          </ul>
        {:else if step === 'name'}
          <div class="preview">
            <DeviceOrb self selfName={app.selfName} size={88} />
          </div>
          <h1 class="title">{t('onboarding.name.title')}</h1>
          <p class="body">{t('onboarding.name.body')}</p>
          <span class="charge name-box">
            <input
              class="input"
              bind:value={app.selfName}
              placeholder={t('onboarding.name.placeholder')}
              maxlength="32"
              data-testid="name-input"
              onkeydown={(e) => e.key === 'Enter' && app.selfName.trim() && go(1)}
            />
          </span>
        {:else if step === 'nearby' || step === 'notify'}
          <span class="big-icon" in:springIn={{ preset: 'bouncy', scale: 0.5, y: 0 }}><Icon name={step === 'nearby' ? 'broadcast' : 'bell'} size={44} /></span>
          <h1 class="title">{t(step === 'nearby' ? 'onboarding.perm.nearby.title' : 'onboarding.perm.notify.title')}</h1>
          <p class="body">{t(step === 'nearby' ? 'onboarding.perm.nearby.body' : 'onboarding.perm.notify.body')}</p>
        {:else if step === 'save'}
          <span class="big-icon" in:springIn={{ preset: 'bouncy', scale: 0.5, y: 0 }}><Icon name="folder-open" size={44} /></span>
          <h1 class="title">{t('onboarding.save.title')}</h1>
          <p class="body">{t('onboarding.save.body')}</p>
          <div class="location">
            <Icon name="folder-open" size={20} />
            <span class="tabular path" title={app.saveLocation}>{prettyPath(app.saveLocation)}</span>
            <Button variant="ghost" size="sm" testid="onboarding-save-change" onclick={() => app.changeSaveLocation()}>{t('onboarding.save.change')}</Button>
          </div>
          {#if canStartAtLogin}
            <div class="login" in:springIn={{ delay: stagger(1) }}>
              <Toggle bind:checked={loginOn} label={t('settings.desktop.startAtLogin')} hint={t('settings.desktop.startAtLogin.hint')} testid="onboarding-login" />
            </div>
          {/if}
        {/if}
      </div>
    {/key}
  </div>

  <div class="actions">
    {#if step === 'nearby' || step === 'notify'}
      <Button variant="plain" size="lg" onclick={() => go(1)}>{t('onboarding.perm.skip')}</Button>
      <Button size="lg" testid="onboarding-next" onclick={() => go(1)}>{t('onboarding.perm.allow')}</Button>
    {:else}
      {#if index > 0}<Button variant="plain" size="lg" icon="caret-left" iconWeight="bold" label="Back" onclick={() => go(-1)} />{/if}
      <Button size="lg" full={app.layout === 'compact'} testid="onboarding-next" disabled={step === 'name' && !app.selfName.trim()} onclick={() => go(1)}>
        {step === 'welcome' ? t('onboarding.welcome.cta') : step === 'save' ? t('onboarding.done.cta') : t('onboarding.name.cta')}
      </Button>
    {/if}
  </div>
</section>

<style>
  .onboarding {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
    min-height: 100dvh;
    max-width: var(--size-content-max-width);
    margin: 0 auto;
    padding: var(--space-8) var(--space-6) calc(var(--space-7) + env(safe-area-inset-bottom));
  }
  .dots {
    display: flex;
    justify-content: center;
    gap: var(--space-2);
  }
  .dots span {
    width: 6px;
    height: 6px;
    border-radius: var(--radius-full);
    background: var(--color-fill-pressed);
    transition:
      width var(--spring-snappy-duration) var(--spring-snappy),
      background-color var(--duration-fast) var(--ease-standard);
  }
  .dots .on {
    width: 22px;
    background: var(--color-accent);
  }
  .dots .past {
    background: var(--color-accent-subtle);
  }
  .stage {
    flex: 1;
    display: grid;
    align-items: center;
  }
  .step {
    grid-area: 1 / 1;
    display: grid;
    justify-items: center;
    text-align: center;
    gap: var(--space-4);
    padding: var(--space-8) 0;
  }
  .title {
    font-size: var(--type-large-title-size);
    line-height: var(--type-large-title-line);
    font-weight: var(--type-large-title-weight);
    letter-spacing: var(--type-large-title-tracking);
    max-width: 20ch;
    text-wrap: balance;
  }
  .body {
    color: var(--color-text-secondary);
    max-width: 36ch;
    text-wrap: pretty;
  }
  .hero {
    display: flex;
    align-items: center;
    margin-bottom: var(--space-6);
  }
  .hero-orb {
    display: grid;
    place-items: center;
    width: 76px;
    height: 76px;
    border-radius: 50%;
    background: var(--color-surface-raised);
    color: var(--color-accent-text);
    box-shadow: var(--elevation-3);
  }
  .hero-orb.b {
    background: var(--color-accent);
    color: var(--color-on-accent);
  }
  .hero-beam {
    width: 72px;
    height: 6px;
    margin: 0 -4px;
    border-radius: var(--radius-full);
    background: var(--color-beam-core);
    box-shadow: 0 0 14px 3px var(--color-beam-glow);
  }
  .points {
    list-style: none;
    padding: 0;
    margin: var(--space-5) 0 0;
    display: grid;
    gap: var(--space-3);
    text-align: left;
  }
  .points li {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    color: var(--color-text-secondary);
  }
  .points :global(.icon) {
    color: var(--color-accent-text);
  }
  .preview {
    margin-bottom: var(--space-4);
  }
  .preview :global(.orb) {
    width: 200px;
  }
  .name-box {
    width: 100%;
    max-width: 340px;
    margin-top: var(--space-4);
  }
  .input {
    width: 100%;
    padding: var(--space-4) var(--space-5);
    border: 0;
    border-radius: var(--radius-md);
    background: var(--color-surface-sunken);
    box-shadow: inset 0 0 0 1px var(--color-separator);
    font-size: var(--type-headline-size);
    text-align: center;
    transition: box-shadow var(--duration-fast) var(--ease-standard);
  }
  .input:focus {
    outline: none;
    box-shadow: inset 0 0 0 2px var(--color-accent);
  }
  .big-icon {
    display: grid;
    place-items: center;
    width: 96px;
    height: 96px;
    margin-bottom: var(--space-4);
    border-radius: var(--radius-xl);
    background: var(--color-accent-subtle);
    color: var(--color-accent-text);
  }
  .location {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin-top: var(--space-4);
    padding: var(--space-3) var(--space-3) var(--space-3) var(--space-5);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    box-shadow: var(--elevation-1);
    color: var(--color-accent-text);
  }
  .location span {
    color: var(--color-text-primary);
    font-weight: 500;
  }
  .location .path {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .login {
    width: 100%;
    max-width: 420px;
    margin-top: var(--space-3);
    padding: 0 var(--space-5);
    border-radius: var(--radius-md);
    background: var(--color-surface);
    box-shadow: var(--elevation-1);
    text-align: start;
  }
  .actions {
    display: flex;
    justify-content: center;
    gap: var(--space-3);
  }
</style>
