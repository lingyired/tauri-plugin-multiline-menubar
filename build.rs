const COMMANDS: &[&str] = &[
    "create",
    "remove",
    "set_text",
    "set_font_sizes",
    "set_layout",
    "set_tooltip",
    "set_visible",
    "set_menu",
    "remove_menu",
    "set_colors",
    "set_bold",
    "set_font_family",
    "rect",
    "is_visible",
    "set_popup_window",
    "set_auto_popup",
    "open_popup",
    "close_popup",
    "toggle_popup",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        // Mobile stubs exist in `src/mobile.rs` but no `android/` / `ios/`
        // platform directories ship with this repo, so mobile targets are not
        // buildable yet. The paths are kept so the generator stays happy.
        .android_path("android")
        .ios_path("ios")
        .build();

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "macos" {
        println!("cargo:rerun-if-changed=src/native/multiline_menubar.mm");
        println!("cargo:rerun-if-changed=src/native/multiline_menubar.h");
        cc::Build::new()
            .file("src/native/multiline_menubar.mm")
            .flag("-fobjc-arc")
            .compile("multiline_menubar_native");

        println!("cargo:rustc-link-lib=framework=Cocoa");
        println!("cargo:rustc-link-lib=framework=CoreText");
        println!("cargo:rustc-link-lib=c++");
    }
}
