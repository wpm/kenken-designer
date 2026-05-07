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
