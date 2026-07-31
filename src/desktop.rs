use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use serde::de::DeserializeOwned;
use tauri::{
    Emitter, Manager, Position, Runtime, WebviewWindow, WindowEvent,
    plugin::PluginApi,
};
use tauri::Wry;

#[cfg(target_os = "macos")]
extern "C" {
    fn multiline_menubar_show();
    fn multiline_menubar_hide();
    fn multiline_menubar_set_text(
        top: *const std::os::raw::c_char,
        bottom: *const std::os::raw::c_char,
    );
    fn multiline_menubar_set_style(top_size: f64, bottom_size: f64);
    fn multiline_menubar_set_tooltip(tooltip: *const std::os::raw::c_char);
    fn multiline_menubar_set_version(version: *const std::os::raw::c_char);
    fn multiline_menubar_is_visible() -> std::os::raw::c_int;
    fn multiline_menubar_set_click_handler(
        callback: Option<extern "C" fn(*const c_char, f64, f64, f64, f64)>,
    );
}

// ---------------------------------------------------------------------------
// Shared state (used by the native click callback and the popup commands)
// ---------------------------------------------------------------------------

/// App handle stored as the concrete runtime (Wry) so the `extern "C"` click
/// callback can emit events and manage the popup window.
static APP_HANDLE: OnceLock<tauri::AppHandle<Wry>> = OnceLock::new();

/// Label of the Tauri window used as the popup (default "popup").
static POPUP_WINDOW: Mutex<Option<String>> = Mutex::new(None);

/// Whether a left click automatically toggles the popup window.
static AUTO_POPUP: Mutex<bool> = Mutex::new(true);

/// Last on-screen rectangle of the status item (x, y, width, height), in
/// macOS screen points (origin bottom-left). Set by the click callback.
static LAST_RECT: Mutex<Option<(f64, f64, f64, f64)>> = Mutex::new(None);

/// While set, the popup's blur handler ignores focus loss (prevents the
/// popup from immediately closing when it opens and steals focus).
static POPUP_IGNORE_BLUR_UNTIL: Mutex<Option<Instant>> = Mutex::new(None);

/// Ensures the popup auto-hide handler is attached only once.
static POPUP_HANDLER_ATTACHED: Mutex<bool> = Mutex::new(false);

// ---------------------------------------------------------------------------
// Native click callback
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
extern "C" fn on_native_click(button: *const c_char, x: f64, y: f64, w: f64, h: f64) {
    let button_str = unsafe {
        if button.is_null() {
            "left"
        } else {
            CStr::from_ptr(button).to_str().unwrap_or("left")
        }
    };

    *LAST_RECT.lock().unwrap() = Some((x, y, w, h));

    if let Some(app) = APP_HANDLE.get() {
        let _ = app.emit(
            "multiline-menubar://click",
            serde_json::json!({
                "button": button_str,
                "x": x,
                "y": y,
                "width": w,
                "height": h,
            }),
        );

        let auto = *AUTO_POPUP.lock().unwrap();
        if auto && button_str == "left" {
            let _ = toggle_popup_window(app);
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
) -> crate::Result<()> {
    if let Some((rx, ry, rw, _rh)) = *LAST_RECT.lock().unwrap() {
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

            // The menu bar sits at the top of the screen. Place the popup's
            // top edge at the bottom of the status item. macOS y grows upward,
            // Tauri y grows downward, so flip it.
            let tauri_y = screen_h - ry - win_h;

            let _ = win.set_position(Position::Logical(tauri::LogicalPosition::new(tauri_x, tauri_y)));
        }
    }
    Ok(())
}

/// Toggle the popup window's visibility.
#[cfg(target_os = "macos")]
fn toggle_popup_window(app: &tauri::AppHandle<Wry>) -> crate::Result<()> {
    let label = POPUP_WINDOW
        .lock()
        .unwrap()
        .clone()
        .unwrap_or_else(|| "popup".to_string());

    if let Some(win) = app.get_webview_window(&label) {
        let visible = win.is_visible().unwrap_or(false);
        if visible {
            let _ = win.hide();
            let _ = app.emit(
                "multiline-menubar://popup-close",
                serde_json::json!({ "window": label }),
            );
        } else {
            position_popup_under_status_item(app, &win)?;
            attach_auto_hide(app, &win, &label);
            *POPUP_IGNORE_BLUR_UNTIL.lock().unwrap() =
                Some(Instant::now() + Duration::from_millis(200));
            let _ = win.show();
            let _ = win.set_focus();
            let _ = app.emit(
                "multiline-menubar://popup-open",
                serde_json::json!({ "window": label }),
            );
        }
    }
    Ok(())
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

    // Clone into an owned, 'static handle so the event closure can use it.
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
                        "multiline-menubar://popup-close",
                        serde_json::json!({ "window": label }),
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
        // Store the app handle (typed as the concrete runtime) so the native
        // click callback can drive the popup and emit events.
        let app_wry: tauri::AppHandle<Wry> = unsafe { std::mem::transmute_copy(app) };
        let _ = APP_HANDLE.set(app_wry);

        // Create the status item and wire the click callback.
        unsafe {
            multiline_menubar_show();
            multiline_menubar_set_click_handler(Some(on_native_click));
            // Feed the app version into the native right-click context menu.
            if let Ok(version) = CString::new(app.package_info().version.to_string()) {
                multiline_menubar_set_version(version.as_ptr());
            }
        }

        // Let the frontend know the status item exists and is ready.
        let _ = app.emit("multiline-menubar://ready", serde_json::json!({}));
    }

    Ok(MultilineMenubar(app.clone()))
}

