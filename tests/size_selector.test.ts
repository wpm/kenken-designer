import { test, expect } from '@playwright/test';
import {
  addInvokeHandler,
  installTauriStubs,
  makeState,
  waitForApp,
} from './helpers';

test.describe('size selector', () => {
  test('size dropdown defaults to the puzzle\'s n', async ({ page }) => {
    await installTauriStubs(page, makeState(5));
    await waitForApp(page);

    const select = page.locator('.size-control select');
    await expect(select).toBeVisible();
    await expect(select).toHaveValue('5');
  });

  test('changing size invokes new_puzzle with the chosen n', async ({ page }) => {
    await installTauriStubs(page, makeState(4));
    await addInvokeHandler(page, 'new_puzzle', (args) => {
      (window as any).__new_puzzle_n__ = args.n;
      const n: number = args.n;
      const cells = Array.from({ length: n }, () =>
        Array.from({ length: n }, () =>
          Array.from({ length: n }, (_, i) => i + 1),
        ),
      );
      const next = { n, cells, cages: [], diff: { changes: [] } };
      (window as any).__setTauriState__(next);
      return next;
    });
    await waitForApp(page);

    await page.locator('.size-control select').selectOption('6');

    await expect.poll(() =>
      page.evaluate(() => (window as any).__new_puzzle_n__),
    ).toBe(6);

    // The dropdown reflects the new value.
    await expect(page.locator('.size-control select')).toHaveValue('6');
  });

  test('all sizes 2-9 are available in the dropdown', async ({ page }) => {
    await installTauriStubs(page, makeState(4));
    await waitForApp(page);

    const options = await page.locator('.size-control select option').allTextContents();
    expect(options).toEqual(['2', '3', '4', '5', '6', '7', '8', '9']);
  });
});
