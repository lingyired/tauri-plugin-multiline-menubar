# Example — multiline-menubar demo

A minimal Tauri v2 app that drives the `multiline-menubar` plugin: it shows a two-line
menu-bar label, lets you switch between the three layout modes (top-emphasis /
bottom-emphasis / equal), pop up a custom window, and open a context menu — all from
plain HTML/JS using the global Tauri API (`window.__TAURI__`).

The plugin is referenced by **relative path** (`../../..`, i.e. the repository root), so
this example always builds against the plugin source in this repo — no publish/crate
install required.

## Prerequisites

- **macOS** (the plugin only renders a native `NSStatusItem`; other platforms compile
  but calls return `UnsupportedPlatform`).
- Rust toolchain (stable) + [`tauri-cli`](https://v2.tauri.app/start/prerequisites/).
- Node.js ≥ 18.

## Run

```bash
cd examples/demo
npm install
npm run tauri dev
```

`tauri dev` serves `src/` directly (no bundler step) and launches the app. You should
see the two-line label in the macOS menu bar.

## What to try

- Use the main window controls to set text, font sizes, layout mode, tooltip and colors.
- Click the menu-bar item to open the popup window, or right-click for the context menu.
- Toggle the second (secondary) instance to see `remove()` in action.

## Layout modes

| Value | Meaning            | Font (top→bottom)        |
| ----- | ------------------ | ------------------------ |
| `0`   | Emphasis bottom    | small (5–11) / large (8–16) |
| `1`   | Emphasis top       | large (8–16) / small (5–11) |
| `2`   | Equal              | equal (5–11) / equal (5–11) |

See the plugin [`API.md`](../../API.md) for the full command/permission reference.
