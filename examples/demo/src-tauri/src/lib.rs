// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use tauri::{Manager, RunEvent, WindowEvent};

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // The single-instance plugin MUST be the first plugin registered.
        // A second launch while the process is alive is swallowed by the
        // plugin, and this callback reveals the running instance's main
        // settings window (menubar app: the process keeps running after the
        // window is closed, so re-launching should just bring the window back).
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_multiline_menubar::init())
        .on_window_event(|window, event| {
            // Menubar app: closing a window hides it instead of destroying it.
            // The main settings window must never be destroyed — its page
            // re-load would re-create the menubar instances. The popup window
            // is also reused by the plugin (get_webview_window + show).
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![greet])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            // macOS: clicking the Dock icon re-opens the main window when it
            // was hidden (its Close button hides instead of destroying).
            if let RunEvent::Reopen { .. } = event {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        });
}
