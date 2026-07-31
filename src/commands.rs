use tauri::{command, AppHandle, Runtime};

use crate::models::*;
use crate::MultilineMenubarExt;

#[command]
pub(crate) async fn set_text<R: Runtime>(
    app: AppHandle<R>,
    payload: SetTextRequest,
) -> crate::Result<()> {
    app.multiline_menubar().set_text(payload.top, payload.bottom)
}

#[command]
pub(crate) async fn set_font_sizes<R: Runtime>(
    app: AppHandle<R>,
    payload: FontSizesRequest,
) -> crate::Result<()> {
    app.multiline_menubar()
        .set_font_sizes(payload.top, payload.bottom)
}

#[command]
pub(crate) async fn set_tooltip<R: Runtime>(
    app: AppHandle<R>,
    payload: TooltipRequest,
) -> crate::Result<()> {
    app.multiline_menubar().set_tooltip(payload.tooltip)
}

#[command]
pub(crate) async fn set_visible<R: Runtime>(
    app: AppHandle<R>,
    payload: SetVisibleRequest,
) -> crate::Result<()> {
    app.multiline_menubar().set_visible(payload.visible)
}

#[command]
pub(crate) async fn set_popup_window<R: Runtime>(
    app: AppHandle<R>,
    payload: PopupWindowRequest,
) -> crate::Result<()> {
    app.multiline_menubar().set_popup_window(payload.label)
}

#[command]
pub(crate) async fn set_auto_popup<R: Runtime>(
    app: AppHandle<R>,
    payload: SetAutoPopupRequest,
) -> crate::Result<()> {
    app.multiline_menubar().set_auto_popup(payload.enabled)
}

#[command]
pub(crate) async fn open_popup<R: Runtime>(app: AppHandle<R>) -> crate::Result<()> {
    app.multiline_menubar().open_popup()
}

#[command]
pub(crate) async fn close_popup<R: Runtime>(app: AppHandle<R>) -> crate::Result<()> {
    app.multiline_menubar().close_popup()
}

#[command]
pub(crate) async fn toggle_popup<R: Runtime>(app: AppHandle<R>) -> crate::Result<()> {
    app.multiline_menubar().toggle_popup()
}

#[command]
pub(crate) async fn show<R: Runtime>(app: AppHandle<R>) -> crate::Result<()> {
    app.multiline_menubar().show()
}

#[command]
pub(crate) async fn hide<R: Runtime>(app: AppHandle<R>) -> crate::Result<()> {
    app.multiline_menubar().hide()
}

#[command]
pub(crate) async fn is_visible<R: Runtime>(
    app: AppHandle<R>,
) -> crate::Result<VisibilityResponse> {
    Ok(VisibilityResponse {
        visible: app.multiline_menubar().is_visible()?,
    })
}
