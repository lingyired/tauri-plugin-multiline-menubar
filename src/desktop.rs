use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;
use tauri::Wry;
use tauri::{
    plugin::PluginApi, Emitter, Manager, Position, Runtime, WebviewWindow, WindowEvent,
};

#[cfg(target_os = "macos")]
use muda::{
    accelerator::Accelerator, CheckMenuItem as MudaCheckMenuItem,
    ContextMenu as MudaContextMenu, IsMenuItem as MudaIsMenuItem, Menu as MudaMenu,
    MenuItem as MudaMenuItem, MenuItemKind as MudaMenuItemKind,
    PredefinedMenuItem as MudaPredefinedMenuItem, Submenu as MudaSubmenu,
};

use crate::models::*;

#[cfg(target_os = "macos")]
extern "C" {
    fn multiline_menubar_create(id: *const c_char);
    fn multiline_menubar_destroy(id: *const c_char);
    fn multiline_menubar_show(id: *const c_char);
    fn multiline_menubar_hide(id: *const c_char);
    fn multiline_menubar_set_text(id: *const c_char, top: *const c_char, bottom: *const c_char);
    fn multiline_menubar_set_style(id: *const c_char, top_size: f64, bottom_size: f64);
    fn multiline_menubar_set_layout(id: *const c_char, layout: std::os::raw::c_int);
    fn multiline_menubar_set_tooltip(id: *const c_char, tooltip: *const c_char);
    fn multiline_menubar_set_menu(id: *const c_char, ns_menu: *mut std::ffi::c_void);
    fn multiline_menubar_set_color(
        id: *const c_char,
        top: *const c_char,
        bottom: *const c_char,
    );
    fn multiline_menubar_set_bold(
        id: *const c_char,
        top_bold: std::os::raw::c_int,
        bottom_bold: std::os::raw::c_int,
    );
    fn multiline_menubar_set_font_family(
        id: *const c_char,
        top: *const c_char,
        bottom: *const c_char,
    );
    fn multiline_menubar_set_monospaced(
        id: *const c_char,
        top_monospaced: std::os::raw::c_int,
        bottom_monospaced: std::os::raw::c_int,
    );
    fn multiline_menubar_set_alignment(
        id: *const c_char,
        top_align: std::os::raw::c_int,
        bottom_align: std::os::raw::c_int,
    );
    fn multiline_menubar_get_rect(
        id: *const c_char,
        x: *mut f64,
        y: *mut f64,
        width: *mut f64,
        height: *mut f64,
    ) -> std::os::raw::c_int;
    fn multiline_menubar_is_visible(id: *const c_char) -> std::os::raw::c_int;
    fn multiline_menubar_set_click_handler(
        callback: Option<
            extern "C" fn(*const c_char, *const c_char, f64, f64, f64, f64, f64, f64),
        >,
    );
    fn multiline_menubar_set_hover_handler(
        callback: Option<extern "C" fn(*const c_char, *const c_char)>,
    );
    fn multiline_menubar_set_remove_handler(
        callback: Option<extern "C" fn(*const c_char)>,
    );
}

// ---------------------------------------------------------------------------
// Shared state
// ---------------------------------------------------------------------------

/// App handle stored as the concrete runtime (Wry) so the `extern "C"` native
/// callbacks can emit events and manage the popup window.
static APP_HANDLE: OnceLock<tauri::AppHandle<Wry>> = OnceLock::new();

#[cfg(target_os = "macos")]
thread_local! {
    /// Context menus, keyed by instance id.
    ///
    /// `muda::Menu` holds `Rc`s, so it is neither `Send` nor `Sync` and must
    /// be created and dropped on the main thread. Keeping the values here
    /// serves two purposes: the menu stays alive for as long as the instance
    /// needs it, and replacing an instance's menu drops the previous one on
    /// the thread that owns it.
    static MAIN_THREAD_MENUS: RefCell<HashMap<String, MudaMenu>> =
        RefCell::new(HashMap::new());
}

/// Maps a menu item id to the menubar instance that owns it.
///
/// muda dispatches menu selections through a process-global handler that only
/// reports the item's `MenuId`, with no notion of which menu it came from.
/// This table restores that association so selections can be re-emitted on the
/// owning instance's event channel. Item ids are expected to be unique across
/// instances; if two instances register the same id, the later one wins.
#[cfg(target_os = "macos")]
static MENU_ITEM_OWNERS: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);

/// Label of the Tauri window used as the popup (default "popup").
static POPUP_WINDOW: RwLock<Option<Arc<str>>> = RwLock::new(None);

/// Whether a left click automatically toggles the popup window.
static AUTO_POPUP: Mutex<bool> = Mutex::new(true);

/// While set, the popup's blur handler ignores focus loss (prevents the
/// popup from immediately closing when it opens and steals focus).
static POPUP_IGNORE_BLUR_UNTIL: Mutex<Option<Instant>> = Mutex::new(None);

/// Ensures the popup auto-hide handler is attached only once.
static POPUP_HANDLER_ATTACHED: Mutex<bool> = Mutex::new(false);

