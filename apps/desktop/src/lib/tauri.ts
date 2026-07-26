import type { AppInfo, AppSettings, ClipItem, ClipQuery, HistoryStats } from '@skrab/ipc-types';
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

// ------------------------------------------------------------------- window

export function hidePanel(): Promise<void> {
  return call<void>('hide_panel');
}

export function openDataDir(): Promise<void> {
  return call<void>('open_data_dir');
}
