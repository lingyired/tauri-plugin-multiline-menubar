# multiline-menubar — API Reference

A Tauri v2 plugin that renders a **two-line label** in the macOS menu bar
(`NSStatusItem` + a custom `NSView`), e.g. a small label on top and a large
value below — inspired by the Stats app's Mini widget.

- **macOS** — full native support.
- **Windows / Linux / mobile** — the API compiles, but every call returns
  `UnsupportedPlatform`.

> **Naming convention (v1.0.0):** the API is aligned with Tauri's `TrayIcon`
> conventions. Historical names were renamed: `getRect → rect`,
> `destroy → remove`, and `show`/`hide` were folded into a single
> `setVisible(bool)`. `removeMenu` is kept, and `setMenu(null)` (or omitting
> `items`) detaches the menu — mirroring Tauri's `setMenu(null)`.

---

## Quick start

### Rust

```rust
// src-tauri/src/lib.rs
tauri::Builder::default()
    .plugin(tauri_plugin_multiline_menubar::init())
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
```

### Capabilities

```jsonc
// src-tauri/capabilities/default.json
{
  "permissions": [
    "multiline-menubar:default"
  ]
}
```

The `default` permission set grants every command (see
[Permissions](#permissions)). For a tighter setup, add only the `allow-*`
identifiers you need.

### JavaScript

Two ways to call the API:

1. **Guest-JS wrappers** (named exports from the plugin's frontend package,
   e.g. `tauri-plugin-multiline-menubar`):

   ```ts
   import { create, setText, setVisible, rect } from "tauri-plugin-multiline-menubar";
   await create({ id: "main" });
   await setText({ id: "main", top: "Sensor", bottom: "16W" });
   await setVisible({ id: "main", visible: true });
   const r = await rect({ id: "main" }); // { x, y, width, height }
   ```

2. **Raw `invoke`** (used by the demo app via `window.__TAURI__.core`):

   ```ts
   import { invoke } from "@tauri-apps/api/core";
   await invoke("plugin:multiline-menubar|set_text", {
     payload: { id: "main", top: "Sensor", bottom: "16W" },
   });
   ```

Every command takes a single `payload` object (matching the TypeScript
`*Options` types below). The full command list is in
[Raw invoke commands](#raw-invoke-commands).

---

## JavaScript API (guest-js)

All functions are `async` and return a `Promise`.

### Lifecycle

| Function | Signature | Description |
| --- | --- | --- |
| `create` | `(options: CreateOptions) => Promise<void>` | Create a menubar instance. If `top`/`bottom` are provided they are applied immediately. Emits `ready`. |
| `remove` | `(options: IdOptions) => Promise<void>` | Destroy a menubar instance and free its menu/text/layout state. (Renamed from `destroy`.) |

### Content

| Function | Signature | Description |
| --- | --- | --- |
| `setText` | `(options: SetTextOptions) => Promise<void>` | Set the top and bottom text. |
| `setFontSizes` | `(options: FontSizesOptions) => Promise<void>` | Set per-line font sizes in points. Clamped on the native side by the active layout's role ranges. |
| `setLayout` | `(options: LayoutOptions) => Promise<void>` | Choose the vertical layout (`0`/`1`/`2`). See [Layout modes](#layout-modes). |
| `setTooltip` | `(options: TooltipOptions) => Promise<void>` | Set the accessibility tooltip shown on hover. |
| `setColors` | `(options: SetColorsOptions) => Promise<void>` | Set per-line text paint (`default` or `solid`). See [Color styles](#color-styles). |

### Visibility & geometry

| Function | Signature | Description |
| --- | --- | --- |
| `setVisible` | `(options: SetVisibleOptions) => Promise<void>` | Show (`true`) or hide (`false`) the instance. Replaces the old `show`/`hide`. |
| `isVisible` | `(options: IdOptions) => Promise<boolean>` | Returns whether the instance is currently visible. |
| `rect` | `(options: IdOptions) => Promise<Rect>` | Returns the on-screen rectangle in macOS screen points (origin bottom-left, y up). (Renamed from `getRect`.) |

### Context menu

| Function | Signature | Description |
| --- | --- | --- |
| `setMenu` | `(options: SetMenuOptions) => Promise<void>` | Attach a context menu built from `MenuItemDescriptor`s. Pass `items: null` (or omit it) to **detach** the menu — mirrors Tauri's `setMenu(null)`. |
| `removeMenu` | `(options: IdOptions) => Promise<void>` | Detach the context menu. (Kept as an explicit helper alongside `setMenu(null)`.) |
| `onMenuSelection` | `(id: string, handler: (e: MenuSelectionEvent) => void) => Promise<UnlistenFn>` | Subscribe to context-menu selections for one instance. |

> Note: menu selections are re-emitted on `multiline-menubar://{id}//menu`,
> **not** Tauri's `@tauri-apps/api/menu` `onMenuEvent`, because the menu is
> built directly with `muda` rather than through Tauri's `menu` plugin
> commands.

### Popup window

A left click opens a Tauri WebView window anchored below the item.

| Function | Signature | Description |
| --- | --- | --- |
| `setPopupWindow` | `(options: PopupWindowOptions) => Promise<void>` | Choose which Tauri window is the popup (default `"popup"`). Call before the first open. |
| `setAutoPopup` | `(options: SetAutoPopupOptions) => Promise<void>` | Enable/disable auto-toggling the popup on left click (default `true`). |
| `openPopup` | `(options: IdOptions) => Promise<void>` | Show + position the popup below the item. |
| `closePopup` | `(options: IdOptions) => Promise<void>` | Hide the popup. |
| `togglePopup` | `(options: IdOptions) => Promise<void>` | Toggle the popup visibility. |

### Events

```ts
import {
  EVENT_READY, EVENT_CLICK, EVENT_ENTER, EVENT_LEAVE,
  EVENT_POPUP_OPEN, EVENT_POPUP_CLOSE, EVENT_MENU,
  eventName,
} from "tauri-plugin-multiline-menubar";
```

| Constant | Event name | Payload |
| --- | --- | --- |
| `EVENT_READY` | `multiline-menubar://{id}//ready` | `{ id }` |
| `EVENT_CLICK` | `multiline-menubar://{id}//click` | `ClickEvent` |
| `EVENT_ENTER` | `multiline-menubar://{id}//enter` | `{ id, rect }` |
| `EVENT_LEAVE` | `multiline-menubar://{id}//leave` | `{ id, rect }` |
| `EVENT_POPUP_OPEN` | `multiline-menubar://{id}//popup-open` | `{ id, window }` |
| `EVENT_POPUP_CLOSE` | `multiline-menubar://{id}//popup-close` | `{ id, window }` |
| `EVENT_MENU` | `multiline-menubar://{id}//menu` | `MenuSelectionEvent` |

`eventName(id, name)` builds a fully-qualified event name. Use `listen(name, handler)`
from `@tauri-apps/api/event` to subscribe.

`ClickEvent = { id, position: { x, y }, rect: Rect, button: "left" | "right", buttonState: "up" | "down" }`

---

## Types

```ts
interface CreateOptions   { id: string; top?: string; bottom?: string }
interface IdOptions       { id: string }
interface SetTextOptions  { id: string; top: string; bottom: string }
interface FontSizesOptions{ id: string; top: number; bottom: number }
interface LayoutOptions   { id: string; layout: number } // 0 | 1 | 2
interface TooltipOptions  { id: string; tooltip: string }
interface SetVisibleOptions { id: string; visible: boolean }
interface PopupWindowOptions { label: string }
interface SetAutoPopupOptions { enabled: boolean }
interface SetColorsOptions   { id: string; top: ColorStyle; bottom: ColorStyle }
interface SetMenuOptions     { id: string; items?: MenuItemDescriptor[] } // null/omit = detach
interface Rect   { x: number; y: number; width: number; height: number }
interface VisibilityResult { visible: boolean }

interface ClickEvent  { id: string; position: { x: number; y: number }; rect: Rect; button: "left" | "right"; buttonState: "up" | "down" }
interface HoverEvent  { id: string; rect: Rect }
interface PopupEvent  { id: string; window: string }
interface ReadyEvent  { id: string }
interface MenuSelectionEvent { id: string; itemId: string; checked?: boolean }
```

### `ColorStyle`

```ts
type ColorStyle =
  | { type: "default" }                 // system textColor (follows light/dark mode)
  | { type: "solid"; value: string }    // "#rrggbb"
```

### `MenuItemDescriptor`

```ts
type MenuItemDescriptor =
  | { type: "item"; id: string; text: string; accelerator?: string; disabled?: boolean }
  | { type: "check"; id: string; text: string; checked?: boolean; accelerator?: string }
  | { type: "separator" }
  | { type: "submenu"; text: string; items: MenuItemDescriptor[] }
```

The `id` becomes the menu item's `MenuId` and is reported back as `itemId`
on the instance's `menu` event. `check` items include `checked` (the state
after the toggle) in the event payload.

### `FONT_SIZE_RANGE`

```ts
export const FONT_SIZE_RANGE = {
  small:  { min: 5, max: 11 },
  large:  { min: 8, max: 16 },
  equal:  { min: 5, max: 11 },
} as const;
```

---

## Layout modes

`setLayout({ id, layout })` selects the vertical arrangement. Sizes are stored
**per role** (emphasized vs de-emphasized), so switching between the two
asymmetric modes mirrors the content without losing either size.

| `layout` | Name | Description | Font range (per role) |
| --- | --- | --- | --- |
| `0` | EmphasisBottom (default) | Small label on top, large value below. | small top 5–11, large bottom 8–16 |
| `1` | EmphasisTop | The vertical mirror — large value on top, small label below. | small bottom 5–11, large top 8–16 |
| `2` | Equal | Both lines share one size, vertically centered & symmetric. | equal 5–11 (default `9`) |

In `Equal` mode, pass equal `top` and `bottom` to `setFontSizes`.

---

## Raw invoke commands

The plugin command prefix is `plugin:multiline-menubar|`. Every command takes
a single `payload` argument.

| Command | Payload |
| --- | --- |
| `create` | `{ id, top?, bottom? }` |
| `remove` | `{ id }` |
| `set_text` | `{ id, top, bottom }` |
| `set_font_sizes` | `{ id, top, bottom }` |
| `set_layout` | `{ id, layout }` |
| `set_tooltip` | `{ id, tooltip }` |
| `set_visible` | `{ id, visible }` |
| `set_colors` | `{ id, top, bottom }` (`top`/`bottom` are `ColorStyle` JSON) |
| `set_menu` | `{ id, items? }` (`items: null`/omitted detaches) |
| `remove_menu` | `{ id }` |
| `rect` | `{ id }` |
| `is_visible` | `{ id }` |
| `set_popup_window` | `{ label }` |
| `set_auto_popup` | `{ enabled }` |
| `open_popup` | `{ id }` |
| `close_popup` | `{ id }` |
| `toggle_popup` | `{ id }` |

---

## Rust API

The plugin exposes a trait `MultilineMenubarExt` (implemented for anything that
is a `Manager`, e.g. `AppHandle`/`App`/`Window`). Methods mirror the commands
above:

| Trait method | Maps to command |
| --- | --- |
| `create(id)` | `create` |
| `remove(id)` | `remove` |
| `set_text(id, top, bottom)` | `set_text` |
| `set_font_sizes(id, top, bottom)` | `set_font_sizes` |
| `set_layout(id, layout)` | `set_layout` |
| `set_tooltip(id, tooltip)` | `set_tooltip` |
| `set_visible(id, visible)` | `set_visible` |
| `set_colors(id, top, bottom)` | `set_colors` |
| `set_menu(id, items)` | `set_menu` |
| `remove_menu(id)` | `remove_menu` |
| `rect(id) -> Rect` | `rect` |
| `is_visible(id) -> bool` | `is_visible` |
| `set_popup_window(label)` | `set_popup_window` |
| `set_auto_popup(enabled)` | `set_auto_popup` |
| `open_popup(id)` | `open_popup` |
| `close_popup(id)` | `close_popup` |
| `toggle_popup(id)` | `toggle_popup` |

> Internally the desktop (macOS) implementation calls native C symbols
> `multiline_menubar_destroy` / `multiline_menubar_get_rect` etc. — those
> names are stable and not affected by the JS/Rust rename.

---

## Permissions

The `default` permission set grants all of:

`allow-create`, `allow-remove`, `allow-set-text`, `allow-set-font-sizes`,
`allow-set-layout`, `allow-set-tooltip`, `allow-set-visible`, `allow-set-menu`,
`allow-remove-menu`, `allow-set-colors`, `allow-rect`, `allow-set-popup-window`,
`allow-set-auto-popup`, `allow-open-popup`, `allow-close-popup`,
`allow-toggle-popup`, `allow-is-visible`.

Each command also has a `deny-*` counterpart (e.g. `multiline-menubar:deny-rect`)
for explicit deny-listing. All identifiers are scoped under
`multiline-menubar:`.

---

## Notes

- The plugin owns its own `NSStatusItem`; it does not extend Tauri's built-in
  system tray / tray icon.
- Text color follows `NSColor.textColor` by default, adapting to light/dark
  mode and accessibility settings.
- Popup positioning assumes the status item is on the primary monitor.
- `transparent: true` popup windows need `"app": { "macOSPrivateApi": true }`
  in `tauri.conf.json` for a clean frosted background on macOS.
