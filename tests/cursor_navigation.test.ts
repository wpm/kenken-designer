import { test, expect } from '@playwright/test';
import {
  clickGridCell,
  getCursorX,
  getCursorY,
  installTauriStubs,
  makeState,
  waitForApp,
} from './helpers';

const N = 4;

test.describe('grid cursor visualization and navigation', () => {
  test('cursor rect is visible at (0,0) on initial load', async ({ page }) => {
    await installTauriStubs(page, makeState(N));
    await waitForApp(page);

    const cursor = page.locator('[data-testid="cursor"]');
    await expect(cursor).toBeVisible();

    // The cursor sits inside cell (0,0); both coords should equal the
    // grid margin (14px) plus the inset (1.5px).
    expect(await getCursorX(page)).toBeCloseTo(15.5, 0);
    expect(await getCursorY(page)).toBeCloseTo(15.5, 0);
  });

  test('arrow keys move the cursor in all four directions', async ({ page }) => {
    await installTauriStubs(page, makeState(N));
    await waitForApp(page);

    const startX = await getCursorX(page);
    const startY = await getCursorY(page);

    await page.keyboard.press('ArrowDown');
    expect(await getCursorY(page)).toBeGreaterThan(startY);
    expect(await getCursorX(page)).toBe(startX);

    await page.keyboard.press('ArrowRight');
    expect(await getCursorX(page)).toBeGreaterThan(startX);

    await page.keyboard.press('ArrowUp');
    expect(await getCursorY(page)).toBe(startY);

    await page.keyboard.press('ArrowLeft');
    expect(await getCursorX(page)).toBe(startX);
  });

  test('clicking a grid cell moves the cursor there', async ({ page }) => {
    await installTauriStubs(page, makeState(N));
    await waitForApp(page);

    const startY = await getCursorY(page);
    await clickGridCell(page, N, 2, 1);

    expect(await getCursorY(page)).toBeGreaterThan(startY);
  });

  test('arrow keys clamp at grid edges (no off-grid movement)', async ({ page }) => {
    await installTauriStubs(page, makeState(N));
    await waitForApp(page);

    // Already at top-left; pressing Up/Left should be a no-op.
    const startX = await getCursorX(page);
    const startY = await getCursorY(page);
    await page.keyboard.press('ArrowUp');
    await page.keyboard.press('ArrowLeft');
    expect(await getCursorX(page)).toBe(startX);
    expect(await getCursorY(page)).toBe(startY);

    // Press Down N-1 times then once more — last press is a no-op.
    for (let i = 0; i < N - 1; i++) await page.keyboard.press('ArrowDown');
    const afterDown = await getCursorY(page);
    await page.keyboard.press('ArrowDown');
    expect(await getCursorY(page)).toBe(afterDown);
  });
});
