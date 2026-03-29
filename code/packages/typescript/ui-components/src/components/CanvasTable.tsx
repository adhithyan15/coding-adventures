/**
 * CanvasTable — Canvas rendering backend for tabular data.
 *
 * Draws the table directly to a `<canvas>` element using the 2D rendering
 * context. An ARIA grid overlay provides full accessibility — screen reader
 * traversal and keyboard navigation — on top of the visual canvas.
 *
 * === Architecture ===
 *
 * ```
 * ┌────────────────────────────────────────────────────┐
 * │ <div role="grid" class="table table--canvas">      │
 * │                                                    │
 * │   ┌──────────────────────────────────────────┐     │
 * │   │ <canvas aria-hidden="true">              │     │  Visual layer
 * │   │   Draws: headers, grid lines, cell text  │     │
 * │   └──────────────────────────────────────────┘     │
 * │                                                    │
 * │   ┌──────────────────────────────────────────┐     │
 * │   │ <div class="table__a11y-overlay">        │     │  A11y layer
 * │   │   role="row", role="gridcell"            │     │  (screen readers
 * │   │   Positioned to match canvas cells       │     │   + keyboard)
 * │   └──────────────────────────────────────────┘     │
 * │                                                    │
 * └────────────────────────────────────────────────────┘
 * ```
 *
 * === Column Resizing ===
 *
 * When `resizable` is true:
 * - Mouse hit-testing on the header row detects column boundaries
 * - Cursor changes to `col-resize` near a boundary
 * - Mousedown near a boundary starts a drag via the `useColumnResize` hook
 * - ARIA overlay includes `role="separator"` handles for keyboard/screen reader
 *
 * === Usage ===
 *
 * ```tsx
 * import { CanvasTable } from "@coding-adventures/ui-components";
 *
 * <CanvasTable
 *   columns={[
 *     { id: "name", header: "Name", accessor: "name", width: 200 },
 *     { id: "age", header: "Age", accessor: "age", width: 80, align: "right" },
 *   ]}
 *   data={[{ name: "Alice", age: 30 }]}
 *   ariaLabel="People"
 *   resizable
 * />
 * ```
 */

import {
  useRef,
  useEffect,
  useCallback,
  useState,
  type ReactNode,
  type MouseEvent as ReactMouseEvent,
} from "react";
import type { TableBaseProps, ColumnDef } from "./Table.js";
import { resolveCellValue } from "./Table.js";
import { useCanvasTheme } from "../hooks/useCanvasTheme.js";
import { useGridKeyboard } from "../hooks/useGridKeyboard.js";
import { useColumnResize, MIN_COL_WIDTH } from "../hooks/useColumnResize.js";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** Font size in logical pixels. */
const FONT_SIZE = 14;

/** Row height as a multiple of font size. */
const ROW_HEIGHT = FONT_SIZE * 2.25;

/** Horizontal padding inside each cell. */
const CELL_PADDING_X = 12;

/** Vertical text offset from the top of a row (centers text vertically). */
const TEXT_OFFSET_Y = ROW_HEIGHT * 0.65;

/** Hit-test tolerance for column boundary detection (pixels from edge). */
const RESIZE_HIT_ZONE = 5;

/** Default column width when resizable and no width specified. */
const DEFAULT_RESIZABLE_WIDTH = 150;

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

interface ColumnLayout {
  x: number;
  width: number;
}

/**
 * Computes the x-position and width of each column.
 *
 * Columns with explicit widths (numbers) are honored first. Remaining
 * canvas width is distributed equally among auto-width columns.
 */
function computeColumnLayout(
  widths: number[],
  canvasWidth: number,
): ColumnLayout[] {
  let fixedTotal = 0;
  let autoCount = 0;

  for (const w of widths) {
    if (w > 0) {
      fixedTotal += w;
    } else {
      autoCount++;
    }
  }

  const autoWidth =
    autoCount > 0 ? Math.max(0, (canvasWidth - fixedTotal) / autoCount) : 0;

  const layouts: ColumnLayout[] = [];
  let x = 0;

  for (const w of widths) {
    const colW = w > 0 ? w : autoWidth;
    layouts.push({ x, width: colW });
    x += colW;
  }

  return layouts;
}

