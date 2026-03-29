/**
 * DataTable — HTML rendering backend for tabular data.
 *
 * Renders a semantic `<table>` element with `<thead>`, `<tbody>`, `<th>`, and
 * `<td>`. This is the accessible, SEO-friendly, and default rendering path.
 *
 * === DOM Structure ===
 *
 * ```
 * <div class="table" role="region" aria-label="..." tabindex="0">
 *   <table class="table__grid">
 *     <caption class="table__caption">...</caption>
 *     <thead class="table__head">
 *       <tr class="table__row table__row--header">
 *         <th class="table__cell table__cell--header table__cell--align-left"
 *             scope="col" style="width: 200px; position: relative">
 *           Name
 *           <div class="table__resize-handle" role="separator" ... />
 *         </th>
 *       </tr>
 *     </thead>
 *     <tbody class="table__body">
 *       <tr class="table__row">
 *         <td class="table__cell table__cell--align-left">Alice</td>
 *       </tr>
 *     </tbody>
 *   </table>
 * </div>
 * ```
 *
 * === Column Resizing ===
 *
 * When `resizable` is true, each header cell contains a resize handle — a
 * thin `<div>` at the right edge (or left edge in RTL) with
 * `role="separator"`. Users can:
 *
 * - **Mouse drag**: click and drag the handle to adjust column width
 * - **Keyboard**: focus the handle, then use Left/Right arrows (10px steps)
 *   or Shift+Arrow (50px steps)
 * - **Screen reader**: hears "Resize [Column] column, separator, [width]"
 *   via `aria-label` and `aria-valuenow`
 *
 * === Accessibility ===
 *
 * The HTML backend is inherently accessible because it uses native `<table>`
 * semantics. Screen readers already understand `<thead>`, `<th scope="col">`,
 * `<tbody>`, `<td>`. No additional ARIA roles are needed beyond
 * `role="region"` and `aria-label` on the scrollable container.
 *
 * === Usage ===
 *
 * ```tsx
 * import { DataTable } from "@coding-adventures/ui-components";
 *
 * <DataTable
 *   columns={[
 *     { id: "name", header: "Name", accessor: "name" },
 *     { id: "age", header: "Age", accessor: "age", align: "right" },
 *   ]}
 *   data={[
 *     { name: "Alice", age: 30 },
 *     { name: "Bob", age: 25 },
 *   ]}
 *   ariaLabel="People"
 *   resizable
 * />
 * ```
 */

import { useRef, useEffect, useState } from "react";
import type { TableBaseProps, ColumnDef } from "./Table.js";
import { resolveCellValue } from "./Table.js";
import { useColumnResize, MIN_COL_WIDTH } from "../hooks/useColumnResize.js";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** Default column width in pixels when resizable is true and no width is set. */
const DEFAULT_RESIZABLE_WIDTH = 150;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Builds the BEM class string for a table cell.
 *
 * Every cell gets the base class. Header cells get the `--header` modifier.
 * Alignment is expressed as a modifier rather than an inline style, following
 * the BEM convention used by all other ui-components.
 *
 * ```
 * cellClass(col, true)  → "table__cell table__cell--header table__cell--align-left"
 * cellClass(col, false) → "table__cell table__cell--align-right"
 * ```
 */
function cellClass<T>(column: ColumnDef<T>, isHeader: boolean): string {
  const parts = ["table__cell"];
  if (isHeader) parts.push("table__cell--header");
  parts.push(`table__cell--align-${column.align ?? "left"}`);
  return parts.join(" ");
}

/**
 * Converts a column width value to a CSS width string.
 *
 * - `undefined` → `undefined` (no inline style, column auto-sizes)
 * - `"200px"` → `"200px"` (CSS string passed through)
 * - `200` → `"200px"` (numeric pixels for Canvas compatibility)
 */
function toWidthStyle(width: string | number | undefined): string | undefined {
  if (width === undefined) return undefined;
  if (typeof width === "number") return `${width}px`;
  return width;
}

/**
 * Resolves the numeric pixel width of a column for resize purposes.
 * Falls back to DEFAULT_RESIZABLE_WIDTH if no width is set.
 */
function resolveNumericWidth<T>(col: ColumnDef<T>): number {
  if (typeof col.width === "number") return col.width;
  if (typeof col.width === "string") {
    const parsed = parseInt(col.width, 10);
    if (!isNaN(parsed)) return parsed;
  }
  return DEFAULT_RESIZABLE_WIDTH;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function DataTable<T>({
  columns,
  data,
  rowKey,
  ariaLabel,
  caption,
  className = "table",
  resizable = false,
  onColumnResize,
}: TableBaseProps<T>) {
  const getKey = rowKey ?? ((_row: T, index: number) => index);
  const containerRef = useRef<HTMLDivElement>(null);

  // Detect text direction for RTL support
  const [direction, setDirection] = useState<"ltr" | "rtl">("ltr");
  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;
    const dir = getComputedStyle(el).direction;
    setDirection(dir === "rtl" ? "rtl" : "ltr");
  }, []);

  const { getColumnWidth, startResize, handleResizeKeyDown, isResizing } =
    useColumnResize({ direction, onColumnResize });

  const containerClass = [
    className,
    isResizing ? "table--resizing" : "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <div
      ref={containerRef}
      className={containerClass}
      role="region"
      aria-label={ariaLabel}
      tabIndex={0}
    >
      <table className="table__grid">
        {caption !== undefined && (
          <caption className="table__caption">{caption}</caption>
        )}

        <thead className="table__head">
          <tr className="table__row table__row--header">
            {columns.map((col) => {
              const baseWidth = resolveNumericWidth(col);
              const effectiveWidth = resizable
                ? getColumnWidth(col.id, baseWidth)
                : undefined;
              const widthStyle = resizable
                ? `${effectiveWidth}px`
                : toWidthStyle(col.width);

              return (
                <th
                  key={col.id}
                  className={cellClass(col, true)}
                  scope="col"
                  style={{
                    ...(widthStyle !== undefined ? { width: widthStyle } : {}),
                    ...(resizable ? { position: "relative" as const } : {}),
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
                        startResize(col.id, e.clientX, effectiveWidth!);
                      }}
                      onKeyDown={(e) =>
                        handleResizeKeyDown(col.id, effectiveWidth!, e)
                      }
                    />
                  )}
                </th>
              );
            })}
          </tr>
        </thead>

        <tbody className="table__body">
          {data.map((row, rowIndex) => (
            <tr key={getKey(row, rowIndex)} className="table__row">
              {columns.map((col) => {
                const baseWidth = resolveNumericWidth(col);
                const effectiveWidth = resizable
                  ? getColumnWidth(col.id, baseWidth)
                  : undefined;
                const widthStyle = resizable
                  ? `${effectiveWidth}px`
                  : toWidthStyle(col.width);

                return (
                  <td
                    key={col.id}
                    className={cellClass(col, false)}
                    style={widthStyle !== undefined ? { width: widthStyle } : undefined}
                  >
                    {resolveCellValue(col, row, rowIndex)}
                  </td>
                );
              })}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
