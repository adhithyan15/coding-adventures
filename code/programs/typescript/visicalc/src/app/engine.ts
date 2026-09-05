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
import SEED from "../../../../mosaic/visicalc/fixtures/budget-v1.json";

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
