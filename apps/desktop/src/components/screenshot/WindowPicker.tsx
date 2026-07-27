import type { WindowInfo } from '@skrab/ipc-types';
import { AppWindow, X } from 'lucide-react';
import { useEffect, useState } from 'react';
import { toast } from 'sonner';
import { captureWindow, listCapturableWindows } from '@/lib/tauri';

/**
 * Picks one on-screen window to capture.
 *
 * A list rather than click-the-window-you-want: Skrab's own panel is in front, and
 * hiding it to let the user click through would leave nothing to cancel with.
 */
export function WindowPicker({ onClose }: { onClose: () => void }) {
  const [windows, setWindows] = useState<WindowInfo[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listCapturableWindows()
      .then(setWindows)
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));
  }, []);

  const capture = async (target: WindowInfo) => {
    onClose();
    try {
      await captureWindow(target.id);
      toast.success('Window copied to clipboard');
    } catch (e) {
      toast.error(e instanceof Error ? e.message : 'Could not capture that window');
    }
  };

  return (
    <div className="bg-background/95 absolute inset-0 z-20 flex flex-col backdrop-blur-sm">
      <header className="border-border flex shrink-0 items-center gap-2 border-b px-3 py-2.5">
        <AppWindow size={15} className="text-primary" />
        <h2 className="text-sm font-semibold">Capture a window</h2>
        <button
          type="button"
          onClick={onClose}
          aria-label="Cancel"
          className="text-muted-foreground hover:text-foreground ml-auto rounded p-1"
        >
          <X size={14} />
        </button>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto p-2">
        {error ? (
          <p className="text-destructive px-2 py-4 text-xs">{error}</p>
        ) : !windows ? (
          <p className="text-muted-foreground px-2 py-4 text-xs">Looking for windows…</p>
        ) : windows.length === 0 ? (
          <p className="text-muted-foreground px-2 py-4 text-xs">No capturable windows found.</p>
        ) : (
          <ul className="flex flex-col gap-1">
            {windows.map((w) => (
              <li key={w.id}>
                <button
                  type="button"
                  onClick={() => void capture(w)}
                  className="hover:bg-surface-muted flex w-full items-center gap-2.5 rounded-lg px-2.5 py-2 text-left transition-colors"
                >
                  <span className="bg-surface-muted text-muted-foreground flex h-7 w-7 shrink-0 items-center justify-center rounded-md">
                    <AppWindow size={13} />
                  </span>
                  <span className="min-w-0 flex-1">
                    <span className="block truncate text-xs font-medium">{w.title}</span>
                    <span className="text-muted-foreground block truncate text-[10px]">
                      {w.appName} · {w.width}×{w.height}
                    </span>
                  </span>
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}
