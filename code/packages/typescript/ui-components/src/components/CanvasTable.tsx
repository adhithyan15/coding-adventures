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
 * The canvas is `aria-hidden="true"` — purely visual. The ARIA grid overlay
 * is a layer of transparent, absolutely-positioned `<div>` elements that
 * screen readers traverse and keyboard users navigate.
 *
 * === Canvas Rendering Pipeline ===
 *
 * 1. Clear canvas
 * 2. Compute layout (column widths, row height, total dimensions)
 * 3. Scale for devicePixelRatio (crisp text on retina displays)
 * 4. Draw header background + text
 * 5. Draw body rows (alternating backgrounds + cell text)
 * 6. Draw grid lines
 *
 * === DPR (Device Pixel Ratio) Handling ===
 *
 * On retina displays, 1 CSS pixel = 2+ physical pixels. Without DPR
 * scaling, canvas text appears blurry. The fix:
 *
 * ```
 * CSS size:    width=800  height=600    (layout size)
 * Buffer size: width=1600 height=1200   (physical pixels)
 * Context:     scale(2, 2)              (draw at 2x)
 * ```
 *
 * This way, `ctx.fillText("Hello", 10, 20)` draws at logical position
 * (10, 20) but renders at physical pixel (20, 40), giving crisp text.
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
 *   data={[
 *     { name: "Alice", age: 30 },
 *     { name: "Bob", age: 25 },
 *   ]}
 *   ariaLabel="People"
 * />
 * ```
 */

import {
  useRef,
  useEffect,
  useCallback,
  type ReactNode,
} from "react";
import type { TableBaseProps, ColumnDef } from "./Table.js";
import { resolveCellValue } from "./Table.js";
import { useCanvasTheme } from "../hooks/useCanvasTheme.js";
import { useGridKeyboard } from "../hooks/useGridKeyboard.js";

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
 * canvas width is distributed equally among auto-width columns. If the
 * explicit widths exceed the canvas width, auto-width columns get 0 width
 * (they'll be clipped).
 *
 * ```
 * Canvas width: 800px
 * Column A: width=200  → x=0,   width=200
 * Column B: auto       → x=200, width=300  (half of remaining 600)
 * Column C: auto       → x=500, width=300
 * ```
 */
function computeColumnLayout<T>(
  columns: ColumnDef<T>[],
  canvasWidth: number,
): ColumnLayout[] {
  let fixedTotal = 0;
  let autoCount = 0;

  for (const col of columns) {
    if (typeof col.width === "number") {
      fixedTotal += col.width;
    } else {
      autoCount++;
    }
  }

  const autoWidth =
    autoCount > 0 ? Math.max(0, (canvasWidth - fixedTotal) / autoCount) : 0;

  const layouts: ColumnLayout[] = [];
  let x = 0;

  for (const col of columns) {
    const w = typeof col.width === "number" ? col.width : autoWidth;
    layouts.push({ x, width: w });
    x += w;
  }

  return layouts;
}

// ---------------------------------------------------------------------------
// Canvas Drawing
// ---------------------------------------------------------------------------

/**
 * Draws the entire table to the canvas.
 *
 * This function is the core rendering pipeline. It runs inside a useEffect
 * that depends on data, columns, theme, and dimensions.
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

  // Scale for DPR
  ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

  // 1. Clear
  ctx.clearRect(0, 0, canvasWidth, totalHeight);

  // 2. Header background
  ctx.fillStyle = theme.headerBg;
  ctx.fillRect(0, 0, canvasWidth, ROW_HEIGHT);

  // 3. Header text
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

  // 4. Body rows
  ctx.font = `${FONT_SIZE}px ${theme.fontFamily}`;

  for (let r = 0; r < data.length; r++) {
    const row = data[r]!;
    const rowY = ROW_HEIGHT + r * ROW_HEIGHT;

    // Alternating row background
    if (r % 2 === 1) {
      ctx.fillStyle = theme.altRowBg;
      ctx.fillRect(0, rowY, canvasWidth, ROW_HEIGHT);
    }

    // Cell text
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

  // 5. Grid lines
  ctx.strokeStyle = theme.borderColor;
  ctx.lineWidth = 1;

  // Horizontal lines
  for (let r = 0; r <= data.length; r++) {
    const y = ROW_HEIGHT + r * ROW_HEIGHT;
    ctx.beginPath();
    ctx.moveTo(0, y);
    ctx.lineTo(canvasWidth, y);
    ctx.stroke();
  }

  // Vertical lines
  for (let c = 0; c <= columns.length; c++) {
    const x = c < colLayouts.length ? colLayouts[c]!.x : canvasWidth;
    ctx.beginPath();
    ctx.moveTo(x, 0);
    ctx.lineTo(x, ROW_HEIGHT + data.length * ROW_HEIGHT);
    ctx.stroke();
  }
}

/**
 * Computes the x-position for text within a cell, respecting alignment.
 *
 * ```
 * align="left"   → x = cellX + padding
 * align="center" → x = cellX + width/2
 * align="right"  → x = cellX + width - padding
 * ```
 */
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

/**
 * Renders the transparent ARIA grid overlay for accessibility.
 *
 * Each cell is an absolutely positioned `<div>` that matches the
 * corresponding canvas cell's position and dimensions. The divs are
 * transparent (styled via CSS) so the canvas shows through. Screen
 * readers traverse these divs; keyboard navigation moves focus between them.
 */
function AriaOverlay<T>({
  columns,
  data,
  colLayouts,
  getCellTabIndex,
  setFocusedCell,
}: {
  columns: ColumnDef<T>[];
  data: T[];
  colLayouts: ColumnLayout[];
  getCellTabIndex: (row: number, col: number) => 0 | -1;
  setFocusedCell: (pos: { row: number; col: number }) => void;
}): ReactNode {
  return (
    <div className="table__a11y-overlay">
      {/* Header row group */}
      <div role="rowgroup">
        <div role="row" aria-rowindex={1}>
          {columns.map((col, c) => {
            const layout = colLayouts[c];
            if (!layout) return null;
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
}: TableBaseProps<T>) {
  const containerRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const theme = useCanvasTheme(containerRef);

  // Total rows = 1 header + data.length body rows
  const totalRows = 1 + data.length;

  const { setFocusedCell, onKeyDown, getCellTabIndex } =
    useGridKeyboard({
      rowCount: totalRows,
      colCount: columns.length,
    });

  // Track container width for responsive sizing
  const widthRef = useRef(800);

  // Compute column layout (memoized by reference)
  const colLayouts = computeColumnLayout(columns, widthRef.current);

  // ---------------------------------------------------------------------------
  // Resize observer
  // ---------------------------------------------------------------------------

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    // Read initial width
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

  // ---------------------------------------------------------------------------
  // Canvas drawing
  // ---------------------------------------------------------------------------

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;

    const dpr = typeof window !== "undefined" ? window.devicePixelRatio || 1 : 1;
    const layouts = computeColumnLayout(columns, widthRef.current);
    const logicalW = Math.max(
      layouts.reduce((s, l) => s + l.width, 0),
      widthRef.current,
    );
    const logicalH = ROW_HEIGHT + data.length * ROW_HEIGHT;

    // Set physical buffer size
    canvas.width = logicalW * dpr;
    canvas.height = logicalH * dpr;

    // Set CSS display size
    canvas.style.width = `${logicalW}px`;
    canvas.style.height = `${logicalH}px`;

    drawTable(ctx, columns, data, layouts, logicalW, theme, dpr);
  }, [columns, data, theme]);

  useEffect(() => {
    draw();
  }, [draw]);

  // Recompute layouts for the overlay using current width
  const overlayLayouts = computeColumnLayout(columns, widthRef.current);

  return (
    <div
      ref={containerRef}
      className={`${className} table--canvas`}
      role="grid"
      aria-label={ariaLabel}
      aria-rowcount={totalRows}
      aria-colcount={columns.length}
      onKeyDown={onKeyDown}
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
      />
    </div>
  );
}
