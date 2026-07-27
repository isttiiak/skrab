import { CLIP_ADDED_EVENT } from '@skrab/ipc-types';
import { listen } from '@tauri-apps/api/event';
import {
  ClipboardList,
  Crop,
  Monitor,
  Pin,
  Search,
  Settings as SettingsIcon,
  X,
} from 'lucide-react';
import { useCallback, useEffect, useRef } from 'react';
import { toast } from 'sonner';
import { ClipRow } from '@/components/clipboard/ClipRow';
import { captureFullscreen, hidePanel, startRegionCapture, togglePinsWidget } from '@/lib/tauri';
import { cn } from '@/lib/utils';
import { type TypeFilter, useClipboardStore } from '@/stores/clipboardStore';

const FILTERS: { value: TypeFilter; label: string }[] = [
  { value: 'all', label: 'All' },
  { value: 'text', label: 'Text' },
  { value: 'image', label: 'Images' },
  { value: 'html', label: 'Rich' },
  { value: 'favorites', label: 'Starred' },
];

export function ClipboardPanel({ onOpenSettings }: { onOpenSettings: () => void }) {
  const clips = useClipboardStore((s) => s.clips);
  const search = useClipboardStore((s) => s.search);
  const filter = useClipboardStore((s) => s.filter);
  const selectedIndex = useClipboardStore((s) => s.selectedIndex);
  const error = useClipboardStore((s) => s.error);
  const setSearch = useClipboardStore((s) => s.setSearch);
  const setFilter = useClipboardStore((s) => s.setFilter);
  const setSelectedIndex = useClipboardStore((s) => s.setSelectedIndex);
  const moveSelection = useClipboardStore((s) => s.moveSelection);
  const refresh = useClipboardStore((s) => s.refresh);
  const copy = useClipboardStore((s) => s.copy);
  const toggleFavorite = useClipboardStore((s) => s.toggleFavorite);
  const togglePinned = useClipboardStore((s) => s.togglePinned);
  const remove = useClipboardStore((s) => s.remove);

  const searchRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLUListElement>(null);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  // Rust nudges us whenever the history changes; we re-query rather than trusting
  // a payload, so the list can never drift from the database.
  useEffect(() => {
    const pending = listen(CLIP_ADDED_EVENT, () => {
      void refresh();
    });
    return () => {
      void pending.then((unlisten) => unlisten());
    };
  }, [refresh]);

  // The panel is summoned by a hotkey, so the search box must already be focused —
  // the user is mid-flow and expects to start typing immediately.
  useEffect(() => {
    searchRef.current?.focus();
  }, []);

  // Keep the highlighted row in view during keyboard navigation.
  useEffect(() => {
    const node = listRef.current?.children[selectedIndex];
    node?.scrollIntoView({ block: 'nearest' });
  }, [selectedIndex]);

  /**
   * Puts one clip on the system clipboard.
   *
   * Takes an explicit id rather than reading the selection, so a row's own copy
   * button always copies that row — using `selectedIndex` here meant clicking copy
   * on a row you had not selected first copied the wrong clip.
   *
   * Closing is separate from copying: the row button leaves the panel open so you
   * can grab several things, while Enter copies and dismisses.
   */
  const copyClip = useCallback(
    async (id: string, { close }: { close: boolean }) => {
      try {
        await copy(id);
      } catch (error) {
        // Surface the real reason. This used to fall back to a generic message
        // while the store replaced the whole list with red text, which hid both
        // the list and the actual cause.
        toast.error(error instanceof Error ? error.message : 'Could not copy that clip');
        return;
      }

      if (!close) {
        toast.success('Copied');
        return;
      }

      try {
        await hidePanel();
      } catch {
        // The copy already succeeded; a window that refused to hide is not a
        // reason to tell the user the copy failed.
        toast.success('Copied');
      }
    },
    [copy],
  );

  const copySelected = useCallback(() => {
    const clip = clips[selectedIndex];
    if (!clip) return;
    void copyClip(clip.id, { close: true });
  }, [clips, selectedIndex, copyClip]);

  const onKeyDown = useCallback(
    (event: KeyboardEvent) => {
      switch (event.key) {
        case 'ArrowDown':
          event.preventDefault();
          moveSelection(1);
          break;
        case 'ArrowUp':
          event.preventDefault();
          moveSelection(-1);
          break;
        case 'Enter':
          event.preventDefault();
          copySelected();
          break;
        case 'Escape':
          event.preventDefault();
          if (search) {
            setSearch('');
          } else {
            void hidePanel();
          }
          break;
        default:
          break;
      }
    },
    [moveSelection, copySelected, search, setSearch],
  );

  // Window-level rather than on a wrapper element: focus can legitimately sit on the
  // search input, a row action button, or nothing, and the shortcuts must work in
  // all three cases. A wrapper handler would only fire for whatever it contains.
  useEffect(() => {
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [onKeyDown]);

  return (
    <div className="flex h-full flex-col">
      <header className="border-border shrink-0 border-b px-3 pt-3 pb-2">
        <div className="mb-2.5 flex items-center gap-2">
          <span className="gradient-brand flex h-7 w-7 items-center justify-center rounded-lg text-white shadow-sm">
            <ClipboardList size={15} />
          </span>
          <h1 className="text-sm font-semibold tracking-tight">Skrab</h1>

          <div className="ml-auto flex items-center gap-0.5">
            <ToolButton
              label="Pinned items — click any pin to copy it"
              onClick={() => void togglePinsWidget()}
            >
              <Pin size={15} />
            </ToolButton>
            <ToolButton label="Capture a region" onClick={() => void startRegionCapture()}>
              <Crop size={15} />
            </ToolButton>
            <ToolButton label="Capture the screen" onClick={() => void captureFullscreen()}>
              <Monitor size={15} />
            </ToolButton>
          </div>

          <button
            type="button"
            onClick={onOpenSettings}
            aria-label="Settings"
            title="Settings"
            className="text-muted-foreground hover:bg-surface-muted hover:text-foreground focus-visible:ring-ring rounded-md p-1.5 transition-colors focus-visible:ring-2 focus-visible:outline-none"
          >
            <SettingsIcon size={15} />
          </button>
        </div>

        <div className="relative">
          <Search
            size={14}
            className="text-muted-foreground pointer-events-none absolute top-1/2 left-2.5 -translate-y-1/2"
          />
          <input
            ref={searchRef}
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Search your clipboard…"
            aria-label="Search clipboard history"
            className="border-border bg-surface focus:border-primary/50 focus:ring-primary/20 w-full rounded-lg border py-1.5 pr-8 pl-8 text-sm transition-colors focus:ring-2 focus:outline-none"
          />
          {search && (
            <button
              type="button"
              onClick={() => setSearch('')}
              aria-label="Clear search"
              className="text-muted-foreground hover:text-foreground absolute top-1/2 right-2 -translate-y-1/2 rounded p-0.5"
            >
              <X size={13} />
            </button>
          )}
        </div>

        <div className="mt-2 flex gap-1 overflow-x-auto pb-0.5">
          {FILTERS.map((f) => (
            <button
              key={f.value}
              type="button"
              onClick={() => setFilter(f.value)}
              className={cn(
                'shrink-0 rounded-full px-2.5 py-1 text-[11px] font-medium transition-colors',
                filter === f.value
                  ? 'gradient-brand text-white shadow-sm'
                  : 'bg-surface-muted text-muted-foreground hover:text-foreground',
              )}
            >
              {f.label}
            </button>
          ))}
        </div>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto px-2 py-2">
        {error ? (
          <p className="text-destructive px-2 py-4 text-xs">{error}</p>
        ) : clips.length === 0 ? (
          <EmptyState searching={search.length > 0} />
        ) : (
          <ul ref={listRef} className="flex flex-col gap-1">
            {clips.map((clip, index) => (
              <ClipRow
                key={clip.id}
                clip={clip}
                selected={index === selectedIndex}
                onSelect={() => setSelectedIndex(index)}
                onCopy={() => void copyClip(clip.id, { close: false })}
                onToggleFavorite={() => void toggleFavorite(clip.id)}
                onTogglePinned={() => void togglePinned(clip.id)}
                onDelete={() => void remove(clip.id)}
              />
            ))}
          </ul>
        )}
      </div>

      <footer className="border-border text-muted-foreground shrink-0 border-t px-3 py-1.5 text-[10px]">
        <span className="flex items-center gap-2.5">
          <Hint keys="↑↓" label="navigate" />
          <Hint keys="↵" label="copy & close" />
          <Hint keys="esc" label="close" />
          <span className="ml-auto">{clips.length} shown</span>
        </span>
      </footer>
    </div>
  );
}

function ToolButton({
  label,
  onClick,
  children,
}: {
  label: string;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      onClick={onClick}
      className="text-muted-foreground hover:bg-primary-soft hover:text-primary focus-visible:ring-ring rounded-md p-1.5 transition-colors focus-visible:ring-2 focus-visible:outline-none"
    >
      {children}
    </button>
  );
}

function Hint({ keys, label }: { keys: string; label: string }) {
  return (
    <span className="flex items-center gap-1">
      <kbd className="bg-surface-muted border-border rounded border px-1 py-px font-sans">
        {keys}
      </kbd>
      {label}
    </span>
  );
}

function EmptyState({ searching }: { searching: boolean }) {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-2 px-6 text-center">
      <span className="bg-surface-muted text-muted-foreground flex h-11 w-11 items-center justify-center rounded-xl">
        {searching ? <Search size={18} /> : <ClipboardList size={18} />}
      </span>
      <p className="text-sm font-medium">{searching ? 'No matches' : 'Nothing copied yet'}</p>
      <p className="text-muted-foreground text-xs">
        {searching ? 'Try a different word.' : 'Copy something and it will show up here instantly.'}
      </p>
    </div>
  );
}
