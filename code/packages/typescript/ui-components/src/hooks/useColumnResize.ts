/**
 * useColumnResize — drag-to-resize and keyboard-resize for table columns.
 *
 * This hook manages the full resize lifecycle for both the HTML and Canvas
 * table backends. It handles:
 *
 * 1. **Mouse drag** — mousedown on a resize handle starts a drag. Document-
 *    level mousemove/mouseup listeners track the cursor and update the column
 *    width in real time. Document-level listeners ensure the drag continues
 *    even when the cursor leaves the table.
 *
 * 2. **Keyboard resize** — when a resize handle is focused, Left/Right arrow
 *    keys adjust the width by 10px (or 50px with Shift). This follows the
 *    WAI-ARIA adjustable separator pattern.
 *
 * 3. **RTL support** — the hook accepts a `direction` parameter. In RTL mode,
 *    the drag delta and arrow key directions are flipped:
 *    - LTR: drag right = wider, Right Arrow = wider
 *    - RTL: drag left = wider, Right Arrow = narrower
 *
 * === Drag Lifecycle ===
 *
 * ```
 * mousedown on handle
 *   → store { colId, startX, startWidth }
 *   → attach document mousemove + mouseup
 *
 * document mousemove
 *   → delta = clientX - startX (flipped in RTL)
 *   → newWidth = max(MIN_WIDTH, startWidth + delta)
 *   → update columnWidths state
 *
 * document mouseup
 *   → clear drag state
 *   → detach listeners
 *   → fire onColumnResize callback
 * ```
 *
 * === Usage ===
 *
 * ```tsx
 * const { getColumnWidth, startResize, handleResizeKeyDown, isResizing } =
 *   useColumnResize({ direction: "ltr", onColumnResize });
 *
 * // In a header cell:
 * <div
 *   role="separator"
 *   onMouseDown={e => startResize(col.id, e.clientX, currentWidth)}
 *   onKeyDown={e => handleResizeKeyDown(col.id, currentWidth, e)}
 * />
 * ```
 */

import { useState, useCallback, useEffect, useRef, type KeyboardEvent } from "react";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** Minimum column width in pixels. Prevents columns from collapsing. */
export const MIN_COL_WIDTH = 40;

/** Pixels to adjust per arrow key press. */
const STEP_FINE = 10;

/** Pixels to adjust per Shift+Arrow key press (coarse). */
const STEP_COARSE = 50;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface UseColumnResizeOptions {
  /** Text direction — affects drag delta and arrow key mapping. */
  direction?: "ltr" | "rtl";

  /** Called when a column resize completes (mouseup or keyboard step). */
  onColumnResize?: (columnId: string, width: number) => void;
}

interface DragState {
  colId: string;
  startX: number;
  startWidth: number;
}

export interface UseColumnResizeReturn {
  /**
   * Returns the effective width for a column. If the user has resized the
   * column, returns the overridden width. Otherwise returns the default.
   */
  getColumnWidth: (colId: string, defaultWidth: number) => number;

  /**
   * Begins a mouse drag resize. Call this from the resize handle's
   * mousedown handler.
   */
  startResize: (colId: string, clientX: number, currentWidth: number) => void;

  /**
   * Keyboard event handler for the resize handle. Adjusts column width
   * with Left/Right arrow keys (10px) and Shift+Arrow (50px).
   */
  handleResizeKeyDown: (
    colId: string,
    currentWidth: number,
    event: KeyboardEvent,
  ) => void;

  /** True while a mouse drag is in progress. */
  isResizing: boolean;

  /** The record of overridden column widths (colId → pixels). */
  columnWidths: Record<string, number>;
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

export function useColumnResize({
  direction = "ltr",
  onColumnResize,
}: UseColumnResizeOptions = {}): UseColumnResizeReturn {
  const [columnWidths, setColumnWidths] = useState<Record<string, number>>({});
  const dragRef = useRef<DragState | null>(null);
  const [isResizing, setIsResizing] = useState(false);

  // Store callback in a ref so document listeners always see the latest
  const onResizeRef = useRef(onColumnResize);
  onResizeRef.current = onColumnResize;

  const dirRef = useRef(direction);
  dirRef.current = direction;

  // -------------------------------------------------------------------------
  // Mouse drag
  // -------------------------------------------------------------------------

  const startResize = useCallback(
    (colId: string, clientX: number, currentWidth: number) => {
      dragRef.current = { colId, startX: clientX, startWidth: currentWidth };
      setIsResizing(true);
    },
    [],
  );

  useEffect(() => {
    if (!isResizing) return;

    const onMouseMove = (e: MouseEvent) => {
      const drag = dragRef.current;
      if (!drag) return;

      const rawDelta = e.clientX - drag.startX;
      const delta = dirRef.current === "rtl" ? -rawDelta : rawDelta;
      const newWidth = Math.max(MIN_COL_WIDTH, drag.startWidth + delta);

      setColumnWidths((prev) => ({ ...prev, [drag.colId]: newWidth }));
    };

    const onMouseUp = () => {
      const drag = dragRef.current;
      if (drag) {
        setColumnWidths((prev) => {
          const finalWidth = prev[drag.colId] ?? drag.startWidth;
          onResizeRef.current?.(drag.colId, finalWidth);
          return prev;
        });
      }
      dragRef.current = null;
      setIsResizing(false);
    };

    document.addEventListener("mousemove", onMouseMove);
    document.addEventListener("mouseup", onMouseUp);

    return () => {
      document.removeEventListener("mousemove", onMouseMove);
      document.removeEventListener("mouseup", onMouseUp);
    };
  }, [isResizing]);

  // -------------------------------------------------------------------------
  // Keyboard resize
  // -------------------------------------------------------------------------

  const handleResizeKeyDown = useCallback(
    (colId: string, currentWidth: number, event: KeyboardEvent) => {
      const { key, shiftKey } = event;

      let delta = 0;
      const step = shiftKey ? STEP_COARSE : STEP_FINE;
      const isRtl = dirRef.current === "rtl";

      switch (key) {
        case "ArrowRight":
          delta = isRtl ? -step : step;
          break;
        case "ArrowLeft":
          delta = isRtl ? step : -step;
          break;
        default:
          return; // Not a resize key — don't prevent default
      }

      event.preventDefault();

      const effectiveWidth = columnWidths[colId] ?? currentWidth;
      const newWidth = Math.max(MIN_COL_WIDTH, effectiveWidth + delta);

      setColumnWidths((prev) => ({ ...prev, [colId]: newWidth }));
      onResizeRef.current?.(colId, newWidth);
    },
    [columnWidths],
  );

  // -------------------------------------------------------------------------
  // Width getter
  // -------------------------------------------------------------------------

  const getColumnWidth = useCallback(
    (colId: string, defaultWidth: number): number => {
      return columnWidths[colId] ?? defaultWidth;
    },
    [columnWidths],
  );

  return {
    getColumnWidth,
    startResize,
    handleResizeKeyDown,
    isResizing,
    columnWidths,
  };
}
