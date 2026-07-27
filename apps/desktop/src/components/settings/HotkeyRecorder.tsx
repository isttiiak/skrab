import { AlertTriangle, Check, Keyboard, X } from 'lucide-react';
import { useCallback, useEffect, useState } from 'react';
import { cn } from '@/lib/utils';

/** Modifier keys never form a shortcut on their own. */
const MODIFIER_KEYS = new Set(['Control', 'Shift', 'Alt', 'Meta', 'CapsLock']);

/**
 * Turns a keydown into a Tauri accelerator string.
 *
 * `CmdOrCtrl` rather than a platform-specific modifier, so a binding recorded on a
 * Mac still means the right thing if the settings travel to a Windows machine.
 * Returns null while the user is still only holding modifiers.
 */
export function accelaratorFrom(event: KeyboardEvent): string | null {
  if (MODIFIER_KEYS.has(event.key)) return null;

  const parts: string[] = [];
  if (event.metaKey || event.ctrlKey) parts.push('CmdOrCtrl');
  if (event.altKey) parts.push('Alt');
  if (event.shiftKey) parts.push('Shift');

  let key = event.key;
  if (key === ' ') key = 'Space';
  else if (key.length === 1) key = key.toUpperCase();

  // A bare letter would swallow ordinary typing everywhere on the system.
  if (parts.length === 0) return null;

  parts.push(key);
  return parts.join('+');
}

/** Renders an accelerator the way the current platform labels its keys. */
export function prettyAccelerator(accelerator: string): string {
  const isMac = navigator.userAgent.includes('Mac');
  return accelerator
    .split('+')
    .map((part) => {
      if (part === 'CmdOrCtrl') return isMac ? '⌘' : 'Ctrl';
      if (part === 'Alt') return isMac ? '⌥' : 'Alt';
      if (part === 'Shift') return isMac ? '⇧' : 'Shift';
      return part;
    })
    .join(isMac ? '' : '+');
}

type Props = {
  label: string;
  accelerator: string;
  registered: boolean;
  problem?: string | null;
  onChange: (accelerator: string) => void;
};

export function HotkeyRecorder({ label, accelerator, registered, problem, onChange }: Props) {
  const [recording, setRecording] = useState(false);

  const capture = useCallback(
    (event: KeyboardEvent) => {
      // Swallow everything while recording, or the shortcut being recorded would
      // also trigger whatever it is currently bound to.
      event.preventDefault();
      event.stopPropagation();

      if (event.key === 'Escape') {
        setRecording(false);
        return;
      }

      const next = accelaratorFrom(event);
      if (next) {
        onChange(next);
        setRecording(false);
      }
    },
    [onChange],
  );

  useEffect(() => {
    if (!recording) return;
    window.addEventListener('keydown', capture, true);
    return () => window.removeEventListener('keydown', capture, true);
  }, [recording, capture]);

  return (
    <div className="space-y-1">
      <div className="flex items-center justify-between gap-3">
        <span className="text-xs font-medium">{label}</span>
        <div className="flex items-center gap-1">
          <button
            type="button"
            onClick={() => setRecording((r) => !r)}
            className={cn(
              'flex min-w-24 items-center justify-center gap-1.5 rounded-md border px-2 py-1 font-mono text-[11px] transition-colors',
              recording
                ? 'border-primary bg-primary-soft text-primary animate-pulse'
                : problem
                  ? 'border-destructive/50 text-destructive'
                  : 'border-border bg-surface hover:border-primary/50',
            )}
          >
            {recording ? (
              <>
                <Keyboard size={12} />
                Press keys…
              </>
            ) : accelerator ? (
              prettyAccelerator(accelerator)
            ) : (
              'Not set'
            )}
          </button>
          {accelerator && !recording && (
            <button
              type="button"
              onClick={() => onChange('')}
              aria-label={`Clear shortcut for ${label}`}
              className="text-muted-foreground hover:text-destructive rounded p-1"
            >
              <X size={12} />
            </button>
          )}
        </div>
      </div>

      {problem ? (
        <p className="text-destructive flex items-start gap-1 text-[10px]">
          <AlertTriangle size={11} className="mt-px shrink-0" />
          {problem}
        </p>
      ) : registered ? (
        <p className="text-muted-foreground flex items-center gap-1 text-[10px]">
          <Check size={11} className="text-primary" />
          Active
        </p>
      ) : null}
    </div>
  );
}
