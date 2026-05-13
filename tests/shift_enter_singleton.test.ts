import { test, expect, Page } from '@playwright/test';
import { installTauriStubs, makeState, waitForApp, clickGridCell, hasCagedCell, CAGE_PALETTE_COLORS } from './helpers';

const N = 3;

test('Shift+Enter on uncovered cell creates singleton cage draft', async ({ page }: { page: Page }) => {
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

test('Shift+Enter on uncovered cell shows ? draft label', async ({ page }: { page: Page }) => {
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
