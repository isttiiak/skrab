import Konva from 'konva';
import { useEffect, useMemo, useRef } from 'react';
import { Arrow, Ellipse, Image as KonvaImage, Layer, Line, Rect, Stage, Text } from 'react-konva';
import type { Shape } from './tools.ts';

type Props = {
  image: HTMLImageElement;
  shapes: Shape[];
  draft: Shape | null;
  scale: number;
  stageRef: React.RefObject<Konva.Stage | null>;
  onPointerDown: (point: { x: number; y: number }) => void;
  onPointerMove: (point: { x: number; y: number }) => void;
  onPointerUp: () => void;
};

/**
 * Renders the screenshot with its annotations on top.
 *
 * Shapes stay as Konva nodes rather than being painted into the bitmap, so undo is
 * just dropping the node and export is a single `toDataURL` at the end.
 */
export function EditorCanvas({
  image,
  shapes,
  draft,
  scale,
  stageRef,
  onPointerDown,
  onPointerMove,
  onPointerUp,
}: Props) {
  const all = useMemo(() => (draft ? [...shapes, draft] : shapes), [shapes, draft]);

  // Pointer coordinates arrive in screen space; annotations live in image space.
  const toImageSpace = (stage: Konva.Stage | null) => {
    const pos = stage?.getPointerPosition();
    if (!pos) return null;
    return { x: pos.x / scale, y: pos.y / scale };
  };

  return (
    <Stage
      ref={stageRef}
      width={image.width * scale}
      height={image.height * scale}
      scale={{ x: scale, y: scale }}
      onMouseDown={(e) => {
        const point = toImageSpace(e.target.getStage());
        if (point) onPointerDown(point);
      }}
      onMouseMove={(e) => {
        const point = toImageSpace(e.target.getStage());
        if (point) onPointerMove(point);
      }}
      onMouseUp={onPointerUp}
      onMouseLeave={onPointerUp}
      className="cursor-crosshair"
    >
      <Layer listening={false}>
        <KonvaImage image={image} width={image.width} height={image.height} />
        {all.map((shape) => (
          <ShapeNode key={shape.id} shape={shape} image={image} />
        ))}
      </Layer>
    </Stage>
  );
}

function ShapeNode({ shape, image }: { shape: Shape; image: HTMLImageElement }) {
  switch (shape.kind) {
    case 'pen':
      return (
        <Line
          points={shape.points}
          stroke={shape.color}
          strokeWidth={shape.width}
          lineCap="round"
          lineJoin="round"
          tension={0.4}
        />
      );

    case 'arrow':
      return (
        <Arrow
          points={shape.points}
          stroke={shape.color}
          fill={shape.color}
          strokeWidth={shape.width}
          pointerLength={Math.max(shape.width * 2.5, 10)}
          pointerWidth={Math.max(shape.width * 2.5, 10)}
          lineCap="round"
        />
      );

    case 'rect':
      return (
        <Rect
          x={shape.x}
          y={shape.y}
          width={shape.width}
          height={shape.height}
          stroke={shape.color}
          strokeWidth={shape.strokeWidth}
          cornerRadius={3}
        />
      );

    case 'ellipse':
      return (
        <Ellipse
          x={shape.x + shape.width / 2}
          y={shape.y + shape.height / 2}
          radiusX={shape.width / 2}
          radiusY={shape.height / 2}
          stroke={shape.color}
          strokeWidth={shape.strokeWidth}
        />
      );

    case 'text':
      return (
        <Text
          x={shape.x}
          y={shape.y}
          text={shape.text}
          fill={shape.color}
          fontSize={shape.size}
          fontStyle="600"
          fontFamily="ui-sans-serif, system-ui, sans-serif"
        />
      );

    case 'highlight':
      return (
        <Rect
          x={shape.x}
          y={shape.y}
          width={shape.width}
          height={shape.height}
          fill={shape.color}
          // Multiply keeps the text underneath readable, the way a real highlighter
          // works — a plain translucent fill washes it out instead.
          globalCompositeOperation="multiply"
          opacity={0.45}
        />
      );

    case 'pixelate':
      return <PixelatedRegion shape={shape} image={image} />;

    default:
      return null;
  }
}

/**
 * Redaction: the same image, cropped to the region and pixelated in place.
 *
 * Genuinely destroys the detail rather than covering it, so the result survives
 * being screenshotted again or having a "black box" layer removed.
 */
function PixelatedRegion({
  shape,
  image,
}: {
  shape: Extract<Shape, { kind: 'pixelate' }>;
  image: HTMLImageElement;
}) {
  const ref = useRef<Konva.Image>(null);

  useEffect(() => {
    const node = ref.current;
    if (!node || shape.width < 1 || shape.height < 1) return;
    // Konva filters only run on a cached node.
    node.cache();
  }, [shape.width, shape.height]);

  return (
    <KonvaImage
      ref={ref}
      image={image}
      x={shape.x}
      y={shape.y}
      width={shape.width}
      height={shape.height}
      crop={{ x: shape.x, y: shape.y, width: shape.width, height: shape.height }}
      filters={[Konva.Filters.Pixelate]}
      pixelSize={Math.max(6, Math.round(Math.min(shape.width, shape.height) / 8))}
    />
  );
}
