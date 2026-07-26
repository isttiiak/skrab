import type { ClipItem } from '@skrab/ipc-types';
import { CLIP_ADDED_EVENT } from '@skrab/ipc-types';
import { listen } from '@tauri-apps/api/event';
import { Check, GripHorizontal, Pin, X } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { closePinsWidget, listClips, pasteClip } from '@/lib/tauri';
import { cn } from '@/lib/utils';

/**
 * The always-on-top smart-paste widget.
 *
 * Park it beside a form and click each value into place. Used items are marked so
 * you can see how far through a form you are, and auto-advance moves the highlight
 * to the next one so a whole form is a run of clicks in the same spot.
 */
export function PinnedWidget() {
  const [clips, setClips] = useState<ClipItem[]>([]);
  const [used, setUsed] = useState<Set<string>>(new Set());
  const [active, setActive] = useState(0);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    try {
      const pinned = await listClips({
        search: null,
        clipType: null,
        favoritesOnly: null,
        pinnedOnly: true,
        limit: 50,
        offset: 0,
      });
      setClips(pinned);
      setError(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  }, []);

  useEffect(() => {
    void refresh();
    const pending = listen(CLIP_ADDED_EVENT, () => void refresh());
    return () => {
      void pending.then((unlisten) => unlisten());
    };
  }, [refresh]);

  const use = useCallback(
    async (clip: ClipItem, index: number) => {
      try {
        await pasteClip(clip.id);
        setUsed((prev) => new Set(prev).add(clip.id));
        // Auto-advance: the next unused item becomes active, so filling a form is a
        // run of clicks in the same place rather than aiming at a new row each time.
        setActive(Math.min(index + 1, Math.max(clips.length - 1, 0)));
      } catch (e) {
        setError(e instanceof Error ? e.message : String(e));
      }
    },
    [clips.length],
  );

  return (
    <div className="bg-surface border-border flex h-full flex-col overflow-hidden rounded-xl border shadow-lg">
      {/* The whole header is the drag handle — the window has no title bar. */}
      <header
        data-tauri-drag-region
        className="border-border flex shrink-0 cursor-grab items-center gap-1.5 border-b px-2.5 py-2 active:cursor-grabbing"
      >
        <GripHorizontal size={13} className="text-muted-foreground pointer-events-none" />
        <span className="pointer-events-none text-xs font-semibold">Pinned</span>
        {used.size > 0 && (
          <button
            type="button"
            onClick={() => setUsed(new Set())}
            className="text-muted-foreground hover:text-foreground ml-auto rounded px-1.5 py-0.5 text-[10px]"
          >
            Reset
          </button>
        )}
        <button
          type="button"
          onClick={() => void closePinsWidget()}
          aria-label="Close"
          className={cn(
            'text-muted-foreground hover:bg-destructive-soft hover:text-destructive rounded p-1',
            used.size > 0 ? '' : 'ml-auto',
          )}
        >
          <X size={13} />
        </button>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto p-1.5">
        {error ? (
          <p className="text-destructive px-2 py-3 text-[11px]">{error}</p>
        ) : clips.length === 0 ? (
          <div className="text-muted-foreground flex h-full flex-col items-center justify-center gap-1.5 px-4 text-center">
            <Pin size={16} />
            <p className="text-[11px] leading-snug">
              Pin clips in the main panel and they will appear here.
            </p>
          </div>
        ) : (
          <ul className="flex flex-col gap-1">
            {clips.map((clip, index) => {
              const isUsed = used.has(clip.id);
              return (
                <li key={clip.id}>
                  <button
                    type="button"
                    onClick={() => void use(clip, index)}
                    className={cn(
                      'flex w-full items-center gap-2 rounded-lg border px-2 py-1.5 text-left transition-colors',
                      index === active && !isUsed
                        ? 'border-primary/50 bg-primary-soft/60'
                        : 'hover:bg-surface-muted border-transparent',
                      isUsed && 'opacity-55',
                    )}
                  >
                    <span
                      className={cn(
                        'flex h-5 w-5 shrink-0 items-center justify-center rounded-md text-[10px] font-semibold',
                        isUsed
                          ? 'gradient-brand text-white'
                          : 'bg-surface-muted text-muted-foreground',
                      )}
                    >
                      {isUsed ? <Check size={11} /> : index + 1}
                    </span>
                    <span
                      className={cn(
                        'min-w-0 flex-1 truncate text-[11px]',
                        isUsed && 'line-through',
                      )}
                    >
                      {clip.preview}
                    </span>
                  </button>
                </li>
              );
            })}
          </ul>
        )}
      </div>

      <footer className="border-border text-muted-foreground shrink-0 border-t px-2.5 py-1 text-[10px]">
        {clips.length > 0 ? `${used.size} of ${clips.length} used` : 'Nothing pinned'}
      </footer>
    </div>
  );
}
