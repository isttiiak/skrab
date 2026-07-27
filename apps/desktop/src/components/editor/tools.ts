/** Everything the editor can draw. */
export type ToolId =
  | 'select'
  | 'pen'
  | 'arrow'
  | 'rect'
  | 'ellipse'
  | 'text'
  | 'highlight'
  | 'pixelate';

/** A single annotation. Shapes are stored, never baked into the image. */
export type Shape =
  | { id: string; kind: 'pen'; points: number[]; color: string; width: number }
  | { id: string; kind: 'arrow'; points: number[]; color: string; width: number }
  | {
      id: string;
      kind: 'rect';
      x: number;
      y: number;
      width: number;
      height: number;
      color: string;
      strokeWidth: number;
    }
  | {
      id: string;
      kind: 'ellipse';
      x: number;
      y: number;
      width: number;
      height: number;
      color: string;
      strokeWidth: number;
    }
  | { id: string; kind: 'text'; x: number; y: number; text: string; color: string; size: number }
  | {
      id: string;
      kind: 'highlight';
      x: number;
      y: number;
      width: number;
      height: number;
      color: string;
    }
  | { id: string; kind: 'pixelate'; x: number; y: number; width: number; height: number };

/**
 * Annotation colours.
 *
 * Chosen to stay legible on both light UI screenshots and dark terminals, which is
 * most of what gets annotated. No pure red — it vanishes on dark backgrounds and
 * reads as an error state.
 */
export const PALETTE = [
  '#f0454f',
  '#f0a23c',
  '#f5d547',
  '#2fb3ae',
  '#3d7fd6',
  '#111827',
  '#ffffff',
] as const;

export const STROKE_WIDTHS = [2, 4, 8, 14] as const;

/** Normalises a drag into a top-left origin plus positive extents. */
export function rectFromDrag(
  start: { x: number; y: number },
  end: { x: number; y: number },
): { x: number; y: number; width: number; height: number } {
  return {
    x: Math.min(start.x, end.x),
    y: Math.min(start.y, end.y),
    width: Math.abs(end.x - start.x),
    height: Math.abs(end.y - start.y),
  };
}

/** Drags smaller than this are treated as accidental clicks, not shapes. */
export const MIN_DRAG = 4;

export function isDrawnShape(shape: Shape): boolean {
  switch (shape.kind) {
    case 'pen':
    case 'arrow':
      return shape.points.length >= 4;
    case 'text':
      return shape.text.trim().length > 0;
    default:
      return shape.width >= MIN_DRAG && shape.height >= MIN_DRAG;
  }
}
