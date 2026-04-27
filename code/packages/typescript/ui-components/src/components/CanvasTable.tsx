/**
 * CanvasTable — Canvas rendering backend for tabular data.
 *
 * Builds a PaintScene from the table data and delegates rendering to
 * paint-vm-canvas. This means the same table can be rendered to any other
 * paint backend (SVG via paint-vm-svg, Metal via paint-vm-metal, etc.) by
 * swapping the VM — the scene is the universal intermediate representation.
 *
 * === Architecture ===
 *
 * ```
 * Table data + columns + theme
 *         │
 *         ▼
 *   buildTableScene()  →  PaintScene (backend-neutral)
 *         │
 *         ├──→ paint-vm-canvas.execute(scene, ctx)  [visual layer]
 *         ├──→ paint-vm-svg.execute(scene)          [export]
 *         └──→ paint-vm-ascii.execute(scene)        [debugging]
 *
 * React component renders:
 *   <canvas> (painted by paint-vm-canvas)
 *   <div role="grid"> (ARIA overlay for accessibility)
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
import { useCanvasTheme, type CanvasTheme } from "../hooks/useCanvasTheme.js";
import { useGridKeyboard } from "../hooks/useGridKeyboard.js";
import { useColumnResize, MIN_COL_WIDTH } from "../hooks/useColumnResize.js";

import {
  paintScene,
  paintRect,
  paintText,
  paintLine,
  paintClip,
  paintGroup,
  type PaintScene,
  type PaintInstruction,
} from "@coding-adventures/paint-instructions";
import { createCanvasVM } from "@coding-adventures/paint-vm-canvas";

// The canvas paint VM is stateless between render calls — one VM per component
// instance is fine. Hoisting to module scope avoids reconstructing handler
// maps on every render.
const canvasPaintVM = createCanvasVM();

/**
 * Build a `canvas:` scheme font_ref string for PaintText.
 *
 * PaintText requires a font_ref with a scheme prefix that the paint backend
 * uses to route dispatch (see spec TXT03d). This helper turns the table's
 * FONT_FAMILY + FONT_SIZE + fontWeight into the canonical form
 * `canvas:<family>@<size>:<weight>`.
 *
 * Example: `makeCanvasFontRef("system-ui", 14, "bold")` → `"canvas:system-ui@14:700"`.
 */
function makeCanvasFontRef(
  family: string,
  size: number,
  weight: "normal" | "bold" = "normal",
): string {
  const weightCode = weight === "bold" ? 700 : 400;
  return `canvas:${family}@${size}:${weightCode}`;
}

/**
 * Map the table's column alignment ("left" | "center" | "right") to PaintText's
 * `text_align` field ("start" | "center" | "end"). DrawText historically used
 * "middle" for center; PaintText uses the Canvas 2D spelling "center".
 */
function toPaintAlign(
  a: "left" | "center" | "right" | undefined,
): "start" | "center" | "end" {
  if (a === "center") return "center";
  if (a === "right") return "end";
  return "start";
}

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

/** Font family used for table text. */
const FONT_FAMILY = "system-ui, -apple-system, sans-serif";

// ---------------------------------------------------------------------------
// Layout
// ---------------------------------------------------------------------------

interface ColumnLayout {
  x: number;
  width: number;
}

/**
 * Computes the x-position and width of each column.
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
          : 0;
    return resizable ? getColumnWidth(col.id, base || DEFAULT_RESIZABLE_WIDTH) : base;
  });
}

// ---------------------------------------------------------------------------
// Scene Builder
//
// This is the key refactor: instead of calling ctx.fillRect() directly,
// we build a PaintScene — a backend-neutral description of what to paint.
// The scene is then handed to paint-vm-canvas.
//
// The same scene could be rendered to SVG, ASCII text, or any future
// backend without changing a single line of table logic.
// ---------------------------------------------------------------------------

/**
 * Computes the text x-anchor for a cell, respecting alignment.
 *
 * Left → left edge of the content box (after padding).
 * Center → midpoint of the content box.
 * Right → right edge of the content box (before padding).
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

/**
 * Builds a PaintScene representing the entire table.
 *
 * This is a pure function: data in, scene out. No side effects, no canvas
 * context, no DOM. The scene can be inspected in tests, serialized to JSON,
 * or rendered by any backend.
 */
