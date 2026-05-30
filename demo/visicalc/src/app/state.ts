// state.ts — AppState, initialState, and the reducer for the VisiCalc demo.
//
// Per UI26 §7. The reducer is the only piece of "spreadsheet logic" in
// the entire suite. It deliberately stores cells as raw strings and
// displays them as-is — formula evaluation is out of scope here (it
// would replace the editCommit case to consult an engine). See
// UI26 §11 for the deferred-track note.

// Event-union types come straight from the generated components. The
// pipeline emitter now writes `export type GridEvent = ...` and
// `export type FormulaBarEvent = ...` so hosts no longer need a
// hand-maintained copy of the shapes — they import them directly.
import type { GridEvent } from "../components/Grid";
import type { FormulaBarEvent } from "../components/FormulaBar";
import { cellKey } from "./util";

// Re-export for downstream files (App.tsx, util.ts) that consume the
// event types via this module's surface.
export type { GridEvent, FormulaBarEvent };

/** Host-internal events not produced by any Mosaic component. */
export type HostEvent =
  | { type: "editStart"; row: number; col: number }
  | { type: "editCommit" }
  | { type: "editCancel" }
  | { type: "scroll"; offset: number }
  | {
      type: "select";
      startRow: number;
      startCol: number;
      endRow: number;
      endCol: number;
    }
  | { type: "loadData"; cells: Record<string, string> };

export type AppAction = GridEvent | FormulaBarEvent | HostEvent;

// ---------------------------------------------------------------------
// State + initialState
// ---------------------------------------------------------------------

export interface AppState {
  /** Cell raw values, keyed by "A1", "B12", etc. */
  cells: Record<string, string>;

  /** Selection */
  selectedRow: number;
  selectedCol: number;

  /** Edit state. editRow === -1 means not editing. */
  editRow: number;
  editCol: number;
  editContent: string;

  /** Viewport */
  viewportOffset: number;
  viewportSize: number;

  /** Column metadata */
  columnHeaders: string[];
  columnWidths: number[];

  /** Total spreadsheet size */
  totalRows: number;
  totalCols: number;
}

export const initialState: AppState = {
  cells: {},
  selectedRow: 0,
  selectedCol: 0,
  editRow: -1,
  editCol: -1,
  editContent: "",
  viewportOffset: 0,
  viewportSize: 30,
  columnHeaders: Array.from({ length: 26 }, (_, i) => String.fromCharCode(65 + i)),
  columnWidths: Array.from({ length: 26 }, () => 80),
  totalRows: 100,
  totalCols: 26,
};

// ---------------------------------------------------------------------
// Reducer — UI26 §7.3.
//
// Cells are stored and displayed as raw strings. `editCommit` /
// `commit` is where a real formula engine would plug in. See UI26 §11
// for the out-of-scope note.
// ---------------------------------------------------------------------

export function reducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case "navigate": {
      // Cancel any in-progress edit when navigating elsewhere.
      const exitEdit =
        state.editRow !== -1 ? { editRow: -1, editCol: -1, editContent: "" } : {};
      return {
        ...state,
        ...exitEdit,
        selectedRow: action.row,
        selectedCol: action.col,
      };
    }

    case "editStart": {
      const k = cellKey(action.row, action.col);
      return {
        ...state,
        editRow: action.row,
        editCol: action.col,
        editContent: state.cells[k] ?? "",
      };
    }

    case "editCommit": {
      // The inline cell HostInput's `onCommit:` emit is void by design
      // (see mosaic-emit-react::emit_host_input_jsx) — the buffered
      // text already lives in state.editContent, accumulated keystroke-
      // by-keystroke via the `formulaChange` action.  Reading from
      // action.value would always be `undefined` because the dispatch
      // payload is `{ type: "editCommit" }` with no fields.
      if (state.editRow === -1) return state;
      const k = cellKey(state.editRow, state.editCol);
      const newCells = { ...state.cells, [k]: state.editContent };
      // Move selection down one row after commit (Excel convention).
      const nextRow = Math.min(state.editRow + 1, state.totalRows - 1);
      const nextCol = state.editCol;
      return {
        ...state,
        cells: newCells,
        editRow: -1,
        editCol: -1,
        editContent: "",
        selectedRow: nextRow,
        selectedCol: nextCol,
      };
    }

    case "editCancel":
      return { ...state, editRow: -1, editCol: -1, editContent: "" };

    case "scroll":
      return { ...state, viewportOffset: action.offset };

    case "select":
      return {
        ...state,
        selectedRow: action.startRow,
        selectedCol: action.startCol,
      };

    // FormulaBar events
    case "formulaChange":
      return { ...state, editContent: action.value };

    case "commit": {
      if (state.editRow === -1) return state;
      const k = cellKey(state.editRow, state.editCol);
      return {
        ...state,
        cells: { ...state.cells, [k]: state.editContent },
        editRow: -1,
        editCol: -1,
        editContent: "",
      };
    }

    case "cancel":
      return { ...state, editRow: -1, editCol: -1, editContent: "" };

    case "loadData":
      return { ...state, cells: { ...action.cells } };
  }
}
