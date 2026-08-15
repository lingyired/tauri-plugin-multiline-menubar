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
npm run tauri:dev
```

## Dev vs release bundle identity (macOS 26)

`examples/demo` must use **different** bundle IDs and app names for dev and
release builds, or the two builds pollute each other's state on macOS 26.

Background: macOS 26 Control Center keeps a "recently used apps" list and
remembers menu-bar visibility **per bundle ID**; that remembered state
survives app updates and is keyed by the bundle ID, not the binary. If dev
and release share a bundle ID, one run can mark the item hidden for the other
— the app launches but **no menu bar item appears at all**, and toggling in
System Settings may not reliably recover it because both builds keep
overwriting the same remembered state. Reference:
https://b-log.to/tech-analysis/macos-26-controlcenter-trackedapplications-ghost/

Rules:

- Dev runs (`npm run tauri:dev`, i.e. `tauri dev --config
  src-tauri/tauri.conf.dev.json`) get a **dev-suffixed identity**:
  `productName = multiline-menubar-demo-new-dev`,
  `identifier = com.tauri.multiline-menubar-demo-new.dev`.
  Tauri v2 merges `tauri.conf.dev.json` only via the explicit `--config`
  flag (JSON Merge Patch, RFC 7396) — it is **not** loaded automatically.
- Release runs (`npm run tauri:build`, i.e. `tauri build`) keep the base
  `tauri.conf.json` identity. Never change the release ID to match dev.
- Any new demo/app must use its own distinct identifier; never reuse another
  app's bundle ID.

Troubleshooting when the status item never appears after launching:

1. Quit the app, then open **System Settings → Menu Bar** and toggle the
   app's entry off and on again.
2. Make sure no other build (dev vs release, or an older copy of the app)
   shares the same bundle ID — conflicting IDs keep re-hiding each other.
3. If it still won't show, rebuild the release app with a **brand-new
   identifier** to rule out stale remembered state in Control Center.

## Versioning
When the plugin's API changes, bump `Cargo.toml` **and** `package.json`
versions together.

## Weight vs layout
Font weight is per-line overridable via `set_bold` (true = force bold,
false = follow `layout`). Font size and color are already per-line. Keep new
per-line styling options orthogonal to `layout`, `setFontSizes`, and
`setColors`.
