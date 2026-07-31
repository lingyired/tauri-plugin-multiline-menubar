import { invoke } from '@tauri-apps/api/core'

export interface SetTextOptions {
  top: string
  bottom: string
}

export interface FontSizesOptions {
  top: number
  bottom: number
}

export interface TooltipOptions {
  tooltip: string
}

export interface SetVisibleOptions {
  visible: boolean
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

export interface ClickEvent {
  button: 'left' | 'right'
  x: number
  y: number
  width: number
  height: number
}

export interface PopupEvent {
  window: string
}

/**
 * Event names emitted by the plugin. Listen with `@tauri-apps/api/event`'s
 * `listen`. These mirror the conventions of the macOS menubar plugin family.
 */
export const EVENT_READY = 'multiline-menubar://ready'
export const EVENT_CLICK = 'multiline-menubar://click'
export const EVENT_POPUP_OPEN = 'multiline-menubar://popup-open'
export const EVENT_POPUP_CLOSE = 'multiline-menubar://popup-close'

/**
 * Supported font-size range (points) for the two lines.
 * Values passed to setFontSizes are clamped to these bounds on the native side.
 */
export const FONT_SIZE_RANGE = {
  top: { min: 5, max: 11 },
  bottom: { min: 8, max: 16 },
} as const

/**
 * Show the multiline menu bar item.
 */
export async function show(): Promise<void> {
  return await invoke('plugin:multiline-menubar|show')
}

/**
 * Hide the multiline menu bar item.
 */
export async function hide(): Promise<void> {
  return await invoke('plugin:multiline-menubar|hide')
}

/**
 * Show or hide the status item.
 */
export async function setVisible(options: SetVisibleOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|set_visible', { payload: options })
}

/**
 * Update the two lines displayed in the menu bar.
 */
export async function setText(options: SetTextOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|set_text', {
    payload: options,
  })
}

/**
 * Update the font size (in points) of the top label and bottom value.
 * Values are clamped to the supported range on the native side.
 */
export async function setFontSizes(options: FontSizesOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|set_font_sizes', {
    payload: options,
  })
}

/**
 * Set the tooltip shown when hovering the menu bar item.
 */
export async function setTooltip(options: TooltipOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|set_tooltip', {
    payload: options,
  })
}

/**
 * Set which Tauri window is used as the popup. Call this before the first
 * open if you use a window label other than "popup".
 */
export async function setPopupWindow(options: PopupWindowOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|set_popup_window', {
    payload: options,
  })
}

/**
 * Enable or disable automatically toggling the popup on left click.
 * When disabled, you can still drive the popup from the `click` event.
 */
export async function setAutoPopup(options: SetAutoPopupOptions): Promise<void> {
  return await invoke('plugin:multiline-menubar|set_auto_popup', {
    payload: options,
  })
}

/**
 * Show and position the popup window below the menu bar item.
 */
export async function openPopup(): Promise<void> {
  return await invoke('plugin:multiline-menubar|open_popup')
}

/**
 * Hide the popup window.
 */
export async function closePopup(): Promise<void> {
  return await invoke('plugin:multiline-menubar|close_popup')
}

/**
 * Toggle the popup window's visibility.
 */
export async function togglePopup(): Promise<void> {
  return await invoke('plugin:multiline-menubar|toggle_popup')
}

/**
 * Check whether the multiline menu bar item is currently visible.
 */
export async function isVisible(): Promise<boolean> {
  const result = await invoke<VisibilityResult>('plugin:multiline-menubar|is_visible')
  return result.visible
}
