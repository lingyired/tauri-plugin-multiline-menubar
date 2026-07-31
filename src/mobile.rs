use serde::de::DeserializeOwned;
use tauri::{
    plugin::{PluginApi, PluginHandle},
    AppHandle, Runtime,
};

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
pub struct MultilineMenubar<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> MultilineMenubar<R> {
    pub fn show(&self) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn hide(&self) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn set_text(&self, _top: String, _bottom: String) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn set_font_sizes(&self, _top: f64, _bottom: f64) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn is_visible(&self) -> crate::Result<bool> {
        Err(crate::Error::UnsupportedPlatform)
    }
}