function buildTableScene<T>(
  columns: ColumnDef<T>[],
  data: T[],
  colLayouts: ColumnLayout[],
  canvasWidth: number,
  theme: CanvasTheme,
): PaintScene {
  const totalHeight = ROW_HEIGHT + data.length * ROW_HEIGHT;
  const instructions: PaintInstruction[] = [];
  const fontFamily = theme.fontFamily || FONT_FAMILY;
  const headerFontRef = makeCanvasFontRef(fontFamily, FONT_SIZE, "bold");
  const bodyFontRef = makeCanvasFontRef(fontFamily, FONT_SIZE, "normal");

  // --- Header background ---
  instructions.push(
    paintRect(0, 0, canvasWidth, ROW_HEIGHT, { fill: theme.headerBg }),
  );

  // --- Header text (bold, clipped per cell) ---
  const headerCells: PaintInstruction[] = [];
  for (let c = 0; c < columns.length; c++) {
    const col = columns[c]!;
    const layout = colLayouts[c]!;
    const textX = computeTextX(col.align ?? "left", layout, CELL_PADDING_X);

    headerCells.push(
      paintClip(layout.x, 0, layout.width, ROW_HEIGHT, [
        paintText(textX, TEXT_OFFSET_Y, col.header, headerFontRef, FONT_SIZE, theme.headerText, {
          text_align: toPaintAlign(col.align),
          metadata: { columnId: col.id, role: "header" },
        }),
      ]),
    );
  }
  instructions.push(paintGroup(headerCells, { metadata: { role: "header-row" } }));

  // --- Body rows ---
  for (let r = 0; r < data.length; r++) {
    const row = data[r]!;
    const rowY = ROW_HEIGHT + r * ROW_HEIGHT;
    const rowInstructions: PaintInstruction[] = [];

    // Alternating row background
    if (r % 2 === 1) {
      rowInstructions.push(
        paintRect(0, rowY, canvasWidth, ROW_HEIGHT, { fill: theme.altRowBg }),
      );
    }

    // Cell text (clipped per cell)
    for (let c = 0; c < columns.length; c++) {
      const col = columns[c]!;
      const layout = colLayouts[c]!;
      const value = resolveCellValue(col, row, r);
      const textX = computeTextX(col.align ?? "left", layout, CELL_PADDING_X);

      rowInstructions.push(
        paintClip(layout.x, rowY, layout.width, ROW_HEIGHT, [
          paintText(textX, rowY + TEXT_OFFSET_Y, value, bodyFontRef, FONT_SIZE, theme.bodyText, {
            text_align: toPaintAlign(col.align),
            metadata: { columnId: col.id, rowIndex: r },
          }),
        ]),
      );
    }

    instructions.push(paintGroup(rowInstructions, { metadata: { role: "body-row", rowIndex: r } }));
  }

  // --- Grid lines ---
  const gridLines: PaintInstruction[] = [];

  // Horizontal lines (below each row)
  for (let r = 0; r <= data.length; r++) {
    const y = ROW_HEIGHT + r * ROW_HEIGHT;
    gridLines.push(paintLine(0, y, canvasWidth, y, theme.borderColor, { stroke_width: 1 }));
  }

  // Vertical lines (at each column boundary)
  for (let c = 0; c <= columns.length; c++) {
    const x = c < colLayouts.length ? colLayouts[c]!.x : canvasWidth;
    gridLines.push(paintLine(x, 0, x, totalHeight, theme.borderColor, { stroke_width: 1 }));
  }

  instructions.push(paintGroup(gridLines, { metadata: { role: "grid-lines" } }));

  return paintScene(canvasWidth, totalHeight, theme.bodyBg, instructions, {
    metadata: {
      component: "table",
      rowCount: data.length,
      columnCount: columns.length,
    },
  });
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

  // ---------------------------------------------------------------------------
  // Canvas drawing via the paint-instructions pipeline
  //
  // Instead of calling ctx.fillRect() directly, we:
  // 1. Build a PaintScene (pure data, backend-neutral)
  // 2. Hand it to paint-vm-canvas
  //
  // The same scene could be rendered to SVG or ASCII text by swapping
  // the VM. The table doesn't know or care which backend is used.
  // ---------------------------------------------------------------------------

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

    // Set canvas dimensions for DPR
    canvas.width = logicalW * dpr;
    canvas.height = logicalH * dpr;
    canvas.style.width = `${logicalW}px`;
    canvas.style.height = `${logicalH}px`;

    // Scale for DPR, then let paint-vm-canvas handle the rest. The VM will
    // clear the viewport and paint the scene background itself; the extra
    // clearRect below is defensive in case the scene omits a background.
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, logicalW, logicalH);

    // Build the scene and dispatch it through the paint VM.
    const scene = buildTableScene(columns, data, layouts, logicalW, theme);
    canvasPaintVM.execute(scene, ctx);
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

      if (y > ROW_HEIGHT) {
        container.style.cursor = "";
        return;
      }

      const widths = resolveWidths(columns, getColumnWidth, resizable);
      const layouts = computeColumnLayout(widths, widthRef.current);

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

/**
 * Exported for testing — builds the PaintScene without rendering it.
 * Consumers can use this to render the table to SVG, ASCII, or any
 * other paint backend.
 */
export { buildTableScene, computeColumnLayout, resolveWidths };
export type { ColumnLayout };
