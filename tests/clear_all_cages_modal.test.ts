import { test, expect } from '@playwright/test';
import {
  addInvokeHandler,
  installTauriStubs,
  makeState,
  waitForApp,
} from './helpers';

const N = 3;

const TWO_CAGES = [
  { cells: [[0, 0]], op: 'Given', target: 1 },
  { cells: [[0, 1]], op: 'Given', target: 2 },
];

test.describe('clear all cages modal', () => {
  test('Cmd/Ctrl+Shift+Delete opens the modal with cage count', async ({ page }) => {
    await installTauriStubs(page, makeState(N, TWO_CAGES));
    await waitForApp(page);

    await page.keyboard.press('Control+Shift+Delete');

    await expect(page.getByText('Clear all cages?')).toBeVisible();
    await expect(page.getByText(/This will remove all 2 cages\./)).toBeVisible();
    await expect(page.getByRole('button', { name: 'Cancel' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Clear all' })).toBeVisible();
  });

  test('Cmd/Ctrl+Shift+Backspace also opens the modal (macOS quirk)', async ({ page }) => {
    await installTauriStubs(page, makeState(N, TWO_CAGES));
    await waitForApp(page);

    await page.keyboard.press('Control+Shift+Backspace');

    await expect(page.getByText('Clear all cages?')).toBeVisible();
  });

  test('Cancel button closes the modal without invoking clear_all_cages', async ({ page }) => {
    await installTauriStubs(page, makeState(N, TWO_CAGES));
    // Track whether the backend was called — it must not be on Cancel.
    await addInvokeHandler(
      page,
      'clear_all_cages',
      `window.__clear_called__ = (window.__clear_called__ ?? 0) + 1; return currentState;`,
    );
    await waitForApp(page);

    await page.keyboard.press('Control+Shift+Delete');
    await expect(page.getByText('Clear all cages?')).toBeVisible();

    await page.getByRole('button', { name: 'Cancel' }).click();
    await expect(page.getByText('Clear all cages?')).toBeHidden();

    const calls = await page.evaluate(() => (window as any).__clear_called__ ?? 0);
    expect(calls).toBe(0);
  });

  test('Escape closes the modal', async ({ page }) => {
    await installTauriStubs(page, makeState(N, TWO_CAGES));
    await waitForApp(page);

    await page.keyboard.press('Control+Shift+Delete');
    await expect(page.getByText('Clear all cages?')).toBeVisible();

    await page.keyboard.press('Escape');
    await expect(page.getByText('Clear all cages?')).toBeHidden();
  });

  test('Confirm button invokes clear_all_cages and refreshes the view', async ({ page }) => {
    await installTauriStubs(page, makeState(N, TWO_CAGES));
    // Stub clear_all_cages to return an empty puzzle (no cages).
    await addInvokeHandler(
      page,
      'clear_all_cages',
      `
      window.__clear_called__ = (window.__clear_called__ ?? 0) + 1;
      const n = currentState.n;
      const cells = Array.from({ length: n }, () =>
        Array.from({ length: n }, () =>
          Array.from({ length: n }, (_, i) => i + 1)
        )
      );
      const next = { n, cells, cages: [], diff: { changes: [] } };
      window.__setTauriState__(next);
      return next;
      `,
    );
    await waitForApp(page);

    await page.keyboard.press('Control+Shift+Delete');
    await page.getByRole('button', { name: 'Clear all' }).click();

    await expect(page.getByText('Clear all cages?')).toBeHidden();

    // The handler must have been called exactly once.
    const calls = await page.evaluate(() => (window as any).__clear_called__);
    expect(calls).toBe(1);
  });

  test('clicking the overlay backdrop dismisses the modal', async ({ page }) => {
    await installTauriStubs(page, makeState(N, TWO_CAGES));
    await waitForApp(page);

    await page.keyboard.press('Control+Shift+Delete');
    await expect(page.getByText('Clear all cages?')).toBeVisible();

    // Click in the upper-left corner of the viewport — well outside the dialog.
    await page.mouse.click(5, 5);
    await expect(page.getByText('Clear all cages?')).toBeHidden();
  });
});
