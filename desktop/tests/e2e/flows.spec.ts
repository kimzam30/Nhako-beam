import { expect, test } from '@playwright/test';
import { center, closePanel, drag, haptics, ready, sim, soundsPlayed } from './helpers';

test.describe('onboarding', () => {
  test('walks through to the radar', async ({ page }, info) => {
    await page.goto('/');
    await expect(page.getByTestId('onboarding')).toHaveAttribute('data-step', 'welcome');
    await page.getByTestId('onboarding-next').click();
    await expect(page.getByTestId('onboarding')).toHaveAttribute('data-step', 'name');
    await page.getByTestId('name-input').fill('Hakim test laptop');
    // Android asks for permissions one at a time; desktop does not.
    const steps = info.project.name === 'phone' ? 4 : 2;
    for (let i = 0; i < steps; i++) await page.getByTestId('onboarding-next').click();
    await expect(page.getByTestId('home')).toBeVisible();
    await expect(page.getByTestId('home').getByText('Hakim test laptop')).toBeVisible();
    await expect(page.getByTestId('orb-d-aisyah')).toBeVisible();
    await expect(page.getByTestId('orb-d-tablet')).toBeVisible();
  });
});

test.describe('sending', () => {
  test('tap a trusted device: transfers and finishes with a check', async ({ page }) => {
    await ready(page, { speed: 30 });
    await sim(page, 'stage-mixed');
    await closePanel(page);
    await expect(page.getByTestId('staged-tray')).toContainText('3 photos and 1 video');
    await page.getByTestId('orb-d-aisyah').click();
    const card = page.locator('[data-testid^="transfer-"]').first();
    await expect(card).toHaveAttribute('data-status', /connecting|transferring/);
    await expect(card).toHaveAttribute('data-status', 'done', { timeout: 15_000 });
    await expect(card).toContainText("Sent to Aisyah's laptop");
    expect(await haptics(page)).toContain('transferDone');
  });

  test('untrusted device declines: says so and offers retry', async ({ page }) => {
    await ready(page);
    await sim(page, 'stage-photos');
    await page.getByTestId('outcome').getByRole('radio', { name: 'Decline' }).click();
    await closePanel(page);
    await page.getByTestId('orb-d-studio').click();
    const card = page.locator('[data-testid^="transfer-"]').first();
    await expect(card).toHaveAttribute('data-status', 'waiting');
    await expect(card).toContainText('Waiting for Studio PC to accept');
    await expect(page.getByTestId('orb-d-studio')).toHaveAttribute('data-state', 'waiting');
    await expect(card).toHaveAttribute('data-status', 'declined', { timeout: 5000 });
    await expect(card).toContainText('Studio PC declined');
    await expect(card.getByTestId('retry')).toBeVisible();
    expect(await haptics(page)).toContain('decline');
  });

  test('drag the staged stack: magnet snaps onto a device, drop sends', async ({ page }) => {
    await ready(page, { speed: 30 });
    await sim(page, 'stage-photos');
    await closePanel(page);
    const from = await center(page, 'staged-stack');
    const orb = await center(page, 'orb-d-tablet');
    // Stop a little short of the orb: the magnet should still lock on.
    const near = { x: orb.x - 20, y: orb.y - 30 };
    await drag(page, from, near, { steps: 20, stepMs: 12, release: false });
    await expect(page.getByTestId('drag-ghost')).toBeVisible();
    await expect(page.getByTestId('orb-d-tablet')).toHaveAttribute('data-state', 'snapped');
    await expect(page.getByTestId('drop-hint')).toHaveText("Send to Mei Ling's tablet");
    await page.mouse.up();
    await expect(page.getByTestId('drag-ghost')).toBeHidden();
    await expect(page.locator('[data-testid^="transfer-"]').first()).toContainText("Mei Ling's tablet");
    const h = await haptics(page);
    expect(h).toContain('dragStart');
    expect(h).toContain('magnetSnap');
  });

  test('drag the stack away and let go: it springs back, nothing is sent', async ({ page }) => {
    await ready(page);
    await sim(page, 'stage-photos');
    await closePanel(page);
    const from = await center(page, 'staged-stack');
    await drag(page, from, { x: from.x + 10, y: from.y + 120 }, { steps: 10, stepMs: 10 });
    await expect(page.getByTestId('drag-ghost')).toBeHidden();
    await expect(page.getByTestId('staged-tray')).toBeVisible();
    await expect(page.locator('[data-testid^="transfer-"]')).toHaveCount(0);
  });
});

