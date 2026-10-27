// Screenshots for the store pages (Flathub's AppStream file, winget, the README), from the
// prototype. Only when asked: STORE_SHOTS=1 npx playwright test store-shots --project=desktop
// They're written to packaging/screenshots/ and committed.
import { expect, test } from '@playwright/test';
import { closePanel, ready, sim } from './helpers';

const OUT = '../packaging/screenshots';
// Playwright runs this in Node; the UI's type settings don't include Node's.
declare const process: { env: Record<string, string | undefined> };

test.skip(!process.env.STORE_SHOTS, 'only with STORE_SHOTS=1');
test.use({ viewport: { width: 1280, height: 800 }, colorScheme: 'light' });

test('store screenshots', async ({ page }, info) => {
  test.skip(info.project.name !== 'desktop');
  await ready(page, { name: "Hakim's laptop" });
  // The prototype's Scenarios button isn't part of the app.
  await page.addStyleTag({ content: '[data-testid="dev-toggle"] { opacity: 0 !important; }' });
  await page.waitForTimeout(1500);
  await page.screenshot({ path: `${OUT}/desktop-1-devices.png` });

  await page.evaluate(() => ((window as any).__nhako.app.engine.speedScale = 0.5));
  await sim(page, 'stage-mixed');
  await closePanel(page);
  await page.getByTestId('radar').getByTestId('orb-d-aisyah').click();
  await expect(page.locator('[data-testid^="transfer-"]').first()).toHaveAttribute('data-status', 'transferring', { timeout: 15_000 });
  await page.waitForTimeout(7000);
  await page.screenshot({ path: `${OUT}/desktop-2-sending.png` });

  await sim(page, 'in-tree');
  await closePanel(page);
  await page.waitForTimeout(1200);
  await page.screenshot({ path: `${OUT}/desktop-3-request.png` });
  await page.getByTestId('incoming-card').getByTestId('decline').click();

  await sim(page, 'stage-photos');
  await closePanel(page);
  await page.getByTestId('choose-device').click();
  await page.getByTestId('share-link').click();
  await sim(page, 'link-visitor');
  await closePanel(page);
  await page.waitForTimeout(1200);
  await page.screenshot({ path: `${OUT}/desktop-4-share-via-link.png` });
});
