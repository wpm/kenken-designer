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

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
