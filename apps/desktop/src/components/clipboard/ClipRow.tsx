import type { ClipItem } from '@skrab/ipc-types';
import {
  Check,
  Code2,
  Copy,
  FileText,
  Image as ImageIcon,
  Pin,
  Star,
  Trash2,
  Type,
} from 'lucide-react';
import { useEffect, useState } from 'react';
import { timeAgo } from '@/lib/format';
import { cn } from '@/lib/utils';

const TYPE_ICON = {
  text: Type,
  html: Code2,
  rtf: FileText,
  file: FileText,
  image: ImageIcon,
} as const;

type Props = {
  clip: ClipItem;
  selected: boolean;
  onSelect: () => void;
  onCopy: () => void;
  onToggleFavorite: () => void;
  onTogglePinned: () => void;
  onDelete: () => void;
};

export function ClipRow({
  clip,
  selected,
  onSelect,
  onCopy,
  onToggleFavorite,
  onTogglePinned,
  onDelete,
}: Props) {
  const Icon = TYPE_ICON[clip.clipType] ?? Type;

  return (
    <li
      className={cn(
        'group relative flex items-stretch gap-1 rounded-lg border pr-1 transition-all',
        selected
          ? 'border-primary/40 bg-primary-soft/60 shadow-sm'
          : 'hover:border-border hover:bg-surface-muted border-transparent',
      )}
    >
      {/* Selected-row accent bar */}
      <span
        aria-hidden
        className={cn(
          'absolute top-2 bottom-2 left-0 w-[3px] rounded-full transition-opacity',
          selected ? 'gradient-brand opacity-100' : 'opacity-0',
        )}
      />

      {/* The row body is a real button: one click selects, double click pastes.
          Keyboard navigation is owned by the panel, so this stays out of the tab
          order and never steals focus from the search field. */}
      <button
        type="button"
        tabIndex={-1}
        onClick={onSelect}
        onDoubleClick={onCopy}
        className="flex min-w-0 flex-1 cursor-pointer items-start gap-3 px-3 py-2.5 text-left"
      >
        {clip.thumb ? (
          <img
            src={clip.thumb}
            alt=""
            className="border-border h-10 w-10 shrink-0 rounded-md border object-cover"
          />
        ) : (
          <span
            className={cn(
              'flex h-10 w-10 shrink-0 items-center justify-center rounded-md',
              selected ? 'gradient-brand text-white' : 'bg-surface-muted text-muted-foreground',
            )}
          >
            <Icon size={16} strokeWidth={2} />
          </span>
        )}

        <span className="min-w-0 flex-1">
          <span className="block truncate text-sm leading-snug" data-selectable>
            {clip.preview || <span className="text-muted-foreground italic">empty</span>}
          </span>
          <span className="text-muted-foreground mt-1 flex items-center gap-1.5 text-[11px]">
            {clip.isPinned && <Pin size={11} className="text-accent" fill="currentColor" />}
            {clip.isFavorite && <Star size={11} className="text-accent" fill="currentColor" />}
            {clip.sourceApp && <span className="max-w-28 truncate">{clip.sourceApp}</span>}
            {clip.sourceApp && <span aria-hidden>·</span>}
            <span>{timeAgo(clip.createdAt)}</span>
          </span>
        </span>
      </button>

      {/* Actions stay mounted for keyboard users; only their opacity is hover-gated.
          Copy is always fully visible — it is the primary action on a row, and
          hiding it behind hover made the list feel read-only. */}
      <div className="flex shrink-0 items-center gap-0.5">
        <CopyAction onCopy={onCopy} />
      </div>
      <div className="flex shrink-0 items-center gap-0.5 opacity-0 transition-opacity group-hover:opacity-100 focus-within:opacity-100">
        <RowAction
          label={clip.isPinned ? 'Unpin' : 'Pin'}
          active={clip.isPinned}
          onClick={onTogglePinned}
        >
          <Pin size={14} />
        </RowAction>
        <RowAction
          label={clip.isFavorite ? 'Remove favorite' : 'Favorite'}
          active={clip.isFavorite}
          onClick={onToggleFavorite}
        >
          <Star size={14} />
        </RowAction>
        <RowAction label="Delete" destructive onClick={onDelete}>
          <Trash2 size={14} />
        </RowAction>
      </div>
    </li>
  );
}

/**
 * Explicit copy button.
 *
 * Briefly swaps to a tick so the click is acknowledged even when the panel stays
 * open — without it there is no feedback that anything happened, because the
 * clipboard is invisible.
 */
function CopyAction({ onCopy }: { onCopy: () => void }) {
  const [copied, setCopied] = useState(false);

  useEffect(() => {
    if (!copied) return;
    const timer = setTimeout(() => setCopied(false), 1200);
    return () => clearTimeout(timer);
  }, [copied]);

  return (
    <button
      type="button"
      title="Copy to clipboard"
      aria-label="Copy to clipboard"
      onClick={() => {
        setCopied(true);
        onCopy();
      }}
      className={cn(
        'focus-visible:ring-ring rounded-md p-1.5 transition-colors focus-visible:ring-2 focus-visible:outline-none',
        copied
          ? 'bg-primary-soft text-primary'
          : 'text-muted-foreground hover:bg-primary-soft hover:text-primary',
      )}
    >
      {copied ? <Check size={14} /> : <Copy size={14} />}
    </button>
  );
}

function RowAction({
  label,
  active = false,
  destructive = false,
  onClick,
  children,
}: {
  label: string;
  active?: boolean;
  destructive?: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      onClick={onClick}
      className={cn(
        'focus-visible:ring-ring rounded-md p-1.5 transition-colors focus-visible:ring-2 focus-visible:outline-none',
        destructive
          ? 'text-muted-foreground hover:bg-destructive-soft hover:text-destructive'
          : active
            ? 'text-accent hover:bg-accent-soft'
            : 'text-muted-foreground hover:bg-surface-muted hover:text-foreground',
      )}
    >
      {children}
    </button>
  );
}
