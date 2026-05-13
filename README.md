# KenKen Designer

[![CI](https://github.com/wpm/kenken-designer/actions/workflows/ci.yml/badge.svg)](https://github.com/wpm/kenken-designer/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/wpm/kenken-designer/branch/main/graph/badge.svg)](https://codecov.io/gh/wpm/kenken-designer)

A KenKen puzzle designer built with Tauri and Leptos.

## Setup

```sh
npm install      # install Tailwind CSS build tooling
```

## Development

```sh
trunk serve      # builds and hot-reloads at http://localhost:1420
```

Trunk runs `npm run tw:build` automatically before each build. For incremental Tailwind rebuilds while editing styles, run this in a second terminal:

```sh
npm run tw:watch
```

## Keyboard shortcuts

Every right-click context-menu action also has a keyboard shortcut, shown
inline next to the item.

| Action               | Shortcut                          |
| -------------------- | --------------------------------- |
| Set operation…       | `Enter`                           |
| Make singleton       | `Space` (also `C`)                |
| Uncage               | `Esc`                             |
| Delete cage          | `Del`                             |
| Move cell…           | `M`                               |
| Clear all cages      | `Cmd/Ctrl+Shift+Del`              |
| Undo / Redo          | `Cmd/Ctrl+Z` / `Cmd/Ctrl+Shift+Z` |
| Save / Save As       | `Cmd/Ctrl+S` / `Cmd/Ctrl+Shift+S` |
| Open                 | `Cmd/Ctrl+O`                      |
| Move cursor          | Arrow keys                        |
| Extend / shrink cage | `Shift+Arrow`                     |
| Cycle cages          | `Tab` / `Shift+Tab`               |

On macOS the regular Delete key reports as `Backspace`, so
`Cmd+Shift+Backspace` also clears all cages.

## Testing

Rust tests live alongside the source and run via `cargo test`. End-to-end UI
behavior is exercised by Playwright; the suite is in `tests/`.

```sh
npx playwright install chromium    # one-time browser download
npx playwright test                # runs the suite, auto-starting trunk if needed
```

`playwright.config.ts` declares a `webServer` block that boots `trunk serve` on
its own if port 1420 isn't already listening. The first run can take a minute
or two while trunk performs the initial wasm build.

If you have a pre-installed chromium binary you'd rather use (offline runs, a
different revision, etc.) set `PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH` to the
binary's path before invoking `npx playwright test`.

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
