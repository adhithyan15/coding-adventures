// engine.ts — the React host's binding to the shared Rust spreadsheet engine.
//
// The other VisiCalc backends (SwiftUI/Qt/Flutter/Compose/Android) reach the Rust
// `spreadsheet-core` engine through a native FFI/JNI bridge. On the web the engine
// is compiled to WebAssembly; this module loads the SAME bundle the HTML demo ships
// (`public/spreadsheet-engine-wasm.js`, added to index.html), so the React grid
// renders the engine's *computed* values and edits recompute — instead of the empty
// placeholder grid the reducer-only version showed (its cells were raw strings with
// no formula evaluation).
//
// The bundle is a global-script IIFE: it resolves `window.__spreadsheetEngineReady`
// with an `Engine` whose `createSpreadsheet()` returns a `workbook` with the same
// operations the C-ABI/WASM facade exposes.

// The one asset the demo loads sets these globals (see index.html).
interface Workbook {
  setCells(cells: Record<string, string>): void;
  setCell(a1: string, raw: string): unknown;
  getRaw(a1: string): string;
  getDisplayWindow(
    row0: number,
    col0: number,
    row1: number,
    col1: number,
  ): { rows: number; cols: number; cells: string[][] };
  columnLetters(index: number): string;
}
declare global {
  interface Window {
    __spreadsheetEngineReady?: Promise<{ createSpreadsheet(): Workbook }>;
  }
}

// The classic cross-footing budget — the identical seed every other VisiCalc demo
// uses (E column = row sums, row 5 = column sums, E5 = grand total 169), so the
// React/Electron grid shows the SAME engine-computed values.
const SEED: Record<string, string> = {
  A1: "15", B1: "3", C1: "12", D1: "8", E1: "=SUM(A1:D1)",
  A2: "8", B2: "14", C2: "7", D2: "22", E2: "=SUM(A2:D2)",
  A3: "12", B3: "9", C3: "18", D3: "6", E3: "=SUM(A3:D3)",
  A4: "4", B4: "11", C4: "3", D4: "17", E4: "=SUM(A4:D4)",
  A5: "=SUM(A1:A4)", B5: "=SUM(B1:B4)", C5: "=SUM(C1:C4)",
  D5: "=SUM(D1:D4)", E5: "=SUM(E1:E4)",
};

/** A thin, React-friendly view of the engine workbook. */
export interface Engine {
  /**
   * The visible window as a 2-D array of display strings — the exact shape the
   * generated Grid consumes (row-major, `totalCols` columns per row, no row
   * label). `viewportOffset` is 0-based; the engine addresses are 1-based.
   */
  window(
    viewportOffset: number,
    viewportSize: number,
    totalRows: number,
    totalCols: number,
  ): string[][];
  /** A cell's typed source (for the formula bar). (row, col) are 0-based. */
  raw(row: number, col: number): string;
  /** Write a cell's source and recompute. (row, col) are 0-based. */
  setCell(row: number, col: number, value: string): void;
}

function a1(row: number, col: number): string {
  return `${String.fromCharCode(65 + col)}${row + 1}`;
}

/** Load + seed the WASM engine, returning the React-friendly view. */
export async function loadEngine(): Promise<Engine> {
  const ready = window.__spreadsheetEngineReady;
  if (!ready) {
    throw new Error(
      "spreadsheet engine bundle not loaded — is <script src='/spreadsheet-engine-wasm.js'> in index.html?",
    );
  }
  const rt = await ready;
  const wb = rt.createSpreadsheet();
  wb.setCells(SEED);
  return {
    window(viewportOffset, viewportSize, totalRows, totalCols) {
      const row1 = Math.min(viewportOffset + viewportSize, totalRows);
      if (row1 < viewportOffset + 1) return [];
      return wb.getDisplayWindow(viewportOffset + 1, 1, row1, totalCols).cells;
    },
    raw(row, col) {
      return wb.getRaw(a1(row, col));
    },
    setCell(row, col, value) {
      wb.setCell(a1(row, col), value);
    },
  };
}
