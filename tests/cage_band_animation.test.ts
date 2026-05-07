import { test, expect, Page } from '@playwright/test';
import { installTauriStubs, waitForApp } from './helpers';

const N = 3;

// A 3×3 PuzzleView with one single-cell cage at (0,0).
function makePuzzleView() {
  const cells = Array.from({ length: N }, () =>
    Array.from({ length: N }, () => Array.from({ length: N }, (_, i) => i + 1)),
  );
  const cages = [{ cells: [[0, 0]], op: 'Given', target: 3 }];
  return { n: N, cells, cages, diff: { changes: [] } };
}

// Install Tauri stubs, set up `rank_active_cage` to return `tupleCount` tuples,
// navigate to the page, and click the caged cell to activate the cage band.
async function setupScrollableBand(page: Page, tupleCount: number) {
  const view = makePuzzleView();
  await installTauriStubs(page, view);

  // Override rank_active_cage to return a list of ranked tuples.
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

  // Click the first cell of the cage (top-left corner of the grid SVG).
  // The SVG fills the left portion of the grid-and-band container.
  const svg = page.locator('.grid-svg');
  const box = await svg.boundingBox();
  if (!box) throw new Error('grid-svg not found');
  // Click near top-left cell center.
  const cellSize = box.width / N;
  await page.mouse.click(
    box.x + cellSize * 0.5,
    box.y + cellSize * 0.5,
  );

  // Wait for thumbnails to appear in the cage band.
  await page.waitForSelector('.cage-band__thumb', { timeout: 8000 });
}

test.describe('cage band scroll animation', () => {
  test('scroll-down arrow triggers CSS transform animation', async ({ page }) => {
    await setupScrollableBand(page, 6);

    const inner = page.locator('.cage-band__strip-inner');
    await expect(inner).toBeVisible();

    // Initially transform is at 0.
    const initialTransform = await inner.evaluate(
      (el) => (el as HTMLElement).style.transform,
    );
    expect(initialTransform).toBe('translateY(0px)');

    // Click scroll-down.
    const downBtn = page.locator('.cage-band__arrow[aria-label="Scroll down"]');
    await downBtn.click();

    // Poll until we observe a non-zero transform (animation in flight).
    let sawAnimation = false;
    for (let i = 0; i < 30; i++) {
      const transform = await inner.evaluate(
        (el) => (el as HTMLElement).style.transform,
      );
      if (transform !== 'translateY(0px)') {
        sawAnimation = true;
        break;
      }
      await page.waitForTimeout(10);
    }
    expect(sawAnimation).toBe(true);

    // After animation completes (200ms + generous buffer), transform is back to 0.
    await page.waitForTimeout(400);
    const finalTransform = await inner.evaluate(
      (el) => (el as HTMLElement).style.transform,
    );
    expect(finalTransform).toBe('translateY(0px)');
  });

  test('scroll-up arrow triggers CSS transform animation', async ({ page }) => {
    await setupScrollableBand(page, 6);

    const inner = page.locator('.cage-band__strip-inner');
    const downBtn = page.locator('.cage-band__arrow[aria-label="Scroll down"]');
    const upBtn = page.locator('.cage-band__arrow[aria-label="Scroll up"]');

    // Scroll down once to make scroll-up available.
    await downBtn.click();
    await page.waitForTimeout(400);

    // Now scroll up and observe animation via computed style (catches CSS transitions).
    await upBtn.click();

    // Poll computed transform — during the CSS transition this will be a matrix
    // with a non-zero translation, not "none" or "matrix(1, 0, 0, 1, 0, 0)".
    let sawAnimation = false;
    for (let i = 0; i < 40; i++) {
      const computedTransform = await inner.evaluate(
        (el) => window.getComputedStyle(el).transform,
      );
      // Identity matrix means translateY(0) — anything else means mid-animation.
      if (
        computedTransform !== 'none' &&
        computedTransform !== 'matrix(1, 0, 0, 1, 0, 0)'
      ) {
        sawAnimation = true;
        break;
      }
      await page.waitForTimeout(8);
    }
    expect(sawAnimation).toBe(true);

    await page.waitForTimeout(400);
    const finalTransform = await inner.evaluate(
      (el) => (el as HTMLElement).style.transform,
    );
    expect(finalTransform).toBe('translateY(0px)');
  });

  test('no-transition class is absent in steady state', async ({ page }) => {
    await setupScrollableBand(page, 6);
    const inner = page.locator('.cage-band__strip-inner');
    await expect(inner).not.toHaveClass(/cage-band__strip--no-transition/);
  });

  test('no-transition class is absent after scroll-down animation completes', async ({ page }) => {
    await setupScrollableBand(page, 6);
    const downBtn = page.locator('.cage-band__arrow[aria-label="Scroll down"]');
    await downBtn.click();
    await page.waitForTimeout(400);
    const inner = page.locator('.cage-band__strip-inner');
    await expect(inner).not.toHaveClass(/cage-band__strip--no-transition/);
  });
});
