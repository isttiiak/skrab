import { useCallback, useState } from 'react';
import type { Shape } from './tools.ts';

type History = {
  past: Shape[][];
  present: Shape[];
  future: Shape[][];
};

/** How many steps back the editor can go. */
const LIMIT = 60;

/**
 * Undo/redo over the shape list.
 *
 * Stores whole snapshots rather than inverse operations. Annotation lists are short
 * — tens of shapes, not thousands — so a snapshot per edit costs nothing, and it
 * makes every operation undoable without writing a matching undo for each one.
 */
export function useEditorHistory() {
  const [history, setHistory] = useState<History>({ past: [], present: [], future: [] });

  const commit = useCallback((next: Shape[]) => {
    setHistory((h) => ({
      past: [...h.past, h.present].slice(-LIMIT),
      present: next,
      // Any new edit invalidates the redo branch.
      future: [],
    }));
  }, []);

  const undo = useCallback(() => {
    setHistory((h) => {
      const previous = h.past.at(-1);
      if (previous === undefined) return h;
      return {
        past: h.past.slice(0, -1),
        present: previous,
        future: [h.present, ...h.future],
      };
    });
  }, []);

  const redo = useCallback(() => {
    setHistory((h) => {
      const [next, ...rest] = h.future;
      if (next === undefined) return h;
      return { past: [...h.past, h.present], present: next, future: rest };
    });
  }, []);

  const reset = useCallback(() => {
    setHistory({ past: [], present: [], future: [] });
  }, []);

  return {
    shapes: history.present,
    commit,
    undo,
    redo,
    reset,
    canUndo: history.past.length > 0,
    canRedo: history.future.length > 0,
  };
}
