# Tauri Plugin multiline-menubar

A Tauri v2 plugin that renders a two-line label in the macOS menu bar, similar to the Stats app's Mini widget.

## Supported platforms

- **macOS** — full native support via `NSStatusItem` + a custom `NSView`.
- **Windows / Linux / mobile** — API compiles but returns `UnsupportedPlatform`.

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

## Frontend usage

```ts
import {
  show,
  hide,
  setText,
  setFontSizes,
  isVisible,
} from "tauri-plugin-multiline-menubar-api";

await show();
await setText({ top: "Sensor", bottom: "16W" });

// Customize the font size (points) for each line. Values are clamped to the
// supported range on the native side (top: 5–11 pt, bottom: 8–16 pt).
await setFontSizes({ top: 8, bottom: 14 });

console.log(await isVisible()); // true
await hide();
```

You can also call the commands directly with `@tauri-apps/api/core`:

```ts
import { invoke } from "@tauri-apps/api/core";

await invoke("plugin:multiline-menubar|set_text", {
  payload: { top: "Sensor", bottom: "16W" },
});
await invoke("plugin:multiline-menubar|show");
```

## How it works

The plugin uses a small Objective-C++ helper that creates an `NSStatusItem` and attaches a custom `NSView`. The view draws two lines of text:

- **Top line**: 7 pt, light weight (label).
- **Bottom line**: 12 pt, regular weight (value).

The view width is computed from the text so the menu bar item stays as narrow as possible.

The font size of each line can be customized independently via `setFontSizes`. Values are clamped on the native side to keep both lines inside the ~22 pt tall menu bar without overlapping:

- **Top label**: 5–11 pt (default 7)
- **Bottom value**: 8–16 pt (default 12)

## Notes

- This plugin creates its own `NSStatusItem`. It does not extend Tauri's built-in system-tray / tray icon.
- Text color follows `NSColor.textColor`, so it adapts automatically to light / dark mode and accessibility settings.
