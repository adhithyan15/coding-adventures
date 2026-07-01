// util.ts — small pure helpers used by both `state.ts` and `App.tsx`.
//
// Per UI26 §9 — column labels A..Z for v1 (no two-letter columns yet).

/** Column-index to A1-style letter. 0 → "A", 1 → "B", ..., 25 → "Z". */
export function colLabel(col: number): string {
  return String.fromCharCode(65 + col);
}

/** (row, col) to A1-style cell address. (0,0) → "A1", (5,2) → "C6". */
export function cellLabel(row: number, col: number): string {
  return `${colLabel(col)}${row + 1}`;
}

/** Identity map onto cellLabel — used as the cells-record key. */
export function cellKey(row: number, col: number): string {
  return cellLabel(row, col);
}

/**
 * Slice the cells record into a 2-D viewport array of display values.
 * Per UI26 §7.4. Missing cells render as the empty string. The cost is
 * O(viewportSize * totalCols) per render, which is negligible for
 * realistic viewport sizes.
 */
export function buildViewportRows(
  cells: Record<string, string>,
  viewportOffset: number,
  viewportSize: number,
  totalRows: number,
  totalCols: number,
): string[][] {
  const rows: string[][] = [];
  for (let r = 0; r < viewportSize; r++) {
    const absRow = viewportOffset + r;
    if (absRow >= totalRows) break;
    const row: string[] = [];
    for (let c = 0; c < totalCols; c++) {
      row.push(cells[cellKey(absRow, c)] ?? "");
    }
    rows.push(row);
  }
  return rows;
}
