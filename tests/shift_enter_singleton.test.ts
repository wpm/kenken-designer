import { test, expect } from '@playwright/test';
import { installTauriStubs, makeState, waitForApp, clickGridCell } from './helpers';

const N = 3;

// Returns true if any SVG rect in the grid has a palette fill (indicating a caged cell).
async function hasCagedCell(page: any): Promise<boolean> {
  return page.evaluate(() => {
    // Palette colors from src/theme.rs CAGE_PALETTE.
    const palette = new Set([
      '#cfe4f2', '#d7ecd5', '#f7ecc6', '#f6d9d3',
      '#e4d9ee', '#f4dec3', '#d6ece7', '#eed5e1',
    ]);
    const rects = Array.from(document.querySelectorAll('.grid-svg rect'));
    return rects.some((r) => palette.has(r.getAttribute('fill') ?? ''));
  });
}

test('Shift+Enter on uncovered cell creates singleton cage draft', async ({ page }) => {
  const state = makeState(N);
  await installTauriStubs(page, state);
  await waitForApp(page);

  // Click cell (1,1) to focus the grid and move the cursor there.
  await clickGridCell(page, N, 1, 1);

  // Verify no cells are caged yet.
  expect(await hasCagedCell(page)).toBe(false);

  // Press Shift+Enter to create a singleton draft.
  await page.keyboard.press('Shift+Enter');
  await page.waitForTimeout(50);

  // After Shift+Enter the draft cage should color the cell with a palette fill.
  expect(await hasCagedCell(page)).toBe(true);
});

test('Shift+Enter on uncovered cell shows ? draft label', async ({ page }) => {
  const state = makeState(N);
  await installTauriStubs(page, state);
  await waitForApp(page);

  await clickGridCell(page, N, 0, 0);
  await page.keyboard.press('Shift+Enter');
  await page.waitForTimeout(50);

  // A singleton draft cage renders "?" as its op label.
  const hasQuestionMark = await page.evaluate(() => {
    const texts = Array.from(document.querySelectorAll('.grid-svg text'));
    return texts.some((t) => t.textContent?.trim() === '?');
  });
  expect(hasQuestionMark).toBe(true);
});
