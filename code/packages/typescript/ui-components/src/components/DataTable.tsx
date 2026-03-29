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
 *             scope="col" style="width: 200px">
 *           Name
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
 * === Why the Wrapping <div>? ===
 *
 * The outer `<div role="region" tabindex="0">` serves two purposes:
 *
 * 1. **Scrollable container** — when the table overflows horizontally, this
 *    div scrolls independently. Without it, the whole page scrolls.
 *
 * 2. **Keyboard scrollable** — `tabindex="0"` makes the container focusable,
 *    so keyboard users can scroll with arrow keys when the table overflows.
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
 * />
 * ```
 */

import type { TableBaseProps, ColumnDef } from "./Table.js";
import { resolveCellValue } from "./Table.js";

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
}: TableBaseProps<T>) {
  const getKey = rowKey ?? ((_row: T, index: number) => index);

  return (
    <div
      className={className}
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
              const widthStyle = toWidthStyle(col.width);
              return (
                <th
                  key={col.id}
                  className={cellClass(col, true)}
                  scope="col"
                  style={widthStyle !== undefined ? { width: widthStyle } : undefined}
                >
                  {col.header}
                </th>
              );
            })}
          </tr>
        </thead>

        <tbody className="table__body">
          {data.map((row, rowIndex) => (
            <tr key={getKey(row, rowIndex)} className="table__row">
              {columns.map((col) => (
                <td key={col.id} className={cellClass(col, false)}>
                  {resolveCellValue(col, row, rowIndex)}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
