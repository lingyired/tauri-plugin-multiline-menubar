# CLAUDE.md — Project conventions

## New APIs must be demonstrated in the demo
Every public API/command added to this plugin MUST also be surfaced in
`examples/demo`, so it can be manually tested. This is a hard requirement, not
optional — the demo doubles as the manual test harness for the plugin.

Concretely, when you add a command:

1. Add the command in `src/commands.rs` and register it in `src/lib.rs`
   (`generate_handler!`). If the desktop (macOS) path needs native work, also
   update `src/native/multiline_menubar.{h,mm}` and the `desktop.rs` FFI/impl.
2. Add a corresponding control in `examples/demo/src/index.html` (a new card
   or fields) **and** wire it in `examples/demo/src/main.js`, calling
   `plugin:multiline-menubar|<command>` with the right payload.
3. Keep the demo in sync with the API surface: if a command later gains new
   options, the demo control should expose them too (don't leave the demo
   stale relative to the API).

Reference pattern: the **Bold** card in the demo (`set_bold`) — a per-line
boolean toggle (`top`/`bottom`) wired to `plugin:multiline-menubar|set_bold`.
New per-line options should follow the same shape (independent top/bottom
controls).

The demo already references the local plugin via `path = "../../.."` in
`src-tauri/Cargo.toml` and uses the `multiline-menubar:default` capability set,
so newly added `allow-*` permissions are picked up automatically. Run it with:

```bash
cd examples/demo
npm install
npm run tauri dev
```

## Versioning
When the plugin's API changes, bump `Cargo.toml` **and** `package.json`
versions together.

## Weight vs layout
Font weight is per-line overridable via `set_bold` (true = force bold,
false = follow `layout`). Font size and color are already per-line. Keep new
per-line styling options orthogonal to `layout`, `setFontSizes`, and
`setColors`.
