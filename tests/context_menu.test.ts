import { test, expect } from '@playwright/test';
import {
  addInvokeHandler,
  installTauriStubs,
  makeState,
  rightClickGridCell,
  waitForApp,
} from './helpers';

const N = 3;

const ONE_SINGLETON = [{ cells: [[0, 0]], op: 'Given', target: 1 }];
const ADJACENT_PAIR = [
  { cells: [[0, 0], [0, 1]], op: 'Add', target: 3 },
  { cells: [[1, 0], [1, 1]], op: 'Add', target: 5 },
];

test.describe('context menu', () => {
  test('right-click on uncovered cell shows only "Make singleton"', async ({ page }) => {
    await installTauriStubs(page, makeState(N));
    await waitForApp(page);

    await rightClickGridCell(page, N, 1, 1);

    await expect(page.getByText('Make singleton')).toBeVisible();
    await expect(page.getByText('Set operation')).toHaveCount(0);
    await expect(page.getByText('Uncage')).toHaveCount(0);
    await expect(page.getByText('Delete cage')).toHaveCount(0);
    await expect(page.getByText('Move cell')).toHaveCount(0);
  });

  test('right-click on singleton cage shows Set operation / Uncage / Delete', async ({ page }) => {
    await installTauriStubs(page, makeState(N, ONE_SINGLETON));
    await waitForApp(page);

    await rightClickGridCell(page, N, 0, 0);

    await expect(page.getByText('Set operation')).toBeVisible();
    await expect(page.getByText('Uncage')).toBeVisible();
    await expect(page.getByText('Delete cage')).toBeVisible();
    // Singleton — Make singleton must NOT be present.
    await expect(page.getByText('Make singleton')).toHaveCount(0);
    // No adjacent cage — Move cell hidden.
    await expect(page.getByText('Move cell')).toHaveCount(0);
  });

  test('right-click on multi-cell cage adjacent to another shows Move cell', async ({ page }) => {
    await installTauriStubs(page, makeState(N, ADJACENT_PAIR));
    await waitForApp(page);

    await rightClickGridCell(page, N, 0, 0);

    await expect(page.getByText('Move cell')).toBeVisible();
    await expect(page.getByText('Set operation')).toBeVisible();
    await expect(page.getByText('Uncage')).toBeVisible();
    await expect(page.getByText('Delete cage')).toBeVisible();
    await expect(page.getByText('Make singleton')).toBeVisible();
  });

  test('Escape closes an open context menu', async ({ page }) => {
    await installTauriStubs(page, makeState(N, ONE_SINGLETON));
    await waitForApp(page);

    await rightClickGridCell(page, N, 0, 0);
    await expect(page.getByText('Set operation')).toBeVisible();

    await page.keyboard.press('Escape');
    await expect(page.getByText('Set operation')).toHaveCount(0);
  });

  test('Delete cage menu item dispatches remove_cage', async ({ page }) => {
    await installTauriStubs(page, makeState(N, ONE_SINGLETON));
    await addInvokeHandler(page, 'remove_cage', (args, currentState) => {
      (window as any).__remove_args__ = args;
      return currentState;
    });
    await waitForApp(page);

    await rightClickGridCell(page, N, 0, 0);
    await page.getByText('Delete cage').click();

    await expect.poll(() =>
      page.evaluate(() => (window as any).__remove_args__),
    ).toBeTruthy();

    const args = await page.evaluate(() => (window as any).__remove_args__);
    expect(args.anchor).toEqual([0, 0]);
  });

  test('left-click outside the menu dismisses it', async ({ page }) => {
    await installTauriStubs(page, makeState(N, ONE_SINGLETON));
    await waitForApp(page);

    await rightClickGridCell(page, N, 0, 0);
    await expect(page.getByText('Set operation')).toBeVisible();

    // Left-click on the grid (cell click handler also clears the menu).
    const svg = page.locator('.grid-svg');
    const box = await svg.boundingBox();
    if (!box) throw new Error('grid-svg not found');
    const cellSize = box.width / N;
    await page.mouse.click(box.x + cellSize * 2.5, box.y + cellSize * 2.5);

    await expect(page.getByText('Set operation')).toHaveCount(0);
  });
});
