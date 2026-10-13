<script lang="ts">
  // Settings > Connection: n0's public servers, or your own (the Docker setup in relay/).
  import { app } from '../lib/state/app.svelte';
  import { t, type CopyKey } from '../lib/copy';
  import type { CheckResult, ServerCheck } from '../lib/engine/types';
  import { hostOf, readOwnForm, type OwnForm } from '../lib/engine/server';
  import { message } from '../lib/engine/tauri';
  import { expand, fadeT, quickOut, springIn, stagger } from '../lib/motion/transitions';
  import { playHaptic } from '../lib/haptics.svelte';
  import Button from './Button.svelte';
  import Icon from './Icon.svelte';
  import IconButton from './IconButton.svelte';
  import Segmented from './Segmented.svelte';

  type Mode = 'public' | 'own';

  const saved = $derived(app.connection);
  let mode = $state<Mode>(app.connection.mode);
  let form = $state<OwnForm>(formFrom());
  let touched = $state({ relay: false, token: false, lookup: false });
  let showToken = $state(false);
  let checking = $state(false);
  /** Saving restarts the engine on the new servers, which takes a moment. */
  let saving = $state(false);
  let check = $state<ServerCheck | null>(null);
  /** Bumped per check so the result rows animate in again. */
  let checkRun = $state(0);
  /** The last check couldn't reach everything; waiting for "Save anyway". */
  let askAnyway = $state(false);

  const read = $derived(readOwnForm(form));
  const problems = $derived(read.problems);
  const isSavedOwn = $derived(
    saved.mode === 'own' &&
      read.connection !== null &&
      saved.relay === read.connection.relay &&
      (saved.token ?? '') === (read.connection.token ?? '') &&
      (saved.lookup ?? '') === (read.connection.lookup ?? ''),
  );

  function formFrom(): OwnForm {
    const c = app.connection;
    return c.mode === 'own' ? { relay: c.relay, token: c.token ?? '', lookup: c.lookup ?? '' } : { relay: '', token: '', lookup: '' };
  }

  async function pickMode(m: Mode) {
    check = null;
    askAnyway = false;
    if (m === 'public' && saved.mode === 'own') {
      saving = true;
      // Couldn't switch (for example, a transfer is running): show the setting still in use.
      if (!(await app.setConnection({ mode: 'public' }))) mode = 'own';
      saving = false;
    }
    if (m === 'own') {
      form = formFrom();
      touched = { relay: false, token: false, lookup: false };
    }
  }

  function edited() {
    check = null;
    askAnyway = false;
  }

  async function runCheck(): Promise<ServerCheck | null> {
    touched = { relay: true, token: true, lookup: true };
    if (!read.connection) return null;
    checking = true;
    askAnyway = false;
    try {
      check = await app.engine.checkServer(read.connection).catch((e) => ({ relay: { status: 'unreachable' as const, reason: message(e) }, lookup: null }));
      checkRun++;
      playHaptic(allOk(check) ? 'accept' : 'transferFailed');
      return check;
    } finally {
      checking = false;
    }
  }

  async function save() {
    const c = await runCheck();
    if (!c) return;
    if (c.relay.status === 'tokenRejected' || c.relay.status === 'invalid' || c.lookup?.status === 'invalid') return;
    if (allOk(c)) commit();
    else askAnyway = true;
  }

  async function commit() {
    if (!read.connection || saving) return;
    saving = true;
    if (await app.setConnection(read.connection)) askAnyway = false;
    saving = false;
  }

  function allOk(c: ServerCheck): boolean {
    return c.relay.status === 'ok' && (c.lookup === null || c.lookup.status === 'ok');
  }

  function problemText(p: string | undefined): string {
    return p ? t(`server.problem.${p}` as CopyKey) : '';
  }

  function resultText(r: CheckResult, isRelay: boolean): string {
    switch (r.status) {
      case 'ok':
        return t('settings.connection.result.ok', { ms: r.ms });
      case 'tokenRejected':
        return t(isRelay && !form.token.trim() ? 'settings.connection.result.tokenNeeded' : 'settings.connection.result.tokenRejected');
      case 'unreachable':
        return t('settings.connection.result.unreachable');
      case 'invalid':
        return t('settings.connection.result.invalid');
    }
  }

  /** The engine's own words for unreachable; our copy for address problems. */
  function why(r: CheckResult): string {
    if (r.status === 'invalid') return problemText(r.reason.startsWith('token: ') ? r.reason.slice(7) : r.reason);
    if (r.status === 'unreachable') return r.reason.charAt(0).toUpperCase() + r.reason.slice(1);
    return '';
  }

  const tone = (r: CheckResult) => (r.status === 'ok' ? 'ok' : r.status === 'unreachable' ? 'warn' : 'bad');
  const icon = (r: CheckResult) => (r.status === 'ok' ? 'check-circle' : r.status === 'unreachable' ? 'warning-circle' : 'x-circle');
  const tokenRejected = $derived(check?.relay.status === 'tokenRejected');
