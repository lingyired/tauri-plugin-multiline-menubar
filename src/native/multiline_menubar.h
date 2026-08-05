#ifndef MULTILINE_MENUBAR_H
#define MULTILINE_MENUBAR_H

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Click callback invoked when a status item is clicked.
 * @param id        The menubar instance id.
 * @param button    "left" or "right".
 * @param rx        On-screen x of the status item rect (screen points, origin bottom-left).
 * @param ry        On-screen y of the status item rect (screen points, origin bottom-left).
 * @param rw        Width of the status item rect in screen points.
 * @param rh        Height of the status item rect in screen points.
 * @param cx        On-screen x of the cursor at click time (screen points).
 * @param cy        On-screen y of the cursor at click time (screen points).
 */
typedef void (*MultilineMenubarClickCallback)(const char *id, const char *button,
                                              double rx, double ry, double rw,
                                              double rh, double cx, double cy);

/**
 * Hover callback invoked when the cursor enters or leaves a status item.
 * @param id          The menubar instance id.
 * @param hover_type  "enter" or "leave".
 */
typedef void (*MultilineMenubarHoverCallback)(const char *id,
                                              const char *hover_type);

/**
 * Create and show a multiline menubar instance with the given id.
 * Safe to call multiple times for different ids. Calling with an existing id
 * re-shows the instance.
 */
void multiline_menubar_create(const char *id);

/**
 * Destroy a multiline menubar instance, removing it from the status bar.
 */
void multiline_menubar_destroy(const char *id);

/**
 * Show a previously created (or hidden) menubar instance.
 */
void multiline_menubar_show(const char *id);

/**
 * Hide a menubar instance without destroying it.
 */
void multiline_menubar_hide(const char *id);

/**
 * Update the two lines displayed in the menubar.
 * Both pointers may be NULL, in which case an empty string is used.
 */
void multiline_menubar_set_text(const char *id, const char *top,
                                const char *bottom);

/**
 * Update the font sizes (in points) for the top label and bottom value.
 * Values are clamped to the supported range on the native side.
 */
void multiline_menubar_set_style(const char *id, double top_size,
                                 double bottom_size);

/**
 * Switch the vertical layout of a menubar instance.
 *
 * @param layout  0 = stacked (default): a small light label on top and a
 *                larger regular value below (Stats-style).
 *                1 = balanced: both lines share a single font size and are
 *                vertically centered and symmetric.
 *
 * In balanced mode `multiline_menubar_set_style` clamps both lines to one
 * shared range, so callers should pass equal top/bottom sizes.
 */
void multiline_menubar_set_layout(const char *id, int layout);

/**
 * Set the text color(s) for the top and bottom lines.
 *
 * Each `top_json` / `bottom_json` argument is a small JSON object describing
 * the paint for that line:
 *   - `{"type":"default"}`        system `textColor` (follows dark mode)
 *   - `{"type":"solid","value":"#rrggbb"}`
 *
 * An empty or NULL string is treated as `default`.
 */
void multiline_menubar_set_color(const char *id, const char *top_json,
                                 const char *bottom_json);

/**
 * Set the per-line bold toggle for the top and bottom lines.
 *
 * @param top_bold     Non-zero forces the top line to render with
 *                     `NSFontWeightBold`, overriding the weight `layout` would
 *                     otherwise assign it. Zero leaves the top line's weight to
 *                     the layout.
 * @param bottom_bold  Same, for the bottom line.
 */
void multiline_menubar_set_bold(const char *id, int top_bold, int bottom_bold);

/**
 * Set the per-line font family for the top and bottom lines.
 *
 * Each argument is a macOS font *family* name (e.g. "Menlo", "PingFang SC"),
 * or NULL/empty to fall back to the system font for that line. A line keeps
 * resolving the weight `layout`/bold asks for, using the closest face the
 * family provides; unknown names silently fall back to the system font.
 */
void multiline_menubar_set_font_family(const char *id, const char *top_family,
                                       const char *bottom_family);

/**
 * Set the tooltip shown when hovering the menubar item. Pass NULL to clear.
 */
void multiline_menubar_set_tooltip(const char *id, const char *tooltip);

/**
 * Attach a native NSMenu to this instance. A right click pops the menu up
 * directly underneath the status item. Pass NULL to clear the menu.
 *
 * The pointer is expected to come from `muda::ContextMenu::ns_menu()`. It is
 * retained by a strong property here, so the menu object stays alive even if
 * the owning Rust value is dropped. Selections still travel through muda's
 * global event handler, which Tauri installs, so they surface in
 * `on_menu_event` as usual.
 *
 * @param ns_menu  Pointer to the native NSMenu (`*mut c_void`), or NULL.
 */
void multiline_menubar_set_menu(const char *id, void *ns_menu);

/**
 * Fill the on-screen rectangle of the status item. Returns 1 on success
 * (instance exists), 0 otherwise. Coordinates use macOS screen space
 * (origin bottom-left, y increasing upward).
 */
int multiline_menubar_get_rect(const char *id, double *x, double *y,
                               double *width, double *height);

/**
 * Returns 1 if the menubar instance exists and is visible, 0 otherwise.
 */
int multiline_menubar_is_visible(const char *id);

/**
 * Register the click callback. The host (Rust) uses it to open a popup window
 * below the item and emit click events. Pass NULL to clear.
 */
void multiline_menubar_set_click_handler(MultilineMenubarClickCallback callback);

/**
 * Register the hover callback. Pass NULL to clear.
 */
void multiline_menubar_set_hover_handler(MultilineMenubarHoverCallback callback);

#ifdef __cplusplus
}
#endif

#endif /* MULTILINE_MENUBAR_H */