/// Last top/bottom text set per instance, so the popup can be pre-filled with
/// the values of whichever instance opened it (rather than showing static
/// placeholder content).
///
/// One map holds all per-instance remembered state (text, font sizes, bold
/// toggles, layout) instead of four separate globals, so reading it for the
/// popup takes a single lock. Text is stored as `Arc<str>` so the popup can
/// borrow it without copying, and setters compare against the previous value
/// to skip redundant native round-trips.
///
/// NOTE: this is the Rust-side *memory* of what was sent to the native layer;
/// the `MenubarInstance` in Objective-C holds the authoritative rendering
/// state (text, sizes, colors, bold, layout). The two are kept in sync
/// because every public setter goes through this map before calling into
/// native code — do not bypass it.
///
/// Access via [`instances()`]; `HashMap::new` is not const, so the map is
/// initialized lazily.
fn instances() -> &'static Mutex<HashMap<String, InstanceState>> {
    static INSTANCES: OnceLock<Mutex<HashMap<String, InstanceState>>> = OnceLock::new();
    INSTANCES.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Per-instance state remembered so the popup can pre-fill with the values of
/// whichever instance opened it.
#[derive(Clone)]
struct InstanceState {
    /// Last top/bottom text.
    text: (Arc<str>, Arc<str>),
    /// Last top/bottom font size.
    style: (f64, f64),
    /// Last top/bottom bold toggle.
    weight: (bool, bool),
    /// Last top/bottom font family. `None` => system font.
    font_family: (Option<Arc<str>>, Option<Arc<str>>),
    /// Last top/bottom monospaced-digit toggle.
    monospaced: (bool, bool),
    /// Last top/bottom horizontal alignment (0 = left, 1 = center, 2 = right).
    alignment: (i32, i32),
    /// Last layout mode (0 = stacked, 1 = balanced).
    layout: i32,
}

impl Default for InstanceState {
    fn default() -> Self {
        Self {
            text: (Arc::from(""), Arc::from("")),
            style: (7.0, 12.0),
            weight: (false, false),
            font_family: (None, None),
            monospaced: (false, false),
            alignment: (0, 0),
            layout: 0,
        }
    }
}

/// Event name used to tell the popup window which instance opened it and what
/// that instance's current text is. Delivered with `emit_to` so only the popup
/// window receives it.
#[cfg(target_os = "macos")]
const POPUP_OPEN_TARGET_EVENT: &str = "multiline-menubar://popup//open";

// ---------------------------------------------------------------------------
// Native callbacks
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
extern "C" fn on_native_click(
    id: *const c_char,
    button: *const c_char,
    rx: f64,
    ry: f64,
    rw: f64,
    rh: f64,
    cx: f64,
    cy: f64,
) {
    let id_str = cstr_to_str(id, "");
    let button_str = cstr_to_str(button, "left");

    if let Some(app) = APP_HANDLE.get() {
        // Payload mirrors Tauri's own `TrayIconEvent::Click`.
        let _ = app.emit(
            format!("multiline-menubar://{}//click", id_str).as_str(),
            serde_json::json!({
                "id": id_str,
                "position": { "x": cx, "y": cy },
                "rect": { "x": rx, "y": ry, "width": rw, "height": rh },
                "button": button_str,
                "buttonState": "up",
            }),
        );

        // The native layer pops the context menu on right click; only a left
        // click drives the popup window.
        let auto = *AUTO_POPUP.lock().unwrap_or_else(|e| e.into_inner());
        if auto && button_str == "left" {
            let _ = toggle_popup_window(app, id_str);
        }
    }
}

#[cfg(target_os = "macos")]
extern "C" fn on_native_hover(id: *const c_char, hover_type: *const c_char) {
    let id_str = cstr_to_str(id, "");
    let type_str = cstr_to_str(hover_type, "");

    if type_str.is_empty() {
        return;
    }

    if let Some(app) = APP_HANDLE.get() {
        // Mirrors Tauri's `Enter`/`Leave` payload shape ({ id, rect }).
        let rect = get_instance_rect(id_str).unwrap_or((0.0, 0.0, 0.0, 0.0));
        let _ = app.emit(
            format!("multiline-menubar://{}//{}", id_str, type_str).as_str(),
            serde_json::json!({
                "id": id_str,
                "rect": { "x": rect.0, "y": rect.1, "width": rect.2, "height": rect.3 },
            }),
        );
    }
}

/// Fired when the user removes the status item (⌘-drag out of the menu bar).
/// Emits `multiline-menubar://{id}//remove` so the frontend can react.
#[cfg(target_os = "macos")]
extern "C" fn on_native_remove(id: *const c_char) {
    let id_str = cstr_to_str(id, "");

    if let Some(app) = APP_HANDLE.get() {
        let _ = app.emit(
            format!("multiline-menubar://{}//remove", id_str).as_str(),
            serde_json::json!({ "id": id_str }),
        );
    }
}

#[cfg(target_os = "macos")]
fn cstr_to_str<'a>(ptr: *const c_char, fallback: &'a str) -> &'a str {
    if ptr.is_null() {
        return fallback;
    }
    unsafe { CStr::from_ptr(ptr) }.to_str().unwrap_or(fallback)
}

