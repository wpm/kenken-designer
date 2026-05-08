import { test, expect, Page } from '@playwright/test';
import { clickGridCell, setupCageBandWithTuples } from './helpers';

const N = 3;
const ONE_CAGE = [{ cells: [[0, 0]], op: 'Given', target: 3 }];

const clickCell = (page: Page, row: number, col: number) =>
  clickGridCell(page, N, row, col);

// Read the Y attribute of the SVG cursor rect. data-testid="cursor" is set on
// the element in grid.rs so it can be targeted unambiguously.
async function getCursorY(page: Page): Promise<number> {
  return page.evaluate(() => {
    const el = document.querySelector('[data-testid="cursor"]');
    if (!el) throw new Error('cursor element not found');
    return parseFloat(el.getAttribute('y') ?? 'NaN');
  });
}

test.describe('arrow nav after cage band interaction', () => {
  test('ArrowDown moves grid cursor after Escape from cage band', async ({ page }) => {
    await setupCageBandWithTuples(page, N, ONE_CAGE, 3);

    const initialY = await getCursorY(page);

    const firstThumb = page.locator('.cage-band__thumb').first();
    await firstThumb.click();
    await expect(firstThumb).toBeFocused();

    await page.keyboard.press('Escape');
    await expect(firstThumb).not.toBeFocused();

    await page.keyboard.press('ArrowDown');

    expect(await getCursorY(page)).toBeGreaterThan(initialY);
  });

  test('ArrowUp moves grid cursor after Escape from cage band', async ({ page }) => {
    await setupCageBandWithTuples(page, N, ONE_CAGE, 3);

    // Start cursor at row 1, then re-activate the band.
    await clickCell(page, 1, 0);
    await clickCell(page, 0, 0);
    await page.waitForSelector('.cage-band__thumb', { timeout: 8000 });

    const initialY = await getCursorY(page);

    const firstThumb = page.locator('.cage-band__thumb').first();
    await firstThumb.click();
    await expect(firstThumb).toBeFocused();
    await page.keyboard.press('Escape');
    await expect(firstThumb).not.toBeFocused();

    // Move down so ArrowUp has somewhere to go.
    await page.keyboard.press('ArrowDown');
    expect(await getCursorY(page)).toBeGreaterThan(initialY);

    await page.keyboard.press('ArrowUp');
    expect(await getCursorY(page)).toBe(initialY);
  });

  test('ArrowDown moves grid cursor after keyboard navigation within cage band', async ({ page }) => {
    await setupCageBandWithTuples(page, N, ONE_CAGE, 3);

    const initialY = await getCursorY(page);

    const firstThumb = page.locator('.cage-band__thumb').first();
    await firstThumb.click();
    await expect(firstThumb).toBeFocused();

    // Navigate within the band, then escape.
    await page.keyboard.press('ArrowDown');
    await page.keyboard.press('Escape');

    await page.keyboard.press('ArrowDown');
    expect(await getCursorY(page)).toBeGreaterThan(initialY);
  });

  test('ArrowDown works after clicking away from focused cage band thumb', async ({ page }) => {
    await setupCageBandWithTuples(page, N, ONE_CAGE, 3);

    const initialY = await getCursorY(page);

    const firstThumb = page.locator('.cage-band__thumb').first();
    await firstThumb.click();
    await expect(firstThumb).toBeFocused();

    await clickCell(page, 0, 0);

    await page.keyboard.press('ArrowDown');
    expect(await getCursorY(page)).toBeGreaterThan(initialY);
  });

  test('ArrowDown works after using scroll-down button then clicking grid', async ({ page }) => {
    await setupCageBandWithTuples(page, N, ONE_CAGE, 6);

    const initialY = await getCursorY(page);

    const firstThumb = page.locator('.cage-band__thumb').first();
    await firstThumb.click();
    await expect(firstThumb).toBeFocused();

    const downBtn = page.locator('.cage-band__arrow[aria-label="Scroll down"]');
    await downBtn.click();
    await page.waitForTimeout(400);

    await clickCell(page, 0, 0);

    await page.keyboard.press('ArrowDown');
    expect(await getCursorY(page)).toBeGreaterThan(initialY);
  });

  test('ArrowDown works after using only scroll buttons (no thumb click)', async ({ page }) => {
    await setupCageBandWithTuples(page, N, ONE_CAGE, 6);

    const initialY = await getCursorY(page);

    const downBtn = page.locator('.cage-band__arrow[aria-label="Scroll down"]');
    await downBtn.click();
    await page.waitForTimeout(400);

    await clickCell(page, 0, 0);

    await page.keyboard.press('ArrowDown');
    expect(await getCursorY(page)).toBeGreaterThan(initialY);
  });

  test('ArrowDown works after Tab-navigation through scroll buttons', async ({ page }) => {
    await setupCageBandWithTuples(page, N, ONE_CAGE, 6);

    const initialY = await getCursorY(page);

    await page.keyboard.press('Tab');
    await page.keyboard.press('Tab');

    await clickCell(page, 0, 0);

    await page.keyboard.press('ArrowDown');
    expect(await getCursorY(page)).toBeGreaterThan(initialY);
  });
});
