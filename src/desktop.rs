use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{Mutex, OnceLock};
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
    fn multiline_menubar_set_tooltip(id: *const c_char, tooltip: *const c_char);
    fn multiline_menubar_set_menu(id: *const c_char, ns_menu: *mut std::ffi::c_void);
    fn multiline_menubar_set_color(
        id: *const c_char,
        top: *const c_char,
        bottom: *const c_char,
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
static POPUP_WINDOW: Mutex<Option<String>> = Mutex::new(None);

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
static INSTANCE_TEXT: Mutex<Option<HashMap<String, (String, String)>>> = Mutex::new(None);

/// Last top/bottom font size set per instance, so the popup can pre-fill its
/// font-size sliders with the values of whichever instance opened it.
static INSTANCE_STYLE: Mutex<Option<HashMap<String, (f64, f64)>>> = Mutex::new(None);

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
        let auto = *AUTO_POPUP.lock().unwrap();
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
    let mut guard = MENU_ITEM_OWNERS.lock().unwrap();
    let owners = guard.get_or_insert_with(HashMap::new);
    owners.retain(|_, owner| owner != instance_id);
    for id in item_ids {
        owners.insert(id, instance_id.to_string());
    }
}

/// Forget every item id owned by an instance.
#[cfg(target_os = "macos")]
fn unregister_menu_owners(instance_id: &str) {
    let mut guard = MENU_ITEM_OWNERS.lock().unwrap();
    if let Some(owners) = guard.as_mut() {
        owners.retain(|_, owner| owner != instance_id);
    }
}

/// Which instance owns this menu item id, if any.
#[cfg(target_os = "macos")]
fn menu_owner_of(item_id: &str) -> Option<String> {
    let guard = MENU_ITEM_OWNERS.lock().unwrap();
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
    if let Some(win) = app.get_webview_window(&label) {
        let rect = get_instance_rect(id).unwrap_or((0.0, 0.0, 0.0, 0.0));
        position_popup_under_status_item(app, &win, rect)?;
        attach_auto_hide(app, &win, &label);
        *POPUP_IGNORE_BLUR_UNTIL.lock().unwrap() =
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
        let text = INSTANCE_TEXT
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|m| m.get(id).cloned())
            .unwrap_or_default();
        let style = INSTANCE_STYLE
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|m| m.get(id).copied())
            .unwrap_or((7.0, 12.0));
        let _ = app.emit_to(
            &label,
            POPUP_OPEN_TARGET_EVENT,
            serde_json::json!({
                "id": id,
                "top": text.0,
                "bottom": text.1,
                "topSize": style.0,
                "bottomSize": style.1
            }),
        );
    }
    Ok(())
}