test.describe('desktop file drop', () => {
  test.skip(({ isMobile }) => isMobile, 'desktop only');

  async function osDrag(page: import('@playwright/test').Page, to: { x: number; y: number }, drop: boolean) {
    await page.evaluate(
      ({ to, drop }) => {
        const dt = new DataTransfer();
        dt.items.add(new File(['x'.repeat(2048)], 'holiday.jpg', { type: 'image/jpeg' }));
        dt.items.add(new File(['y'.repeat(4096)], 'notes.pdf', { type: 'application/pdf' }));
        const fire = (type: string, x: number, y: number) =>
          document.elementFromPoint(x, y)!.dispatchEvent(new DragEvent(type, { bubbles: true, cancelable: true, clientX: x, clientY: y, dataTransfer: dt }));
        fire('dragenter', 40, 40);
        for (let i = 1; i <= 10; i++) fire('dragover', 40 + ((to.x - 40) * i) / 10, 40 + ((to.y - 40) * i) / 10);
        if (drop) fire('drop', to.x, to.y);
      },
      { to, drop },
    );
  }

  test('drop on a device sends straight away', async ({ page }) => {
    await ready(page);
    const orb = await center(page, 'orb-d-aisyah');
    await osDrag(page, orb, false);
    await expect(page.getByTestId('orb-d-aisyah')).toHaveAttribute('data-state', 'snapped');
    await expect(page.getByTestId('drop-hint')).toHaveText("Send to Aisyah's laptop");
    await osDrag(page, orb, true);
    await expect(page.locator('[data-testid^="transfer-"]').first()).toContainText("Aisyah's laptop");
  });

  test('drop on empty space opens Send to', async ({ page }) => {
    await ready(page);
    await osDrag(page, { x: 200, y: 700 }, true);
    await expect(page.getByTestId('sheet-send')).toBeVisible();
    await expect(page.getByTestId('sheet-send')).toContainText('1 photo and 1 document');
    await page.getByTestId('send-row-d-studio').click();
    await expect(page.getByTestId('sheet-send')).toBeHidden();
    await expect(page.locator('[data-testid^="transfer-"]').first()).toContainText('Studio PC');
  });
});

test.describe('incoming', () => {
  test('swipe right past the threshold accepts', async ({ page }) => {
    await ready(page, { speed: 30 });
    await sim(page, 'in-studio');
    await closePanel(page);
    const card = await center(page, 'incoming-card');
    await drag(page, { x: card.x, y: card.y + 20 }, { x: card.x + card.w * 0.6, y: card.y + 30 }, { steps: 18, stepMs: 16 });
    await expect(page.getByTestId('incoming-card')).toBeHidden();
    const tr = page.locator('[data-testid^="transfer-"]').first();
    await expect(tr).toContainText('Studio PC');
    const h = await haptics(page);
    expect(h).toContain('thresholdActivate');
    expect(h).toContain('accept');
  });

  test('swipe left declines, a short slow swipe springs back', async ({ page }) => {
    await ready(page);
    await sim(page, 'in-aisyah');
    await closePanel(page);
    const card = await center(page, 'incoming-card');
    await drag(page, { x: card.x, y: card.y + 20 }, { x: card.x + 40, y: card.y + 20 }, { steps: 10, stepMs: 30 });
    await page.waitForTimeout(600);
    await expect(page.getByTestId('incoming-card')).toBeVisible();
    await drag(page, { x: card.x, y: card.y + 20 }, { x: card.x - card.w * 0.6, y: card.y + 20 }, { steps: 18, stepMs: 16 });
    await expect(page.getByTestId('incoming-card')).toBeHidden();
    await expect(page.locator('[data-testid^="transfer-"]')).toHaveCount(0);
    expect(await haptics(page)).toContain('decline');
  });

  test('keyboard: Enter accepts, Escape declines', async ({ page }, info) => {
    test.skip(info.project.name !== 'desktop', 'desktop only');
    await ready(page);
    await sim(page, 'in-studio');
    await closePanel(page);
    await page.getByTestId('incoming-card').focus();
    await page.keyboard.press('Escape');
    await expect(page.getByTestId('incoming-card')).toBeHidden();
    await sim(page, 'in-studio');
    await closePanel(page);
    await page.getByTestId('incoming-card').focus();
    await page.keyboard.press('Enter');
    await expect(page.locator('[data-testid^="transfer-"]').first()).toContainText('Studio PC');
  });

  test('not enough space disables Accept and says how much is needed', async ({ page }) => {
    await ready(page);
    await sim(page, 'in-big');
    await closePanel(page);
    await expect(page.getByTestId('incoming-card')).toContainText('Not enough space');
    await expect(page.getByTestId('accept')).toBeDisabled();
  });

  test('trusted auto-accept device skips the card', async ({ page }) => {
    await ready(page);
    await sim(page, 'in-auto');
    await closePanel(page);
    await expect(page.getByTestId('incoming-card')).toBeHidden();
    await expect(page.getByTestId('toast').first()).toContainText('Receiving from Work phone');
  });
});