/**
 * Resolves effective column widths, applying resize overrides.
 */
function resolveWidths<T>(
  columns: ColumnDef<T>[],
  getColumnWidth: (id: string, defaultWidth: number) => number,
  resizable: boolean,
): number[] {
  return columns.map((col) => {
    const base =
      typeof col.width === "number"
        ? col.width
        : resizable
          ? DEFAULT_RESIZABLE_WIDTH
          : 0; // 0 signals auto-width
    return resizable ? getColumnWidth(col.id, base || DEFAULT_RESIZABLE_WIDTH) : base;
  });
}

// ---------------------------------------------------------------------------
// Canvas Drawing
// ---------------------------------------------------------------------------

/**
 * Draws the entire table to the canvas.
 */
function drawTable<T>(
  ctx: CanvasRenderingContext2D,
  columns: ColumnDef<T>[],
  data: T[],
  colLayouts: ColumnLayout[],
  canvasWidth: number,
  theme: ReturnType<typeof useCanvasTheme>,
  dpr: number,
): void {
  const totalHeight = ROW_HEIGHT + data.length * ROW_HEIGHT;

  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
  ctx.clearRect(0, 0, canvasWidth, totalHeight);

  // Header background
  ctx.fillStyle = theme.headerBg;
  ctx.fillRect(0, 0, canvasWidth, ROW_HEIGHT);

  // Header text
  ctx.fillStyle = theme.headerText;
  ctx.font = `600 ${FONT_SIZE}px ${theme.fontFamily}`;

  for (let c = 0; c < columns.length; c++) {
    const col = columns[c]!;
    const layout = colLayouts[c]!;
    const textX = computeTextX(col.align ?? "left", layout, CELL_PADDING_X);

    ctx.textAlign = col.align ?? "left";
    ctx.save();
    ctx.beginPath();
    ctx.rect(layout.x, 0, layout.width, ROW_HEIGHT);
    ctx.clip();
    ctx.fillText(col.header, textX, TEXT_OFFSET_Y);
    ctx.restore();
  }

  // Body rows
  ctx.font = `${FONT_SIZE}px ${theme.fontFamily}`;

  for (let r = 0; r < data.length; r++) {
    const row = data[r]!;
    const rowY = ROW_HEIGHT + r * ROW_HEIGHT;

    if (r % 2 === 1) {
      ctx.fillStyle = theme.altRowBg;
      ctx.fillRect(0, rowY, canvasWidth, ROW_HEIGHT);
    }

    ctx.fillStyle = theme.bodyText;

    for (let c = 0; c < columns.length; c++) {
      const col = columns[c]!;
      const layout = colLayouts[c]!;
      const value = resolveCellValue(col, row, r);
      const textX = computeTextX(col.align ?? "left", layout, CELL_PADDING_X);

      ctx.textAlign = col.align ?? "left";
      ctx.save();
      ctx.beginPath();
      ctx.rect(layout.x, rowY, layout.width, ROW_HEIGHT);
      ctx.clip();
      ctx.fillText(value, textX, rowY + TEXT_OFFSET_Y);
      ctx.restore();
    }
  }

  // Grid lines
  ctx.strokeStyle = theme.borderColor;
  ctx.lineWidth = 1;

  for (let r = 0; r <= data.length; r++) {
    const y = ROW_HEIGHT + r * ROW_HEIGHT;
    ctx.beginPath();
    ctx.moveTo(0, y);
    ctx.lineTo(canvasWidth, y);
    ctx.stroke();
  }

  for (let c = 0; c <= columns.length; c++) {
    const x = c < colLayouts.length ? colLayouts[c]!.x : canvasWidth;
    ctx.beginPath();
    ctx.moveTo(x, 0);
    ctx.lineTo(x, ROW_HEIGHT + data.length * ROW_HEIGHT);
    ctx.stroke();
  }
}