/// Hide the popup window.
#[cfg(target_os = "macos")]
fn close_popup_window(app: &tauri::AppHandle<Wry>, id: &str) -> crate::Result<()> {
    let label = popup_label();
    if let Some(win) = app.get_webview_window(&label) {
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
    if let Some(win) = app.get_webview_window(&label) {
        if win.is_visible().unwrap_or(false) {
            return close_popup_window(app, id);
        }
        return open_popup_window(app, id);
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn popup_label() -> String {
    POPUP_WINDOW
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_else(|| "popup".to_string())
}

/// Close the popup window when it loses focus (menubar-app behaviour).
/// Attached once to the popup window.
#[cfg(target_os = "macos")]
fn attach_auto_hide(app: &tauri::AppHandle<Wry>, win: &WebviewWindow, label: &str) {
    let mut attached = POPUP_HANDLER_ATTACHED.lock().unwrap();
    if *attached {
        return;
    }
    *attached = true;

    let app = app.clone();
    let label = label.to_string();
    win.on_window_event(move |event| {
        if let WindowEvent::Focused(false) = event {
            let now = Instant::now();
            let ignore = POPUP_IGNORE_BLUR_UNTIL.lock().unwrap();
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
        let app_wry: tauri::AppHandle<Wry> = unsafe { std::mem::transmute_copy(app) };
        let _ = APP_HANDLE.set(app_wry);

        unsafe {
            multiline_menubar_set_click_handler(Some(on_native_click));
            multiline_menubar_set_hover_handler(Some(on_native_hover));
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
            if item_id == "quit" || item_id == "quit2" {
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
            let c = CString::new(id.as_str()).map_err(|_| crate::Error::UnsupportedPlatform)?;
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

    pub fn destroy(&self, id: String) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            let c = CString::new(id.as_str()).map_err(|_| crate::Error::UnsupportedPlatform)?;
            unsafe { multiline_menubar_destroy(c.as_ptr()) };
            unregister_menu_owners(&id);
            if let Some(map) = INSTANCE_TEXT.lock().unwrap().as_mut() {
                map.remove(&id);
            }
            if let Some(map) = INSTANCE_STYLE.lock().unwrap().as_mut() {
                map.remove(&id);
            }

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

    pub fn show(&self, id: String) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            let c = CString::new(id).map_err(|_| crate::Error::UnsupportedPlatform)?;
            unsafe { multiline_menubar_show(c.as_ptr()) };
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = id;
            Err(crate::Error::UnsupportedPlatform)
        }
    }

    pub fn hide(&self, id: String) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            let c = CString::new(id).map_err(|_| crate::Error::UnsupportedPlatform)?;
            unsafe { multiline_menubar_hide(c.as_ptr()) };
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
            // Remember the text so the popup can show this instance's values.
            INSTANCE_TEXT
                .lock()
                .unwrap()
                .get_or_insert_with(HashMap::new)
                .insert(id.clone(), (top.clone(), bottom.clone()));

            let id_c = CString::new(id).map_err(|_| crate::Error::UnsupportedPlatform)?;
            let top_c = CString::new(top).map_err(|_| crate::Error::UnsupportedPlatform)?;
            let bottom_c = CString::new(bottom).map_err(|_| crate::Error::UnsupportedPlatform)?;
            unsafe {
                multiline_menubar_set_text(id_c.as_ptr(), top_c.as_ptr(), bottom_c.as_ptr())
            };
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
            // (mirrors INSTANCE_TEXT for the text content).
            INSTANCE_STYLE
                .lock()
                .unwrap()
                .get_or_insert_with(HashMap::new)
                .insert(id.clone(), (top, bottom));

            let c = CString::new(id).map_err(|_| crate::Error::UnsupportedPlatform)?;
            unsafe { multiline_menubar_set_style(c.as_ptr(), top, bottom) };
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (id, top, bottom);
            Err(crate::Error::UnsupportedPlatform)
        }
    }

    pub fn set_tooltip(&self, id: String, tooltip: String) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            let id_c = CString::new(id).map_err(|_| crate::Error::UnsupportedPlatform)?;
            let c = CString::new(tooltip).map_err(|_| crate::Error::UnsupportedPlatform)?;
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
            // Fail fast on bad accelerators while we can still return an error.
            validate_accelerators(&items)?;

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
            let c = CString::new(id.as_str()).map_err(|_| crate::Error::UnsupportedPlatform)?;
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

            let id_c = CString::new(id).map_err(|_| crate::Error::UnsupportedPlatform)?;
            let top_c =
                CString::new(top_json).map_err(|_| crate::Error::UnsupportedPlatform)?;
            let bottom_c =
                CString::new(bottom_json).map_err(|_| crate::Error::UnsupportedPlatform)?;
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

    /// Returns the on-screen rectangle of an instance in macOS screen points
    /// (origin bottom-left, y increasing upward).
    pub fn get_rect(&self, id: String) -> crate::Result<Rect> {
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

    pub fn set_visible(&self, id: String, visible: bool) -> crate::Result<()> {
        if visible {
            self.show(id)
        } else {
            self.hide(id)
        }
    }

    pub fn is_visible(&self, id: String) -> crate::Result<bool> {
        #[cfg(target_os = "macos")]
        {
            let c = CString::new(id).map_err(|_| crate::Error::UnsupportedPlatform)?;
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
        *POPUP_WINDOW.lock().unwrap() = Some(label);
        Ok(())
    }

    /// Enable/disable automatically toggling the popup on left click.
    pub fn set_auto_popup(&self, enabled: bool) -> crate::Result<()> {
        *AUTO_POPUP.lock().unwrap() = enabled;
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