test.describe('codes', () => {
  test('show my code: QR, three words, countdown, expiry', async ({ page }) => {
    await ready(page);
    await page.getByTestId('send-code').click();
    const sheet = page.getByTestId('sheet-show-code');
    await expect(sheet.getByRole('img', { name: 'QR code for pairing' })).toBeVisible();
    await expect(page.getByTestId('code-words').locator('.word')).toHaveCount(3);
    await expect(page.getByTestId('code-timer')).toContainText('Expires in 10 min');
    await sim(page, 'code-expire-soon');
    await expect(page.getByTestId('code-timer')).toContainText(/Expires in 2\ds/);
    await sim(page, 'code-expire');
    await expect(sheet).toContainText('This code expired');
    await closePanel(page);
    await page.getByTestId('new-code').click();
    await expect(page.getByTestId('code-timer')).toContainText('Expires in 10 min');
  });

  test('other device enters my code: pairs and offers trust', async ({ page }) => {
    await ready(page);
    await page.getByTestId('send-code').click();
    await expect(page.getByTestId('sheet-show-code')).toBeVisible();
    await sim(page, 'code-peer');
    await closePanel(page);
    await expect(page.getByTestId('sheet-paired')).toBeVisible();
    await expect(page.getByTestId('pair-check')).toHaveText(/5D19 E04A/);
    await page.getByTestId('paired-continue').click();
    await expect(page.getByTestId('sheet-paired')).toBeHidden();
    expect(await haptics(page)).toContain('pairMatched');
  });

  test('enter a code: wrong word, then the right code pairs', async ({ page }) => {
    await ready(page);
    await page.evaluate(() => ((window as any).__nhako.app.sheet = { kind: 'enterCode' }));
    await page.getByTestId('code-word-0').fill('river');
    await page.getByTestId('code-word-1').fill('lanternz');
    await page.getByTestId('code-word-2').fill('seven');
    await page.getByTestId('connect').click();
    await expect(page.getByTestId('sheet-enter-code')).toContainText("isn't in the list");
    await page.getByTestId('code-word-1').fill('lantern');
    await page.getByTestId('connect').click();
    await expect(page.getByTestId('sheet-paired')).toBeVisible();
  });

  test('autocomplete: Tab finishes the word', async ({ page }, info) => {
    test.skip(info.project.name !== 'desktop', 'keyboard flow');
    await ready(page);
    await page.getByTestId('receive-code').click();
    await page.getByTestId('code-word-0').fill('riv');
    await page.getByTestId('code-word-0').press('Tab');
    await expect(page.getByTestId('code-word-0')).toHaveValue('river');
    await expect(page.getByTestId('code-word-1')).toBeFocused();
  });

  test('scan a code on Android', async ({ page }, info) => {
    test.skip(info.project.name !== 'phone', 'android only');
    await ready(page);
    await page.getByTestId('receive-code').click();
    await expect(page.getByTestId('qr-scanner')).toBeVisible();
    await expect(page.getByTestId('qr-scanner')).toHaveAttribute('data-found', 'true', { timeout: 4000 });
    await expect(page.getByTestId('sheet-paired')).toBeVisible({ timeout: 6000 });
  });
});

