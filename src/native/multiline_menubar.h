#ifndef MULTILINE_MENUBAR_H
#define MULTILINE_MENUBAR_H

#ifdef __cplusplus
extern "C" {
#endif

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
 * Returns 1 if the menubar item exists and is visible, 0 otherwise.
 */
int multiline_menubar_is_visible(void);

#ifdef __cplusplus
}
#endif

#endif /* MULTILINE_MENUBAR_H */
