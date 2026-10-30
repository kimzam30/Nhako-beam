<script lang="ts">
  import { app, prettyPath } from '../lib/state/app.svelte';
  import { fileMenu as fileMenuSetting, keepRunning as keepRunningSetting, startAtLogin as startAtLoginSetting, type FileMenu } from '../lib/shell';
  import { prefs, type MotionPref, type SoundPref, type ThemePref } from '../lib/state/prefs.svelte';
  import { t } from '../lib/copy';
  import { springIn, quickOut, springFlip, stagger } from '../lib/motion/transitions';
  import DeviceOrb from '../components/DeviceOrb.svelte';
  import Button from '../components/Button.svelte';
  import Icon from '../components/Icon.svelte';
  import IconButton from '../components/IconButton.svelte';
  import Segmented from '../components/Segmented.svelte';
  import Toggle from '../components/Toggle.svelte';
  import ConnectionSettings from '../components/ConnectionSettings.svelte';
  import type { WhoCanSend } from '../lib/engine/types';

  const nearbyUntrusted = $derived(app.devices.filter((d) => !d.trusted && d.reachable && d.route === 'nearby'));
  const trusted = $derived(app.devices.filter((d) => d.trusted));

  // Desktop app only: start at login and staying in the tray.
  let startAtLogin = $state(false);
  let keepRunning = $state(true);
  let fileMenu = $state<FileMenu>({ manager: null, on: false });
  $effect(() => {
    if (!app.native) return;
    startAtLoginSetting.get().then((v) => (startAtLogin = v), () => {});
    keepRunningSetting.get().then((v) => (keepRunning = v), () => {});
    fileMenuSetting.get().then((v) => (fileMenu = v), () => {});
  });

  async function change(setting: { set(on: boolean): Promise<unknown> }, on: boolean, undo: () => void) {
    try {
      await setting.set(on);
    } catch {
      undo();
      app.toast(t('settings.desktop.failed'), 'danger');
    }
  }

  let diagnosticsCopied = $state(false);
  async function copyDiagnostics() {
    if (!(await app.copyDiagnostics())) return;
    diagnosticsCopied = true;
    setTimeout(() => (diagnosticsCopied = false), 1600);
  }

  function back() {
    app.navigate('home');
  }
</script>

<svelte:window onkeydown={(e) => e.key === 'Escape' && !app.sheet && back()} />