test.describe('transfer states', () => {
  test('pause, resume, reconnect, route change, fail and retry', async ({ page }) => {
    await ready(page);
    await sim(page, 'stage-video');
    await closePanel(page);
    await page.getByTestId('orb-d-aisyah').click();
    const card = page.locator('[data-testid^="transfer-"]').first();
    await expect(card).toHaveAttribute('data-status', 'transferring');
    await expect(card.getByTestId('beam')).toHaveAttribute('data-mode', 'moving');

    await card.getByTestId('pause').click();
    await expect(card).toHaveAttribute('data-status', 'paused');
    await expect(card.getByTestId('beam')).toHaveAttribute('data-mode', 'paused');
    await card.getByTestId('resume').click();
    await expect(card).toHaveAttribute('data-status', 'transferring');

    await sim(page, 'tr-drop');
    await expect(card).toHaveAttribute('data-status', 'reconnecting');
    await expect(card).toContainText('Reconnecting. Your progress is safe.');
    await expect(card).toHaveAttribute('data-status', 'transferring', { timeout: 6000 });
    await expect(page.getByTestId('toast').filter({ hasText: 'Back on track' })).toBeVisible();

    await sim(page, 'tr-relayed');
    await expect(card.getByTestId('route-badge')).toHaveAttribute('data-route', 'relayed');

    await sim(page, 'tr-fail');
    await expect(card).toHaveAttribute('data-status', 'failed');
    await expect(card).toContainText("The connection dropped and couldn't be restored.");
    await closePanel(page);
    await card.getByTestId('retry').click();
    await expect(card).toHaveAttribute('data-status', /connecting|transferring/);
    expect(await haptics(page)).toContain('transferFailed');
  });

  test('disk full pauses with a clear message', async ({ page }) => {
    await ready(page);
    await sim(page, 'stage-video');
    await closePanel(page);
    await page.getByTestId('orb-d-aisyah').click();
    const card = page.locator('[data-testid^="transfer-"]').first();
    await expect(card).toHaveAttribute('data-status', 'transferring');
    await sim(page, 'tr-disk');
    await expect(card).toContainText("Aisyah's laptop ran out of space");
  });
});

test.describe('settings', () => {
  test('theme and motion switch live; trusted device can be removed', async ({ page }) => {
    await ready(page);
    await page.getByTestId('open-settings').click();
    await expect(page.getByTestId('settings')).toBeVisible();
    await page.getByTestId('theme').getByRole('radio', { name: 'Dark' }).click();
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
    await page.getByTestId('motion').getByRole('radio', { name: 'Reduced' }).click();
    await expect(page.locator('html')).toHaveAttribute('data-reduced', 'true');
    await page.getByTestId('remove-d-aisyah').click();
    await page.getByTestId('confirm-remove').click();
    await expect(page.getByTestId('trusted-d-aisyah')).toBeHidden();
    await page.getByTestId('settings-back').click();
    await expect(page.getByTestId('home')).toBeVisible();
  });
});

