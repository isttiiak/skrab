// Public IPC surface. Types come from `./generated/`, which ts-rs writes from
// the Rust structs — see README.md. Add one re-export line per new type.
export type { AppInfo } from './generated/AppInfo.ts';
export type { AppSettings } from './generated/AppSettings.ts';
export type { CaptureMode } from './generated/CaptureMode.ts';
export type { CaptureRegion } from './generated/CaptureRegion.ts';
export type { CaptureResult } from './generated/CaptureResult.ts';
export type { ClipItem } from './generated/ClipItem.ts';
export type { ClipQuery } from './generated/ClipQuery.ts';
export type { ClipType } from './generated/ClipType.ts';
export type { HistoryStats } from './generated/HistoryStats.ts';
export type { HotkeyAction } from './generated/HotkeyAction.ts';
export type { HotkeyBindings } from './generated/HotkeyBindings.ts';
export type { HotkeyStatus } from './generated/HotkeyStatus.ts';
export type { MonitorInfo } from './generated/MonitorInfo.ts';
export type { OverlayFrame } from './generated/OverlayFrame.ts';
export type { WindowInfo } from './generated/WindowInfo.ts';

/// Event emitted by Rust whenever the clipboard history changes.
export const CLIP_ADDED_EVENT = 'skrab://clip-added';