/// Access to the multiline-menubar APIs.
pub struct MultilineMenubar<R: Runtime>(tauri::AppHandle<R>);

impl<R: Runtime> MultilineMenubar<R> {
    pub fn show(&self) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        unsafe {
            multiline_menubar_show();
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn hide(&self) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        unsafe {
            multiline_menubar_hide();
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn set_text(&self, top: String, bottom: String) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        unsafe {
            let top_c = CString::new(top).map_err(|_| crate::Error::UnsupportedPlatform)?;
            let bottom_c = CString::new(bottom).map_err(|_| crate::Error::UnsupportedPlatform)?;
            multiline_menubar_set_text(top_c.as_ptr(), bottom_c.as_ptr());
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn set_font_sizes(&self, top: f64, bottom: f64) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        unsafe {
            multiline_menubar_set_style(top, bottom);
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn set_tooltip(&self, tooltip: String) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        unsafe {
            let c = CString::new(tooltip).map_err(|_| crate::Error::UnsupportedPlatform)?;
            multiline_menubar_set_tooltip(c.as_ptr());
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        Err(crate::Error::UnsupportedPlatform)
    }

    /// Set the application version shown in the right-click context menu.
    pub fn set_version(&self, version: String) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        unsafe {
            let c = CString::new(version).map_err(|_| crate::Error::UnsupportedPlatform)?;
            multiline_menubar_set_version(c.as_ptr());
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        Err(crate::Error::UnsupportedPlatform)
    }

    /// Show or hide the status item (aligns with the typical `set_visible`
    /// menubar plugin API).
    pub fn set_visible(&self, visible: bool) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        unsafe {
            if visible {
                multiline_menubar_show();
            } else {
                multiline_menubar_hide();
            }
            return Ok(());
        }
        #[cfg(not(target_os = "macos"))]
        Err(crate::Error::UnsupportedPlatform)
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

    pub fn open_popup(&self) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            if let Some(app) = APP_HANDLE.get() {
                let label = POPUP_WINDOW
                    .lock()
                    .unwrap()
                    .clone()
                    .unwrap_or_else(|| "popup".to_string());
                if let Some(win) = app.get_webview_window(&label) {
                    position_popup_under_status_item(app, &win)?;
                    attach_auto_hide(app, &win, &label);
                    *POPUP_IGNORE_BLUR_UNTIL.lock().unwrap() =
                        Some(Instant::now() + Duration::from_millis(200));
                    let _ = win.show();
                    let _ = win.set_focus();
                    let _ = app.emit(
                        "multiline-menubar://popup-open",
                        serde_json::json!({ "window": label }),
                    );
                }
                return Ok(());
            }
            return Err(crate::Error::UnsupportedPlatform);
        }
        #[cfg(not(target_os = "macos"))]
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn close_popup(&self) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            if let Some(app) = APP_HANDLE.get() {
                let label = POPUP_WINDOW
                    .lock()
                    .unwrap()
                    .clone()
                    .unwrap_or_else(|| "popup".to_string());
                if let Some(win) = app.get_webview_window(&label) {
                    let _ = win.hide();
                    let _ = app.emit(
                        "multiline-menubar://popup-close",
                        serde_json::json!({ "window": label }),
                    );
                }
                return Ok(());
            }
            return Err(crate::Error::UnsupportedPlatform);
        }
        #[cfg(not(target_os = "macos"))]
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn toggle_popup(&self) -> crate::Result<()> {
        #[cfg(target_os = "macos")]
        {
            if let Some(app) = APP_HANDLE.get() {
                return toggle_popup_window(app);
            }
            return Err(crate::Error::UnsupportedPlatform);
        }
        #[cfg(not(target_os = "macos"))]
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn is_visible(&self) -> crate::Result<bool> {
        #[cfg(target_os = "macos")]
        unsafe {
            return Ok(multiline_menubar_is_visible() != 0);
        }
        #[cfg(not(target_os = "macos"))]
        Err(crate::Error::UnsupportedPlatform)
    }
}
