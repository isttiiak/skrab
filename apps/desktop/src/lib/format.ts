/** Compact relative time for a list row: "now", "4m", "3h", "2d", "5 Mar". */
export function timeAgo(epochMillis: number, now: number = Date.now()): string {
  const seconds = Math.max(0, Math.round((now - epochMillis) / 1000));

  if (seconds < 45) return 'now';
  if (seconds < 3600) return `${Math.round(seconds / 60)}m`;
  if (seconds < 86400) return `${Math.round(seconds / 3600)}h`;
  if (seconds < 604800) return `${Math.round(seconds / 86400)}d`;

  return new Date(epochMillis).toLocaleDateString(undefined, {
    day: 'numeric',
    month: 'short',
  });
}

/** Human-readable byte count. */
export function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}