test.describe('connection settings', () => {
  const shot = (name: string) => `test-results/connection-${name}.png`;

  async function openConnection(page: import('@playwright/test').Page) {
    await ready(page);
    await page.getByTestId('open-settings').click();
    await expect(page.getByTestId('connection')).toBeVisible();
    await page.getByTestId('connection').scrollIntoViewIfNeeded();
  }

  test('your own server: typos are explained, a wrong token is not saved, a good one is', async ({ page }, info) => {
    await openConnection(page);
    await expect(page.getByTestId('conn-mode').getByRole('radio', { name: 'Public' })).toHaveAttribute('aria-checked', 'true');
    await page.getByTestId('connection').screenshot({ path: shot(`${info.project.name}-public`) });

    await page.getByTestId('conn-mode').getByRole('radio', { name: 'Your own' }).click();
    await expect(page.getByTestId('conn-relay')).toBeVisible();

    // A typo is explained under the field, and nothing is checked yet.
    await page.getByTestId('conn-relay').fill('192.168.1.20:3340');
    await page.getByTestId('conn-save').click();
    await expect(page.getByTestId('conn-error-relay')).toHaveText('Start it with http:// or https://');
    await expect(page.getByTestId('conn-results')).toHaveCount(0);

    // Wrong token: the relay turns it down and nothing is saved.
    await page.getByTestId('conn-relay').fill('http://192.168.1.20:3340');
    await page.getByTestId('conn-token').fill('wrong');
    await page.getByTestId('conn-lookup').fill('http://192.168.1.20:3341');
    await expect(page.getByTestId('conn-error-relay')).toBeHidden();
    await page.getByTestId('conn-save').click();
    await expect(page.getByTestId('conn-result-relay')).toHaveAttribute('data-status', 'tokenRejected');
    await expect(page.getByTestId('conn-result-relay')).toContainText('Turned the token down');
    await expect(page.getByTestId('conn-result-lookup')).toContainText('Answered in 6 ms');
    await expect(page.getByTestId('conn-not-saved')).toBeVisible();
    await page.waitForTimeout(700);
    await page.getByTestId('connection').screenshot({ path: shot(`${info.project.name}-token-rejected`) });
    expect(await page.evaluate(() => (window as any).__nhako.app.connection.mode)).toBe('public');

    // The eye shows the token.
    await expect(page.getByTestId('conn-token')).toHaveAttribute('type', 'password');
    await page.getByTestId('conn-token-eye').click();
    await expect(page.getByTestId('conn-token')).toHaveAttribute('type', 'text');

    // Right token: checked, saved, and the app says so.
    await page.getByTestId('conn-token').fill('my-long-token');
    await expect(page.getByTestId('conn-results')).toHaveCount(0);
    await page.getByTestId('conn-save').click();
    await expect(page.getByTestId('toast').filter({ hasText: 'Now using your own server' })).toBeVisible();
    await expect(page.getByTestId('conn-result-relay')).toContainText('Answered in 14 ms');
    await expect(page.getByTestId('conn-save')).toBeDisabled();
    expect(await page.evaluate(() => (window as any).__nhako.app.connection)).toEqual({
      mode: 'own',
      relay: 'http://192.168.1.20:3340',
      token: 'my-long-token',
      lookup: 'http://192.168.1.20:3341',
    });
    await page.waitForTimeout(700);
    await page.getByTestId('connection').screenshot({ path: shot(`${info.project.name}-saved`) });

    // It survives a reload, and shows which relay is in use.
    await page.reload();
    await page.getByTestId('open-settings').click();
    await expect(page.getByTestId('conn-using')).toHaveText(/Using your relay at 192\.168\.1\.20/);
    await expect(page.getByTestId('conn-relay')).toHaveValue('http://192.168.1.20:3340');

    // Back to public in one tap.
    await page.getByTestId('conn-mode').getByRole('radio', { name: 'Public' }).click();
    await expect(page.getByTestId('toast').filter({ hasText: 'Back on the public servers' })).toBeVisible();
    await expect(page.getByTestId('conn-relay')).toBeHidden();
  });

  test('a server that does not answer can still be saved on purpose', async ({ page }, info) => {
    await openConnection(page);
    await page.getByTestId('conn-mode').getByRole('radio', { name: 'Your own' }).click();
    await page.getByTestId('conn-relay').fill('http://offline.lan:3340');
    await page.getByTestId('conn-token').fill('secret');
    await page.getByTestId('conn-save').click();
    await expect(page.getByTestId('conn-result-relay')).toHaveAttribute('data-status', 'unreachable');
    await expect(page.getByTestId('conn-result-relay')).toContainText("Can't reach it");
    await expect(page.getByTestId('conn-ask-anyway')).toBeVisible();
    await page.waitForTimeout(700);
    await page.getByTestId('connection').screenshot({ path: shot(`${info.project.name}-ask-anyway`) });
    expect(await page.evaluate(() => (window as any).__nhako.app.connection.mode)).toBe('public');
    await page.evaluate(() => document.documentElement.setAttribute('data-theme', 'dark'));
    await page.waitForTimeout(300);
    await page.getByTestId('connection').screenshot({ path: shot(`${info.project.name}-ask-anyway-dark`) });
    await page.getByTestId('conn-save-anyway').click();
    await expect(page.getByTestId('toast').filter({ hasText: 'Now using your own server' })).toBeVisible();
    await expect(page.getByTestId('conn-ask-anyway')).toBeHidden();
  });

  test('your own server going wrong shows a banner, and a code says the token was turned down', async ({ page }, info) => {
    await openConnection(page);
    await page.getByTestId('conn-mode').getByRole('radio', { name: 'Your own' }).click();
    await page.getByTestId('conn-relay').fill('http://offline.lan:3340');
    await page.getByTestId('conn-token').fill('secret');
    await page.getByTestId('conn-save').click();
    await page.getByTestId('conn-save-anyway').click();
    await page.getByTestId('settings-back').click();

    // Saved anyway, and it really doesn't answer: Home says so, with a way to Settings.
    const banner = page.getByTestId('banner-server');
    await expect(banner).toContainText("Can't reach your server right now");

    // Later the relay turns this device's token down.
    await sim(page, 'server-token');
    await closePanel(page);
    await expect(banner).toContainText('Your server turned this device away. Check the token in Settings.');
    await page.waitForTimeout(500);
    await page.screenshot({ path: shot(`${info.project.name}-server-banner`) });

    // Entering a code says why it failed instead of "didn't match".
    await page.evaluate(() => ((window as any).__nhako.app.sheet = { kind: 'enterCode' }));
    await page.getByTestId('code-word-0').fill('river');
    await page.getByTestId('code-word-1').fill('lantern');
    await page.getByTestId('code-word-2').fill('seven');
    await page.getByTestId('connect').click();
    await expect(page.getByTestId('sheet-enter-code')).toContainText("Your server turned this device away, so the other device couldn't be reached.");
    await page.getByTestId('sheet-close').click();

    // Fixed on the server: the banner goes away; the action opens Settings.
    await sim(page, 'server-ok');
    await closePanel(page);
    await expect(banner).toBeHidden();
    await sim(page, 'server-down');
    await closePanel(page);
    await banner.getByRole('button', { name: 'Settings' }).click();
    await expect(page.getByTestId('settings')).toBeVisible();
  });
});

