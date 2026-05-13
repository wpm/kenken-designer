import { test, expect } from '@playwright/test';
import { installTauriStubs, makeState, waitForApp } from './helpers';

const N = 4;

// Regression: in WKWebView (Tauri/macOS), clicking a non-focusable SVG element
// does NOT automatically blur a previously focused <select>. When the size
// <select> retains focus, is_text_input_focused() returns true and all
// keyboard shortcuts are silently ignored, including Shift+Enter.
//
// The fix: on_cell_click explicitly blurs any focused text input so shortcuts
// work immediately after clicking a grid cell.
//
// We simulate the WKWebView scenario by dispatching a synthetic mousedown on
// the SVG rect (triggering on_cell_click) without the browser's natural
// focus-transfer, then verifying Shift+Enter creates a draft.
test('cell click blurs SELECT so Shift+Enter works after size selector interaction', async ({ page }) => {
  const state = makeState(N);
  await installTauriStubs(page, state);
  await waitForApp(page);

  // Simulate the user interacting with the size selector (which focuses it).
  await page.focus('select');
  expect(await page.evaluate(() => document.activeElement?.tagName)).toBe('SELECT');

  // Dispatch mousedown directly on the SVG grid cell without triggering the
  // browser's natural blur-on-click (simulates WKWebView behavior).
  await page.evaluate((n) => {
    const svg = document.querySelector('.grid-svg');
    if (!svg) throw new Error('no grid-svg');
    const rect = svg.getBoundingClientRect();
    const cellSize = rect.width / n;
    const el = document.elementFromPoint(rect.left + cellSize * 0.5, rect.top + cellSize * 0.5);
    if (!el) throw new Error('no element at cell position');
    el.dispatchEvent(new MouseEvent('mousedown', {
      bubbles: true,
      cancelable: true,
      button: 0,
      clientX: rect.left + cellSize * 0.5,
      clientY: rect.top + cellSize * 0.5,
    }));
  }, N);

  await page.waitForTimeout(100);

  // The fix must have blurred the select; keyboard shortcuts must now work.
  expect(await page.evaluate(() => document.activeElement?.tagName)).not.toBe('SELECT');

  await page.keyboard.press('Shift+Enter');

  await page.waitForFunction(
    () => Array.from(document.querySelectorAll('.grid-svg text'))
      .some(t => t.textContent?.trim() === '?'),
    { timeout: 3000 },
  );
});
