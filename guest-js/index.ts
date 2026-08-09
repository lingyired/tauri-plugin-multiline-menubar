import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface CreateOptions {
  id: string
  top?: string
  bottom?: string
}

export interface IdOptions {
  id: string
}

export interface SetTextOptions {
  id: string
  top: string
  bottom: string
}

export interface FontSizesOptions {
  id: string
  top: number
  bottom: number
}

export interface LayoutOptions {
  id: string
  /**
   * Vertical layout:
   * - `0` (emphasis-bottom, default): small label on top (light weight), large value below (regular weight).
   * - `1` (emphasis-top): the vertical mirror — large value on top, small label below.
   * - `2` (equal): both lines share one size, vertically centered & symmetric.
   */
  layout: number
}

export interface TooltipOptions {
  id: string
  tooltip: string
}

export interface SetVisibleOptions {
  id: string
  visible: boolean
}

export interface Rect {
  x: number
  y: number
  width: number
  height: number
}

export interface PopupWindowOptions {
  label: string
}

export interface SetAutoPopupOptions {
  enabled: boolean
}

export interface VisibilityResult {
  visible: boolean
}

/**
 * Supported font-size range (points) for the two roles. The asymmetric layouts
 * store one size per *role*; in `emphasis-bottom` the top line is `small` and
 * the bottom is `large`, reversed in `emphasis-top`. Values passed to
 * setFontSizes are clamped to these bounds on the native side based on the
 * role each line currently plays.
 */
export const FONT_SIZE_RANGE = {
  small: { min: 5, max: 11 },
  large: { min: 8, max: 16 },
  equal: { min: 5, max: 11 },
} as const

/**
 * A menu item descriptor. The `id` becomes the menu item's `MenuId`, and is
 * reported back as `itemId` on the instance's `menu` event (and to Tauri's
 * global `on_menu_event` on the Rust side).
 */
export type MenuItemDescriptor =
  | {
      type: 'item'
      id: string
      text: string
      accelerator?: string
      disabled?: boolean
    }
  | {
      type: 'check'
      id: string
      text: string
      checked?: boolean
      accelerator?: string
    }
  | { type: 'separator' }
  | { type: 'submenu'; text: string; items: MenuItemDescriptor[] }

export interface SetMenuOptions {
  id: string
  /** Menu items. Omit or pass `null` to detach the menu. */
  items?: MenuItemDescriptor[]
}

/** How a menubar line should be painted. */
export type ColorStyle =
  | { type: 'default' }
  | { type: 'solid'; value: string }

export interface SetColorsOptions {
  id: string
  /** Paint for the top line. */
  top: ColorStyle
  /** Paint for the bottom line. */
  bottom: ColorStyle
}

/** Per-line bold toggle for the two menubar lines. */
export interface SetBoldOptions {
  id: string
  /** Force the top line bold, overriding the weight `layout` derives. */
  top: boolean
  /** Force the bottom line bold, overriding the weight `layout` derives. */
  bottom: boolean
}

/**
 * Per-line font family for the two menubar lines.
 *
 * Each field is a macOS font *family* name — the name shown in Font Book,
 * e.g. `'Menlo'`, `'PingFang SC'`. `null` (or an empty string) keeps the
 * system font for that line. A line still resolves the weight `setLayout` /
 * `setBold` ask for, using the closest face the family provides; unknown
 * names silently fall back to the system font.
 */
export interface SetFontFamilyOptions {
  id: string
  /** Font family for the top line, or `null`/`''` for the system font. */
  top: string | null
  /** Font family for the bottom line, or `null`/`''` for the system font. */
  bottom: string | null
}

/**
 * Per-line monospaced-digit toggle for the two menubar lines.
 *
 * When a line has no explicit font family, `true` switches that line to the
 * system monospaced-digit font (`monospacedDigitSystemFont`), which keeps
 * every digit the same width so frequently-updating numeric text (e.g. a
 * network speed readout) does not jitter as values change. `false` uses the
 * regular system font. An explicit font family takes precedence over this
 * toggle.
 */
