import { expect, type Page } from '@playwright/test';

/** Skips onboarding, opens home, speeds up the mock engine and waits for devices. */
export async function ready(page: Page, opts: { speed?: number; name?: string } = {}) {
  await page.addInitScript((name) => {
    try {
      localStorage.setItem('nhako.app.v1', JSON.stringify({ onboarded: true, selfName: name, saveLocation: 'Downloads/Nhako Beam' }));
    } catch {
      // ignore
    }
  }, opts.name ?? 'Test device');
  await page.goto('/');
  await expect(page.getByTestId('home')).toBeVisible();
  await page.evaluate((s) => ((window as any).__nhako.app.engine.speedScale = s), opts.speed ?? 1);
  await expect(page.getByTestId('orb-d-studio')).toBeVisible();
}

export async function sim(page: Page, id: string) {
  const panel = page.getByTestId('dev-panel');
  if (!(await panel.isVisible())) await page.getByTestId('dev-toggle').click();
  await page.getByTestId(`sim-${id}`).click();
}

export async function closePanel(page: Page) {
  if (await page.getByTestId('dev-panel').isVisible()) await page.getByTestId('dev-toggle').click();
  await expect(page.getByTestId('dev-panel')).toBeHidden();
}

export const haptics = (page: Page) => page.evaluate(() => (window as any).__nhako.haptics.entries.map((e: { name: string }) => e.name) as string[]);

export async function center(page: Page, testid: string) {
  const box = await page.getByTestId(testid).first().boundingBox();
  if (!box) throw new Error(`no box for ${testid}`);
  return { x: box.x + box.width / 2, y: box.y + box.height / 2, w: box.width, h: box.height };
}

/** Drags with the mouse in small timed steps so velocity looks human. */
export async function drag(page: Page, from: { x: number; y: number }, to: { x: number; y: number }, opts: { steps?: number; stepMs?: number; release?: boolean } = {}) {
  const steps = opts.steps ?? 16;
  await page.mouse.move(from.x, from.y);
  await page.mouse.down();
  for (let i = 1; i <= steps; i++) {
    await page.mouse.move(from.x + ((to.x - from.x) * i) / steps, from.y + ((to.y - from.y) * i) / steps);
    if (opts.stepMs) await page.waitForTimeout(opts.stepMs);
  }
  if (opts.release !== false) await page.mouse.up();
}

/** The sounds played so far, oldest first (lib/sounds.svelte.ts logs every one). Leaves out the
 *  launch sound and devices showing up, which come on their own schedule. */
export const soundsPlayed = (page: Page) =>
  page.evaluate(() =>
    ((window as any).__nhako.sounds.entries as { name: string }[])
      .map((e) => e.name)
      .reverse()
      .filter((n) => n !== 'intro' && n !== 'found'),
  );