</script>

<div class="conn" data-testid="connection">
  <div class="field col">
    <span>{t('settings.connection.servers')}</span>
    <Segmented
      label={t('settings.connection.servers')}
      testid="conn-mode"
      bind:value={mode}
      onchange={pickMode}
      options={[
        { value: 'public' as Mode, label: t('settings.connection.public'), icon: 'globe-hemisphere-west' },
        { value: 'own' as Mode, label: t('settings.connection.own'), icon: 'hard-drives' },
      ]}
    />
    {#key mode}
      <p class="hint" in:fadeT>{t(mode === 'public' ? 'settings.connection.public.hint' : 'settings.connection.own.hint')}</p>
    {/key}
  </div>

  {#if mode === 'own'}
    <form class="own" transition:expand onsubmit={(e) => (e.preventDefault(), save())} novalidate>
      {#if isSavedOwn && !check}
        <p class="using" in:springIn={{ y: 4 }} out:quickOut data-testid="conn-using">
          <span class="dot" aria-hidden="true"></span>
          {t('settings.connection.using', { host: hostOf(form.relay) })}
        </p>
      {/if}

      <label class="input-field" in:springIn={{ delay: stagger(0) }}>
        <span>{t('settings.connection.relay')}</span>
        <span class="charge">
          <input
            bind:value={form.relay}
            oninput={edited}
            onblur={() => (touched.relay = true)}
            placeholder={t('settings.connection.relay.placeholder')}
            inputmode="url"
            autocapitalize="off"
            autocomplete="off"
            spellcheck="false"
            aria-invalid={touched.relay && !!problems.relay}
            data-testid="conn-relay"
          />
        </span>
        {#if touched.relay && problems.relay}
          <small class="problem" in:springIn={{ y: -4 }} out:quickOut={{ y: -4 }} data-testid="conn-error-relay">{problemText(problems.relay)}</small>
        {/if}
      </label>

      <label class="input-field" in:springIn={{ delay: stagger(1) }}>
        <span>{t('settings.connection.token')}</span>
        <span class="with-button charge">
          <input
            bind:value={form.token}
            oninput={edited}
            onblur={() => (touched.token = true)}
            type={showToken ? 'text' : 'password'}
            placeholder={t('settings.connection.token.placeholder')}
            autocapitalize="off"
            autocomplete="off"
            spellcheck="false"
            aria-invalid={(touched.token && !!problems.token) || tokenRejected}
            data-testid="conn-token"
          />
          <IconButton
            icon={showToken ? 'eye-slash' : 'eye'}
            label={t(showToken ? 'settings.connection.token.hide' : 'settings.connection.token.show')}
            size={18}
            testid="conn-token-eye"
            onclick={(e) => (e.preventDefault(), (showToken = !showToken))}
          />
        </span>
        {#if touched.token && problems.token}
          <small class="problem" in:springIn={{ y: -4 }} out:quickOut={{ y: -4 }} data-testid="conn-error-token">{problemText(problems.token)}</small>
        {/if}
      </label>

      <label class="input-field" in:springIn={{ delay: stagger(2) }}>
        <span>{t('settings.connection.lookup')}</span>
        <span class="charge">
          <input
            bind:value={form.lookup}
            oninput={edited}
            onblur={() => (touched.lookup = true)}
            placeholder={t('settings.connection.lookup.placeholder')}
            inputmode="url"
            autocapitalize="off"
            autocomplete="off"
            spellcheck="false"
            aria-invalid={touched.lookup && !!problems.lookup}
            data-testid="conn-lookup"
          />
        </span>
        {#if touched.lookup && problems.lookup}
          <small class="problem" in:springIn={{ y: -4 }} out:quickOut={{ y: -4 }} data-testid="conn-error-lookup">{problemText(problems.lookup)}</small>
        {:else}
          <small class="hint">{t('settings.connection.lookup.hint')}</small>
        {/if}
      </label>

      <div class="results" aria-live="polite">
        {#if check}
          {#key checkRun}
            <ul transition:expand data-testid="conn-results">
              {#each [{ key: 'relay', label: t('settings.connection.result.relay'), r: check.relay }, ...(check.lookup ? [{ key: 'lookup', label: t('settings.connection.result.lookup'), r: check.lookup }] : [])] as row, i (row.key)}
                <li class={tone(row.r)} data-testid="conn-result-{row.key}" data-status={row.r.status}>
                  <span class="badge" in:springIn={{ scale: 0.4, y: 0, preset: 'bouncy', delay: stagger(i, 90) }}>
                    <Icon name={icon(row.r)} weight="fill" size={20} />
                  </span>
                  <span class="label">{row.label}</span>
                  <span class="what" in:springIn={{ x: -6, y: 0, delay: stagger(i, 90) + 40 }}>{resultText(row.r, row.key === 'relay')}</span>
                  {#if row.r.status === 'unreachable' || row.r.status === 'invalid'}
                    <small class="why">{why(row.r)}</small>
                  {/if}
                </li>
              {/each}
            </ul>
          {/key}
        {/if}
      </div>

      {#if tokenRejected}
        <p class="note bad" transition:expand data-testid="conn-not-saved">{t('settings.connection.notSaved.token')}</p>
      {/if}
      {#if askAnyway}
        <div class="note warn" transition:expand data-testid="conn-ask-anyway">
          <p>{t('settings.connection.notReached')}</p>
          <Button variant="secondary" size="sm" loading={saving} testid="conn-save-anyway" onclick={commit}>{t('settings.connection.saveAnyway')}</Button>
        </div>
      {/if}

      <div class="actions">
        <Button variant="secondary" icon="arrows-clockwise" loading={checking && !askAnyway} testid="conn-check" onclick={(e) => (e.preventDefault(), runCheck())}>
          {t('settings.connection.check')}
        </Button>
        <Button type="submit" icon="check" loading={saving} disabled={isSavedOwn || checking || saving} testid="conn-save">{t('settings.connection.save')}</Button>
      </div>
    </form>
  {/if}
</div>

<style>
  .conn {
    display: grid;
    gap: var(--space-5);
  }
  .field.col {
    display: grid;
    gap: var(--space-3);
  }
  .hint {
    margin: 0;
    font-size: var(--type-footnote-size);
    color: var(--color-text-secondary);
  }
  .own {
    display: grid;
    gap: var(--space-5);
    padding-top: var(--space-5);
    border-top: 1px solid var(--color-separator);
  }
  .using {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin: 0;
    font-size: var(--type-callout-size);
    font-weight: 600;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: var(--radius-full);
    background: var(--color-success);
    box-shadow: 0 0 0 0 var(--color-success-subtle);
    animation: breathe 2.4s ease-in-out infinite;
  }
  @keyframes breathe {
    50% {
      box-shadow: 0 0 0 6px var(--color-success-subtle);
    }
  }
  .input-field {
    display: grid;
    gap: var(--space-2);
  }
  .input-field > span:first-child {
    font-size: var(--type-footnote-size);
    font-weight: 600;
    color: var(--color-text-secondary);
  }
  .with-button {
    position: relative;
    display: flex;
    align-items: center;
  }
  .with-button input {
    padding-right: calc(var(--size-hit-min-touch) + var(--space-2));
  }
  .with-button :global(.ib) {
    position: absolute;
    right: 0;
  }
  input {
    width: 100%;
    min-height: var(--size-control-lg);
    padding: var(--space-3) var(--space-4);
    border: 0;
    border-radius: var(--radius-md);
    background: var(--color-surface-sunken);
    font-family: var(--font-mono, ui-monospace, monospace);
    font-size: var(--type-callout-size);
    transition: box-shadow var(--spring-snappy-duration, 200ms) var(--spring-snappy, ease-out);
  }
  input::placeholder {
    font-family: var(--font-sans, inherit);
    color: var(--color-text-tertiary);
  }
  input:focus {
    outline: none;
    box-shadow: inset 0 0 0 2px var(--color-accent);
  }
  input[aria-invalid='true'] {
    box-shadow: inset 0 0 0 2px var(--color-danger);
  }
  .problem {
    font-size: var(--type-footnote-size);
    color: var(--color-danger);
  }
  small.hint {
    font-size: var(--type-footnote-size);
  }
  .results ul {
    display: grid;
    gap: var(--space-3);
    margin: 0;
    padding: var(--space-4);
    list-style: none;
    border-radius: var(--radius-md);
    background: var(--color-surface-sunken);
  }
  .results li {
    display: grid;
    grid-template-columns: auto auto 1fr;
    align-items: center;
    column-gap: var(--space-3);
  }
  .badge {
    display: inline-grid;
  }
  .ok .badge {
    color: var(--color-success);
  }
  .warn .badge {
    color: var(--color-warning);
  }
  .bad .badge {
    color: var(--color-danger);
  }
  .label {
    font-weight: 600;
  }
  .what {
    color: var(--color-text-secondary);
  }
  .why {
    grid-column: 3;
    font-size: var(--type-footnote-size);
    color: var(--color-text-tertiary);
  }
  .note {
    display: grid;
    gap: var(--space-3);
    justify-items: start;
    margin: 0;
    padding: var(--space-4);
    border-radius: var(--radius-md);
    font-size: var(--type-callout-size);
  }
  .note p {
    margin: 0;
  }
  .note.bad {
    background: var(--color-danger-subtle);
  }
  .note.warn {
    background: var(--color-warning-subtle);
  }
  .actions {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: var(--space-3);
  }
</style>
