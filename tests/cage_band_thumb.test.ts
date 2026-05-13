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
    await addInvokeHandler(page, 'rank_active_cage', (_args, currentState) => [
      { tuple: [1], view: currentState, total_reduction: 0, newly_singleton: 0 },
      { tuple: [2], view: currentState, total_reduction: 0, newly_singleton: 0 },
      { tuple: [3], view: currentState, total_reduction: 0, newly_singleton: 0 },
    ]);
    await addInvokeHandler(page, 'apply_narrowing', (args, currentState) => {
      (window as any).__narrow_args__ = args;
      return currentState;
    });

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

  test('switching to a different cage refetches tuples for the new anchor', async ({ page }) => {
    const TWO_CAGES = [
      { cells: [[0, 0]], op: 'Given', target: 1 },
      { cells: [[2, 2]], op: 'Given', target: 3 },
    ];
    const view = makeState(N, TWO_CAGES);
    await installTauriStubs(page, view);

    // Record every anchor passed to rank_active_cage so we can verify that
    // clicking the second cage actually triggers a refetch (not just a no-op
    // re-render).
    await addInvokeHandler(page, 'rank_active_cage', (args, currentState) => {
      const calls = ((window as any).__rank_anchors__ ?? []) as Array<[number, number]>;
      calls.push(args.anchor);
      (window as any).__rank_anchors__ = calls;
      return [
        { tuple: [1], view: currentState, total_reduction: 0, newly_singleton: 0 },
        { tuple: [2], view: currentState, total_reduction: 0, newly_singleton: 0 },
      ];
    });

    await waitForApp(page);
    const svg = page.locator('.grid-svg');
    const box = await svg.boundingBox();
    if (!box) throw new Error('grid-svg not found');
    const cellSize = box.width / N;

    // Click cage at (0,0) — band loads.
    await page.mouse.click(box.x + cellSize * 0.5, box.y + cellSize * 0.5);
    await page.waitForSelector('.cage-band__thumb', { timeout: 5000 });
    expect(await page.locator('.cage-band__thumb').count()).toBeGreaterThan(0);

    // Click the cage at (2,2) — band must refetch for the new anchor.
    await page.mouse.click(box.x + cellSize * 2.5, box.y + cellSize * 2.5);
    await expect
      .poll(() =>
        page.evaluate(() => ((window as any).__rank_anchors__ ?? []).length),
      )
      .toBe(2);

    const anchors = await page.evaluate(
      () => (window as any).__rank_anchors__ as Array<[number, number]>,
    );
    expect(anchors[0]).toEqual([0, 0]);
    expect(anchors[1]).toEqual([2, 2]);

    // Thumbnails remain rendered after the switch.
    expect(await page.locator('.cage-band__thumb').count()).toBeGreaterThan(0);
  });
});