<section class="settings" data-testid="settings">
  <header class="bar">
    <IconButton icon="caret-left" label={t('settings.back')} size={24} testid="settings-back" onclick={back} />
    <h1>{t('settings.title')}</h1>
  </header>

  <div class="sections">
    <section class="group" in:springIn={{ delay: stagger(0) }}>
      <h2>{t('settings.section.device')}</h2>
      <div class="card">
        <label class="field">
          <span>{t('settings.device.name')}</span>
          <span class="charge name"><input bind:value={app.selfName} maxlength="32" onchange={() => (app.persist(), app.engine.renameSelf(app.selfName))} data-testid="settings-name" /></span>
        </label>
        <div class="sep"></div>
        <div class="field">
          <span>{t('settings.device.save')}</span>
          <button class="value" title={app.saveLocation} onclick={() => app.changeSaveLocation()} data-testid="settings-save">
            <Icon name="folder-open" size={18} />
            <span class="path">{prettyPath(app.saveLocation)}</span>
            <Icon name="caret-right" size={16} />
          </button>
        </div>
      </div>
    </section>

    <section class="group" in:springIn={{ delay: stagger(1) }}>
      <h2>{t('settings.section.receiving')}</h2>
      {#if app.isAndroid}
        <div class="card always">
          <Toggle bind:checked={app.alwaysReady} label={t('settings.receive.always')} hint={t('settings.receive.always.hint')} testid="always-ready" />
        </div>
      {/if}
      <div class="card stack">
        <div class="field col">
          <span>{t('settings.receive.who')}</span>
          <Segmented
            label={t('settings.receive.who')}
            testid="who-can-send"
            value={app.whoCanSend}
            onchange={(who) => void app.setWhoCanSend(who)}
            options={[
              { value: 'yourDevices' as WhoCanSend, label: t('settings.receive.who.yours'), icon: 'shield-check' },
              { value: 'everyoneNearby' as WhoCanSend, label: t('settings.receive.who.everyone'), icon: 'broadcast' },
            ]}
          />
          {#key app.whoCanSend}
            <span class="hint" data-testid="who-can-send-hint" in:springIn={{ preset: 'snappy', y: -4, scale: 1 }}>
              {t(app.whoCanSend === 'yourDevices' ? 'settings.receive.who.yours.hint' : 'settings.receive.who.everyone.hint')}
            </span>
          {/key}
        </div>
      </div>
    </section>

    <section class="group" in:springIn={{ delay: stagger(2) }}>
      <h2>{t('settings.trusted.title')}</h2>
      <div class="card">
        {#each trusted as d, i (d.id)}
          <div class="device" animate:springFlip out:quickOut={{ x: -24, y: 0 }} data-testid="trusted-{d.id}">
            {#if i > 0}<div class="sep"></div>{/if}
            <div class="device-row">
              <DeviceOrb device={d} size={40} showName={false} />
              <div class="device-text">
                <strong>{d.name}</strong>
                <span class="muted tabular">{d.fingerprint}</span>
              </div>
              <IconButton icon="trash" label="{t('settings.trusted.remove')} {d.name}" tone="danger" testid="remove-{d.id}" onclick={() => (app.sheet = { kind: 'confirmRemove', deviceId: d.id })} />
            </div>
            <Toggle checked={d.autoAccept} label={t('settings.trusted.autoAccept')} testid="auto-{d.id}" onchange={(v) => app.engine.setAutoAccept(d.id, v)} />
          </div>
        {:else}
          <p class="muted empty">{t('settings.trusted.empty')}</p>
        {/each}
      </div>
      {#if nearbyUntrusted.length}
        <!-- Devices here right now that aren't yours yet: trusting one asks the other to agree. -->
        <h3 class="sub-h" in:springIn={{ y: 6 }}>{t('settings.trusted.nearby')}</h3>
        <div class="card">
          {#each nearbyUntrusted as d, i (d.id)}
            <div class="device" animate:springFlip in:springIn={{ delay: stagger(i), y: 8 }} out:quickOut={{ x: -24, y: 0 }} data-testid="nearby-{d.id}">
              {#if i > 0}<div class="sep"></div>{/if}
              <div class="device-row">
                <DeviceOrb device={d} size={40} showName={false} />
                <div class="device-text">
                  <strong>{d.name}</strong>
                  <span class="muted tabular">{d.fingerprint}</span>
                </div>
                <Button variant="secondary" size="sm" icon="handshake" testid="settings-trust-{d.id}" onclick={() => app.askTrust(d.id)}>{t('send.choose.trust')}</Button>
              </div>
            </div>
          {/each}
        </div>
      {/if}
    </section>

    <section class="group" in:springIn={{ delay: stagger(3) }}>
      <h2>{t('settings.section.connection')}</h2>
      <div class="card lan">
        <Toggle
          checked={app.lanDiscovery}
          label={t('settings.network.lan')}
          hint={t('settings.network.lan.hint')}
          testid="lan-discovery"
          onchange={async (on) => {
            if (!(await app.setLanDiscovery(on))) app.lanDiscovery = !on;
          }}
        />
      </div>
      <div class="card stack">
        <ConnectionSettings />
      </div>
    </section>

    {#if app.native}
      <section class="group" in:springIn={{ delay: stagger(4) }} data-testid="settings-desktop">
        <h2>{t('settings.section.desktop')}</h2>
        <div class="card">
          {#if !fileMenu.sandboxed}
            <Toggle
              bind:checked={startAtLogin}
              label={t('settings.desktop.startAtLogin')}
              hint={t('settings.desktop.startAtLogin.hint')}
              testid="start-at-login"
              onchange={(on) => change(startAtLoginSetting, on, () => (startAtLogin = !on))}
            />
            <div class="sep"></div>
          {/if}
          <Toggle
            bind:checked={keepRunning}
            label={t('settings.desktop.keepRunning')}
            hint={t('settings.desktop.keepRunning.hint')}
            testid="keep-running"
            onchange={(on) => change(keepRunningSetting, on, () => (keepRunning = !on))}
          />
          {#if fileMenu.manager}
            <div class="sep"></div>
            <Toggle
              bind:checked={fileMenu.on}
              label={t(fileMenu.manager === 'explorer' ? 'settings.desktop.fileMenu.explorer' : 'settings.desktop.fileMenu.files')}
              hint={t(fileMenu.manager === 'explorer' ? 'settings.desktop.fileMenu.explorer.hint' : 'settings.desktop.fileMenu.files.hint')}
              testid="file-menu"
              onchange={(on) => change(fileMenuSetting, on, () => (fileMenu.on = !on))}
            />
          {/if}
        </div>
      </section>
    {/if}

    <section class="group" in:springIn={{ delay: stagger(5) }}>
      <h2>{t('settings.section.appearance')}</h2>
      <div class="card stack">
        <div class="field col">
          <span>{t('settings.appearance.theme')}</span>
          <Segmented
            label={t('settings.appearance.theme')}
            testid="theme"
            bind:value={prefs.theme}
            onchange={() => prefs.save()}
            options={[
              { value: 'system' as ThemePref, label: t('settings.appearance.theme.system'), icon: 'circle-half' },
              { value: 'light' as ThemePref, label: t('settings.appearance.theme.light'), icon: 'sun' },
              { value: 'dark' as ThemePref, label: t('settings.appearance.theme.dark'), icon: 'moon' },
            ]}
          />
        </div>
        <div class="field col">
          <span>{t('settings.appearance.motion')}</span>
          <Segmented
            label={t('settings.appearance.motion')}
            testid="motion"
            bind:value={prefs.motion}
            onchange={() => prefs.save()}
            options={[
              { value: 'system' as MotionPref, label: t('settings.appearance.motion.system') },
              { value: 'full' as MotionPref, label: t('settings.appearance.motion.full') },
              { value: 'reduced' as MotionPref, label: t('settings.appearance.motion.reduced') },
            ]}
          />
          <span class="hint">{t('settings.appearance.motion.hint')}</span>
        </div>
        <div class="field col">
          <span>{t('settings.appearance.sounds')}</span>
          <Segmented
            label={t('settings.appearance.sounds')}
            testid="sounds"
            bind:value={prefs.sounds}
            onchange={() => prefs.save()}
            options={[
              { value: 'loud' as SoundPref, label: t('settings.appearance.sounds.loud'), icon: 'speaker-high' },
              { value: 'soft' as SoundPref, label: t('settings.appearance.sounds.soft'), icon: 'speaker-low' },
              { value: 'off' as SoundPref, label: t('settings.appearance.sounds.off'), icon: 'speaker-slash' },
            ]}
          />
          <span class="hint">{t('settings.appearance.sounds.hint')}</span>
        </div>
      </div>
    </section>

    <section class="group" in:springIn={{ delay: stagger(6) }}>
      <h2>{t('settings.about.title')}</h2>
      <div class="card">
        <div class="field"><span>{t('settings.about.version', { version: app.version || '0.1.0' })}</span></div>
        {#if app.update}
          <div class="sep"></div>
          <button class="field link update" onclick={() => app.openUpdate()} data-testid="settings-update" in:springIn={{ y: 6 }}>
            <span>{t('update.available', { version: app.update.version })}</span>
            <span class="update-go">{t('update.download')}<Icon name="arrow-square-out" size={16} weight="bold" /></span>
          </button>
        {/if}
        {#if app.updateCheck !== null}
          <div class="sep"></div>
          <Toggle
            checked={app.updateCheck}
            label={t('settings.about.updates')}
            hint={t('settings.about.updates.hint')}
            testid="update-check"
            onchange={async (on) => {
              if (!(await app.setUpdateCheck(on))) app.updateCheck = !on;
            }}
          />
        {/if}
        {#if app.fingerprint}
          <div class="sep"></div>
          <div class="field"><span>{t('settings.about.fingerprint')}</span><span class="muted tabular" data-testid="fingerprint">{app.fingerprint}</span></div>
        {/if}
        <div class="sep"></div>
        <button class="field link" onclick={() => app.navigate('licenses')} data-testid="open-licenses">
          <span>{t('settings.about.licenses')}</span>
          <Icon name="caret-right" size={16} />
        </button>
        <div class="sep"></div>
        <button class="field link" onclick={copyDiagnostics} data-testid="copy-diagnostics">
          <span class="two-line">
            <span>{t('settings.about.diagnostics')}</span>
            <span class="hint">{t('settings.about.diagnostics.hint')}</span>
          </span>
          <Icon name={diagnosticsCopied ? 'check' : 'copy'} size={18} weight="duotone" />
        </button>
      </div>
    </section>
  </div>
</section>

<style>
  .settings {
    max-width: 640px;
    margin: 0 auto;
    padding: var(--space-5) var(--space-5) calc(var(--space-9) + env(safe-area-inset-bottom));
  }
  .bar {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0 0 var(--space-6) calc(var(--space-3) * -1);
  }
  h1 {
    font-size: var(--type-title1-size);
    line-height: var(--type-title1-line);
    font-weight: var(--type-title1-weight);
    letter-spacing: var(--type-title1-tracking);
  }
  .sections {
    display: grid;
    /* One column as wide as the screen: a long row wraps inside it instead of widening the page. */
    grid-template-columns: minmax(0, 1fr);
    gap: var(--space-7);
  }
  .sub-h {
    margin: var(--space-5) 0 var(--space-3) var(--space-2);
    font-size: var(--type-footnote-size);
    font-weight: 600;
    color: var(--color-text-secondary);
  }
  .group h2 {
    margin: 0 0 var(--space-3) var(--space-2);
    font-size: var(--type-footnote-size);
    font-weight: 600;
    color: var(--color-text-secondary);
  }
  .card {
    padding: var(--space-2) var(--space-5);
    border-radius: var(--radius-lg);
    background: var(--color-surface);
    box-shadow: var(--elevation-1);
  }
  .card.lan,
  .card.always {
    margin-bottom: var(--space-4);
  }
  .update {
    color: var(--color-accent-text);
    font-weight: 600;
  }
  .update-go {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
  }
  .field.link {
    width: 100%;
    color: inherit;
    text-align: left;
  }
  .field.link :global(svg) {
    color: var(--color-text-tertiary);
  }
  .card.stack {
    display: grid;
    gap: var(--space-5);
    padding: var(--space-5);
  }
  .sep {
    height: 1px;
    background: var(--color-separator);
  }
  .field {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    min-height: var(--size-control-lg);
  }
  .field.col {
    display: grid;
    gap: var(--space-3);
    min-height: 0;
  }
  .field .name {
    flex: 1;
    min-width: 0;
    max-width: 260px;
  }
  .field input {
    flex: 1;
    min-width: 0;
    max-width: 260px;
    padding: var(--space-2) var(--space-3);
    border: 0;
    border-radius: var(--radius-sm);
    background: var(--color-surface-sunken);
    text-align: right;
  }
  .field input:focus {
    outline: none;
    box-shadow: inset 0 0 0 2px var(--color-accent);
  }
  .value {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    min-width: 0;
    color: var(--color-text-secondary);
  }
  .path {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .device {
    display: grid;
  }
  .device-row {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding-top: var(--space-3);
  }
  .device-row :global(.orb) {
    width: auto;
  }
  .device-text {
    flex: 1;
    display: grid;
    min-width: 0;
  }
  .device-text span {
    font-size: var(--type-footnote-size);
  }
  .empty {
    padding: var(--space-4) 0;
    font-size: var(--type-callout-size);
  }
  .hint {
    font-size: var(--type-footnote-size);
    color: var(--color-text-secondary);
  }
  .two-line {
    display: grid;
    gap: var(--space-1);
    padding-block: var(--space-3);
  }
</style>
