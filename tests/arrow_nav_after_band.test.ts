import { test, expect, Page } from '@playwright/test';
import { installTauriStubs, waitForApp } from './helpers';

const N = 3;

function makePuzzleView() {
  const cells = Array.from({ length: N }, () =>
    Array.from({ length: N }, () => Array.from({ length: N }, (_, i) => i + 1)),
  );
  const cages = [{ cells: [[0, 0]], op: 'Given', target: 3 }];
  return { n: N, cells, cages, diff: { changes: [] } };
}

async function setupBandWithTuples(page: Page, tupleCount: number) {
  const view = makePuzzleView();
  await installTauriStubs(page, view);

  await page.addInitScript(
    ({ count, puzzleView }: { count: number; puzzleView: any }) => {
      const tuples = Array.from({ length: count }, (_, i) => ({
        tuple: [i + 1],
        view: puzzleView,
        total_reduction: 0,
        newly_singleton: 0,
      }));
      (window as any).__tauri_invoke_handlers__ = {
        ...((window as any).__tauri_invoke_handlers__ ?? {}),
        rank_active_cage: () => tuples,
      };
    },
    { count: tupleCount, puzzleView: view },
  );

  await waitForApp(page);
  await clickCell(page, 0, 0);
  await page.waitForSelector('.cage-band__thumb', { timeout: 8000 });
}

async function clickCell(page: Page, row: number, col: number) {
  const svg = page.locator('.grid-svg');
  const box = await svg.boundingBox();
  if (!box) throw new Error('grid-svg not found');
  const cellSize = box.width / N;
  await page.mouse.click(
    box.x + cellSize * (col + 0.5),
    box.y + cellSize * (row + 0.5),
  );
}

// Read the cursor rect's Y attribute from the SVG. The cursor rect carries
// data-testid="cursor" so it can be targeted unambiguously.
async function getCursorY(page: Page): Promise<number> {
  return page.evaluate(() => {
    const el = document.querySelector('[data-testid="cursor"]');
    if (!el) return -1;
    return parseFloat(el.getAttribute('y') ?? '-1');
  });
}

// A setup that also returns a scroll-band with 6 items so scroll buttons are enabled.
async function setupScrollableBand(page: Page) {
  return setupBandWithTuples(page, 6);
}

