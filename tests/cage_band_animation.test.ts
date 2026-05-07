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

// A 3×3 PuzzleView with two single-cell cages: (0,0) and (0,2).
function makeTwoCagePuzzleView() {
  const cells = Array.from({ length: N }, () =>
    Array.from({ length: N }, () => Array.from({ length: N }, (_, i) => i + 1)),
  );
  const cages = [
    { cells: [[0, 0]], op: 'Given', target: 3 },
    { cells: [[0, 2]], op: 'Given', target: 2 },
  ];
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

  await clickCell(page, 0, 0);

  // Wait for thumbnails to appear in the cage band.
  await page.waitForSelector('.cage-band__thumb', { timeout: 8000 });
}

// Click the grid cell at (row, col) by computing its position from the SVG bounds.
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

  test('render_extra: one extra thumb rendered during animation, gone after', async ({ page }) => {
    await setupScrollableBand(page, 6);

    // With a tall enough viewport, visible_count should be at least 1.
    // Count thumbs at rest (should equal visible_count, not +1).
    const restCount = await page.locator('.cage-band__thumb').count();
    expect(restCount).toBeGreaterThanOrEqual(1);

    const downBtn = page.locator('.cage-band__arrow[aria-label="Scroll down"]');
    await downBtn.click();

    // During the animation there should be one extra thumb rendered.
    let sawExtra = false;
    for (let i = 0; i < 30; i++) {
      const count = await page.locator('.cage-band__thumb').count();
      if (count > restCount) {
        sawExtra = true;
        break;
      }
      await page.waitForTimeout(10);
    }
    expect(sawExtra).toBe(true);

    // After animation completes the count returns to the resting value.
    await page.waitForTimeout(400);
    const finalCount = await page.locator('.cage-band__thumb').count();
    expect(finalCount).toBe(restCount);
  });

  test('anchor change mid-animation: strip is clean after new cage loads', async ({ page }) => {
    // Use a puzzle with two cages so we can switch between them.
    const view = makeTwoCagePuzzleView();
    await installTauriStubs(page, view);

    await page.addInitScript(
      ({ puzzleView }: { puzzleView: any }) => {
        // Return 6 tuples for any cage — enough to scroll.
        const tuples = Array.from({ length: 6 }, (_, i) => ({
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
      { puzzleView: view },
    );

    await waitForApp(page);
    await clickCell(page, 0, 0);
    await page.waitForSelector('.cage-band__thumb', { timeout: 8000 });

    // Start a scroll-down animation.
    const downBtn = page.locator('.cage-band__arrow[aria-label="Scroll down"]');
    await downBtn.click();

    // Mid-animation: switch to the second cage (col 2) to cancel in-flight callbacks.
    await clickCell(page, 0, 2);

    // Wait well past the original animation timeout.
    await page.waitForTimeout(400);

    // Strip should be clean: transform at 0, no-transition class absent.
    const inner = page.locator('.cage-band__strip-inner');
    const transform = await inner.evaluate(
      (el) => (el as HTMLElement).style.transform,
    );
    expect(transform).toBe('translateY(0px)');
    await expect(inner).not.toHaveClass(/cage-band__strip--no-transition/);
  });

  test('prefers-reduced-motion: scroll buttons re-enable without 200ms lockout', async ({
    page,
  }) => {
    // Set up stubs and navigate first, then inject the 0ms override before
    // activating the cage so scroll_anim_ms() reads 0 when animate_scroll runs.
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
      { count: 6, puzzleView: view },
    );

    await waitForApp(page);

    // Inject 0ms override now that the document exists.
    await page.addStyleTag({
      content: ':root { --scroll-anim-duration: 0ms !important; }',
    });

    await clickCell(page, 0, 0);
    await page.waitForSelector('.cage-band__thumb', { timeout: 8000 });

    const downBtn = page.locator('.cage-band__arrow[aria-label="Scroll down"]');
    const upBtn = page.locator('.cage-band__arrow[aria-label="Scroll up"]');

    await downBtn.click();

    // With 0ms animation, the scroll-up button should become enabled well
    // within 150ms — not locked out for the 200ms nominal duration.
    await expect(upBtn).toBeEnabled({ timeout: 150 });
  });
});
