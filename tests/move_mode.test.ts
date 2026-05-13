import { test, expect } from '@playwright/test';
import {
  addInvokeHandler,
  clickGridCell,
  installTauriStubs,
  makeState,
  waitForApp,
} from './helpers';

const N = 3;

const TWO_ADJACENT = [
  { cells: [[0, 0], [0, 1]], op: 'Add', target: 3 },
  { cells: [[1, 0], [1, 1]], op: 'Add', target: 5 },
];

test.describe('move mode', () => {
  test('pressing M on a movable cell shows source overlay and target borders', async ({ page }) => {
    await installTauriStubs(page, makeState(N, TWO_ADJACENT));
    await waitForApp(page);

    await clickGridCell(page, N, 0, 0);
    await page.keyboard.press('m');

    // Source overlay: a white rect with 0.5 fill-opacity inside .grid-svg.
    const sourceOverlay = page.locator(
      '.grid-svg rect[fill="white"][fill-opacity="0.5"]',
    );
    await expect(sourceOverlay).toHaveCount(1);

    // Target cage cells should be outlined (no dash since none selected yet).
    const targetBorders = page.locator(
      '.grid-svg rect[stroke="#1a4e7a"][stroke-width="1.0"]',
    );
    await expect(targetBorders).toHaveCount(2); // both cells of target cage
  });

  test('Tab in move mode selects a target cage (dashed border)', async ({ page }) => {
    await installTauriStubs(page, makeState(N, TWO_ADJACENT));
    await waitForApp(page);

    await clickGridCell(page, N, 0, 0);
    await page.keyboard.press('m');
    await page.keyboard.press('Tab');

    // After Tab, the selected target gets stroke-width="2.0" + dash.
    const selectedBorders = page.locator(
      '.grid-svg rect[stroke="#1a4e7a"][stroke-width="2.0"]',
    );
    await expect(selectedBorders).toHaveCount(2);
    await expect(selectedBorders.first()).toHaveAttribute('stroke-dasharray', '4,3');
  });

  test('Escape exits move mode without invoking move_cell', async ({ page }) => {
    await installTauriStubs(page, makeState(N, TWO_ADJACENT));
    await addInvokeHandler(
      page,
      'move_cell',
      `window.__move_called__ = (window.__move_called__ ?? 0) + 1;
       return { view: currentState, drafts: [] };`,
    );
    await waitForApp(page);

    await clickGridCell(page, N, 0, 0);
    await page.keyboard.press('m');

    const sourceOverlay = page.locator(
      '.grid-svg rect[fill="white"][fill-opacity="0.5"]',
    );
    await expect(sourceOverlay).toHaveCount(1);

    await page.keyboard.press('Escape');
    await expect(sourceOverlay).toHaveCount(0);

    expect(await page.evaluate(() => (window as any).__move_called__ ?? 0)).toBe(0);
  });

  test('Tab + Enter in move mode invokes move_cell with the chosen target', async ({ page }) => {
    await installTauriStubs(page, makeState(N, TWO_ADJACENT));
    await addInvokeHandler(
      page,
      'move_cell',
      `
      window.__move_args__ = args;
      return { view: currentState, drafts: [] };
      `,
    );
    await waitForApp(page);

    await clickGridCell(page, N, 0, 0);
    await page.keyboard.press('m');
    await page.keyboard.press('Tab');
    await page.keyboard.press('Enter');

    await expect.poll(() =>
      page.evaluate(() => (window as any).__move_args__),
    ).toBeTruthy();

    const args = await page.evaluate(() => (window as any).__move_args__);
    expect(args.cell).toEqual([0, 0]);
    // Only one possible target (the other cage anchored at (1,0)).
    expect(args.targetAnchor).toEqual([1, 0]);
  });

  test('right-click "Move cell" menu item also enters move mode', async ({ page }) => {
    await installTauriStubs(page, makeState(N, TWO_ADJACENT));
    await waitForApp(page);

    // Right-click on (0,0)
    const svg = page.locator('.grid-svg');
    const box = await svg.boundingBox();
    if (!box) throw new Error('grid-svg not found');
    const cellSize = box.width / N;
    await page.mouse.click(
      box.x + cellSize * 0.5,
      box.y + cellSize * 0.5,
      { button: 'right' },
    );

    await page.getByText('Move cell').click();

    const sourceOverlay = page.locator(
      '.grid-svg rect[fill="white"][fill-opacity="0.5"]',
    );
    await expect(sourceOverlay).toHaveCount(1);
  });
});
