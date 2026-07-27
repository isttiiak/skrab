import type {
  AppInfo,
  AppSettings,
  CaptureRegion,
  CaptureResult,
  ClipItem,
  ClipQuery,
  HistoryStats,
  HotkeyBindings,
  HotkeyStatus,
  OverlayFrame,
  WindowInfo,
} from '@skrab/ipc-types';
import { invoke } from '@tauri-apps/api/core';

/**
 * Typed wrappers around the Rust command surface.
 *
 * Argument and return types come from `@skrab/ipc-types`, which ts-rs generates
 * from the Rust structs. Nothing here invents a shape by hand — if a wrapper stops
 * compiling, the Rust side changed.
 *
 * Every call goes through `call()` so that a Rust `Err` surfaces as a real
 * `Error` with a usable message rather than a bare string rejection.
 */
async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (cause) {
    const message = typeof cause === 'string' ? cause : String(cause);
    throw new Error(`${command} failed: ${message}`, { cause });
  }
}

export function getAppInfo(): Promise<AppInfo> {
  return call<AppInfo>('get_app_info');
}

// ------------------------------------------------------------------ history

export function listClips(query: ClipQuery): Promise<ClipItem[]> {
  return call<ClipItem[]>('list_clips', { query });
}

export function getClipContent(id: string): Promise<string | null> {
  return call<string | null>('get_clip_content', { id });
}

export function copyClip(id: string): Promise<void> {
  return call<void>('copy_clip', { id });
}

export function setClipFavorite(id: string, value: boolean): Promise<void> {
  return call<void>('set_clip_favorite', { id, value });
}

export function setClipPinned(id: string, value: boolean): Promise<void> {
  return call<void>('set_clip_pinned', { id, value });
}

export function deleteClip(id: string): Promise<void> {
  return call<void>('delete_clip', { id });
}

export function clearHistory(): Promise<void> {
  return call<void>('clear_history');
}

export function historyStats(): Promise<HistoryStats> {
  return call<HistoryStats>('history_stats');
}

// ----------------------------------------------------------------- settings

export function getSettings(): Promise<AppSettings> {
  return call<AppSettings>('get_settings');
}

export function saveSettings(settings: AppSettings): Promise<AppSettings> {
  return call<AppSettings>('save_settings', { settings });
}

export function setMonitoring(enabled: boolean): Promise<void> {
  return call<void>('set_monitoring', { enabled });
}

// ------------------------------------------------------------------ hotkeys

export function setHotkeys(bindings: HotkeyBindings): Promise<HotkeyStatus[]> {
  return call<HotkeyStatus[]>('set_hotkeys', { bindings });
}

export function hotkeyStatus(): Promise<HotkeyStatus[]> {
  return call<HotkeyStatus[]>('hotkey_status');
}

export function defaultHotkeys(): Promise<HotkeyBindings> {
  return call<HotkeyBindings>('default_hotkeys');
}

// ------------------------------------------------------------------ capture

export function startRegionCapture(): Promise<void> {
  return call<void>('start_region_capture');
}

export function captureFullscreen(): Promise<void> {
  return call<void>('capture_fullscreen');
}

export function getOverlayFrame(): Promise<OverlayFrame | null> {
  return call<OverlayFrame | null>('get_overlay_frame');
}

export function cancelRegionCapture(): Promise<void> {
  return call<void>('cancel_region_capture');
}

export function finishRegionCapture(region: CaptureRegion): Promise<CaptureResult> {
  return call<CaptureResult>('finish_region_capture', { region });
}

// ---------------------------------------------------------------- smart paste

/**
 * Copies a pinned clip, and pastes it into the app behind when the user has
 * enabled auto-paste. Resolves to `true` only if a keystroke was actually sent.
 */
export function pasteClip(id: string): Promise<boolean> {
  return call<boolean>('paste_clip', { id });
}

export function setAlwaysOnTop(pinned: boolean): Promise<void> {
  return call<void>('set_always_on_top', { pinned });
}

export function getAlwaysOnTop(): Promise<boolean> {
  return call<boolean>('get_always_on_top');
}

export function listCapturableWindows(): Promise<WindowInfo[]> {
  return call<WindowInfo[]>('list_capturable_windows');
}

export function captureWindow(windowId: number): Promise<CaptureResult> {
  return call<CaptureResult>('capture_window_by_id', { windowId });
}

// ------------------------------------------------------------------- window

export function hidePanel(): Promise<void> {
  return call<void>('hide_panel');
}

export function openDataDir(): Promise<void> {
  return call<void>('open_data_dir');
}