test.describe('discovery and layout', () => {
  test('offline shows a banner and dims trusted devices', async ({ page }) => {
    await ready(page);
    await sim(page, 'disc-offline');
    await closePanel(page);
    await expect(page.getByTestId('banner-offline')).toBeVisible();
    await expect(page.getByTestId('orb-d-aisyah')).toHaveAttribute('data-state', 'offline');
    await expect(page.getByTestId('orb-d-studio')).toBeHidden();
  });

  test('more than 8 devices shows a More button', async ({ page }) => {
    await ready(page);
    await sim(page, 'disc-many');
    await closePanel(page);
    await expect(page.getByTestId('more-devices')).toContainText('more devices');
  });

  test('no sideways scrolling', async ({ page }) => {
    await ready(page);
    await page.waitForTimeout(800);
    const r = await page.evaluate(() => ({ sw: document.documentElement.scrollWidth, cw: document.documentElement.clientWidth }));
    expect(r.sw).toBeLessThanOrEqual(r.cw);
  });

  test('every button has an accessible name', async ({ page }) => {
    await ready(page);
    await sim(page, 'stage-photos');
    await sim(page, 'in-aisyah');
    const unnamed = await page.evaluate(() =>
      [...document.querySelectorAll('button, [role="button"], [role="switch"], [role="radio"]')]
        .filter((b) => (b as HTMLElement).offsetParent !== null)
        .filter((b) => !(b.getAttribute('aria-label') || b.getAttribute('aria-labelledby') || b.textContent?.trim() || b.getAttribute('title')))
        .map((b) => b.outerHTML.slice(0, 120)),
    );
    expect(unnamed).toEqual([]);
  });
});

