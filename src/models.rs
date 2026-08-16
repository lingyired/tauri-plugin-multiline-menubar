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

/// Per-line bold toggle for the two menubar lines.
///
/// Each line is independent: `top`/`bottom` being `true` forces that line to
/// render with `NSFontWeightBold`, overriding the weight that `layout` would
/// otherwise derive for it. `false` leaves the line's weight to the layout
/// (the emphasized line is regular, the de-emphasized one is light).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetBoldRequest {
    pub id: String,
    pub top: bool,
    pub bottom: bool,
}

/// Per-line font family for the two menubar lines.
///
/// Each field is a macOS font *family* name (e.g. `"Helvetica"`).
/// `None` (or an empty string) keeps the system font for that line. The line
/// still resolves the weight `layout`/`set_bold` asks for, using the closest
/// face the family provides; unknown names silently fall back to the system
/// font.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetFontFamilyRequest {
    pub id: String,
    pub top: Option<String>,
    pub bottom: Option<String>,
}

/// Per-line monospaced-digit toggle for the two menubar lines.
///
/// When a line has no explicit font family, `top`/`bottom` being `true`
/// switches that line to the system monospaced-digit font
/// (`monospacedDigitSystemFont`), which keeps every digit the same width so
/// frequently-updating numeric text (e.g. a network speed readout) does not
/// jitter as values change. `false` uses the regular system font. An explicit
/// font family (see [`SetFontFamilyRequest`]) takes precedence over this
/// toggle.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetMonospacedRequest {
    pub id: String,
    pub top: bool,
    pub bottom: bool,
}

/// Per-line horizontal alignment for the two menubar lines.
///
/// Each line is independent: `top`/`bottom` is `0` = left (default),
/// `1` = center, `2` = right. Any other value is treated as left on the
/// native side, so instances that never call `set_alignment` keep rendering
/// left-aligned (the historical behavior).
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAlignmentRequest {
    pub id: String,
    pub top: i32,
    pub bottom: i32,
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