export interface SetMonospacedOptions {
  id: string
  /** Render the top line with monospaced digits (`true`) or the system font (`false`). */
  top: boolean
  /** Render the bottom line with monospaced digits (`true`) or the system font (`false`). */
  bottom: boolean
}

/**
 * Per-line horizontal alignment for the two menubar lines.
 *
 * Each field is `0` (left, default), `1` (center) or `2` (right). Any other
 * value is treated as left on the native side, so an instance that never
 * calls {@link setAlignment} keeps rendering left-aligned (the historical
 * behavior). Alignment is orthogonal to size, color, bold, family and
 * monospaced — combine {@link setAlignment} with any of them freely.
 */
export interface SetAlignmentOptions {
  id: string
  /** Horizontal alignment of the top line: `0` left, `1` center, `2` right. */
  top: number
  /** Horizontal alignment of the bottom line: `0` left, `1` center, `2` right. */
  bottom: number
}

/**
 * Horizontal alignment values, for use with {@link setAlignment}.
 * - `0` = left (default)
 * - `1` = center
 * - `2` = right
 */
export const ALIGN_LEFT = 0
export const ALIGN_CENTER = 1
export const ALIGN_RIGHT = 2

export interface ClickEvent {
  id: string
  position: { x: number; y: number }
  rect: Rect
  button: 'left' | 'right'
  buttonState: 'up' | 'down'
}

export interface HoverEvent {
  id: string
  rect: Rect
}

export interface PopupEvent {
  id: string
  window: string
}

export interface ReadyEvent {
  id: string
}

/** Emitted when an item in an instance's context menu is selected. */
export interface MenuSelectionEvent {
  /** The menubar instance the menu belongs to. */
  id: string
  /** The `id` of the selected menu item. */
  itemId: string
  /** Present only for `check` items: the state after the toggle. */
  checked?: boolean
}

// ---------------------------------------------------------------------------
// Event name helpers
// ---------------------------------------------------------------------------

/**
 * Build the fully-qualified event name for a menubar instance. Events are
 * namespaced per instance: `multiline-menubar://{id}//{name}`.
 */
export function eventName(id: string, name: string): string {
  return `multiline-menubar://${id}//${name}`
}

export const EVENT_READY = (id: string) => eventName(id, 'ready')
export const EVENT_CLICK = (id: string) => eventName(id, 'click')
export const EVENT_ENTER = (id: string) => eventName(id, 'enter')
export const EVENT_LEAVE = (id: string) => eventName(id, 'leave')
export const EVENT_POPUP_OPEN = (id: string) => eventName(id, 'popup-open')
export const EVENT_POPUP_CLOSE = (id: string) => eventName(id, 'popup-close')
export const EVENT_MENU = (id: string) => eventName(id, 'menu')

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

export async function create(options: CreateOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|create', { payload: options })
}

export async function remove(options: IdOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|remove', { payload: options })
}

export async function setText(options: SetTextOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|set_text', { payload: options })
}

export async function setFontSizes(options: FontSizesOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|set_font_sizes', {
    payload: options,
  })
}

/**
 * Choose the vertical layout for an instance.
 *
 * - `0` (emphasis-bottom, default): Stats-style — small label on top (light
 *   weight), larger value below (regular weight).
 * - `1` (emphasis-top): the vertical mirror — large value on top (regular
 *   weight), small label below (light weight).
 * - `2` (equal): both lines share one font size, vertically centered and
 *   symmetric.
 *
 * Sizes are stored per *role* (emphasized vs de-emphasized), so switching
 * between the two asymmetric layouts mirrors the item without losing either
 * size, and the equal layout keeps its own remembered size. In equal mode pass
 * equal `top` and `bottom` to {@link setFontSizes}.
 */
export async function setLayout(options: LayoutOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|set_layout', { payload: options })
}

export async function setTooltip(options: TooltipOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|set_tooltip', {
    payload: options,
  })
}

export async function setVisible(options: SetVisibleOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|set_visible', {
    payload: options,
  })
}