function computeTextX(
  align: "left" | "center" | "right",
  layout: ColumnLayout,
  padding: number,
): number {
  switch (align) {
    case "center":
      return layout.x + layout.width / 2;
    case "right":
      return layout.x + layout.width - padding;
    default:
      return layout.x + padding;
  }
}

// ---------------------------------------------------------------------------
// ARIA Grid Overlay
// ---------------------------------------------------------------------------

function AriaOverlay<T>({
  columns,
  data,
  colLayouts,
  getCellTabIndex,
  setFocusedCell,
  resizable,
  startResize,
  handleResizeKeyDown,
}: {
  columns: ColumnDef<T>[];
  data: T[];
  colLayouts: ColumnLayout[];
  getCellTabIndex: (row: number, col: number) => 0 | -1;
  setFocusedCell: (pos: { row: number; col: number }) => void;
  resizable: boolean;
  startResize: (colId: string, clientX: number, currentWidth: number) => void;
  handleResizeKeyDown: (
    colId: string,
    currentWidth: number,
    event: React.KeyboardEvent,
  ) => void;
}): ReactNode {
  return (
    <div className="table__a11y-overlay">
      {/* Header row group */}
      <div role="rowgroup">
        <div role="row" aria-rowindex={1}>
          {columns.map((col, c) => {
            const layout = colLayouts[c];
            if (!layout) return null;
            const effectiveWidth = layout.width;
            return (
              <div
                key={col.id}
                role="columnheader"
                aria-colindex={c + 1}
                data-row={0}
                data-col={c}
                tabIndex={getCellTabIndex(0, c)}
                onFocus={() => setFocusedCell({ row: 0, col: c })}
                style={{
                  left: layout.x,
                  top: 0,
                  width: layout.width,
                  height: ROW_HEIGHT,
                }}
              >
                {col.header}
                {resizable && (
                  <div
                    className="table__resize-handle"
                    role="separator"
                    aria-orientation="vertical"
                    aria-label={`Resize ${col.header} column`}
                    aria-valuenow={effectiveWidth}
                    aria-valuemin={MIN_COL_WIDTH}
                    tabIndex={0}
                    onMouseDown={(e) => {
                      e.preventDefault();
                      e.stopPropagation();
                      startResize(col.id, e.clientX, effectiveWidth);
                    }}
                    onKeyDown={(e) => {
                      e.stopPropagation();
                      handleResizeKeyDown(col.id, effectiveWidth, e);
                    }}
                  />
                )}
              </div>
            );
          })}
        </div>
      </div>

      {/* Body row group */}
      <div role="rowgroup">
        {data.map((row, r) => (
          <div key={r} role="row" aria-rowindex={r + 2}>
            {columns.map((col, c) => {
              const layout = colLayouts[c];
              if (!layout) return null;
              return (
                <div
                  key={col.id}
                  role="gridcell"
                  aria-colindex={c + 1}
                  data-row={r + 1}
                  data-col={c}
                  tabIndex={getCellTabIndex(r + 1, c)}
                  onFocus={() => setFocusedCell({ row: r + 1, col: c })}
                  style={{
                    left: layout.x,
                    top: ROW_HEIGHT + r * ROW_HEIGHT,
                    width: layout.width,
                    height: ROW_HEIGHT,
                  }}
                >
                  {resolveCellValue(col, row, r)}
                </div>
              );
            })}
          </div>
        ))}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function CanvasTable<T>({
  columns,
  data,
  ariaLabel,
  caption,
  className = "table",
  resizable = false,
  onColumnResize,
}: TableBaseProps<T>) {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const theme = useCanvasTheme(containerRef);

  // Detect text direction
  const [direction, setDirection] = useState<"ltr" | "rtl">("ltr");
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const dir = getComputedStyle(el).direction;
    setDirection(dir === "rtl" ? "rtl" : "ltr");
  }, []);

  const totalRows = 1 + data.length;

  const { setFocusedCell, onKeyDown, getCellTabIndex } =
    useGridKeyboard({ rowCount: totalRows, colCount: columns.length });

  const {
    getColumnWidth,
    startResize,
    handleResizeKeyDown,
    isResizing,
    columnWidths,
  } = useColumnResize({ direction, onColumnResize });

  const widthRef = useRef(800);

  // Resize observer
  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    widthRef.current = container.clientWidth || 800;

    if (typeof ResizeObserver === "undefined") return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        widthRef.current = entry.contentRect.width;
      }
    });

    observer.observe(container);
    return () => observer.disconnect();
  }, []);

  // Canvas drawing
  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr =
      typeof window !== "undefined" ? window.devicePixelRatio || 1 : 1;
    const widths = resolveWidths(columns, getColumnWidth, resizable);
    const layouts = computeColumnLayout(widths, widthRef.current);
    const logicalW = Math.max(
      layouts.reduce((s, l) => s + l.width, 0),
      widthRef.current,
    );
    const logicalH = ROW_HEIGHT + data.length * ROW_HEIGHT;

    canvas.width = logicalW * dpr;
    canvas.height = logicalH * dpr;
    canvas.style.width = `${logicalW}px`;
    canvas.style.height = `${logicalH}px`;

    drawTable(ctx, columns, data, layouts, logicalW, theme, dpr);
  }, [columns, data, theme, getColumnWidth, resizable]);

  useEffect(() => {
    draw();
  }, [draw, columnWidths]);

  // Mouse hit-testing for resize cursor on header row
  const handleMouseMove = useCallback(
    (e: ReactMouseEvent) => {
      if (!resizable || isResizing) return;
      const container = containerRef.current;
      if (!container) return;

      const rect = container.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;

      // Only show resize cursor in header row
      if (y > ROW_HEIGHT) {
        container.style.cursor = "";
        return;
      }

      const widths = resolveWidths(columns, getColumnWidth, resizable);
      const layouts = computeColumnLayout(widths, widthRef.current);

      // Check if cursor is near a column boundary
      for (let c = 0; c < layouts.length; c++) {
        const colRight = layouts[c]!.x + layouts[c]!.width;
        if (Math.abs(x - colRight) <= RESIZE_HIT_ZONE) {
          container.style.cursor = "col-resize";
          return;
        }
      }
      container.style.cursor = "";
    },
    [resizable, isResizing, columns, getColumnWidth],
  );

  const handleMouseDown = useCallback(
    (e: ReactMouseEvent) => {
      if (!resizable) return;
      const container = containerRef.current;
      if (!container) return;

      const rect = container.getBoundingClientRect();
      const x = e.clientX - rect.left;
      const y = e.clientY - rect.top;

      if (y > ROW_HEIGHT) return;

      const widths = resolveWidths(columns, getColumnWidth, resizable);
      const layouts = computeColumnLayout(widths, widthRef.current);

      for (let c = 0; c < layouts.length; c++) {
        const colRight = layouts[c]!.x + layouts[c]!.width;
        if (Math.abs(x - colRight) <= RESIZE_HIT_ZONE) {
          e.preventDefault();
          startResize(columns[c]!.id, e.clientX, layouts[c]!.width);
          return;
        }
      }
    },
    [resizable, columns, getColumnWidth, startResize],
  );

  // Compute overlay layouts
  const overlayWidths = resolveWidths(columns, getColumnWidth, resizable);
  const overlayLayouts = computeColumnLayout(overlayWidths, widthRef.current);

  const containerClass = [
    className,
    "table--canvas",
    isResizing ? "table--resizing" : "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <div
      ref={containerRef}
      className={containerClass}
      role="grid"
      aria-label={ariaLabel}
      aria-rowcount={totalRows}
      aria-colcount={columns.length}
      onKeyDown={onKeyDown}
      onMouseMove={handleMouseMove}
      onMouseDown={handleMouseDown}
    >
      {caption !== undefined && (
        <div className="table__caption" role="presentation">
          {caption}
        </div>
      )}

      <canvas ref={canvasRef} aria-hidden="true" />

      <AriaOverlay
        columns={columns}
        data={data}
        colLayouts={overlayLayouts}
        getCellTabIndex={getCellTabIndex}
        setFocusedCell={setFocusedCell}
        resizable={resizable}
        startResize={startResize}
        handleResizeKeyDown={handleResizeKeyDown}
      />
    </div>
  );
}
