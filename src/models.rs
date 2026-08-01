use serde::{Deserialize, Serialize};

/// On-screen rectangle of a menubar item, in macOS screen points
/// (origin bottom-left, y increasing upward). Mirrors Tauri's `Rect`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// A menu item descriptor used to build a real Tauri `Menu` from the frontend.
/// The `id` field becomes the menu item's `MenuId`, which is reported back
/// through Tauri's global `on_menu_event`.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MenuItemDescriptor {
    Item {
        id: String,
        text: String,
        accelerator: Option<String>,
        disabled: Option<bool>,
    },
    Check {
        id: String,
        text: String,
        checked: Option<bool>,
        accelerator: Option<String>,
    },
    Separator,
    Submenu {
        text: String,
        items: Vec<MenuItemDescriptor>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRequest {
    pub id: String,
    #[serde(default)]
    pub top: Option<String>,
    #[serde(default)]
    pub bottom: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveRequest {
    pub id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdRequest {
    pub id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetTextRequest {
    pub id: String,
    pub top: String,
    pub bottom: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FontSizesRequest {
    pub id: String,
    pub top: f64,
    pub bottom: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutRequest {
    pub id: String,
    /// 0 = emphasis-bottom (default, small label on top / large value below),
    /// 1 = emphasis-top (the vertical mirror), 2 = equal (both lines share one
    /// size, vertically centered & symmetric).
    pub layout: i32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TooltipRequest {
    pub id: String,
    pub tooltip: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetVisibleRequest {
    pub id: String,
    pub visible: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetMenuRequest {
    pub id: String,
    /// Menu items. Omit (or send `null`) to detach the menu, mirroring
    /// Tauri's `setMenu(null)` semantics.
    #[serde(default)]
    pub items: Option<Vec<MenuItemDescriptor>>,
}

/// How the text of a menubar line should be painted.
///
/// `default` keeps the system `textColor` (follows light/dark mode). `solid`
/// uses a single hex color.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ColorStyle {
    Default,
    Solid { value: String },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetColorsRequest {
    pub id: String,
    pub top: ColorStyle,
    pub bottom: ColorStyle,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RectRequest {
    pub id: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PopupWindowRequest {
    pub label: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAutoPopupRequest {
    pub enabled: bool,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VisibilityResponse {
    pub visible: bool,
}
