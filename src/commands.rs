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
