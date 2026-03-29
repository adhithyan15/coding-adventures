/**
 * useGridKeyboard — keyboard navigation for ARIA grid patterns.
 *
 * Implements the WAI-ARIA Grid pattern keyboard interactions:
 * https://www.w3.org/WAI/ARIA/apg/patterns/grid/
 *
 * === Keyboard Bindings ===
 *
 * | Key              | Action                                    |
 * |------------------|-------------------------------------------|
 * | ArrowRight       | Move focus to the next cell in the row    |
 * | ArrowLeft        | Move focus to the previous cell in the row|
 * | ArrowDown        | Move focus to the cell below               |
 * | ArrowUp          | Move focus to the cell above               |
 * | Home             | Move focus to the first cell in the row    |
 * | End              | Move focus to the last cell in the row     |
 * | Ctrl+Home        | Move focus to the first cell in the grid   |
 * | Ctrl+End         | Move focus to the last cell in the grid    |
 *
 * === Roving Tabindex ===
 *
 * The grid uses the "roving tabindex" pattern (same as TabList):
 * - The focused cell has tabIndex=0 (reachable via Tab)
 * - All other cells have tabIndex=-1 (skipped by Tab, reachable via arrows)
 * - Tab exits the grid entirely (standard grid behavior)
 *
 * This means a grid with 1000 cells has exactly ONE tab stop, not 1000.
 * Users press Tab to enter the grid, arrows to navigate within it, and
 * Tab again to leave.
 *
 * === Usage ===
 *
 * ```tsx
 * const { focusedCell, onKeyDown, getCellTabIndex } = useGridKeyboard({
 *   rowCount: data.length,
 *   colCount: columns.length,
 * });
 *
 * <div role="grid" onKeyDown={onKeyDown}>
 *   {rows.map((row, r) => (
 *     <div role="row">
 *       {cols.map((col, c) => (
 *         <div role="gridcell" tabIndex={getCellTabIndex(r, c)}>
 *           {cell}
 *         </div>
 *       ))}
 *     </div>
 *   ))}
 * </div>
 * ```
 */

import { useState, useCallback, type KeyboardEvent } from "react";

/** (row, col) coordinates identifying a cell in the grid. */
export interface CellPosition {
  row: number;
  col: number;
}

export interface UseGridKeyboardOptions {
  /** Total number of rows (including header). */
  rowCount: number;
  /** Total number of columns. */
  colCount: number;
}

export interface UseGridKeyboardReturn {
  /** The currently focused cell position. */
  focusedCell: CellPosition;

  /** Set the focused cell directly (e.g., on click). */
  setFocusedCell: (pos: CellPosition) => void;

  /** Keyboard event handler — attach to the grid container. */
  onKeyDown: (event: KeyboardEvent) => void;

  /**
   * Returns the tabIndex for a cell at (row, col).
   * The focused cell gets 0; all others get -1.
   */
  getCellTabIndex: (row: number, col: number) => 0 | -1;
}

/**
 * Clamps a value to [0, max). If max is 0, returns 0.
 */
function clamp(value: number, max: number): number {
  if (max <= 0) return 0;
  return Math.max(0, Math.min(value, max - 1));
}

export function useGridKeyboard({
  rowCount,
  colCount,
}: UseGridKeyboardOptions): UseGridKeyboardReturn {
  const [focusedCell, setFocusedCell] = useState<CellPosition>({
    row: 0,
    col: 0,
  });

  const onKeyDown = useCallback(
    (event: KeyboardEvent) => {
      const { key, ctrlKey, metaKey } = event;
      const ctrl = ctrlKey || metaKey;

      let nextRow = focusedCell.row;
      let nextCol = focusedCell.col;
      let handled = true;

      switch (key) {
        case "ArrowRight":
          nextCol = clamp(focusedCell.col + 1, colCount);
          break;

        case "ArrowLeft":
          nextCol = clamp(focusedCell.col - 1, colCount);
          break;

        case "ArrowDown":
          nextRow = clamp(focusedCell.row + 1, rowCount);
          break;

        case "ArrowUp":
          nextRow = clamp(focusedCell.row - 1, rowCount);
          break;

        case "Home":
          if (ctrl) {
            nextRow = 0;
            nextCol = 0;
          } else {
            nextCol = 0;
          }
          break;

        case "End":
          if (ctrl) {
            nextRow = clamp(rowCount - 1, rowCount);
            nextCol = clamp(colCount - 1, colCount);
          } else {
            nextCol = clamp(colCount - 1, colCount);
          }
          break;

        default:
          handled = false;
      }

      if (handled) {
        event.preventDefault();
        const next = { row: nextRow, col: nextCol };
        setFocusedCell(next);

        // Focus the corresponding DOM element via data attributes.
        // The grid cells should have data-row and data-col attributes.
        const grid = event.currentTarget;
        const target = grid.querySelector(
          `[data-row="${next.row}"][data-col="${next.col}"]`,
        ) as HTMLElement | null;
        target?.focus();
      }
    },
    [focusedCell, rowCount, colCount],
  );

  const getCellTabIndex = useCallback(
    (row: number, col: number): 0 | -1 => {
      return row === focusedCell.row && col === focusedCell.col ? 0 : -1;
    },
    [focusedCell],
  );

  return { focusedCell, setFocusedCell, onKeyDown, getCellTabIndex };
}