test.describe('reduced motion', () => {
  test.use({ colorScheme: 'dark' });

  test('system reduced motion: no sonar, swipes fade instead of flying', async ({ page }) => {
    await page.emulateMedia({ reducedMotion: 'reduce' });
    await ready(page);
    await expect(page.locator('html')).toHaveAttribute('data-reduced', 'true');
    await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
    await expect(page.locator('.sonar')).toHaveCount(0);
    await sim(page, 'in-studio');
    await closePanel(page);
    await page.getByTestId('accept').click();
    await expect(page.getByTestId('incoming-card')).toBeHidden();
    await expect(page.locator('[data-testid^="transfer-"]').first()).toContainText('Studio PC');
  });
});

test.describe('phase 8', () => {
  test('history lists earlier transfers, shows details, adds new ones and clears', async ({ page }) => {
    await ready(page, { speed: 8 });
    await page.getByTestId('open-history').click();
    await expect(page.getByTestId('history')).toBeVisible();
    await expect(page.getByTestId('history-row-h1')).toContainText('From Work phone');
    await expect(page.getByTestId('history-row-h3')).toContainText('Cancelled');
    await page.getByTestId('history-row-h1').click();
    await expect(page.getByTestId('sheet-history')).toContainText('Received from Work phone');
    await expect(page.getByTestId('sheet-history')).toContainText('Average speed');
    await expect(page.getByTestId('history-open')).toBeVisible();
    await page.keyboard.press('Escape');
    await expect(page.getByTestId('sheet-history')).toBeHidden();

    // A new transfer lands at the top once it ends.
    await page.getByTestId('history-back').click();
    await sim(page, 'stage-photos');
    await closePanel(page);
    await page.getByTestId('orb-d-aisyah').click();
    await expect(page.locator('[data-testid^="transfer-"]').first()).toContainText('Sent to', { timeout: 15_000 });
    await page.getByTestId('open-history').click();
    await expect(page.locator('[data-testid^="history-row-"]').first()).toContainText("To Aisyah's laptop");

    await page.getByTestId('history-clear').click();
    await page.getByTestId('confirm-clear-history').click();
    await expect(page.getByTestId('history-empty')).toBeVisible();
  });

  test('open source licenses: searchable, each with its text', async ({ page }) => {
    await ready(page);
    await page.getByTestId('open-settings').click();
    await page.getByTestId('open-licenses').click();
    await expect(page.getByTestId('licenses')).toBeVisible();
    await page.getByTestId('licenses-search').fill('svelte');
    const row = page.getByRole('button', { name: /^svelte/ }).first();
    await row.click();
    await expect(page.getByTestId('licenses').locator('pre').first()).toContainText(/MIT|Permission/);
    await page.getByTestId('licenses-back').click();
    await expect(page.getByTestId('settings')).toBeVisible();
  });

  test('finding nearby devices can be turned off and on', async ({ page }) => {
    await ready(page);
    await page.getByTestId('open-settings').click();
    const lan = page.getByTestId('lan-discovery');
    await expect(lan).toHaveAttribute('aria-checked', 'true');
    await lan.click();
    await expect(lan).toHaveAttribute('aria-checked', 'false');
    await page.getByTestId('settings-back').click();
    // Only trusted devices, reached over the internet, are left.
    await expect(page.getByTestId('orb-d-aisyah')).toBeVisible();
    await expect(page.getByTestId('orb-d-studio')).toBeHidden();
    await page.getByTestId('open-settings').click();
    await page.getByTestId('lan-discovery').click();
    await page.getByTestId('settings-back').click();
    await expect(page.getByTestId('orb-d-studio')).toBeVisible();
  });

  test('the launch intro plays once and lets clicks through', async ({ page }) => {
    await page.goto('/');
    await expect(page.locator('.intro')).toHaveCount(1);
    await expect(page.locator('.intro')).toHaveCount(0, { timeout: 3000 });
  });

  test('the tray popover shows your devices and opens the app', async ({ page }) => {
    await page.addInitScript(() => {
      localStorage.setItem('nhako.app.v1', JSON.stringify({ onboarded: true, selfName: 'Test device', saveLocation: 'Downloads/Nhako Beam' }));
    });
    await page.setViewportSize({ width: 360, height: 520 });
    await page.goto('/?view=popover');
    await expect(page.getByTestId('popover')).toBeVisible();
    await expect(page.getByTestId('pop-device-d-aisyah')).toBeVisible();
    await expect(page.getByTestId('pop-open')).toBeVisible();
    const r = await page.evaluate(() => ({ sw: document.documentElement.scrollWidth, cw: document.documentElement.clientWidth }));
    expect(r.sw).toBeLessThanOrEqual(r.cw);
  });
});

