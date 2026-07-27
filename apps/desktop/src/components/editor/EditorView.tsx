import type Konva from 'konva';
import {
  ArrowUpRight,
  Circle,
  Copy,
  Highlighter,
  Pencil,
  Redo2,
  Square,
  SquareDashedBottom,
  Trash2,
  Type as TypeIcon,
  Undo2,
  X,
} from 'lucide-react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { toast } from 'sonner';
import { closeEditor, readClipImage, saveEditedImage } from '@/lib/tauri';
import { cn } from '@/lib/utils';
import { EditorCanvas } from './EditorCanvas';
import {
  isDrawnShape,
  PALETTE,
  rectFromDrag,
  type Shape,
  STROKE_WIDTHS,
  type ToolId,
} from './tools';
import { useEditorHistory } from './useEditorHistory';

const TOOLS: { id: ToolId; label: string; icon: React.ReactNode }[] = [
  { id: 'pen', label: 'Draw', icon: <Pencil size={15} /> },
  { id: 'arrow', label: 'Arrow', icon: <ArrowUpRight size={15} /> },
  { id: 'rect', label: 'Rectangle', icon: <Square size={15} /> },
  { id: 'ellipse', label: 'Ellipse', icon: <Circle size={15} /> },
  { id: 'text', label: 'Text', icon: <TypeIcon size={15} /> },
  { id: 'highlight', label: 'Highlight', icon: <Highlighter size={15} /> },
  { id: 'pixelate', label: 'Redact', icon: <SquareDashedBottom size={15} /> },
];

const newId = () => Math.random().toString(36).slice(2);