/// Fetch the on-screen rect of an instance from the native layer.
#[cfg(target_os = "macos")]
fn get_instance_rect(id: &str) -> Option<(f64, f64, f64, f64)> {
    let id_c = CString::new(id).ok()?;
    let mut x = 0.0;
    let mut y = 0.0;
    let mut w = 0.0;
    let mut h = 0.0;
    let ok =
        unsafe { multiline_menubar_get_rect(id_c.as_ptr(), &mut x, &mut y, &mut w, &mut h) };
    if ok != 0 {
        Some((x, y, w, h))
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// Menu ownership bookkeeping
// ---------------------------------------------------------------------------

/// Menu item ids whose selection terminates the whole app. Centralized so a
/// host can adjust the set in one place; note that these ids are then
/// unavailable for regular menu items.
const QUIT_ITEM_IDS: &[&str] = &["quit", "quit2"];

/// Max nesting depth of menu submenus (guards the recursive menu builders
/// against stack exhaustion).
const MAX_MENU_DEPTH: usize = 8;
/// Max total number of items in one menu, including nested submenu items.
const MAX_MENU_ITEMS: usize = 100;
/// Max length (bytes) of a menu item text.
const MAX_MENU_TEXT_BYTES: usize = 256;

/// Max length (bytes) of a single line of menubar text.
const MAX_TEXT_BYTES: usize = 1024;

/// Validate the descriptor tree before it is consumed by the (recursive)
/// menu builders: enforce nesting depth, total item count and text lengths.
/// The accelerators are validated separately by `validate_accelerators`.
#[cfg(target_os = "macos")]
fn validate_menu_tree(items: &[MenuItemDescriptor]) -> crate::Result<()> {
    fn walk(
        items: &[MenuItemDescriptor],
        depth: usize,
        count: &mut usize,
    ) -> crate::Result<()> {
        if depth > MAX_MENU_DEPTH {
            return Err(crate::Error::InvalidArgument(format!(
                "menu nesting exceeds {MAX_MENU_DEPTH} levels"
            )));
        }
        for item in items {
            *count += 1;
            if *count > MAX_MENU_ITEMS {
                return Err(crate::Error::InvalidArgument(format!(
                    "menu exceeds {MAX_MENU_ITEMS} items"
                )));
            }
            match item {
                MenuItemDescriptor::Item { text, .. }
                | MenuItemDescriptor::Check { text, .. } => {
                    if text.len() > MAX_MENU_TEXT_BYTES {
                        return Err(crate::Error::InvalidArgument(format!(
                            "menu item text exceeds {MAX_MENU_TEXT_BYTES} bytes"
                        )));
                    }
                }
                MenuItemDescriptor::Separator => {}
                MenuItemDescriptor::Submenu { text, items } => {
                    if text.len() > MAX_MENU_TEXT_BYTES {
                        return Err(crate::Error::InvalidArgument(format!(
                            "submenu text exceeds {MAX_MENU_TEXT_BYTES} bytes"
                        )));
                    }
                    walk(items, depth + 1, count)?;
                }
            }
        }
        Ok(())
    }

    let mut count = 0;
    walk(items, 0, &mut count)
}

/// Collect the ids of every selectable item in a descriptor tree.
/// Separators are skipped (muda assigns them throwaway ids) and submenus
/// contribute their children rather than themselves.
#[cfg(target_os = "macos")]
fn collect_item_ids(items: &[MenuItemDescriptor], out: &mut Vec<String>) {
    for item in items {
        match item {
            MenuItemDescriptor::Item { id, .. } => out.push(id.clone()),
            MenuItemDescriptor::Check { id, .. } => out.push(id.clone()),
            MenuItemDescriptor::Separator => {}
            MenuItemDescriptor::Submenu { items, .. } => collect_item_ids(items, out),
        }
    }
}

/// Point every given item id at `instance_id`, dropping any ids the instance
/// previously owned (so replacing a menu does not leak stale entries).
#[cfg(target_os = "macos")]
fn register_menu_owners(instance_id: &str, item_ids: Vec<String>) {
    let mut guard = MENU_ITEM_OWNERS
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let owners = guard.get_or_insert_with(HashMap::new);
    owners.retain(|_, owner| owner != instance_id);
    for id in item_ids {
        owners.insert(id, instance_id.to_string());
    }
}

/// Forget every item id owned by an instance.
#[cfg(target_os = "macos")]
fn unregister_menu_owners(instance_id: &str) {
    let mut guard = MENU_ITEM_OWNERS
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(owners) = guard.as_mut() {
        owners.retain(|_, owner| owner != instance_id);
    }
}

/// Which instance owns this menu item id, if any.
#[cfg(target_os = "macos")]
fn menu_owner_of(item_id: &str) -> Option<String> {
    let guard = MENU_ITEM_OWNERS.lock().unwrap_or_else(|e| e.into_inner());
    guard.as_ref()?.get(item_id).cloned()
}

/// Current checked state of a check item, or `None` if the id is not a check
/// item (or the menu is not reachable from the calling thread).
#[cfg(target_os = "macos")]
fn menu_item_checked(instance_id: &str, item_id: &str) -> Option<bool> {
    fn walk(items: &[MudaMenuItemKind], item_id: &str) -> Option<bool> {
        for item in items {
            match item {
                MudaMenuItemKind::Check(check) if check.id().0 == item_id => {
                    return Some(check.is_checked());
                }
                MudaMenuItemKind::Submenu(sub) => {
                    if let Some(found) = walk(&sub.items(), item_id) {
                        return Some(found);
                    }
                }
                _ => {}
            }
        }
        None
    }

    // Menus live in a main-thread `thread_local`; `on_menu_event` listeners run
    // on the event loop thread, which on macOS is the main thread. If that ever
    // stops holding, the lookup simply yields `None` instead of misbehaving.
    MAIN_THREAD_MENUS.with(|menus| {
        let menus = menus.borrow();
        let menu = menus.get(instance_id)?;
        walk(&menu.items(), item_id)
    })
}

// ---------------------------------------------------------------------------
// Menu building
// ---------------------------------------------------------------------------

/// Validate every accelerator string up front so `set_menu` can report a
/// useful error synchronously (the menu itself is built on the main thread,
/// where returning an error is no longer possible).
#[cfg(target_os = "macos")]
fn validate_accelerators(items: &[MenuItemDescriptor]) -> crate::Result<()> {
    fn check(value: &Option<String>) -> crate::Result<()> {
        if let Some(raw) = value {
            raw.parse::<Accelerator>()
                .map_err(|e| crate::Error::Menu(format!("invalid accelerator {raw:?}: {e}")))?;
        }
        Ok(())
    }

    for item in items {
        match item {
            MenuItemDescriptor::Item { accelerator, .. } => check(accelerator)?,
            MenuItemDescriptor::Check { accelerator, .. } => check(accelerator)?,
            MenuItemDescriptor::Separator => {}
            MenuItemDescriptor::Submenu { items, .. } => validate_accelerators(items)?,
        }
    }
    Ok(())
}

/// Build a muda menu item from a descriptor, recursing into submenus.
///
/// muda is used directly rather than `tauri::menu` because only muda exposes
/// the underlying `NSMenu` pointer publicly (`ContextMenu::ns_menu`), which is
/// what lets the native layer anchor the menu under the status item. Menu
/// selections are unaffected: muda dispatches them through a process-global
/// handler that Tauri installs at startup, so they still arrive in
/// `on_menu_event` keyed by the `id` given here.
#[cfg(target_os = "macos")]
fn build_muda_item(desc: MenuItemDescriptor) -> crate::Result<Box<dyn MudaIsMenuItem>> {
    fn accel(value: Option<String>) -> Option<Accelerator> {
        value.and_then(|raw| raw.parse::<Accelerator>().ok())
    }

    match desc {
        MenuItemDescriptor::Item {
            id,
            text,
            accelerator,
            disabled,
        } => Ok(Box::new(MudaMenuItem::with_id(
            id,
            text,
            !disabled.unwrap_or(false),
            accel(accelerator),
        ))),
        MenuItemDescriptor::Check {
            id,
            text,
            checked,
            accelerator,
        } => Ok(Box::new(MudaCheckMenuItem::with_id(
            id,
            text,
            true,
            checked.unwrap_or(false),
            accel(accelerator),
        ))),
        MenuItemDescriptor::Separator => Ok(Box::new(MudaPredefinedMenuItem::separator())),
        MenuItemDescriptor::Submenu { text, items } => {
            let children = items
                .into_iter()
                .map(build_muda_item)
                .collect::<crate::Result<Vec<_>>>()?;
            let refs: Vec<&dyn MudaIsMenuItem> = children.iter().map(|b| b.as_ref()).collect();
            let submenu = MudaSubmenu::with_items(text, true, &refs)
                .map_err(|e| crate::Error::Menu(e.to_string()))?;
            Ok(Box::new(submenu))
        }
    }
}

// ---------------------------------------------------------------------------
// Popup window helpers
// ---------------------------------------------------------------------------

/// Position a popup window directly below the status item on the primary
/// monitor, centered horizontally on the item.
#[cfg(target_os = "macos")]
fn position_popup_under_status_item(
    app: &tauri::AppHandle<Wry>,
    win: &WebviewWindow,
    rect: (f64, f64, f64, f64),
) -> crate::Result<()> {
    let (rx, ry, rw, _rh) = rect;
    if let Ok(Some(monitor)) = app.primary_monitor() {
        let scale = win.scale_factor().unwrap_or(1.0);
        let outer = win.outer_size().unwrap_or_default();
        let win_w = outer.width as f64 / scale;
        let win_h = outer.height as f64 / scale;

        let msize = monitor.size();
        let mscale = monitor.scale_factor();
        let screen_w = msize.width as f64 / mscale;
        let screen_h = msize.height as f64 / mscale;

        // Center the popup horizontally under the status item.
        let mut tauri_x = rx + rw / 2.0 - win_w / 2.0;
        if tauri_x < 0.0 {
            tauri_x = 0.0;
        }
        let max_x = screen_w - win_w;
        if tauri_x > max_x {
            tauri_x = max_x.max(0.0);
        }

        // The menu bar sits at the top of the screen. Place the popup's top
        // edge at the bottom of the status item. macOS y grows upward,
        // Tauri y grows downward, so flip it.
        let tauri_y = screen_h - ry - win_h;

        let _ = win.set_position(Position::Logical(tauri::LogicalPosition::new(
            tauri_x, tauri_y,
        )));
    }
    Ok(())
}

/// Show the popup window anchored under the given instance.
#[cfg(target_os = "macos")]
fn open_popup_window(app: &tauri::AppHandle<Wry>, id: &str) -> crate::Result<()> {
    let label = popup_label();
    if let Some(win) = app.get_webview_window(label.as_ref()) {
        let rect = get_instance_rect(id).unwrap_or((0.0, 0.0, 0.0, 0.0));
        position_popup_under_status_item(app, &win, rect)?;
        attach_auto_hide(app, &win, label.as_ref());
        *POPUP_IGNORE_BLUR_UNTIL
            .lock()
            .unwrap_or_else(|e| e.into_inner()) =
            Some(Instant::now() + Duration::from_millis(200));
        let _ = win.show();
        let _ = win.set_focus();
        let _ = app.emit(
            format!("multiline-menubar://{}//popup-open", id).as_str(),
            serde_json::json!({ "id": id, "window": label }),
        );

        // Tell the popup window which instance opened it and what that
        // instance's current text and font sizes are, so it can show
        // instance-specific content instead of static placeholder values.
        let state = instances()
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(id)
            .cloned()
            .unwrap_or_default();
        let _ = app.emit_to(
            &label,
            POPUP_OPEN_TARGET_EVENT,
            serde_json::json!({
                "id": id,
                "top": state.text.0,
                "bottom": state.text.1,
                "topSize": state.style.0,
                "bottomSize": state.style.1,
                "layout": state.layout,
                "topBold": state.weight.0,
                "bottomBold": state.weight.1,
                "topFontFamily": state.font_family.0,
                "bottomFontFamily": state.font_family.1,
                "topMonospaced": state.monospaced.0,
                "bottomMonospaced": state.monospaced.1,
                "topAlign": state.alignment.0,
                "bottomAlign": state.alignment.1
            }),
        );
    }
    Ok(())
}

/// Hide the popup window.
#[cfg(target_os = "macos")]
fn close_popup_window(app: &tauri::AppHandle<Wry>, id: &str) -> crate::Result<()> {
    let label = popup_label();
    if let Some(win) = app.get_webview_window(label.as_ref()) {
        let _ = win.hide();
        let _ = app.emit(
            format!("multiline-menubar://{}//popup-close", id).as_str(),
            serde_json::json!({ "id": id, "window": label }),
        );
    }
    Ok(())
}

/// Toggle the popup window's visibility, anchored under the given instance.
#[cfg(target_os = "macos")]
fn toggle_popup_window(app: &tauri::AppHandle<Wry>, id: &str) -> crate::Result<()> {
    let label = popup_label();
    if let Some(win) = app.get_webview_window(label.as_ref()) {
        if win.is_visible().unwrap_or(false) {
            return close_popup_window(app, id);
        }
        return open_popup_window(app, id);
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn popup_label() -> Arc<str> {
    POPUP_WINDOW
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone()
        .unwrap_or_else(|| Arc::from("popup"))
}

/// Close the popup window when it loses focus (menubar-app behaviour).
/// Attached once to the popup window.
#[cfg(target_os = "macos")]
fn attach_auto_hide(app: &tauri::AppHandle<Wry>, win: &WebviewWindow, label: &str) {
    let mut attached = POPUP_HANDLER_ATTACHED
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if *attached {
        return;
    }
    *attached = true;

    let app = app.clone();
    let label = label.to_string();
    // NOTE: tauri's `on_window_event` returns `()` and offers no API to remove
    // the listener, but the listener lives in a window-scoped handler map that
    // tauri drops when the window is destroyed, so the `app` clone captured
    // here is released then too — nothing leaks for the app's lifetime.
    win.on_window_event(move |event| {
        if let WindowEvent::Focused(false) = event {
            let now = Instant::now();
            let ignore = POPUP_IGNORE_BLUR_UNTIL
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            if let Some(until) = *ignore {
                if now < until {
                    return;
                }
            }
            if let Some(w) = app.get_webview_window(&label) {
                if w.is_visible().unwrap_or(false) {
                    let _ = w.hide();
                    let _ = app.emit(
                        format!("multiline-menubar://{}//popup-close", label).as_str(),
                        serde_json::json!({ "id": label, "window": label }),
                    );
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Plugin init
// ---------------------------------------------------------------------------

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &tauri::AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<MultilineMenubar<R>> {
    #[cfg(target_os = "macos")]
    {
        // `transmute_copy` copies the Arc bytes without bumping the refcount,
        // so clone first to take a real strong reference, then transmute the
        // (now guaranteed-alive) copy. Without the clone, dropping `app` after
        // init could leave `APP_HANDLE` dangling.
        let app_wry: tauri::AppHandle<Wry> = unsafe { std::mem::transmute_copy(&app.clone()) };
        let _ = APP_HANDLE.set(app_wry);

        unsafe {
            multiline_menubar_set_click_handler(Some(on_native_click));
            multiline_menubar_set_hover_handler(Some(on_native_hover));
            multiline_menubar_set_remove_handler(Some(on_native_remove));
        }

        // Menus built here are plain muda menus, so their selections reach
        // Tauri's global `on_menu_event` but never the `menu` plugin's JS
        // channel (that map is only populated for menus created through the
        // `menu` plugin's own commands). Re-emit ours on the owning instance's
        // event channel so the frontend can listen alongside click/hover.
        app.on_menu_event(|app, event| {
            let item_id = event.id().0.as_str();
            let Some(instance_id) = menu_owner_of(item_id) else {
                // Not one of ours — another menu in the app owns this id.
                return;
            };

            // Standard menubar "Quit" entries: terminate the whole app.
            // Handled here in Rust so it works without depending on the
            // frontend, the `tauri-plugin-process` crate, or any extra
            // capability — `window.__TAURI__.app.exit` does not exist in the
            // base `app` module, so a JS-side quit would silently fail.
            //
            // NOTE: hosts must not reuse these ids for regular menu items —
            // selecting them always exits the app. The ids are centralized
            // here so adjusting the set is a one-line change.
            if QUIT_ITEM_IDS.contains(&item_id) {
                app.exit(0);
                return;
            }

            let mut payload = serde_json::json!({
                "id": instance_id,
                "itemId": item_id,
            });
            if let Some(checked) = menu_item_checked(&instance_id, item_id) {
                payload["checked"] = serde_json::Value::Bool(checked);
            }

            let _ = app.emit(
                format!("multiline-menubar://{}//menu", instance_id).as_str(),
                payload,
            );
        });
    }

    Ok(MultilineMenubar(app.clone()))
}

/// Access to the multiline-menubar APIs.
pub struct MultilineMenubar<R: Runtime>(tauri::AppHandle<R>);

// ---------------------------------------------------------------------------
// Instance API
// ---------------------------------------------------------------------------

impl<R: Runtime> MultilineMenubar<R> {
    pub fn create(&self, id: String) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            let c = CString::new(id.as_str())
                .map_err(|_| crate::Error::InvalidArgument("id contains a NUL byte".into()))?;
            unsafe { multiline_menubar_create(c.as_ptr()) };
            if let Some(app) = APP_HANDLE.get() {
                let _ = app.emit(
                    format!("multiline-menubar://{}//ready", id).as_str(),
                    serde_json::json!({ "id": id }),
                );
            }
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = id;
            Err(crate::Error::UnsupportedPlatform)
        }
    }

    pub fn remove(&self, id: String) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            let c = CString::new(id.as_str())
                .map_err(|_| crate::Error::InvalidArgument("id contains a NUL byte".into()))?;
            unsafe { multiline_menubar_destroy(c.as_ptr()) };
            unregister_menu_owners(&id);
            instances()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&id);

            // Drop the menu on the main thread, where it was created.
            if let Some(app) = APP_HANDLE.get() {
                let _ = app.run_on_main_thread(move || {
                    MAIN_THREAD_MENUS.with(|menus| {
                        menus.borrow_mut().remove(&id);
                    });
                });
            }
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = id;
            Err(crate::Error::UnsupportedPlatform)
        }
    }

    pub fn set_text(&self, id: String, top: String, bottom: String) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            if top.len() > MAX_TEXT_BYTES || bottom.len() > MAX_TEXT_BYTES {
                return Err(crate::Error::InvalidArgument(format!(
                    "text exceeds {MAX_TEXT_BYTES} bytes per line"
                )));
            }

            // Remember the text so the popup can show this instance's values,
            // and skip the native round-trip entirely when nothing changed —
            // this is the hot path for e.g. per-second market refreshes.
            let changed = {
                let mut instances = instances().lock().unwrap_or_else(|e| e.into_inner());
                let state = instances.entry(id.clone()).or_default();
                if state.text.0.as_ref() == top && state.text.1.as_ref() == bottom {
                    false
                } else {
                    state.text = (Arc::from(top.as_str()), Arc::from(bottom.as_str()));
                    true
                }
            };
            if changed {
                let id_c = CString::new(id).map_err(|_| {
                    crate::Error::InvalidArgument("id contains a NUL byte".into())
                })?;
                let top_c = CString::new(top).map_err(|_| {
                    crate::Error::InvalidArgument("top contains a NUL byte".into())
                })?;
                let bottom_c = CString::new(bottom).map_err(|_| {
                    crate::Error::InvalidArgument("bottom contains a NUL byte".into())
                })?;
                unsafe {
                    multiline_menubar_set_text(id_c.as_ptr(), top_c.as_ptr(), bottom_c.as_ptr())
                };
            }
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (id, top, bottom);
            Err(crate::Error::UnsupportedPlatform)
        }
    }

    pub fn set_font_sizes(&self, id: String, top: f64, bottom: f64) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            // Remember the sizes so the popup can show this instance's values
            // (mirrors the text bookkeeping above).
            instances()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .entry(id.clone())
                .or_default()
                .style = (top, bottom);

            let c = CString::new(id).map_err(|_| {
                crate::Error::InvalidArgument("id contains a NUL byte".into())
            })?;
            unsafe { multiline_menubar_set_style(c.as_ptr(), top, bottom) };
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (id, top, bottom);
            Err(crate::Error::UnsupportedPlatform)
        }
    }

    /// Choose the vertical layout for an instance. The two asymmetric modes
    /// are exact vertical mirrors that keep the emphasized (large, regular
    /// weight) and de-emphasized (small, light weight) lines, only swapping
    /// which one is on top:
    ///   * 0 = emphasis-bottom (default): small label on top, large value below.
    ///   * 1 = emphasis-top: the mirror — large value on top, small label below.
    ///   * 2 = equal: both lines share one size, vertically centered & symmetric.
    /// Sizes are stored per role, so switching layouts never loses a value,
    /// and the chosen layout is remembered per instance so the popup can
    /// pre-select it on open.
    pub fn set_layout(&self, id: String, layout: i32) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            instances()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .entry(id.clone())
                .or_default()
                .layout = layout;

            let c = CString::new(id).map_err(|_| {
                crate::Error::InvalidArgument("id contains a NUL byte".into())
            })?;
            unsafe { multiline_menubar_set_layout(c.as_ptr(), layout as std::os::raw::c_int) };
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (id, layout);
            Err(crate::Error::UnsupportedPlatform)
        }
    }

    pub fn set_tooltip(&self, id: String, tooltip: String) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            let id_c = CString::new(id)
                .map_err(|_| crate::Error::InvalidArgument("id contains a NUL byte".into()))?;
            let c = CString::new(tooltip)
                .map_err(|_| crate::Error::InvalidArgument("tooltip contains a NUL byte".into()))?;
            unsafe { multiline_menubar_set_tooltip(id_c.as_ptr(), c.as_ptr()) };
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (id, tooltip);
            Err(crate::Error::UnsupportedPlatform)
        }
    }

    /// Build a context menu from the descriptors and attach it to the
    /// instance. A right click on the item pops it up underneath the status
    /// item; selecting an entry surfaces in Tauri's global `on_menu_event`
    /// with the `id` given in the descriptor.
    pub fn set_menu(&self, id: String, items: Vec<MenuItemDescriptor>) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            // Fail fast on bad accelerators and oversized menus while we can
            // still return an error.
            validate_accelerators(&items)?;
            validate_menu_tree(&items)?;

            let app = APP_HANDLE
                .get()
                .ok_or(crate::Error::UnsupportedPlatform)?
                .clone();

            // Record which instance owns these item ids before the descriptors
            // are consumed, so selections can be routed back to this instance.
            let mut item_ids = Vec::new();
            collect_item_ids(&items, &mut item_ids);
            register_menu_owners(&id, item_ids);

            // muda menus must be constructed on the main thread.
            app.run_on_main_thread(move || {
                let built = match items
                    .into_iter()
                    .map(build_muda_item)
                    .collect::<crate::Result<Vec<_>>>()
                {
                    Ok(built) => built,
                    Err(e) => {
                        eprintln!("[multiline-menubar] failed to build menu items: {e}");
                        return;
                    }
                };
                let refs: Vec<&dyn MudaIsMenuItem> =
                    built.iter().map(|b| b.as_ref()).collect();
                let menu = match MudaMenu::with_items(&refs) {
                    Ok(menu) => menu,
                    Err(e) => {
                        eprintln!("[multiline-menubar] failed to build menu: {e}");
                        return;
                    }
                };

                // Hand the NSMenu to the native layer, which retains it.
                if let Ok(id_c) = CString::new(id.as_str()) {
                    unsafe { multiline_menubar_set_menu(id_c.as_ptr(), menu.ns_menu()) };
                }

                // Retain the Rust-side menu; this also drops any previous one
                // for this id on the main thread.
                MAIN_THREAD_MENUS.with(|menus| {
                    menus.borrow_mut().insert(id, menu);
                });
            })?;
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (id, items);
            Err(crate::Error::UnsupportedPlatform)
        }
    }

    /// Remove the context menu attached to an instance.
    pub fn remove_menu(&self, id: String) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            let c = CString::new(id.as_str())
                .map_err(|_| crate::Error::InvalidArgument("id contains a NUL byte".into()))?;
            unsafe { multiline_menubar_set_menu(c.as_ptr(), std::ptr::null_mut()) };
            unregister_menu_owners(&id);
            if let Some(app) = APP_HANDLE.get() {
                let _ = app.run_on_main_thread(move || {
                    MAIN_THREAD_MENUS.with(|menus| {
                        menus.borrow_mut().remove(&id);
                    });
                });
            }
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = id;
            Err(crate::Error::UnsupportedPlatform)
        }
    }

    /// Set the text paint for the top and bottom lines. `top`/`bottom` are
    /// `ColorStyle` values; they are serialized to the small JSON shape the
    /// native layer parses (`{"type":"default"|"solid", ...}`).
    pub fn set_colors(
        &self,
        id: String,
        top: crate::models::ColorStyle,
        bottom: crate::models::ColorStyle,
    ) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            let top_json =
                serde_json::to_string(&top).map_err(|e| crate::Error::Menu(e.to_string()))?;
            let bottom_json = serde_json::to_string(&bottom)
                .map_err(|e| crate::Error::Menu(e.to_string()))?;

            let id_c = CString::new(id)
                .map_err(|_| crate::Error::InvalidArgument("id contains a NUL byte".into()))?;
            let top_c = CString::new(top_json)
                .map_err(|_| crate::Error::InvalidArgument("top color contains a NUL byte".into()))?;
            let bottom_c = CString::new(bottom_json).map_err(|_| {
                crate::Error::InvalidArgument("bottom color contains a NUL byte".into())
            })?;
            unsafe {
                multiline_menubar_set_color(id_c.as_ptr(), top_c.as_ptr(), bottom_c.as_ptr());
            }
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (id, top, bottom);
            Err(crate::Error::UnsupportedPlatform)
        }
    }

    /// Set the per-line bold toggle for the top and bottom lines. `top_bold` /
    /// `bottom_bold` being `true` forces that line to render bold, overriding
    /// the weight `layout` would otherwise assign it. `false` leaves the line's
    /// weight to the layout.
    pub fn set_bold(
        &self,
        id: String,
        top_bold: bool,
        bottom_bold: bool,
    ) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            // Remember the toggle so the popup can pre-fill this instance's
            // value (mirrors the style bookkeeping for font sizes).
            instances()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .entry(id.clone())
                .or_default()
                .weight = (top_bold, bottom_bold);

            let c = CString::new(id).map_err(|_| {
                crate::Error::InvalidArgument("id contains a NUL byte".into())
            })?;
            unsafe {
                multiline_menubar_set_bold(
                    c.as_ptr(),
                    top_bold as std::os::raw::c_int,
                    bottom_bold as std::os::raw::c_int,
                )
            };
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (id, top_bold, bottom_bold);
            Err(crate::Error::UnsupportedPlatform)
        }
    }

    /// Set the per-line font family for the top and bottom lines. Each is a
    /// macOS font *family* name (e.g. `"Helvetica"`); `None` or
    /// an empty string falls back to the system font for that line.
    pub fn set_font_family(
        &self,
        id: String,
        top: Option<String>,
        bottom: Option<String>,
    ) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            // Remember the families so the popup can pre-fill this instance's
            // values (mirrors the style bookkeeping for font sizes).
            instances()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .entry(id.clone())
                .or_default()
                .font_family = (top.as_deref().map(Arc::from), bottom.as_deref().map(Arc::from));

            let id_c = CString::new(id).map_err(|_| {
                crate::Error::InvalidArgument("id contains a NUL byte".into())
            })?;
            let top_c = CString::new(top.unwrap_or_default()).map_err(|_| {
                crate::Error::InvalidArgument("top font family contains a NUL byte".into())
            })?;
            let bottom_c = CString::new(bottom.unwrap_or_default()).map_err(|_| {
                crate::Error::InvalidArgument("bottom font family contains a NUL byte".into())
            })?;
            unsafe {
                multiline_menubar_set_font_family(
                    id_c.as_ptr(),
                    top_c.as_ptr(),
                    bottom_c.as_ptr(),
                )
            };
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (id, top, bottom);
            Err(crate::Error::UnsupportedPlatform)
        }
    }

    /// Set the per-line monospaced-digit toggle for the top and bottom lines.
    /// When a line has no explicit font family, `top_monospaced` /
    /// `bottom_monospaced` being `true` renders that line with the system
    /// monospaced-digit font (constant digit width — numeric readouts like a
    /// speed display don't jitter); `false` restores the regular system font.
    /// An explicit font family (see [`Self::set_font_family`]) takes
    /// precedence over this toggle.
    pub fn set_monospaced(
        &self,
        id: String,
        top_monospaced: bool,
        bottom_monospaced: bool,
    ) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            // Remember the toggle so the popup can pre-fill this instance's
            // value (mirrors the style bookkeeping for font sizes).
            instances()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .entry(id.clone())
                .or_default()
                .monospaced = (top_monospaced, bottom_monospaced);

            let c = CString::new(id).map_err(|_| {
                crate::Error::InvalidArgument("id contains a NUL byte".into())
            })?;
            unsafe {
                multiline_menubar_set_monospaced(
                    c.as_ptr(),
                    top_monospaced as std::os::raw::c_int,
                    bottom_monospaced as std::os::raw::c_int,
                )
            };
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (id, top_monospaced, bottom_monospaced);
            Err(crate::Error::UnsupportedPlatform)
        }
    }

    /// Set the per-line horizontal alignment for the top and bottom lines.
    /// `top_align` / `bottom_align` are `0` = left (default), `1` = center,
    /// `2` = right; any other value is treated as left on the native side, so
    /// instances that never call this keep rendering left-aligned.
    pub fn set_alignment(
        &self,
        id: String,
        top_align: i32,
        bottom_align: i32,
    ) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            // Remember the alignment so the popup can pre-fill this instance's
            // value (mirrors the style bookkeeping for font sizes).
            instances()
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .entry(id.clone())
                .or_default()
                .alignment = (top_align, bottom_align);

            let c = CString::new(id).map_err(|_| {
                crate::Error::InvalidArgument("id contains a NUL byte".into())
            })?;
            unsafe {
                multiline_menubar_set_alignment(
                    c.as_ptr(),
                    top_align as std::os::raw::c_int,
                    bottom_align as std::os::raw::c_int,
                )
            };
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (id, top_align, bottom_align);
            Err(crate::Error::UnsupportedPlatform)
        }
    }

    /// Returns the on-screen rectangle of an instance in macOS screen points
    /// (origin bottom-left, y increasing upward).
    pub fn rect(&self, id: String) -> crate::Result<Rect> {
        #[cfg(target_os = "macos")]
        {
            let (x, y, width, height) =
                get_instance_rect(&id).ok_or(crate::Error::InstanceNotFound)?;
            return Ok(Rect {
                x,
                y,
                width,
                height,
            });
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = id;
            Err(crate::Error::UnsupportedPlatform)
        }
    }

    /// Show or hide an instance. Note the asymmetry: showing an instance that
    /// was never created implicitly creates it (mirroring `create`), while
    /// hiding one that does not exist is a no-op.
    pub fn set_visible(&self, id: String, visible: bool) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            let c = CString::new(id)
                .map_err(|_| crate::Error::InvalidArgument("id contains a NUL byte".into()))?;
            if visible {
                unsafe { multiline_menubar_show(c.as_ptr()) };
            } else {
                unsafe { multiline_menubar_hide(c.as_ptr()) };
            }
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (id, visible);
            Err(crate::Error::UnsupportedPlatform)
        }
    }

    pub fn is_visible(&self, id: String) -> crate::Result<bool> {
        #[cfg(target_os = "macos")]
        {
            let c = CString::new(id)
                .map_err(|_| crate::Error::InvalidArgument("id contains a NUL byte".into()))?;
            return Ok(unsafe { multiline_menubar_is_visible(c.as_ptr()) } != 0);
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = id;
            Err(crate::Error::UnsupportedPlatform)
        }
    }

    /// Set which Tauri window is used as the popup. Call before the first open.
    pub fn set_popup_window(&self, label: String) -> crate::Result<()> {
        *POPUP_WINDOW.write().unwrap_or_else(|e| e.into_inner()) = Some(label.into());
        Ok(())
    }

    /// Enable/disable automatically toggling the popup on left click.
    pub fn set_auto_popup(&self, enabled: bool) -> crate::Result<()> {
        *AUTO_POPUP.lock().unwrap_or_else(|e| e.into_inner()) = enabled;
        Ok(())
    }

    pub fn open_popup(&self, id: String) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            let app = APP_HANDLE.get().ok_or(crate::Error::UnsupportedPlatform)?;
            return open_popup_window(app, &id);
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = id;
            Err(crate::Error::UnsupportedPlatform)
        }
    }

    pub fn close_popup(&self, id: String) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            let app = APP_HANDLE.get().ok_or(crate::Error::UnsupportedPlatform)?;
            return close_popup_window(app, &id);
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = id;
            Err(crate::Error::UnsupportedPlatform)
        }
    }

    pub fn toggle_popup(&self, id: String) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            let app = APP_HANDLE.get().ok_or(crate::Error::UnsupportedPlatform)?;
            return toggle_popup_window(app, &id);
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = id;
            Err(crate::Error::UnsupportedPlatform)
        }
    }
}
