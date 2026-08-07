use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

pub use models::*;

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

mod commands;
mod error;
mod models;

pub use error::{Error, Result};

#[cfg(desktop)]
use desktop::MultilineMenubar;
#[cfg(mobile)]
use mobile::MultilineMenubar;

/// Extensions to [`tauri::App`], [`tauri::AppHandle`] and [`tauri::Window`] to access the multiline-menubar APIs.
pub trait MultilineMenubarExt<R: Runtime> {
    fn multiline_menubar(&self) -> &MultilineMenubar<R>;
}

impl<R: Runtime, T: Manager<R>> crate::MultilineMenubarExt<R> for T {
    fn multiline_menubar(&self) -> &MultilineMenubar<R> {
        self.state::<MultilineMenubar<R>>().inner()
    }
}

/// Initializes the plugin.
pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("multiline-menubar")
        .invoke_handler(tauri::generate_handler![
            commands::create,
            commands::remove,
            commands::set_text,
            commands::set_font_sizes,
            commands::set_layout,
            commands::set_tooltip,
            commands::set_visible,
            commands::set_menu,
            commands::remove_menu,
            commands::set_colors,
            commands::set_bold,
            commands::set_font_family,
            commands::set_monospaced,
            commands::rect,
            commands::is_visible,
            commands::set_popup_window,
            commands::set_auto_popup,
            commands::open_popup,
            commands::close_popup,
            commands::toggle_popup
        ])
        .setup(|app, api| {
            #[cfg(mobile)]
            let multiline_menubar = mobile::init(app, api)?;
            #[cfg(desktop)]
            let multiline_menubar = desktop::init(app, api)?;
            app.manage(multiline_menubar);
            Ok(())
        })
        .build()
}
