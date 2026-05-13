import { test, expect, Page } from '@playwright/test';
import {
  installTauriStubs,
  makeState,
  waitForApp,
  clickGridCell,
  hasCagedCell,
  setupCageBandWithTuples,
  CAGE_PALETTE_COLORS,
} from './helpers';

const N = 3;

async function countDraftLabels(page: Page): Promise<number> {
  return page.evaluate(() =>
    Array.from(document.querySelectorAll('.grid-svg text'))
      .filter((t) => t.textContent?.trim() === '?').length,
  );
}

test('Shift+Enter on uncovered cell creates singleton cage draft', async ({ page }) => {
  const state = makeState(N);
  await installTauriStubs(page, state);
  await waitForApp(page);
  await clickGridCell(page, N, 1, 1);

  expect(await hasCagedCell(page)).toBe(false);

  await page.keyboard.press('Shift+Enter');
  await page.waitForFunction((palette) => {
    const paletteSet = new Set(palette);
    return Array.from(document.querySelectorAll('.grid-svg rect'))
      .some((r) => paletteSet.has(r.getAttribute('fill') ?? ''));
  }, [...CAGE_PALETTE_COLORS]);

  expect(await hasCagedCell(page)).toBe(true);
});

test('Shift+Enter on uncovered cell shows ? draft label', async ({ page }) => {
  const state = makeState(N);
  await installTauriStubs(page, state);
  await waitForApp(page);
  await clickGridCell(page, N, 0, 0);

  await page.keyboard.press('Shift+Enter');
  await page.waitForFunction(() =>
    Array.from(document.querySelectorAll('.grid-svg text'))
      .some((t) => t.textContent?.trim() === '?'),
  );
});

// Regression: when a cage-band thumbnail has keyboard focus, Shift+Enter was
// intercepted by the band's local handler (which treats Enter as "commit
// selected tuple") and the global splinter dispatch never ran. Shift+Enter
// must always splinter the cursor cell, regardless of whether a thumb has
// focus. We park the grid cursor on an uncovered cell so the splinter
// creates a draft locally without needing a Tauri backend response.
test('Shift+Enter creates singleton draft even when cage band thumb is focused', async ({ page }) => {
  const cages = [{ cells: [[0, 0]], op: 'Given', target: 3 }];
  await setupCageBandWithTuples(page, N, cages, 3);

  // Click the thumb to focus it; cursor is still at (0,0) from setup.
  const firstThumb = page.locator('.cage-band__thumb').first();
  await firstThumb.click();
  await expect(firstThumb).toBeFocused();

  // Move grid cursor to an uncovered cell (0,1) while keeping the thumb
  // focused: ArrowLeft/Right are not band-owned, so they bubble to the
  // global dispatcher and move the grid cursor without blurring the thumb.
  await page.keyboard.press('ArrowRight');
  await expect(firstThumb).toBeFocused();
  expect(await countDraftLabels(page)).toBe(0);

  await page.keyboard.press('Shift+Enter');

  await page.waitForFunction(
    () => Array.from(document.querySelectorAll('.grid-svg text'))
      .some((t) => t.textContent?.trim() === '?'),
    null,
    { timeout: 5000 },
  );

  expect(await countDraftLabels(page)).toBeGreaterThan(0);
});
