import { test, expect } from '@playwright/test';
import {
  ACCENT_COLOR,
  ACTIVE_FILL_OPACITY,
  INK_COLOR,
  addInvokeHandler,
  clickGridCell,
  installTauriStubs,
  makeState,
  waitForApp,
} from './helpers';

const N = 3;

const ONE_PAIR = [{ cells: [[0, 0], [0, 1]], op: 'Add', target: 3 }];

const overlaySelector = `.grid-svg rect[fill="${ACCENT_COLOR}"][fill-opacity="${ACTIVE_FILL_OPACITY}"]`;

test.describe('active cage overlay', () => {
  test('no overlay on initial load (no cage active until a caged cell is clicked)', async ({ page }) => {
    await installTauriStubs(page, makeState(N, ONE_PAIR));
    await waitForApp(page);

    await expect(page.locator(overlaySelector)).toHaveCount(0);
  });

  test('clicking a caged cell renders accent overlay over all its cells', async ({ page }) => {
    await installTauriStubs(page, makeState(N, ONE_PAIR));
    await waitForApp(page);

    await clickGridCell(page, N, 0, 0);

    // Cage has 2 cells, so 2 overlay rects.
    await expect(page.locator(overlaySelector)).toHaveCount(2);
  });

  test('cursor stroke turns ink-color during operator entry', async ({ page }) => {
    const singleton = [{ cells: [[1, 1]], op: 'Given', target: 1 }];
    await installTauriStubs(page, makeState(N, singleton));
    await addInvokeHandler(page, 'cage_options', () => [
      { op: 'Given', targets: [1, 2, 3] },
    ]);
    await waitForApp(page);

    await clickGridCell(page, N, 1, 1);

    const cursor = page.locator('[data-testid="cursor"]');
    // Outside entry: cursor stroke is ACCENT.
    await expect(cursor).toHaveAttribute('stroke', ACCENT_COLOR);

    await page.keyboard.press('Enter');

    // In entry: stroke turns INK with wider width.
    await expect(cursor).toHaveAttribute('stroke', INK_COLOR);
    await expect(cursor).toHaveAttribute('stroke-width', '3');
  });
});
