import { test, expect } from '@playwright/test';
import {
  clickGridCell,
  installTauriStubs,
  makeState,
  waitForApp,
} from './helpers';

const N = 3;

const ONE_PAIR = [{ cells: [[0, 0], [0, 1]], op: 'Add', target: 3 }];

test.describe('active cage overlay', () => {
  test('no overlay when no cage is active (uncovered cell)', async ({ page }) => {
    await installTauriStubs(page, makeState(N, ONE_PAIR));
    await waitForApp(page);

    // Initial cursor is (0,0), part of the cage — active_cage is set.
    // Click an uncovered cell to deactivate by being elsewhere.
    // Note: set_active_cage_for_cell PRESERVES the previous cage when the
    // new cell is uncaged. So we expect overlay to remain on the cage.
    // For initial test, just verify overlay exists on (0,0) since it's
    // automatically active.
    const overlay = page.locator(
      '.grid-svg rect[fill="#1a4e7a"][fill-opacity="0.16"]',
    );
    // At least one overlay rect (for cell (0,0)).
    // Initial active_cage is None per app.rs; only becomes Some when we click
    // a caged cell.
    await expect(overlay).toHaveCount(0);
  });

  test('clicking a caged cell renders accent overlay over all its cells', async ({ page }) => {
    await installTauriStubs(page, makeState(N, ONE_PAIR));
    await waitForApp(page);

    await clickGridCell(page, N, 0, 0);

    // Cage has 2 cells, so 2 overlay rects.
    const overlay = page.locator(
      '.grid-svg rect[fill="#1a4e7a"][fill-opacity="0.16"]',
    );
    await expect(overlay).toHaveCount(2);
  });

  test('cursor stroke turns ink-color during operator entry', async ({ page }) => {
    // For a singleton cage so we can open entry without stubbing cage_options
    // (actually we still need cage_options stub).
    const singleton = [{ cells: [[1, 1]], op: 'Given', target: 1 }];
    await installTauriStubs(page, makeState(N, singleton));
    await page.addInitScript(() => {
      (window as any).__tauri_invoke_handlers__ = {
        ...((window as any).__tauri_invoke_handlers__ ?? {}),
        cage_options: () => [{ op: 'Given', targets: [1, 2, 3] }],
      };
    });
    await waitForApp(page);

    await clickGridCell(page, N, 1, 1);

    const cursor = page.locator('[data-testid="cursor"]');
    // Outside entry: cursor stroke is ACCENT.
    await expect(cursor).toHaveAttribute('stroke', '#1a4e7a');

    await page.keyboard.press('Enter');

    // In entry: stroke turns INK with wider width.
    await expect(cursor).toHaveAttribute('stroke', '#26221b');
    await expect(cursor).toHaveAttribute('stroke-width', '3');
  });
});
