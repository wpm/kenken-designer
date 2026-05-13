import { test, expect } from '@playwright/test';
import {
  CAGE_PALETTE_COLORS,
  addInvokeHandler,
  clickGridCell,
  hasCagedCell,
  installTauriStubs,
  makeState,
  waitForApp,
} from './helpers';

const N = 3;

test.describe('shift+arrow draft creation', () => {
  test('Shift+Right on uncovered cell creates a 2-cell draft', async ({ page }) => {
    await installTauriStubs(page, makeState(N));
    await waitForApp(page);

    await clickGridCell(page, N, 0, 0);
    expect(await hasCagedCell(page)).toBe(false);

    await page.keyboard.press('Shift+ArrowRight');

    await page.waitForFunction((palette) => {
      const set = new Set(palette);
      return Array.from(document.querySelectorAll('.grid-svg rect'))
        .some((r) => set.has(r.getAttribute('fill') ?? ''));
    }, [...CAGE_PALETTE_COLORS]);

    expect(await hasCagedCell(page)).toBe(true);
    // A 2-cell draft has a "?" label (multiple valid op glyphs aren't shown for n=2 doubt — actually
    // for 2-cell drafts the glyphs are "+ − × ÷"; check that the draft label is non-empty).
    const labels = await page.locator('.grid-svg text').allTextContents();
    expect(labels.some((t) => t.includes('+') || t.includes('?'))).toBe(true);
  });

  test('Shift+Right then Shift+Right grows the draft to 3 cells', async ({ page }) => {
    await installTauriStubs(page, makeState(N));
    await waitForApp(page);

    await clickGridCell(page, N, 0, 0);
    await page.keyboard.press('Shift+ArrowRight');
    await page.keyboard.press('Shift+ArrowRight');

    // Count caged cells: a draft of 3 should color 3 grid cells.
    const cagedCount = await page.evaluate((palette) => {
      const set = new Set(palette);
      return Array.from(document.querySelectorAll('.grid-svg rect'))
        .filter((r) => set.has(r.getAttribute('fill') ?? '')).length;
    }, [...CAGE_PALETTE_COLORS]);
    expect(cagedCount).toBe(3);
  });

  test('Escape on a draft cell removes that cell from the draft', async ({ page }) => {
    await installTauriStubs(page, makeState(N));
    await waitForApp(page);

    await clickGridCell(page, N, 0, 0);
    await page.keyboard.press('Shift+ArrowRight');
    await page.keyboard.press('Shift+ArrowRight'); // 3-cell draft

    // Escape on current cursor (last appended cell) shrinks back to 2.
    await page.keyboard.press('Escape');

    const cagedCount = await page.evaluate((palette) => {
      const set = new Set(palette);
      return Array.from(document.querySelectorAll('.grid-svg rect'))
        .filter((r) => set.has(r.getAttribute('fill') ?? '')).length;
    }, [...CAGE_PALETTE_COLORS]);
    expect(cagedCount).toBe(2);
  });

  test('Delete on a draft cell clears the entire draft', async ({ page }) => {
    await installTauriStubs(page, makeState(N));
    await waitForApp(page);

    await clickGridCell(page, N, 0, 0);
    await page.keyboard.press('Shift+ArrowRight');
    expect(await hasCagedCell(page)).toBe(true);

    await page.keyboard.press('Delete');
    await page.waitForFunction((palette) => {
      const set = new Set(palette);
      return !Array.from(document.querySelectorAll('.grid-svg rect'))
        .some((r) => set.has(r.getAttribute('fill') ?? ''));
    }, [...CAGE_PALETTE_COLORS]);
    expect(await hasCagedCell(page)).toBe(false);
  });
});

test.describe('draft commit via Enter', () => {
  test('Enter on a draft opens picker and commits via insert_cage', async ({ page }) => {
    await installTauriStubs(page, makeState(N));
    // Singleton draft → cage_options returns Given only.
    await addInvokeHandler(
      page,
      'cage_options',
      `return [{ op: 'Given', targets: [1, 2, 3] }];`,
    );
    await addInvokeHandler(
      page,
      'insert_cage',
      `
      window.__insert_args__ = args;
      const n = currentState.n;
      const cells = Array.from({ length: n }, () =>
        Array.from({ length: n }, () =>
          Array.from({ length: n }, (_, i) => i + 1)
        )
      );
      const next = {
        n,
        cells,
        cages: [{ cells: args.cells, op: args.op, target: args.target }],
        diff: { changes: [] },
      };
      window.__setTauriState__(next);
      return next;
      `,
    );
    await waitForApp(page);

    await clickGridCell(page, N, 1, 1);
    await page.keyboard.press('Shift+Enter'); // creates singleton draft
    await page.keyboard.press('Enter'); // opens picker
    // Singleton picker is in TargetPicker — Enter commits target 1 (default).
    await page.keyboard.press('Enter');

    await expect.poll(() =>
      page.evaluate(() => (window as any).__insert_args__),
    ).toBeTruthy();

    const args = await page.evaluate(() => (window as any).__insert_args__);
    expect(args.op).toBe('Given');
    expect(args.target).toBe(1);
    expect(args.cells).toEqual([[1, 1]]);
  });
});
