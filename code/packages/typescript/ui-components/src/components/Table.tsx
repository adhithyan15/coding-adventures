/**
 * Table — unified entry point and shared data model for tabular rendering.
 *
 * This file defines the types that both rendering backends (HTML and Canvas)
 * consume, plus a thin router component that delegates to the correct backend
 * based on the `renderer` prop.
 *
 * === Two Backends, One Data Model ===
 *
 * A table has two dimensions: columns (the schema) and rows (the data).
 *
 * ```
 * ┌──────────────────────────────────────────────┐
 * │  Column 1     Column 2     Column 3          │  ← ColumnDef[]
 * ├──────────────────────────────────────────────┤
 * │  row[0].a     row[0].b     row[0].c          │  ← data[0]
 * │  row[1].a     row[1].b     row[1].c          │  ← data[1]
 * └──────────────────────────────────────────────┘
 * ```
 *
 * The `ColumnDef` type tells the table what to display in each column:
 * - `id`: stable identity (for React keys, future sorting/filtering state)
 * - `header`: text in the column header
 * - `accessor`: how to extract a cell value — either a property key or function
 * - `width`: optional column width (CSS string for HTML, pixels for Canvas)
 * - `align`: text alignment within cells
 *
 * Both backends use `resolveCellValue()` to turn a (column, row) pair into a
 * display string. This shared utility ensures consistent rendering.
 *
 * === Usage ===
 *
 * ```tsx
 * import { Table } from "@coding-adventures/ui-components";
 *
 * interface Person { name: string; age: number; }
 *
 * const columns = [
 *   { id: "name", header: "Name", accessor: "name" as const },
 *   { id: "age", header: "Age", accessor: "age" as const, align: "right" as const },
 * ];
 *
 * // HTML backend (default):
 * <Table<Person> columns={columns} data={people} />
 *
 * // Canvas backend:
 * <Table<Person> columns={columns} data={people} renderer="canvas" />
 * ```
 */

import { DataTable } from "./DataTable.js";
import { CanvasTable } from "./CanvasTable.js";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/** Horizontal alignment of cell content. */
export type CellAlignment = "left" | "center" | "right";

/**
 * Defines a single column in the table.
 *
 * The generic parameter `T` is the row type. This enables type-safe accessors:
 * if `T = { name: string; age: number }`, then `accessor` can only be "name"
 * or "age" (when using a property key). Function accessors bypass this check
 * and can compute derived values.
 */
export interface ColumnDef<T> {
  /** Unique identifier for this column. Used as React key and for future
   *  sorting/filtering state. */
  id: string;

  /** Text displayed in the column header. */
  header: string;

  /**
   * How to extract the cell value from a row.
   *
   * Two forms:
   * - Property key: `accessor: "name"` reads `row["name"]`
   * - Function: `accessor: (row, i) => row.first + " " + row.last`
   *
   * Function accessors enable computed columns without pre-transforming data.
   */
  accessor: keyof T & string | ((row: T, rowIndex: number) => string | number);

  /**
   * Column width.
   *
   * For the HTML backend: a CSS value string ("200px", "30%", "auto").
   * For the Canvas backend: a pixel number (200).
   * Default: auto (HTML) or equal distribution of available space (Canvas).
   */
  width?: string | number;

  /** Horizontal text alignment within cells. Default: "left". */
  align?: CellAlignment;
}

/**
 * Extracts a stable identity from a row.
 *
 * Used as the React key for row elements. The default (array index) works for
 * static tables but breaks when rows are reordered, filtered, or paginated.
 * For interactive tables, use a unique field from the data (e.g., row.id).
 */
export type RowKeyFn<T> = (row: T, index: number) => string | number;

/**
 * Props shared by both DataTable (HTML) and CanvasTable (Canvas).
 *
 * These are the pure data props — no renderer selection. Consumers who import
 * a specific backend directly use this type.
 */
export interface TableBaseProps<T> {
  /** Column definitions describing the table schema. */
  columns: ColumnDef<T>[];

  /** The row data to render. Each element becomes one table row. */
  data: T[];

  /** Function to extract a stable key from each row. Defaults to array index. */
  rowKey?: RowKeyFn<T>;

  /** Accessible label for the table region. Announced by screen readers. */
  ariaLabel?: string;

  /** Optional caption rendered above the table. Visible to all users. */
  caption?: string;

  /** CSS class for the outermost container element. Default: "table". */
  className?: string;
}

/**
 * Props for the unified Table component, which adds the renderer selector
 * on top of the shared base props.
 */
export interface TableProps<T> extends TableBaseProps<T> {
  /**
   * Which rendering backend to use.
   *
   * - "html" (default): Semantic `<table>` element. Best for accessibility,
   *   SEO, and small-to-medium datasets.
   * - "canvas": 2D canvas rendering with ARIA grid overlay. Best for large
   *   datasets where DOM node count becomes a bottleneck.
   */
  renderer?: "html" | "canvas";
}

// ---------------------------------------------------------------------------
// Shared utilities
// ---------------------------------------------------------------------------

/**
 * Resolves a cell's display value from a column definition and row.
 *
 * Both the HTML and Canvas backends call this function for every cell. It
 * handles the two accessor forms (property key vs. function) and ensures
 * null/undefined values render as empty strings rather than "undefined".
 *
 * ```
 * resolveCellValue(column, row, rowIndex)
 *   ├─ accessor is function? → call it, String(result ?? "")
 *   └─ accessor is key?      → read row[key], String(result ?? "")
 * ```
 */
export function resolveCellValue<T>(
  column: ColumnDef<T>,
  row: T,
  rowIndex: number,
): string {
  const raw =
    typeof column.accessor === "function"
      ? column.accessor(row, rowIndex)
      : (row as Record<string, unknown>)[column.accessor];
  return String(raw ?? "");
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

/**
 * Unified Table component — delegates to DataTable or CanvasTable based on
 * the `renderer` prop.
 *
 * This is a thin router with no state. Future versions may add shared state
 * here (sort direction, selection set) that both backends consume.
 */
export function Table<T>({
  renderer = "html",
  ...props
}: TableProps<T>) {
  if (renderer === "canvas") {
    return <CanvasTable<T> {...props} />;
  }
  return <DataTable<T> {...props} />;
}
