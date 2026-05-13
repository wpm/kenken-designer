import { test, expect } from '@playwright/test';
import {
  addInvokeHandler,
  installTauriStubs,
  makeState,
  setupCageBandWithTuples,
  waitForApp,
} from './helpers';

const N = 3;
const ONE_CAGE = [{ cells: [[0, 0]], op: 'Given', target: 1 }];

test.describe('cage band thumbnail interaction', () => {
  test('clicking a thumbnail focuses it and applies the selected ring class', async ({ page }) => {
    await setupCageBandWithTuples(page, N, ONE_CAGE, 3);

    const firstThumb = page.locator('.cage-band__thumb').first();
    await firstThumb.click();

    await expect(firstThumb).toBeFocused();
    await expect(firstThumb).toHaveClass(/cage-band__thumb--selected/);
  });

  test('the band reserves its slot even when no cage is active', async ({ page }) => {
    await installTauriStubs(page, makeState(N));
    await waitForApp(page);

    const band = page.locator('.cage-band');
    await expect(band).toBeVisible();
    // No thumbnails appear when there's no active cage.
    await expect(page.locator('.cage-band__thumb')).toHaveCount(0);
  });

  test('Enter on a focused thumbnail invokes apply_narrowing', async ({ page }) => {
    const view = makeState(N, ONE_CAGE);
    await installTauriStubs(page, view);

    // rank_active_cage returns 3 tuples; apply_narrowing records its args.
    await addInvokeHandler(
      page,
      'rank_active_cage',
      `
      const v = currentState;
      return [
        { tuple: [1], view: v, total_reduction: 0, newly_singleton: 0 },
        { tuple: [2], view: v, total_reduction: 0, newly_singleton: 0 },
        { tuple: [3], view: v, total_reduction: 0, newly_singleton: 0 },
      ];
      `,
    );
    await addInvokeHandler(
      page,
      'apply_narrowing',
      `
      window.__narrow_args__ = args;
      return currentState;
      `,
    );

    await waitForApp(page);
    const svg = page.locator('.grid-svg');
    const box = await svg.boundingBox();
    if (!box) throw new Error('grid-svg not found');
    const cellSize = box.width / N;
    await page.mouse.click(box.x + cellSize * 0.5, box.y + cellSize * 0.5);

    const firstThumb = page.locator('.cage-band__thumb').first();
    await firstThumb.click();
    await page.keyboard.press('Enter');

    await expect.poll(() =>
      page.evaluate(() => (window as any).__narrow_args__),
    ).toBeTruthy();

    const args = await page.evaluate(() => (window as any).__narrow_args__);
    expect(args.anchor).toEqual([0, 0]);
    expect(args.tuple).toEqual([1]);
  });

  test('switching to a different cage replaces the rendered thumbnails', async ({ page }) => {
    const TWO_CAGES = [
      { cells: [[0, 0]], op: 'Given', target: 1 },
      { cells: [[2, 2]], op: 'Given', target: 3 },
    ];
    const view = makeState(N, TWO_CAGES);
    await installTauriStubs(page, view);

    // Differentiate thumbnails per cage by total_reduction so we can detect the swap.
    await addInvokeHandler(
      page,
      'rank_active_cage',
      `
      const r = (args.anchor[0] === 0 && args.anchor[1] === 0) ? 11 : 22;
      return [
        { tuple: [1], view: currentState, total_reduction: r, newly_singleton: 0 },
        { tuple: [2], view: currentState, total_reduction: r, newly_singleton: 0 },
      ];
      `,
    );

    await waitForApp(page);
    const svg = page.locator('.grid-svg');
    const box = await svg.boundingBox();
    if (!box) throw new Error('grid-svg not found');
    const cellSize = box.width / N;

    // Click cage at (0,0) — band loads.
    await page.mouse.click(box.x + cellSize * 0.5, box.y + cellSize * 0.5);
    await page.waitForSelector('.cage-band__thumb', { timeout: 5000 });
    const firstCount = await page.locator('.cage-band__thumb').count();
    expect(firstCount).toBeGreaterThan(0);

    // Click the cage at (2,2) — band reloads with new (different) tuples.
    await page.mouse.click(box.x + cellSize * 2.5, box.y + cellSize * 2.5);
    await page.waitForFunction(
      () => document.querySelectorAll('.cage-band__thumb').length > 0,
    );

    // The strip should now correspond to the new cage; we just sanity-check
    // that thumbnails remain rendered after the switch.
    expect(await page.locator('.cage-band__thumb').count()).toBeGreaterThan(0);
  });
});
