import { test, expect, Page } from '@playwright/test';
import { clickGridCell, setupCageBandWithTuples, getCursorY } from './helpers';

const N = 3;
const ONE_CAGE = [{ cells: [[0, 0]], op: 'Given', target: 3 }];

const clickCell = (page: Page, row: number, col: number) =>
  clickGridCell(page, N, row, col);

test('ArrowUp works within band after pressing down to last thumbnail', async ({ page }) => {
  await setupCageBandWithTuples(page, N, ONE_CAGE, 3);

  const firstThumb = page.locator('.cage-band__thumb').first();
  await firstThumb.click();
  await expect(firstThumb).toBeFocused();

  await page.keyboard.press('ArrowDown');
  await page.waitForTimeout(400);
  await page.keyboard.press('ArrowDown');
  await page.waitForTimeout(400);
  await page.keyboard.press('ArrowDown'); // no-op at bottom

  const initialY = await getCursorY(page);
  await page.keyboard.press('ArrowUp');
  await page.waitForTimeout(400);

  expect(await getCursorY(page)).toBe(initialY);

  const active = await page.evaluate(() => document.activeElement?.className ?? null);
  expect(active).toContain('cage-band__thumb');
});

test('ArrowDown moves grid cursor after Escape from bottom of band', async ({ page }) => {
  await setupCageBandWithTuples(page, N, ONE_CAGE, 3);

  const firstThumb = page.locator('.cage-band__thumb').first();
  await firstThumb.click();
  await expect(firstThumb).toBeFocused();

  await page.keyboard.press('ArrowDown');
  await page.waitForTimeout(400);
  await page.keyboard.press('ArrowDown');
  await page.waitForTimeout(400);
  await page.keyboard.press('ArrowDown'); // no-op at bottom

  await page.keyboard.press('Escape');

  const cursorBefore = await getCursorY(page);
  await page.keyboard.press('ArrowDown');
  expect(await getCursorY(page)).toBeGreaterThan(cursorBefore);
});
