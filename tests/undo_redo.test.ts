import { test, expect } from '@playwright/test';
import {
  addInvokeHandler,
  installTauriStubs,
  makeState,
  waitForApp,
} from './helpers';

const N = 3;

test.describe('undo / redo / save / open keyboard shortcuts', () => {
  test('Cmd/Ctrl+Z invokes undo', async ({ page }) => {
    await installTauriStubs(page, makeState(N));
    await addInvokeHandler(page, 'undo', (_args, currentState) => {
      (window as any).__undo_calls__ = ((window as any).__undo_calls__ ?? 0) + 1;
      return currentState;
    });
    await waitForApp(page);

    await page.keyboard.press('Control+z');

    await expect.poll(() =>
      page.evaluate(() => (window as any).__undo_calls__ ?? 0),
    ).toBe(1);
  });

  test('Cmd/Ctrl+Shift+Z invokes redo (not undo)', async ({ page }) => {
    await installTauriStubs(page, makeState(N));
    await addInvokeHandler(page, 'redo', (_args, currentState) => {
      (window as any).__redo_calls__ = ((window as any).__redo_calls__ ?? 0) + 1;
      return currentState;
    });
    await addInvokeHandler(page, 'undo', (_args, currentState) => {
      (window as any).__undo_calls__ = ((window as any).__undo_calls__ ?? 0) + 1;
      return currentState;
    });
    await waitForApp(page);

    await page.keyboard.press('Control+Shift+z');

    await expect.poll(() =>
      page.evaluate(() => (window as any).__redo_calls__ ?? 0),
    ).toBe(1);
    expect(await page.evaluate(() => (window as any).__undo_calls__ ?? 0)).toBe(0);
  });

  test('Cmd/Ctrl+S triggers the save dialog flow', async ({ page }) => {
    await installTauriStubs(page, makeState(N));
    // Stub the dialog.save to return a path; capture save_puzzle args.
    await page.addInitScript(() => {
      (window as any).__TAURI__.dialog.save = () => Promise.resolve('/tmp/test.kenken');
    });
    await addInvokeHandler(page, 'save_puzzle', (args) => {
      (window as any).__save_path__ = args.path;
      return null;
    });
    await waitForApp(page);

    await page.keyboard.press('Control+s');

    await expect.poll(() =>
      page.evaluate(() => (window as any).__save_path__),
    ).toBe('/tmp/test.kenken');
  });

  test('Cmd/Ctrl+O triggers the open dialog flow', async ({ page }) => {
    await installTauriStubs(page, makeState(N));
    await page.addInitScript(() => {
      (window as any).__TAURI__.dialog.open = () => Promise.resolve('/tmp/loaded.kenken');
    });
    await addInvokeHandler(page, 'load_puzzle', (args, currentState) => {
      (window as any).__load_path__ = args.path;
      // Return a valid PuzzleView so the app updates.
      return { ...(currentState as any), diff: { changes: [] } };
    });
    await waitForApp(page);

    await page.keyboard.press('Control+o');

    await expect.poll(() =>
      page.evaluate(() => (window as any).__load_path__),
    ).toBe('/tmp/loaded.kenken');
  });
});
