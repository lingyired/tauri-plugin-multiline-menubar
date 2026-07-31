use tauri::{command, AppHandle, Runtime};

use crate::models::*;
use crate::MultilineMenubarExt;

#[command]
pub(crate) async fn create<R: Runtime>(
    app: AppHandle<R>,
    payload: CreateRequest,
) -> crate::Result<()> {
    app.multiline_menubar().create(payload.id.clone())?;
    if let (Some(top), Some(bottom)) = (payload.top, payload.bottom) {
        app.multiline_menubar().set_text(payload.id, top, bottom)?;
    }
    Ok(())
}

#[command]
pub(crate) async fn destroy<R: Runtime>(
    app: AppHandle<R>,
    payload: DestroyRequest,
) -> crate::Result<()> {
    app.multiline_menubar().destroy(payload.id)
}

#[command]
pub(crate) async fn show<R: Runtime>(app: AppHandle<R>, payload: IdRequest) -> crate::Result<()> {
    app.multiline_menubar().show(payload.id)
}

#[command]
pub(crate) async fn hide<R: Runtime>(app: AppHandle<R>, payload: IdRequest) -> crate::Result<()> {
    app.multiline_menubar().hide(payload.id)
}

#[command]
pub(crate) async fn set_text<R: Runtime>(
    app: AppHandle<R>,
    payload: SetTextRequest,
) -> crate::Result<()> {
    app.multiline_menubar()
        .set_text(payload.id, payload.top, payload.bottom)
}

#[command]
pub(crate) async fn set_font_sizes<R: Runtime>(
    app: AppHandle<R>,
    payload: FontSizesRequest,
) -> crate::Result<()> {
    app.multiline_menubar()
        .set_font_sizes(payload.id, payload.top, payload.bottom)
}

#[command]
pub(crate) async fn set_tooltip<R: Runtime>(
    app: AppHandle<R>,
    payload: TooltipRequest,
) -> crate::Result<()> {
    app.multiline_menubar().set_tooltip(payload.id, payload.tooltip)
}

#[command]
pub(crate) async fn set_visible<R: Runtime>(
    app: AppHandle<R>,
    payload: SetVisibleRequest,
) -> crate::Result<()> {
    app.multiline_menubar().set_visible(payload.id, payload.visible)
}

#[command]
pub(crate) async fn set_menu<R: Runtime>(
    app: AppHandle<R>,
    payload: SetMenuRequest,
) -> crate::Result<()> {
    app.multiline_menubar()
        .set_menu(payload.id, payload.items)
}

#[command]
pub(crate) async fn remove_menu<R: Runtime>(
    app: AppHandle<R>,
    payload: IdRequest,
) -> crate::Result<()> {
    app.multiline_menubar().remove_menu(payload.id)
}

#[command]
pub(crate) async fn get_rect<R: Runtime>(
    app: AppHandle<R>,
    payload: GetRectRequest,
) -> crate::Result<Rect> {
    app.multiline_menubar().get_rect(payload.id)
}

#[command]
pub(crate) async fn is_visible<R: Runtime>(
    app: AppHandle<R>,
    payload: IdRequest,
) -> crate::Result<VisibilityResponse> {
    Ok(VisibilityResponse {
        visible: app.multiline_menubar().is_visible(payload.id)?,
    })
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
pub(crate) async fn open_popup<R: Runtime>(
    app: AppHandle<R>,
    payload: IdRequest,
) -> crate::Result<()> {
    app.multiline_menubar().open_popup(payload.id)
}

#[command]
pub(crate) async fn close_popup<R: Runtime>(
    app: AppHandle<R>,
    payload: IdRequest,
) -> crate::Result<()> {
    app.multiline_menubar().close_popup(payload.id)
}

#[command]
pub(crate) async fn toggle_popup<R: Runtime>(
    app: AppHandle<R>,
    payload: IdRequest,
) -> crate::Result<()> {
    app.multiline_menubar().toggle_popup(payload.id)
}
