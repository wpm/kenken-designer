import { Page } from '@playwright/test';

export function makeState(n: number, cages: any[] = []) {
  const cells = Array.from({ length: n }, () =>
    Array.from({ length: n }, () =>
      Array.from({ length: n }, (_, i) => i + 1)
    )
  );
  return { n, cells, cages, diff: { changes: [] } };
}

export function makeEditResult(state: any, drafts: any[] = []) {
  return { view: state, drafts };
}

// Installs __TAURI__ stubs. invoke() is driven by the provided handler map;
// unrecognised commands return the current state.
export async function installTauriStubs(page: Page, initialState: any) {
  await page.addInitScript((state) => {
    let currentState = state;

    (window as any).__TAURI__ = {
      core: {
        invoke: (cmd: string, args: any) => {
          const handler = (window as any).__tauri_invoke_handlers__?.[cmd];
          if (handler) return Promise.resolve(handler(args, currentState));
          return Promise.resolve(currentState);
        },
      },
      event: {
        listen: (_event: string, _handler: any) => Promise.resolve(() => {}),
      },
      dialog: {
        open: () => Promise.resolve(null),
        save: () => Promise.resolve(null),
      },
    };

    // Allow tests to update currentState and trigger handlers
    (window as any).__setTauriState__ = (s: any) => { currentState = s; };
    (window as any).__getTauriState__ = () => currentState;
    (window as any).__tauri_invoke_handlers__ = {};
  }, initialState);
}

export async function waitForApp(page: Page) {
  await page.goto('/');
  await page.waitForSelector('.grid-svg', { timeout: 10000 });
}

// Read the Y attribute of the SVG cursor rect.
export async function getCursorY(page: Page): Promise<number> {
  return page.evaluate(() => {
    const el = document.querySelector('[data-testid="cursor"]');
    if (!el) throw new Error('cursor element not found');
    return parseFloat(el.getAttribute('y') ?? 'NaN');
  });
}

// Click the grid cell at (row, col) in an N×N grid by computing its position
// from the SVG bounds.
export async function clickGridCell(page: Page, n: number, row: number, col: number) {
  const svg = page.locator('.grid-svg');
  const box = await svg.boundingBox();
  if (!box) throw new Error('grid-svg not found');
  const cellSize = box.width / n;
  await page.mouse.click(
    box.x + cellSize * (col + 0.5),
    box.y + cellSize * (row + 0.5),
  );
}

// Install Tauri stubs with a rank_active_cage handler returning `tupleCount`
// synthetic tuples, navigate to the app, click cell (0,0) to activate the
// cage band, and wait for thumbnails to appear.
export async function setupCageBandWithTuples(
  page: Page,
  n: number,
  cages: any[],
  tupleCount: number,
) {
  const view = makeState(n, cages);
  await installTauriStubs(page, view);

  await page.addInitScript(
    ({ count, puzzleView }: { count: number; puzzleView: any }) => {
      const tuples = Array.from({ length: count }, (_, i) => ({
        tuple: [i + 1],
        view: puzzleView,
        total_reduction: 0,
        newly_singleton: 0,
      }));
      (window as any).__tauri_invoke_handlers__ = {
        ...((window as any).__tauri_invoke_handlers__ ?? {}),
        rank_active_cage: () => tuples,
      };
    },
    { count: tupleCount, puzzleView: view },
  );

  await waitForApp(page);
  await clickGridCell(page, n, 0, 0);
  await page.waitForSelector('.cage-band__thumb', { timeout: 8000 });
}
