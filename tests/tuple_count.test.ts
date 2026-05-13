import { test, expect } from '@playwright/test';
import {
  installTauriStubs,
  makeState,
  setupCageBandWithTuples,
  waitForApp,
} from './helpers';

const N = 3;
const ONE_CAGE = [{ cells: [[0, 0]], op: 'Given', target: 1 }];

test.describe('tuple count caption', () => {
  test('shows nothing when no cage is active', async ({ page }) => {
    await installTauriStubs(page, makeState(N));
    await waitForApp(page);

    // The .tuple-count element is rendered but empty (zero-height) when no
    // cage is active — Playwright reports empty containers as hidden, so we
    // assert text content rather than visibility.
    const caption = page.locator('.tuple-count');
    await expect(caption).toHaveCount(1);
    await expect(caption).toHaveText('');
  });

  test('shows "N Tuples" pluralized after activating a cage', async ({ page }) => {
    await setupCageBandWithTuples(page, N, ONE_CAGE, 5);

    const caption = page.locator('.tuple-count');
    await expect(caption).toHaveText('5 Tuples');
  });

  test('shows "1 Tuple" singular for a single tuple', async ({ page }) => {
    await setupCageBandWithTuples(page, N, ONE_CAGE, 1);

    const caption = page.locator('.tuple-count');
    await expect(caption).toHaveText('1 Tuple');
  });
});
