import { test, expect } from '@playwright/test';
import {
  getCursorX,
  getCursorY,
  installTauriStubs,
  makeState,
  waitForApp,
} from './helpers';

const N = 4;

const THREE_CAGES = [
  { cells: [[0, 0]], op: 'Given', target: 1 },
  { cells: [[1, 1]], op: 'Given', target: 2 },
  { cells: [[3, 3]], op: 'Given', target: 4 },
];

test.describe('Tab and Shift+Tab cycle between cages', () => {
  test('Tab from no active cage jumps cursor to the first cage anchor', async ({ page }) => {
    await installTauriStubs(page, makeState(N, THREE_CAGES));
    await waitForApp(page);

    const startY = await getCursorY(page);
    const startX = await getCursorX(page);
    expect(startX).toBeCloseTo(15.5, 0);
    expect(startY).toBeCloseTo(15.5, 0);

    // Tab — anchor of first cage is (0,0); cursor stays.
    await page.keyboard.press('Tab');
    expect(await getCursorY(page)).toBeCloseTo(startY, 0);

    // Tab again — second cage anchor is (1,1); cursor moves down + right.
    await page.keyboard.press('Tab');
    const y2 = await getCursorY(page);
    const x2 = await getCursorX(page);
    expect(y2).toBeGreaterThan(startY);
    expect(x2).toBeGreaterThan(startX);

    // Tab again — third cage anchor is (3,3); cursor at bottom-right.
    await page.keyboard.press('Tab');
    expect(await getCursorY(page)).toBeGreaterThan(y2);
    expect(await getCursorX(page)).toBeGreaterThan(x2);
  });

  test('Tab wraps from the last cage back to the first', async ({ page }) => {
    await installTauriStubs(page, makeState(N, THREE_CAGES));
    await waitForApp(page);

    // Cycle to the third cage.
    await page.keyboard.press('Tab');
    await page.keyboard.press('Tab');
    await page.keyboard.press('Tab');
    const yLast = await getCursorY(page);

    // One more Tab — should wrap to the first cage at (0,0) ⇒ cursor moves up.
    await page.keyboard.press('Tab');
    expect(await getCursorY(page)).toBeLessThan(yLast);
  });

  test('Shift+Tab from no active cage jumps to the last cage anchor', async ({ page }) => {
    await installTauriStubs(page, makeState(N, THREE_CAGES));
    await waitForApp(page);

    const startY = await getCursorY(page);
    await page.keyboard.press('Shift+Tab');

    // Last cage anchor is (3,3) — cursor must move down significantly.
    expect(await getCursorY(page)).toBeGreaterThan(startY);
  });

  test('Tab is a no-op when there are no cages', async ({ page }) => {
    await installTauriStubs(page, makeState(N));
    await waitForApp(page);

    const startX = await getCursorX(page);
    const startY = await getCursorY(page);

    await page.keyboard.press('Tab');
    expect(await getCursorX(page)).toBe(startX);
    expect(await getCursorY(page)).toBe(startY);
  });
});
