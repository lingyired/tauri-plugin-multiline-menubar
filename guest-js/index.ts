import { invoke } from '@tauri-apps/api/core'

export interface SetTextOptions {
  top: string
  bottom: string
}

export interface FontSizesOptions {
  top: number
  bottom: number
}

export interface VisibilityResult {
  visible: boolean
}

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
 * Check whether the multiline menu bar item is currently visible.
 */
export async function isVisible(): Promise<boolean> {
  const result = await invoke<VisibilityResult>('plugin:multiline-menubar|is_visible')
  return result.visible
}
