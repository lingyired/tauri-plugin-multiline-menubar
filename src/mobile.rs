use serde::de::DeserializeOwned;
use tauri::{
    plugin::{PluginApi, PluginHandle},
    AppHandle, Runtime,
};

use crate::models::*;

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_multiline_menubar);

pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
) -> crate::Result<MultilineMenubar<R>> {
    #[cfg(target_os = "android")]
    let handle = api
        .register_android_plugin("", "ExamplePlugin")
        .map_err(|e| crate::Error::Mobile(e.to_string()))?;
    #[cfg(target_os = "ios")]
    let handle = api
        .register_ios_plugin(init_plugin_multiline_menubar)
        .map_err(|e| crate::Error::Mobile(e.to_string()))?;
    Ok(MultilineMenubar(handle))
}

/// Access to the multiline-menubar APIs.
///
/// The plugin is macOS-only; every method below is a stub mirroring the
/// desktop API surface (same signatures) and returns `UnsupportedPlatform`.
pub struct MultilineMenubar<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> MultilineMenubar<R> {
    pub fn create(&self, _id: String) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn remove(&self, _id: String) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn set_text(&self, _id: String, _top: String, _bottom: String) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn set_font_sizes(&self, _id: String, _top: f64, _bottom: f64) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn set_layout(&self, _id: String, _layout: i32) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn set_tooltip(&self, _id: String, _tooltip: String) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn set_visible(&self, _id: String, _visible: bool) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn set_menu(&self, _id: String, _items: Vec<MenuItemDescriptor>) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn remove_menu(&self, _id: String) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn set_colors(
        &self,
        _id: String,
        _top: ColorStyle,
        _bottom: ColorStyle,
    ) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn set_bold(&self, _id: String, _top_bold: bool, _bottom_bold: bool) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn set_font_family(
        &self,
        _id: String,
        _top: Option<String>,
        _bottom: Option<String>,
    ) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn rect(&self, _id: String) -> crate::Result<Rect> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn set_popup_window(&self, _label: String) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn set_auto_popup(&self, _enabled: bool) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn open_popup(&self, _id: String) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn close_popup(&self, _id: String) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn toggle_popup(&self, _id: String) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn is_visible(&self, _id: String) -> crate::Result<bool> {
        Err(crate::Error::UnsupportedPlatform)
    }
}