export function EditorView({ clipId }: { clipId: string }) {
  const [image, setImage] = useState<HTMLImageElement | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [tool, setTool] = useState<ToolId>('pen');
  const [color, setColor] = useState<string>(PALETTE[0]);
  const [strokeWidth, setStrokeWidth] = useState<number>(STROKE_WIDTHS[1]);
  const [draft, setDraft] = useState<Shape | null>(null);
  const [scale, setScale] = useState(1);
  const [saving, setSaving] = useState(false);

  const stageRef = useRef<Konva.Stage>(null);
  const viewportRef = useRef<HTMLDivElement>(null);
  const dragStart = useRef<{ x: number; y: number } | null>(null);
  const { shapes, commit, undo, redo, canUndo, canRedo } = useEditorHistory();

  // --- load the image -------------------------------------------------------
  useEffect(() => {
    let objectUrl: string | null = null;

    readClipImage(clipId)
      .then((bytes) => {
        const blob = new Blob([new Uint8Array(bytes)], { type: 'image/png' });
        objectUrl = URL.createObjectURL(blob);
        const img = new Image();
        img.onload = () => setImage(img);
        img.onerror = () => setError('That image could not be decoded.');
        img.src = objectUrl;
      })
      .catch((e: unknown) => setError(e instanceof Error ? e.message : String(e)));

    return () => {
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [clipId]);

  // --- fit the image to the viewport ---------------------------------------
  useEffect(() => {
    if (!image) return;
    const fit = () => {
      const box = viewportRef.current;
      if (!box) return;
      // Never scale above 1: enlarging a screenshot just makes it blurry.
      const next = Math.min(
        (box.clientWidth - 32) / image.width,
        (box.clientHeight - 32) / image.height,
        1,
      );
      setScale(Math.max(next, 0.1));
    };
    fit();
    window.addEventListener('resize', fit);
    return () => window.removeEventListener('resize', fit);
  }, [image]);

  // --- drawing --------------------------------------------------------------
  const onPointerDown = useCallback(
    (point: { x: number; y: number }) => {
      dragStart.current = point;

      if (tool === 'text') {
        const text = window.prompt('Text to add');
        dragStart.current = null;
        if (!text?.trim()) return;
        commit([
          ...shapes,
          {
            id: newId(),
            kind: 'text',
            x: point.x,
            y: point.y,
            text,
            color,
            size: Math.max(strokeWidth * 5, 16),
          },
        ]);
        return;
      }

      if (tool === 'pen') {
        setDraft({
          id: newId(),
          kind: 'pen',
          points: [point.x, point.y],
          color,
          width: strokeWidth,
        });
      }
    },
    [tool, color, strokeWidth, shapes, commit],
  );

  const onPointerMove = useCallback(
    (point: { x: number; y: number }) => {
      const start = dragStart.current;
      if (!start) return;

      setDraft((current) => {
        switch (tool) {
          case 'pen':
            return current?.kind === 'pen'
              ? { ...current, points: [...current.points, point.x, point.y] }
              : current;
          case 'arrow':
            return {
              id: current?.id ?? newId(),
              kind: 'arrow',
              points: [start.x, start.y, point.x, point.y],
              color,
              width: strokeWidth,
            };
          case 'rect':
            return {
              id: current?.id ?? newId(),
              kind: 'rect',
              ...rectFromDrag(start, point),
              color,
              strokeWidth,
            };
          case 'ellipse':
            return {
              id: current?.id ?? newId(),
              kind: 'ellipse',
              ...rectFromDrag(start, point),
              color,
              strokeWidth,
            };
          case 'highlight':
            return {
              id: current?.id ?? newId(),
              kind: 'highlight',
              ...rectFromDrag(start, point),
              color,
            };
          case 'pixelate':
            return { id: current?.id ?? newId(), kind: 'pixelate', ...rectFromDrag(start, point) };
          default:
            return current;
        }
      });
    },
    [tool, color, strokeWidth],
  );

  const onPointerUp = useCallback(() => {
    dragStart.current = null;
    setDraft((current) => {
      // A click with no drag is not an annotation — dropping it here keeps stray
      // zero-size shapes out of the undo history entirely.
      if (current && isDrawnShape(current)) commit([...shapes, current]);
      return null;
    });
  }, [shapes, commit]);

  // --- export ---------------------------------------------------------------
  const exportPng = useCallback((): Uint8Array | null => {
    const stage = stageRef.current;
    if (!stage || !image) return null;

    // Export at full resolution regardless of how the canvas is displayed.
    const uri = stage.toDataURL({ pixelRatio: 1 / scale, mimeType: 'image/png' });
    const base64 = uri.split(',')[1];
    if (!base64) return null;

    const binary = atob(base64);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i);
    return bytes;
  }, [image, scale]);

  const save = useCallback(async () => {
    const bytes = exportPng();
    if (!bytes) {
      toast.error('Could not render the image');
      return;
    }
    setSaving(true);
    try {
      await saveEditedImage(Array.from(bytes));
      toast.success('Copied to clipboard and saved to history');
      await closeEditor();
    } catch (e) {
      toast.error(e instanceof Error ? e.message : 'Could not save the image');
    } finally {
      setSaving(false);
    }
  }, [exportPng]);

  // --- keyboard -------------------------------------------------------------
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const mod = event.metaKey || event.ctrlKey;
      if (mod && event.key.toLowerCase() === 'z') {
        event.preventDefault();
        if (event.shiftKey) redo();
        else undo();
      } else if (mod && event.key === 'Enter') {
        event.preventDefault();
        void save();
      } else if (event.key === 'Escape') {
        void closeEditor();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [undo, redo, save]);

  if (error) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
        <p className="text-sm font-medium">Could not open that image</p>
        <p className="text-muted-foreground max-w-md text-xs">{error}</p>
        <button
          type="button"
          onClick={() => void closeEditor()}
          className="gradient-brand rounded-md px-3 py-1.5 text-xs font-medium text-white"
        >
          Close
        </button>
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <header className="border-border flex shrink-0 flex-wrap items-center gap-2 border-b px-3 py-2">
        <div className="flex items-center gap-0.5">
          {TOOLS.map((t) => (
            <button
              key={t.id}
              type="button"
              title={t.label}
              aria-label={t.label}
              aria-pressed={tool === t.id}
              onClick={() => setTool(t.id)}
              className={cn(
                'rounded-md p-1.5 transition-colors',
                tool === t.id
                  ? 'gradient-brand text-white'
                  : 'text-muted-foreground hover:bg-surface-muted hover:text-foreground',
              )}
            >
              {t.icon}
            </button>
          ))}
        </div>

        <div className="bg-border h-5 w-px" />

        <div className="flex items-center gap-1">
          {PALETTE.map((c) => (
            <button
              key={c}
              type="button"
              aria-label={`Colour ${c}`}
              onClick={() => setColor(c)}
              style={{ backgroundColor: c }}
              className={cn(
                'h-5 w-5 rounded-full border transition-transform',
                color === c
                  ? 'border-primary scale-110 ring-2 ring-offset-1'
                  : 'border-border hover:scale-105',
              )}
            />
          ))}
        </div>

        <div className="bg-border h-5 w-px" />

        <div className="flex items-center gap-0.5">
          {STROKE_WIDTHS.map((w) => (
            <button
              key={w}
              type="button"
              aria-label={`Stroke ${w}`}
              onClick={() => setStrokeWidth(w)}
              className={cn(
                'flex h-7 w-7 items-center justify-center rounded-md transition-colors',
                strokeWidth === w ? 'bg-primary-soft' : 'hover:bg-surface-muted',
              )}
            >
              <span
                className="bg-foreground rounded-full"
                style={{ width: w + 2, height: w + 2 }}
              />
            </button>
          ))}
        </div>

        <div className="ml-auto flex items-center gap-0.5">
          <IconButton label="Undo" disabled={!canUndo} onClick={undo}>
            <Undo2 size={15} />
          </IconButton>
          <IconButton label="Redo" disabled={!canRedo} onClick={redo}>
            <Redo2 size={15} />
          </IconButton>
          <IconButton
            label="Clear all annotations"
            disabled={shapes.length === 0}
            onClick={() => commit([])}
          >
            <Trash2 size={15} />
          </IconButton>

          <button
            type="button"
            disabled={saving}
            onClick={() => void save()}
            className="gradient-brand ml-1 flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium text-white disabled:opacity-60"
          >
            <Copy size={13} />
            {saving ? 'Saving…' : 'Copy'}
          </button>
          <IconButton label="Close" onClick={() => void closeEditor()}>
            <X size={15} />
          </IconButton>
        </div>
      </header>

      <div
        ref={viewportRef}
        className="bg-surface-muted flex min-h-0 flex-1 items-center justify-center overflow-auto p-4"
      >
        {image ? (
          <div className="shadow-lg">
            <EditorCanvas
              image={image}
              shapes={shapes}
              draft={draft}
              scale={scale}
              stageRef={stageRef}
              onPointerDown={onPointerDown}
              onPointerMove={onPointerMove}
              onPointerUp={onPointerUp}
            />
          </div>
        ) : (
          <p className="text-muted-foreground text-sm">Loading image…</p>
        )}
      </div>

      <footer className="border-border text-muted-foreground flex shrink-0 items-center gap-3 border-t px-3 py-1.5 text-[10px]">
        <span>
          {image ? `${image.width} × ${image.height}` : '—'}
          {scale < 1 && image ? ` · shown at ${Math.round(scale * 100)}%` : ''}
        </span>
        <span className="ml-auto flex items-center gap-2.5">
          <Hint keys="⌘/Ctrl+Z" label="undo" />
          <Hint keys="⌘/Ctrl+↵" label="copy" />
          <Hint keys="esc" label="close" />
        </span>
      </footer>
    </div>
  );
}

function IconButton({
  label,
  disabled = false,
  onClick,
  children,
}: {
  label: string;
  disabled?: boolean;
  onClick: () => void;
  children: React.ReactNode;
}) {
  return (
    <button
      type="button"
      title={label}
      aria-label={label}
      disabled={disabled}
      onClick={onClick}
      className="text-muted-foreground hover:bg-surface-muted hover:text-foreground rounded-md p-1.5 transition-colors disabled:pointer-events-none disabled:opacity-35"
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
