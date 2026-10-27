// Accessibility (Phase 15): every main screen and sheet checked with axe against WCAG 2.2 A and
// AA, in light and dark, plus keyboard-only use. The prototype's Scenarios panel isn't part of the
// app, so it's left out of the scans.
import AxeBuilder from '@axe-core/playwright';
import { expect, test, type Page } from '@playwright/test';
import { closePanel, ready, sim } from './helpers';

async function scan(page: Page, what: string) {
  // Let entrances finish, so colors are checked at rest rather than mid-fade.
  await page.waitForTimeout(700);
  const result = await new AxeBuilder({ page })
    .withTags(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'])
    .exclude('[data-testid="dev-panel"]')
    .exclude('[data-testid="dev-toggle"]')
    .analyze();
  const found = result.violations.map((v) => `${v.id} (${v.impact}): ${v.help}\n    ${v.nodes.map((n) => n.target.join(' ')).slice(0, 6).join('\n    ')}`);
  expect(found, `${what}:\n${found.join('\n')}`).toEqual([]);
}

async function sheet(page: Page, open: () => Promise<void>, what: string) {
  await open();
  await scan(page, what);
  await page.keyboard.press('Escape');
  await expect(page.getByRole('dialog')).toHaveCount(0);
}

for (const scheme of ['light', 'dark'] as const) {
  test.describe(`accessibility, ${scheme}`, () => {
    test.use({ colorScheme: scheme });

    test('onboarding', async ({ page }) => {
      await page.goto('/');
      await expect(page.getByTestId('onboarding')).toBeVisible();
      await scan(page, 'welcome');
      await page.getByTestId('onboarding-next').click();
      await scan(page, 'name');
    });

    test('home, its banners, a request and a transfer', async ({ page }) => {
      await ready(page, { speed: 30 });
      await scan(page, 'home');
      await sim(page, 'stage-photos');
      await sim(page, 'update-out');
      await closePanel(page);
      await page.getByTestId('radar').getByTestId('orb-d-aisyah').click();
      await sim(page, 'stage-photos');
      await closePanel(page);
      await scan(page, 'home with a tray, a transfer and a banner');
      await sim(page, 'in-aisyah');
      await closePanel(page);
      await scan(page, 'a request');
    });

    test('every sheet', async ({ page }) => {
      await ready(page);
      await sheet(page, async () => {
        await sim(page, 'stage-photos');
        await closePanel(page);
        await page.getByTestId('choose-device').click();
      }, 'send to');
      await sheet(page, () => page.getByTestId('send-code').click(), 'show a code');
      await sheet(page, () => page.getByTestId('receive-code').click(), 'enter a code');
      await page.getByTestId('clear-staged').click();
      await sheet(page, () => page.getByTestId('send-text').click(), 'write text');
      await sim(page, 'stage-photos');
      await closePanel(page);
      await sheet(page, async () => {
        await page.getByTestId('choose-device').click();
        await page.getByTestId('share-link').click();
        await sim(page, 'link-visitor');
        await closePanel(page);
      }, 'share via link');
      await page.getByTestId('banner-link').getByRole('button', { name: 'Show' }).click();
      await page.getByTestId('link-stop').click();
      await sheet(page, () => sim(page, 'trust-ask').then(() => closePanel(page)), 'being asked to trust');
    });

    test('settings, history and licenses', async ({ page }) => {
      // The licenses list is long, and axe reads all of it.
      test.setTimeout(120_000);
      await ready(page);
      await page.getByTestId('open-settings').click();
      await scan(page, 'settings');
      await page.getByTestId('open-licenses').click();
      await scan(page, 'licenses');
      await page.goto('/');
      await page.getByTestId('open-history').click();
      await scan(page, 'history');
    });
  });
}

test.describe('keyboard', () => {
  test('a sheet keeps focus inside, Escape closes it, and focus goes back', async ({ page }) => {
    await ready(page);
    await sim(page, 'stage-photos');
    await closePanel(page);
    const trigger = page.getByTestId('choose-device');
    await trigger.focus();
    await page.keyboard.press('Enter');
    const dialog = page.getByRole('dialog', { name: 'Send to' });
    await expect(dialog).toBeVisible();
    for (let i = 0; i < 25; i++) {
      await page.keyboard.press('Tab');
      expect(await dialog.evaluate((d) => d.contains(document.activeElement)), `Tab ${i + 1} stays in the sheet`).toBe(true);
    }
    for (let i = 0; i < 5; i++) {
      await page.keyboard.press('Shift+Tab');
      expect(await dialog.evaluate((d) => d.contains(document.activeElement))).toBe(true);
    }
    await page.keyboard.press('Escape');
    await expect(dialog).toHaveCount(0);
    await expect(trigger).toBeFocused();
  });

  test('home works from the keyboard alone: a device takes what is staged', async ({ page }) => {
    await ready(page, { speed: 30 });
    await sim(page, 'stage-photos');
    await closePanel(page);
    // Every control on the way is reachable with Tab and shows where focus is.
    const seen = new Set<string>();
    for (let i = 0; i < 40 && !seen.has('orb-d-aisyah'); i++) {
      await page.keyboard.press('Tab');
      const id = await page.evaluate(() => {
        const el = document.activeElement as HTMLElement | null;
        if (!el || el === document.body) return null;
        const style = getComputedStyle(el);
        const ring = style.outlineStyle !== 'none' && parseFloat(style.outlineWidth) > 0;
        return { id: el.dataset.testid ?? el.closest('[data-testid]')?.getAttribute('data-testid') ?? el.tagName, ring: ring || style.boxShadow !== 'none' };
      });
      if (!id) continue;
      expect(id.ring, `${id.id} shows focus`).toBe(true);
      seen.add(id.id);
    }
    expect(seen.has('orb-d-aisyah'), `reached: ${[...seen].join(', ')}`).toBe(true);
    await page.keyboard.press('Enter');
    await expect(page.locator('[data-testid^="transfer-"]').first()).toContainText("Aisyah's laptop");
  });

  test('a request can be answered from the keyboard', async ({ page }) => {
    await ready(page);
    await sim(page, 'in-studio');
    await closePanel(page);
    const accept = page.getByTestId('incoming-card').getByTestId('accept');
    await accept.focus();
    await expect(accept).toBeFocused();
    await page.keyboard.press('Enter');
    await expect(page.locator('[data-testid^="transfer-"]').first()).toBeVisible();
  });
});

test.describe('reflow', () => {
  // WCAG 1.4.10: at 320 CSS pixels wide (a 1280 px window zoomed to 400%), nothing scrolls sideways.
  test.use({ viewport: { width: 320, height: 640 } });

  test('home, a sheet, settings and history fit in 320 px', async ({ page }) => {
    await ready(page);
    const fits = async (what: string) => {
      // Screens and sheets slide in on springs: measure once they've settled.
      await page.waitForTimeout(1200);
      const wide = await page.evaluate(() => document.documentElement.scrollWidth);
      const culprits = await page.evaluate(() =>
        [...document.querySelectorAll('body *')]
          .filter((e) => e.getBoundingClientRect().right > 320.5 && !e.closest('svg, .fab, [data-testid^="dev-"]'))
          .map((e) => `${e.tagName.toLowerCase()}.${[...e.classList].join('.')} ends at ${Math.round(e.getBoundingClientRect().right)}`)
          .slice(0, 8),
      );
      expect(wide, `${what} is ${wide} px wide:\n${culprits.join('\n')}`).toBeLessThanOrEqual(320);
    };
    await fits('home');
    await sim(page, 'stage-photos');
    await closePanel(page);
    await fits('home with a tray');
    await page.getByTestId('choose-device').click();
    await fits('send to');
    await page.getByTestId('share-link').click();
    await sim(page, 'link-visitor');
    await closePanel(page);
    await fits('share via link');
    await page.getByTestId('link-stop').click();
    await page.getByTestId('open-settings').click();
    await fits('settings');
    await page.goto('/');
    await page.getByTestId('open-history').click();
    await fits('history');
  });
});
