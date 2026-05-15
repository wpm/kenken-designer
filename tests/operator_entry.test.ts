import { test, expect, type Page } from '@playwright/test';
import {
  addInvokeHandler,
  clickGridCell,
  installTauriStubs,
  makeState,
  waitForApp,
} from './helpers';

const N = 3;

const PAIR_CAGE = [{ cells: [[0, 0], [0, 1]], op: 'Add', target: 3 }];

// Stub cage_options so the picker has data to show. Returns valid ops/targets
// for a 2-cell cage in a 3×3 puzzle (Add 3..5, Sub 1..2, Mul 2..6, Div 2..3).
async function stubBinaryCageOptions(page: Page) {
  await addInvokeHandler(page, 'cage_options', () => [
    { op: 'Add', targets: [3, 4, 5] },
    { op: 'Sub', targets: [1, 2] },
    { op: 'Mul', targets: [2, 3, 4, 6] },
    { op: 'Div', targets: [2, 3] },
  ]);
}

async function stubSingletonCageOptions(page: Page) {
  await addInvokeHandler(page, 'cage_options', () => [
    { op: 'Given', targets: [1, 2, 3] },
  ]);
}

test.describe('operator entry picker', () => {
  test('Enter on a multi-cell cage opens OpPicker showing valid op glyphs', async ({ page }) => {
    await installTauriStubs(page, makeState(N, PAIR_CAGE));
    await stubBinaryCageOptions(page);
    await waitForApp(page);

    // Click cell (0,0) so it's the active cage, then press Enter.
    await clickGridCell(page, N, 0, 0);
    await page.keyboard.press('Enter');

    // OpPicker label is "+ − × ÷" (joined operator glyphs) at the cage anchor.
    await expect(page.locator('.grid-svg text', { hasText: /\+ − × ÷/ }))
      .toBeVisible({ timeout: 5000 });
  });

  test('Escape during entry cancels the picker (label restored)', async ({ page }) => {
    await installTauriStubs(page, makeState(N, PAIR_CAGE));
    await stubBinaryCageOptions(page);
    await waitForApp(page);

    await clickGridCell(page, N, 0, 0);
    await page.keyboard.press('Enter');
    await expect(page.locator('.grid-svg text', { hasText: /\+ − × ÷/ }))
      .toBeVisible({ timeout: 5000 });

    await page.keyboard.press('Escape');

    // The OpPicker glyphs should disappear; the original "3+" label returns.
    await expect(page.locator('.grid-svg text', { hasText: /\+ − × ÷/ }))
      .toHaveCount(0);
    await expect(page.locator('.grid-svg text', { hasText: /^3\+$/ }))
      .toHaveCount(1);
  });

  test('typing an operator key jumps directly to TargetPicker dropdown', async ({ page }) => {
    await installTauriStubs(page, makeState(N, PAIR_CAGE));
    await stubBinaryCageOptions(page);
    await waitForApp(page);

    await clickGridCell(page, N, 0, 0);
    // Pressing "+" directly: Playwright's `Shift+=` sends `KeyboardEvent.key === "="`
    // (not "+"), so we use the character form which translates to the right key event.
    await page.keyboard.press('+');

    // TargetPicker dropdown should render text rows with "3+", "4+", "5+".
    await expect(page.locator('.grid-svg text', { hasText: /^3\+$/ }))
      .toHaveCount(1, { timeout: 5000 });
    await expect(page.locator('.grid-svg text', { hasText: /^4\+$/ }))
      .toHaveCount(1);
    await expect(page.locator('.grid-svg text', { hasText: /^5\+$/ }))
      .toHaveCount(1);
  });

  test('Enter in TargetPicker commits the selected op/target via insert_cage or set_cage_operation', async ({ page }) => {
    await installTauriStubs(page, makeState(N, PAIR_CAGE));
    await stubBinaryCageOptions(page);

    // Stub set_cage_operation to record the call and return the original state.
    await addInvokeHandler(page, 'set_cage_operation', (args, currentState) => {
      (window as any).__set_op_args__ = args;
      return currentState;
    });

    await waitForApp(page);

    await clickGridCell(page, N, 0, 0);
    await page.keyboard.press('Enter');

    // Pick "+" via key, then Enter to commit at default selection (index 0 → 3).
    await page.keyboard.press('+');
    await page.keyboard.press('Enter');

    await expect.poll(() =>
      page.evaluate(() => (window as any).__set_op_args__),
    ).toBeTruthy();

    const args = await page.evaluate(() => (window as any).__set_op_args__);
    expect(args.op).toBe('Add');
    expect(args.target).toBe(3);
    expect(args.anchor).toEqual([0, 0]);
  });

  test('Backspace in TargetPicker (multi-cell cage) returns to OpPicker', async ({ page }) => {
    await installTauriStubs(page, makeState(N, PAIR_CAGE));
    await stubBinaryCageOptions(page);
    await waitForApp(page);

    await clickGridCell(page, N, 0, 0);
    await page.keyboard.press('Enter');
    await page.keyboard.press('+');

    // In TargetPicker now — empty buffer Backspace goes back to OpPicker.
    await expect(page.locator('.grid-svg text', { hasText: /^3\+\|$/ }))
      .toHaveCount(1, { timeout: 5000 });

    await page.keyboard.press('Backspace');

    await expect(page.locator('.grid-svg text', { hasText: /\+ − × ÷/ }))
      .toBeVisible({ timeout: 5000 });
  });

  test('Singleton cage skips OpPicker and opens TargetPicker directly', async ({ page }) => {
    const singletonCage = [{ cells: [[1, 1]], op: 'Given', target: 2 }];
    await installTauriStubs(page, makeState(N, singletonCage));
    await stubSingletonCageOptions(page);
    await waitForApp(page);

    await clickGridCell(page, N, 1, 1);
    await page.keyboard.press('Enter');

    // The anchor label gains the entry caret "|". Singletons skip OpPicker;
    // current target 2 is pre-selected, so the label is "2|" (Given has no glyph).
    await expect(page.locator('.grid-svg text', { hasText: /^2\|$/ }))
      .toHaveCount(1, { timeout: 5000 });
  });

  test('typing a digit in TargetPicker jumps to the matching target', async ({ page }) => {
    await installTauriStubs(page, makeState(N, PAIR_CAGE));
    await stubBinaryCageOptions(page);
    await waitForApp(page);

    await clickGridCell(page, N, 0, 0);
    await page.keyboard.press('Enter');
    await page.keyboard.press('+');

    // Type "5" — should select target 5 of [3, 4, 5]; label becomes "5+|".
    await page.keyboard.press('5');

    await expect(page.locator('.grid-svg text', { hasText: /^5\+\|$/ }))
      .toHaveCount(1, { timeout: 5000 });
  });

  test('typing an invalid digit in TargetPicker is ignored and does not commit a bogus target', async ({ page }) => {
    await installTauriStubs(page, makeState(N, PAIR_CAGE));
    await stubBinaryCageOptions(page);

    await addInvokeHandler(page, 'set_cage_operation', (args, currentState) => {
      (window as any).__set_op_args__ = args;
      return currentState;
    });

    await waitForApp(page);

    await clickGridCell(page, N, 0, 0);
    await page.keyboard.press('Enter');
    await page.keyboard.press('+');

    // Add targets are [3, 4, 5]; "9" is not a prefix of any valid target. The
    // keystroke should be ignored: label stays at the default selection "3+|"
    // rather than showing the invalid "9+|".
    await page.keyboard.press('9');

    await expect(page.locator('.grid-svg text', { hasText: /^3\+\|$/ }))
      .toHaveCount(1, { timeout: 5000 });
    await expect(page.locator('.grid-svg text', { hasText: /^9\+\|$/ }))
      .toHaveCount(0);

    // Enter commits the default selection (3), not the typed-but-invalid 9.
    await page.keyboard.press('Enter');

    await expect.poll(() =>
      page.evaluate(() => (window as any).__set_op_args__),
    ).toBeTruthy();

    const args = await page.evaluate(() => (window as any).__set_op_args__);
    expect(args.op).toBe('Add');
    expect(args.target).toBe(3);
  });
});
