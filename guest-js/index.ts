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

/** A menu item descriptor. The `id` becomes the menu item's `MenuId`, and is
 *  reported back as `itemId` on the instance's `menu` event (and to Tauri's
 *  global `on_menu_event` on the Rust side). */
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
  items: MenuItemDescriptor[]
}

/** How a menubar line should be painted. */
export type ColorStyle =
  | { type: 'default' }
  | { type: 'solid'; value: string }

export interface SetColorsOptions {
  id: string
  /** Paint for the top (small) line. */
  top: ColorStyle
  /** Paint for the bottom (large) line. */
  bottom: ColorStyle
}

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

/**
 * Supported font-size range (points) for the two lines.
 * Values passed to setFontSizes are clamped to these bounds on the native side.
 */
export const FONT_SIZE_RANGE = {
  top: { min: 5, max: 11 },
  bottom: { min: 8, max: 16 },
} as const

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

export async function create(options: CreateOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|create', { payload: options })
}

export async function destroy(options: IdOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|destroy', { payload: options })
}

export async function show(options: IdOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|show', { payload: options })
}

export async function hide(options: IdOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|hide', { payload: options })
}

export async function setText(options: SetTextOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|set_text', { payload: options })
}

export async function setFontSizes(options: FontSizesOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|set_font_sizes', {
    payload: options,
  })
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
 * the menubar item.
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
export async function getRect(options: IdOptions): Promise<Rect> {
  return await invoke<Rect>('plugin:multiline-menubar|get_rect', {
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