test.describe('found on real devices after phase 8', () => {
  test('a removed trusted device that is away leaves the radar at once', async ({ page }) => {
    await ready(page);
    await expect(page.getByTestId('orb-d-office')).toBeVisible();
    await page.getByTestId('open-settings').click();
    await page.getByTestId('remove-d-office').click();
    await page.getByTestId('confirm-remove').click();
    await page.getByTestId('settings-back').click();
    await expect(page.getByTestId('home')).toBeVisible();
    await expect(page.getByTestId('orb-d-office')).toHaveCount(0);
  });

  test('settings scrolls with the mouse wheel', async ({ page, isMobile }) => {
    test.skip(isMobile, 'the wheel is for the desktop window');
    await ready(page);
    await page.getByTestId('open-settings').click();
    await expect(page.getByTestId('settings')).toBeVisible();
    await page.mouse.move(640, 400);
    await page.mouse.wheel(0, 2000);
    await expect.poll(() => page.evaluate(() => window.scrollY)).toBeGreaterThan(0);
  });

  test('a long file name stays inside the request card', async ({ page }) => {
    await ready(page);
    await sim(page, 'in-longname');
    await closePanel(page);
    const card = page.getByTestId('incoming-card');
    await expect(card).toBeVisible();
    await expect(page.getByTestId('accept')).toBeVisible();
    const spill = await card.evaluate((el) => {
      const edge = el.getBoundingClientRect().right;
      return [...el.querySelectorAll('*')]
        .filter((n) => !n.closest('.edge') && n.getBoundingClientRect().width > 0)
        .map((n) => n.getBoundingClientRect().right - edge)
        .filter((over) => over > 0.5).length;
    });
    expect(spill).toBe(0);
  });
});

test.describe('sounds', () => {
  test('taps, screens, and Loud, Soft and Off', async ({ page }) => {
    await ready(page);
    await page.getByTestId('open-settings').click();
    await page.getByTestId('theme').getByRole('radio').nth(2).click();
    await expect.poll(() => soundsPlayed(page)).toEqual(['open', 'tap']);

    const level = page.getByTestId('sounds').getByRole('radio');
    await expect(level.nth(1)).toHaveAttribute('aria-checked', 'true'); // Soft by default
    await level.nth(2).click(); // Off: this click is silent, and so is the next
    await page.getByTestId('theme').getByRole('radio').nth(1).click();
    await page.waitForTimeout(100);
    expect(await soundsPlayed(page)).toEqual(['open', 'tap']);

    await level.nth(0).click(); // Loud
    await page.getByTestId('settings-back').click();
    await expect.poll(() => soundsPlayed(page)).toEqual(['open', 'tap', 'tap', 'close']);
    // The choice stays after a reload.
    await page.reload();
    await page.getByTestId('open-settings').click();
    await expect(page.getByTestId('sounds').getByRole('radio').nth(0)).toHaveAttribute('aria-checked', 'true');
  });

  test('a request, accepting it and the transfer finishing', async ({ page }) => {
    await ready(page, { speed: 40 });
    await sim(page, 'in-studio');
    await closePanel(page);
    await expect(page.getByTestId('incoming-card')).toBeVisible();
    await page.getByTestId('accept').click();
    await expect.poll(() => soundsPlayed(page), { timeout: 15_000 }).toContain('done');
    const heard = (await soundsPlayed(page)).filter((n) => n !== 'tap');
    expect(heard).toEqual(['incoming', 'accept', 'done']);
  });
});
