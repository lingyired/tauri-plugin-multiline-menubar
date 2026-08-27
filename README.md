# Tauri Plugin multiline-menubar

[![Tauri](https://img.shields.io/badge/Tauri-2.x-24C8DB?logo=tauri&logoColor=white)](https://v2.tauri.app/)
[![Platform](https://img.shields.io/badge/platform-macOS-000000?logo=apple&logoColor=white)](https://www.apple.com/macos/)
[![Release](https://img.shields.io/github/v/release/lingyired/tauri-plugin-multiline-menubar)](https://github.com/lingyired/tauri-plugin-multiline-menubar/releases)
[![GitHub](https://img.shields.io/badge/GitHub-lingyired%2Ftauri--plugin--multiline--menubar-181717?logo=github)](https://github.com/lingyired/tauri-plugin-multiline-menubar)

A Tauri v2 plugin that renders a two-line label in the macOS menu bar, similar to the Stats app's Mini widget.

## Screenshots

### Multiple menu-bar instances

Each `create()` call adds an independent status item — run as many as you need.

![Multiple instances](screenshots/menubar.png)

### Per-line style customization

Every line can have its own color, font family, weight (bold), and monospaced digits — all independently.

![Per-line styles](screenshots/menubar-style.png)

### Real-world example: [Fund01](https://github.com/lingyired/fund01)

Fund01 is a portfolio & market tracker that uses this plugin to display real-time index and P&L data in the menu bar. Clicking any item opens a rich popup with index quotes, holdings, and more.

![Fund01](screenshots/fund0.png)

## Supported platforms

- **macOS** — full native support via `NSStatusItem` + a custom `NSView`.
- **Windows / Linux / mobile** — API compiles but returns `UnsupportedPlatform`.

## Example

A runnable demo lives in [`examples/demo`](./examples/demo). It is a minimal Tauri v2
app that drives the plugin from plain HTML/JS (no framework). The plugin is pulled in
by relative path (`../../..`), so it always builds against the source in this repo:

```bash
cd examples/demo
npm install
npm run tauri:dev   # macOS only — uses the dev-suffixed bundle identity
```

> Dev runs use a separate `dev`-suffixed bundle ID / app name
> (`src-tauri/tauri.conf.dev.json`) so they never collide with release builds
> on macOS 26 — see [Troubleshooting](#troubleshooting-status-item-missing-on-macos-26)
> below.

## Troubleshooting: status item missing on macOS 26

On macOS 26, a menu-bar app can silently fail to show its status item if
Control Center's "recently used apps" list still remembers an older build of
the same app. macOS remembers menu-bar visibility **per bundle ID** and that
state survives app updates, so a single run can mark the item hidden for every
later run that shares the same bundle ID. This is tracked in
[this analysis](https://b-log.to/tech-analysis/macos-26-controlcenter-trackedapplications-ghost/).

If the status item does not appear after launching:

1. Quit the app, then open **System Settings → Menu Bar** (Control Center)
   and toggle the app's entry off and on again.
2. Make sure you are not running two builds with the same bundle ID (e.g.
   dev and release at the same time) — conflicting IDs keep re-hiding each
   other.
3. If it still does not show, rebuild the release app with a **brand-new
   bundle ID** to rule out stale remembered state.

To prevent this, always use a distinct bundle ID per flavor: the demo's
`npm run tauri:dev` appends `dev` to the identity
(`productName` `multiline-menubar-demo-new-dev`,
`identifier` `com.tauri.multiline-menubar-demo-new.dev`), while
`npm run tauri:build` keeps the release identity, so dev and release never
share a bundle ID.

## Rust usage

Add the plugin to your Tauri app:

```rust
// src-tauri/src/lib.rs
tauri::Builder::default()
    .plugin(tauri_plugin_multiline_menubar::init())
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
```

Add the default capability:

```json
{
  "permissions": [
    "multiline-menubar:default"
  ]
}
```

### Permissions

The `multiline-menubar:default` permission set covers core rendering and
read-only queries: `create`, `set_text`, `set_font_sizes`, `set_layout`,
`set_tooltip`, `set_visible`, `set_colors`, `set_bold`, `set_font_family`,
`set_monospaced`, `set_alignment`, `rect`, `is_visible`, `set_auto_popup`.

Higher-impact commands are **not** included by default and must be granted
explicitly when needed:

| Command | Permission | Why it's not default |
|---|---|---|
| `remove` | `multiline-menubar:allow-remove` | Destroys a status item |
| `set_menu` / `remove_menu` | `multiline-menubar:allow-set-menu` / `allow-remove-menu` | Injects arbitrary menu items |
| `set_popup_window` | `multiline-menubar:allow-set-popup-window` | Repoints which window opens as popup |
| `open_popup` / `close_popup` / `toggle_popup` | `allow-open-popup` / `allow-close-popup` / `allow-toggle-popup` | Shows/hides a window |

```json
{
  "permissions": [
    "multiline-menubar:default",
    "multiline-menubar:allow-remove",
    "multiline-menubar:allow-set-menu",
    "multiline-menubar:allow-open-popup"
  ]
}
```

> **Breaking change (v1.2.0):** earlier versions shipped all commands in the
> default set. If your app relied on the removed ones, add the matching
> `allow-*` permissions above to your capability file.

## Frontend usage

```ts
import {
  create,
  remove,
  setVisible,
  setText,
  setFontSizes,
  setLayout,
  setTooltip,
  setColors,
  setBold,
  setFontFamily,
  setMenu,
  removeMenu,
  onMenuSelection,
  setPopupWindow,
  setAutoPopup,
  openPopup,
  closePopup,
  togglePopup,
  isVisible,
  rect,
  EVENT_CLICK,
  EVENT_READY,
  EVENT_POPUP_OPEN,
  EVENT_POPUP_CLOSE,
  listen,
} from "tauri-plugin-multiline-menubar";

await create({ id: "main" });
await setText({ id: "main", top: "Sensor", bottom: "16W" });

// Customize the font size (points) for each line. Values are clamped to the
// supported range on the native side (small 5–11 pt, large 8–16 pt).
await setFontSizes({ id: "main", top: 8, bottom: 14 });

// Choose the vertical layout: 0 = small label on top / large value below
// (default), 1 = the mirror, 2 = equal lines.
await setLayout({ id: "main", layout: 1 });

// A left click on the item automatically opens the "popup" window below it.

// Force the top and/or bottom line bold, independent of the layout:
await setBold({ id: "main", top: true, bottom: false });

// Swap a line to a specific font family (macOS Font Book name); pass
// null/'' to restore the system font. Unknown names fall back silently.
await setFontFamily({ id: "main", top: null, bottom: "Helvetica" });
// Listen to the click event if you want to drive the popup yourself instead.
await listen(EVENT_CLICK, (e) => {
  console.log("clicked", e.payload); // { button, x, y, width, height }
});

console.log(await isVisible({ id: "main" })); // true
await setVisible({ id: "main", visible: false }); // was: hide()
```

You can also call the commands directly with `@tauri-apps/api/core`:

```ts
import { invoke } from "@tauri-apps/api/core";

await invoke("plugin:multiline-menubar|set_text", {
  payload: { id: "main", top: "Sensor", bottom: "16W" },
});
// Show/hide is a single setVisible(bool) command:
await invoke("plugin:multiline-menubar|set_visible", {
  payload: { id: "main", visible: true },
});
```

## Popup window

Clicking the menu bar item opens a Tauri WebView window ("popup" by default)
anchored directly below the item, centered on it. The window is toggled on
left click and auto-hides when it loses focus — the standard menubar-app
behaviour.

To use it, define a window in `tauri.conf.json` (the plugin ships with a
`popup` example window):

```jsonc
{
  "label": "popup",
  "url": "popup.html",
  "width": 320,
  "height": 400,
  "decorations": false,
  "transparent": true,
  "alwaysOnTop": true,
  "visible": false,
  "skipTaskbar": true
}
```

> `transparent: true` needs `"app": { "macOSPrivateApi": true }` in
> `tauri.conf.json` (and the `macos-private-api` cargo feature) for a clean
> frosted background on macOS.

Popup-related commands:

| Command | Description |
| --- | --- |
| `set_popup_window({ label })` | Choose which window is the popup (default `"popup"`). Call before the first open. |
| `set_auto_popup({ enabled })` | Toggle automatic popup on left click (default `true`). |
| `open_popup()` | Show + position the popup below the item. |
| `close_popup()` | Hide the popup. |
| `toggle_popup()` | Toggle visibility. |

## API alignment with the Tauri menubar convention

The plugin intentionally mirrors the command/event conventions used by the
macOS menubar plugin family (e.g. `tauri-plugin-menubar-dnd`):

- It owns its own `NSStatusItem` (like the tray-based menubar plugins), so you
  drive the icon/text through this plugin instead of Tauri's system tray.
- Commands use the same naming style: `set_visible`, `set_tooltip`,
  `set_popup_window`, `open_popup`/`close_popup`/`toggle_popup`.
- Events are emitted on the `multiline-menubar://` scheme:
  - `multiline-menubar://ready` — status item created.
  - `multiline-menubar://click` — `{ button: "left" | "right", x, y, width, height }`.
  - `multiline-menubar://popup-open` / `multiline-menubar://popup-close` — `{ window }`.

> **v1.0.0:** the API was aligned with Tauri's `TrayIcon`. Historical
> names `getRect → rect`, `destroy → remove`, and `show`/`hide` were folded
> into `setVisible(bool)`. `removeMenu` is retained and `setMenu(null)` detaches
> the menu. See [`API.md`](./API.md) for the full reference.

## How it works

The plugin uses a small Objective-C++ helper that creates an `NSStatusItem` and attaches a custom `NSView`. The view draws two lines of text:

- **Top line**: 7 pt, light weight (label).
- **Bottom line**: 12 pt, regular weight (value).

The view width is computed from the text so the menu bar item stays as narrow as possible.

The font size of each line can be customized independently via `setFontSizes`. Values are clamped on the native side to keep both lines inside the ~22 pt tall menu bar without overlapping:

- **Top label**: 5–11 pt (default 7)
- **Bottom value**: 8–16 pt (default 12)

The weight of each line can also be overridden independently via `setBold`: pass `top`/`bottom` as `true` to force that line bold (overriding the weight `layout` assigns), or `false` to leave it to the layout.

The font **family** of each line can likewise be set independently via `setFontFamily`: pass a macOS font family name from Font Book (e.g. `"Helvetica"`) for `top`/`bottom`, or `null`/`''` to keep the system font. The line still resolves the weight `layout`/`setBold` asks for, using the closest face the family provides; unknown names fall back to the system font.

On click, the native helper measures the status item's on-screen rectangle and
calls back into Rust, which emits the `click` event and (when auto-popup is
on) positions the popup window below the item using the primary monitor's
geometry (macOS y grows upward, Tauri y grows downward, so the y axis is
flipped).

## Notes

- This plugin creates its own `NSStatusItem`. It does not extend Tauri's built-in system-tray / tray icon.
- Text color follows `NSColor.textColor`, so it adapts automatically to light / dark mode and accessibility settings.
- Popup positioning assumes the status item is on the primary monitor.
