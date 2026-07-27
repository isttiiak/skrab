import type { OverlayFrame } from '@skrab/ipc-types';
import { convertFileSrc } from '@tauri-apps/api/core';
import { useCallback, useEffect, useRef, useState } from 'react';
import { cancelRegionCapture, finishRegionCapture, getOverlayFrame } from '@/lib/tauri';

type Point = { x: number; y: number };

/**
 * Region-selection overlay.
 *
 * Drawn over a *frozen* screenshot rather than the live desktop, so the overlay can
 * never appear in its own capture and the selection matches exactly what was on
 * screen when the hotkey fired.
 */
export function CaptureOverlay() {
  const [frame, setFrame] = useState<OverlayFrame | null>(null);
  const [origin, setOrigin] = useState<Point | null>(null);
  const [cursor, setCursor] = useState<Point | null>(null);
  const [busy, setBusy] = useState(false);
  const surfaceRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    void getOverlayFrame()
      .then(setFrame)
      .catch(() => setFrame(null));
  }, []);

  const cancel = useCallback(() => {
    void cancelRegionCapture();
  }, []);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        event.preventDefault();
        cancel();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [cancel]);

  const rect =
    origin && cursor
      ? {
          left: Math.min(origin.x, cursor.x),
          top: Math.min(origin.y, cursor.y),
          width: Math.abs(cursor.x - origin.x),
          height: Math.abs(cursor.y - origin.y),
        }
      : null;

  const finish = useCallback(async () => {
    if (!frame || !rect || rect.width < 2 || rect.height < 2) {
      setOrigin(null);
      setCursor(null);
      return;
    }

    const surface = surfaceRef.current;
    if (!surface) return;

    // The frame is in physical pixels; the overlay is laid out in CSS pixels.
    // Scale by the actual rendered size rather than the reported device ratio —
    // the window is sized to the display, so this stays correct on mixed-DPI setups.
    const scaleX = frame.width / surface.clientWidth;
    const scaleY = frame.height / surface.clientHeight;

    setBusy(true);
    try {
      await finishRegionCapture({
        x: Math.round(rect.left * scaleX),
        y: Math.round(rect.top * scaleY),
        width: Math.round(rect.width * scaleX),
        height: Math.round(rect.height * scaleY),
      });
    } catch {
      cancel();
    }
  }, [frame, rect, cancel]);

  if (!frame) {
    return <div className="h-full w-full bg-black/40" />;
  }

  return (
    // role="application" rather than a bare div: this is a freehand drag surface
    // with no keyboard equivalent, so assistive tech should hand keys straight
    // through. Escape still cancels, via the window-level listener above.
    <div
      role="application"
      aria-label="Drag to select a region of the screen. Press Escape to cancel."
      ref={surfaceRef}
      className="relative h-full w-full cursor-crosshair overflow-hidden select-none"
      onMouseDown={(e) => {
        setOrigin({ x: e.clientX, y: e.clientY });
        setCursor({ x: e.clientX, y: e.clientY });
      }}
      onMouseMove={(e) => origin && setCursor({ x: e.clientX, y: e.clientY })}
      onMouseUp={() => void finish()}
    >
      <img
        src={convertFileSrc(frame.path)}
        alt=""
        draggable={false}
        className="pointer-events-none absolute inset-0 h-full w-full object-fill"
      />

      {/* Dim everything, then punch the selection back to full brightness. */}
      <div className="pointer-events-none absolute inset-0 bg-black/50" />

      {rect && rect.width > 1 && (
        <>
          <div
            className="pointer-events-none absolute overflow-hidden"
            style={{
              left: rect.left,
              top: rect.top,
              width: rect.width,
              height: rect.height,
            }}
          >
            <img
              src={convertFileSrc(frame.path)}
              alt=""
              draggable={false}
              className="absolute h-full w-full object-fill"
              style={{
                width: surfaceRef.current?.clientWidth,
                height: surfaceRef.current?.clientHeight,
                left: -rect.left,
                top: -rect.top,
                maxWidth: 'none',
              }}
            />
          </div>
          <div
            className="border-primary pointer-events-none absolute border-2"
            style={{
              left: rect.left,
              top: rect.top,
              width: rect.width,
              height: rect.height,
            }}
          />
          <div
            className="bg-primary pointer-events-none absolute rounded px-1.5 py-0.5 font-mono text-[10px] text-white"
            style={{ left: rect.left, top: Math.max(rect.top - 20, 2) }}
          >
            {Math.round(rect.width)} × {Math.round(rect.height)}
          </div>
        </>
      )}

      {!origin && !busy && (
        <div className="pointer-events-none absolute inset-x-0 top-10 flex justify-center">
          <p className="rounded-full bg-black/70 px-4 py-1.5 text-xs text-white">
            Drag to select an area · Esc to cancel
          </p>
        </div>
      )}
    </div>
  );
}