/**
 * Attach a context menu to an instance, shown on right click directly beneath
 * the menubar item. Pass `items: null` (or omit it) to detach the menu,
 * mirroring Tauri's `setMenu(null)` semantics.
 *
 * The menu is a real Tauri/muda menu built on the Rust side from this
 * descriptor. Listen for selections with {@link onMenuSelection} — note that
 * `onMenuEvent` from `@tauri-apps/api/menu` will *not* fire for these items,
 * because that channel only carries menus created through Tauri's own `menu`
 * plugin commands.
 */
export async function setMenu(options: SetMenuOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|set_menu', { payload: options })
}

/**
 * Subscribe to context-menu selections for one instance.
 *
 * Returns the usual Tauri unlisten function.
 *
 * ```ts
 * await onMenuSelection('main', (e) => {
 *   if (e.itemId === 'quit') exit()
 * })
 * ```
 */
export async function onMenuSelection(
  id: string,
  handler: (event: MenuSelectionEvent) => void
): Promise<UnlistenFn> {
  return await listen<MenuSelectionEvent>(EVENT_MENU(id), (e) =>
    handler(e.payload)
  )
}

/** Detach the context menu from an instance. */
export async function removeMenu(options: IdOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|remove_menu', {
    payload: options,
  })
}

/** Returns the on-screen rectangle of an instance in macOS screen points. */
export async function rect(options: IdOptions): Promise<Rect> {
  return await invoke<Rect>('plugin:multiline-menubar|rect', {
    payload: options,
  })
}

/**
 * Set the text paint for the top and bottom lines of an instance. Each line
 * accepts a {@link ColorStyle}: `default` (system color, follows dark mode),
 * or `solid` (`#rrggbb`).
 */
export async function setColors(options: SetColorsOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|set_colors', {
    payload: options,
  })
}

/**
 * Force the top and/or bottom line bold, independently of `layout`.
 *
 * Each line is controlled separately: `top`/`bottom` set to `true` renders
 * that line with a bold weight, overriding the weight the layout would
 * otherwise assign it. `false` leaves the line's weight to the layout (the
 * emphasized line regular, the de-emphasized one light).
 */
export async function setBold(options: SetBoldOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|set_bold', {
    payload: options,
  })
}

/**
 * Set the font family of the top and/or bottom line of an instance.
 *
 * Pass `null` (or `''`) for a line to restore its system font. See
 * {@link SetFontFamilyOptions} for how the name is resolved.
 */
export async function setFontFamily(options: SetFontFamilyOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|set_font_family', {
    payload: options,
  })
}

/**
 * Toggle monospaced digits for the top and/or bottom line of an instance,
 * independently of the layout and font family.
 *
 * Each line is controlled separately: `top`/`bottom` set to `true` renders
 * that line with the system monospaced-digit font (constant digit width — a
 * numeric readout like a speed display doesn't jitter), `false` restores the
 * regular system font. An explicit font family set via {@link setFontFamily}
 * takes precedence over this toggle.
 */
export async function setMonospaced(options: SetMonospacedOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|set_monospaced', {
    payload: options,
  })
}

/**
 * Set the horizontal alignment of the top and/or bottom line of an instance,
 * independently of the layout and every other per-line style.
 *
 * Each line is controlled separately via {@link SetAlignmentOptions}: `top`/
 * `bottom` is `0` (left, default), `1` (center) or `2` (right). Alignment does
 * not change the measured text width, so the item is simply repainted in the
 * new alignment. The `ALIGN_LEFT` / `ALIGN_CENTER` / `ALIGN_RIGHT` constants
 * document the integer values.
 */
export async function setAlignment(options: SetAlignmentOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|set_alignment', {
    payload: options,
  })
}

export async function isVisible(options: IdOptions): Promise<boolean> {
  const result = await invoke<VisibilityResult>(
    'plugin:multiline-menubar|is_visible',
    { payload: options }
  )
  return result.visible
}

export async function setPopupWindow(options: PopupWindowOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|set_popup_window', {
    payload: options,
  })
}

export async function setAutoPopup(options: SetAutoPopupOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|set_auto_popup', {
    payload: options,
  })
}

export async function openPopup(options: IdOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|open_popup', { payload: options })
}

export async function closePopup(options: IdOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|close_popup', { payload: options })
}

export async function togglePopup(options: IdOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|toggle_popup', {
    payload: options,
  })
}