test.describe('arrow nav after cage band interaction', () => {
  test('ArrowDown moves grid cursor after Escape from cage band', async ({ page }) => {
    await setupBandWithTuples(page, 3);

    // Record initial cursor Y (at row 0).
    const initialY = await getCursorY(page);
    expect(initialY).toBeGreaterThanOrEqual(0);

    // Click the first thumbnail to give the cage band focus.
    const firstThumb = page.locator('.cage-band__thumb').first();
    await firstThumb.click();
    await expect(firstThumb).toBeFocused();

    // Escape returns focus to the grid.
    await page.keyboard.press('Escape');
    await expect(firstThumb).not.toBeFocused();

    // ArrowDown must move the grid cursor to row 1.
    await page.keyboard.press('ArrowDown');

    const newY = await getCursorY(page);
    expect(newY).toBeGreaterThan(initialY);
  });

  test('ArrowUp moves grid cursor after Escape from cage band', async ({ page }) => {
    await setupBandWithTuples(page, 3);

    // Move cursor to row 1 first by clicking there.
    await clickCell(page, 1, 0);

    // Re-activate cage band by clicking the caged cell.
    await clickCell(page, 0, 0);
    await page.waitForSelector('.cage-band__thumb', { timeout: 8000 });

    const initialY = await getCursorY(page);

    // Focus a thumb and Escape.
    const firstThumb = page.locator('.cage-band__thumb').first();
    await firstThumb.click();
    await expect(firstThumb).toBeFocused();
    await page.keyboard.press('Escape');
    await expect(firstThumb).not.toBeFocused();

    // Move down first so ArrowUp has somewhere to go.
    await page.keyboard.press('ArrowDown');
    const afterDownY = await getCursorY(page);
    expect(afterDownY).toBeGreaterThan(initialY);

    // ArrowUp should move cursor back up.
    await page.keyboard.press('ArrowUp');
    const afterUpY = await getCursorY(page);
    expect(afterUpY).toBeLessThan(afterDownY);
  });

  test('ArrowDown moves grid cursor after keyboard navigation within cage band', async ({ page }) => {
    await setupBandWithTuples(page, 3);

    const initialY = await getCursorY(page);
    expect(initialY).toBeGreaterThanOrEqual(0);

    // Focus the first thumb via click.
    const firstThumb = page.locator('.cage-band__thumb').first();
    await firstThumb.click();
    await expect(firstThumb).toBeFocused();

    // Navigate down within the band (if more than 1 thumb visible).
    await page.keyboard.press('ArrowDown');

    // Escape back to the grid.
    await page.keyboard.press('Escape');

    // ArrowDown should now move the grid cursor.
    await page.keyboard.press('ArrowDown');

    const newY = await getCursorY(page);
    expect(newY).toBeGreaterThan(initialY);
  });

  test('ArrowDown works after clicking away from focused cage band thumb', async ({ page }) => {
    await setupBandWithTuples(page, 3);

    const initialY = await getCursorY(page);

    // Focus a thumb.
    const firstThumb = page.locator('.cage-band__thumb').first();
    await firstThumb.click();
    await expect(firstThumb).toBeFocused();

    // Click the main grid SVG to move focus back to the grid context.
    await clickCell(page, 0, 0);

    // ArrowDown must move the cursor.
    await page.keyboard.press('ArrowDown');
    const newY = await getCursorY(page);
    expect(newY).toBeGreaterThan(initialY);
  });

  test('ArrowDown works after using scroll-down button then clicking grid', async ({ page }) => {
    await setupScrollableBand(page);

    const initialY = await getCursorY(page);

    // Focus a thumb to establish band context.
    const firstThumb = page.locator('.cage-band__thumb').first();
    await firstThumb.click();
    await expect(firstThumb).toBeFocused();

    // Click the scroll-down button — this moves focus away from the thumb.
    const downBtn = page.locator('.cage-band__arrow[aria-label="Scroll down"]');
    await downBtn.click();
    await page.waitForTimeout(400); // wait for animation

    // Click the grid to return focus to the grid area.
    await clickCell(page, 0, 0);

    // Arrow keys should move the cursor.
    await page.keyboard.press('ArrowDown');
    const newY = await getCursorY(page);
    expect(newY).toBeGreaterThan(initialY);
  });

  test('ArrowDown works after using only scroll buttons (no thumb click)', async ({ page }) => {
    await setupScrollableBand(page);

    const initialY = await getCursorY(page);

    // Use only the scroll button, never clicking a thumb.
    const downBtn = page.locator('.cage-band__arrow[aria-label="Scroll down"]');
    await downBtn.click();
    await page.waitForTimeout(400);

    // Click the grid to give grid context.
    await clickCell(page, 0, 0);

    // Arrow keys should move cursor.
    await page.keyboard.press('ArrowDown');
    const newY = await getCursorY(page);
    expect(newY).toBeGreaterThan(initialY);
  });

  test('ArrowDown works after Tab-navigation through scroll buttons', async ({ page }) => {
    await setupScrollableBand(page);

    const initialY = await getCursorY(page);

    // Tab into the cage band scroll-up button (first focusable in band).
    await page.keyboard.press('Tab');

    // Tab again to scroll-down button or thumbs.
    await page.keyboard.press('Tab');

    // Click grid to return.
    await clickCell(page, 0, 0);

    await page.keyboard.press('ArrowDown');
    const newY = await getCursorY(page);
    expect(newY).toBeGreaterThan(initialY);
  });
});
