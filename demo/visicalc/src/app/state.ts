// state.ts — AppState, initialState, and the reducer for the VisiCalc demo.
//
// Per UI26 §7. The reducer is the only piece of "spreadsheet logic" in
// the entire suite. It deliberately stores cells as raw strings and
// displays them as-is — formula evaluation is out of scope here (it
// would replace the editCommit case to consult an engine). See
// UI26 §11 for the deferred-track note.

import { cellKey } from "./util";

// ---------------------------------------------------------------------
// Event-union types (kept in sync by structure with the generated
// `GridEvent` and `FormulaBarEvent` types inside Grid.tsx / FormulaBar.tsx).
//
// The pipeline emitter currently produces these unions as non-exported
// `type ... = ...` declarations inside the generated .tsx files, so we
// define matching copies here. TypeScript's structural typing makes
// `dispatch` work as long as the shapes line up.
// ---------------------------------------------------------------------

export type GridEvent =
  | { type: "navigate"; row: number; col: number };

export type FormulaBarEvent =
  | { type: "formulaChange"; value: string }
  | { type: "commit" }
  | { type: "cancel" };

/** Host-internal events not produced by any Mosaic component. */
export type HostEvent =
  | { type: "editStart"; row: number; col: number }
  | { type: "editCommit"; value: string }
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
      if (state.editRow === -1) return state;
      const k = cellKey(state.editRow, state.editCol);
      const newCells = { ...state.cells, [k]: action.value };
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
