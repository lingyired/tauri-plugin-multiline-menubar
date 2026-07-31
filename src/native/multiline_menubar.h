#ifndef MULTILINE_MENUBAR_H
#define MULTILINE_MENUBAR_H

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Click callback invoked when the status item is clicked.
 * @param button  "left" or "right".
 * @param x       On-screen x of the status item (screen points, origin bottom-left).
 * @param y       On-screen y of the status item (screen points, origin bottom-left).
 * @param width   Width of the status item in screen points.
 * @param height  Height of the status item in screen points.
 */
typedef void (*MultilineMenubarClickCallback)(const char *button, double x,
                                              double y, double width,
                                              double height);

/**
 * Create and show the multiline menubar item.
 * Safe to call multiple times.
 */
void multiline_menubar_show(void);

/**
 * Hide the multiline menubar item without destroying it.
 */
void multiline_menubar_hide(void);

/**
 * Update the two lines displayed in the menubar.
 * Both pointers may be NULL, in which case an empty string is used.
 */
void multiline_menubar_set_text(const char *top, const char *bottom);

/**
 * Update the font sizes (in points) for the top label and bottom value.
 * Values are clamped to the supported range on the native side.
 */
void multiline_menubar_set_style(double top_size, double bottom_size);

/**
 * Set the tooltip shown when hovering the menubar item. Pass NULL to clear.
 */
void multiline_menubar_set_tooltip(const char *tooltip);

/**
 * Set the application version string shown in the right-click context menu.
 * Pass NULL to clear. The string is copied.
 */
void multiline_menubar_set_version(const char *version);

/**
 * Register a click callback. The host (Rust) uses it to open a popup window
 * below the item and emit click events. Pass NULL to clear.
 */
void multiline_menubar_set_click_handler(MultilineMenubarClickCallback callback);

/**
 * Returns 1 if the menubar item exists and is visible, 0 otherwise.
 */
int multiline_menubar_is_visible(void);

#ifdef __cplusplus
}
#endif

#endif /* MULTILINE_MENUBAR_H */
