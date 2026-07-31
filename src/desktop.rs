use serde::de::DeserializeOwned;
use std::ffi::CString;
use tauri::{plugin::PluginApi, AppHandle, Runtime};

#[cfg(target_os = "macos")]
extern "C" {
    fn multiline_menubar_show();
    fn multiline_menubar_hide();
    fn multiline_menubar_set_text(
        top: *const std::os::raw::c_char,
        bottom: *const std::os::raw::c_char,
    );
    fn multiline_menubar_set_style(top_size: f64, bottom_size: f64);
    fn multiline_menubar_is_visible() -> std::os::raw::c_int;
}

pub fn init<R: Runtime, C: DeserializeOwned>(
    app: &AppHandle<R>,
    _api: PluginApi<R, C>,
) -> crate::Result<MultilineMenubar<R>> {
    Ok(MultilineMenubar(app.clone()))
}

/// Access to the multiline-menubar APIs.
pub struct MultilineMenubar<R: Runtime>(AppHandle<R>);

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

    pub fn is_visible(&self) -> crate::Result<bool> {
        #[cfg(target_os = "macos")]
        unsafe {
            return Ok(multiline_menubar_is_visible() != 0);
        }
        #[cfg(not(target_os = "macos"))]
        Err(crate::Error::UnsupportedPlatform)
    }
}
