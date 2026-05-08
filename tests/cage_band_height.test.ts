import { test, expect, Page } from '@playwright/test';
import { setupCageBandWithTuples } from './helpers';

const N = 3;
const ONE_CAGE = [{ cells: [[0, 0]], op: 'Given', target: 3 }];

const clickCell = (page: Page, row: number, col: number) =>
  // Import clickGridCell indirectly via arrow
  page.evaluate(
    ({ n, row, col }: { n: number; row: number; col: number }) => {
      const svg = document.querySelector('.grid-svg') as SVGElement | null;
      if (!svg) throw new Error('grid-svg not found');
      const rect = svg.getBoundingClientRect();
      const cellSize = rect.width / n;
      const x = rect.left + cellSize * (col + 0.5);
      const y = rect.top + cellSize * (row + 0.5);
      (document.elementFromPoint(x, y) as HTMLElement | null)?.click();
    },
    { n: N, row, col },
  );

test.describe('cage band height stability', () => {
  test('cage band height does not change during arrow-key scroll animation', async ({ page }) => {
    // 6 tuples and a small viewport so only 1-2 are visible — forces scroll on arrow key.
    await page.setViewportSize({ width: 900, height: 600 });
    await setupCageBandWithTuples(page, N, ONE_CAGE, 6);

    const band = page.locator('.cage-band');
    await expect(band).toBeVisible();

    // Record the resting height.
    const restHeight = await band.evaluate((el) => el.getBoundingClientRect().height);
    expect(restHeight).toBeGreaterThan(0);

    // Focus the first thumbnail and navigate down repeatedly, sampling height each time.
    const firstThumb = page.locator('.cage-band__thumb').first();
    await firstThumb.focus();

    const heights: number[] = [];
    const sample = async () => {
      const h = await band.evaluate((el) => el.getBoundingClientRect().height);
      heights.push(h);
    };

    // Press ArrowDown several times, sampling height frequently between presses.
    for (let i = 0; i < 4; i++) {
      await page.keyboard.press('ArrowDown');
      // Sample rapidly during the animation window (200ms nominal).
      for (let j = 0; j < 10; j++) {
        await sample();
        await page.waitForTimeout(25);
      }
    }

    // Wait for any final animation to settle.
    await page.waitForTimeout(300);
    await sample();

    const minH = Math.min(...heights);
    const maxH = Math.max(...heights);

    // Allow 1px tolerance for subpixel rounding.
    expect(maxH - minH).toBeLessThanOrEqual(1);
  });

  test('cage band fills available vertical space (taller than or equal to grid)', async ({ page }) => {
    await page.setViewportSize({ width: 900, height: 700 });
    await setupCageBandWithTuples(page, N, ONE_CAGE, 3);

    const band = page.locator('.cage-band');
    const grid = page.locator('.grid-svg');

    const bandHeight = await band.evaluate((el) => el.getBoundingClientRect().height);
    const gridHeight = await grid.evaluate((el) => el.getBoundingClientRect().height);

    // The band should be at least as tall as the grid.
    expect(bandHeight).toBeGreaterThanOrEqual(gridHeight - 1); // 1px tolerance
  });
});
