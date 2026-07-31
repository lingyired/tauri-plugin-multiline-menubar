const COMMANDS: &[&str] = &["set_text", "set_font_sizes", "show", "hide", "is_visible"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS)
        .android_path("android")
        .ios_path("ios")
        .build();

    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "macos" {
        cc::Build::new()
            .file("src/native/multiline_menubar.mm")
            .flag("-fobjc-arc")
            .compile("multiline_menubar_native");

        println!("cargo:rustc-link-lib=framework=Cocoa");
        println!("cargo:rustc-link-lib=c++");
    }
}
