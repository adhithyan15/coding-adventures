// App.tsx — the VisiCalc host application (UI26 §7.5).
//
// Pulls together the two Mosaic-generated components (Grid and
// FormulaBar) and wires them to a single `useReducer` dispatch. The
// components themselves know nothing about spreadsheets — they just
// render the props they receive and fire events.
//
// `viewport-rows` is typed `list<list<text>>` in Grid.mil, which the
// mosmodel-compiler now lowers to `Array<Array<string>>` in the
// generated TS prop. Each row is a list of cell strings — the natural
// shape of a spreadsheet viewport slice. Before mosmodel-compiler's
// `ListInnerType::List` variant existed, the .mil had to spell
// `list<text>` and this file carried a `@ts-expect-error` to mask the
// resulting `Array<string>` vs `Array<Array<string>>` mismatch.

import { useReducer, useMemo, useEffect, useCallback } from "react";
import { Grid } from "../components/Grid";
import { FormulaBar } from "../components/FormulaBar";
import { initialState, reducer } from "./state";
import { buildViewportRows, cellLabel, cellKey } from "./util";

export function App() {
  const [state, dispatch] = useReducer(reducer, initialState);

  // Derived: the 2-D viewport slice the Grid component is given.
  // Recomputed each render; O(viewportSize * totalCols) is negligible.
  const viewportRows = useMemo(
    () =>
      buildViewportRows(
        state.cells,
        state.viewportOffset,
        state.viewportSize,
        state.totalRows,
        state.totalCols,
      ),
    [
      state.cells,
      state.viewportOffset,
      state.viewportSize,
      state.totalRows,
      state.totalCols,
    ],
  );

  // The formula bar shows either the live edit content (when editing)
  // or the raw value of the selected cell (when not editing).
  const isEditingSelected =
    state.editRow === state.selectedRow && state.editCol === state.selectedCol;
  const formulaBarValue = isEditingSelected
    ? state.editContent
    : state.cells[cellKey(state.selectedRow, state.selectedCol)] ?? "";

  // Keyboard handling (UI26 §8). We attach a single keydown listener
  // on window because navigation is a host concern, not a Grid concern.
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      // While editing, let the FormulaBar's <input> handle keystrokes.
      if (state.editRow !== -1) return;

      const r = state.selectedRow;
      const c = state.selectedCol;
      switch (e.key) {
        case "ArrowUp":
          if (r > 0) dispatch({ type: "navigate", row: r - 1, col: c });
          e.preventDefault();
          break;
        case "ArrowDown":
          if (r + 1 < state.totalRows)
            dispatch({ type: "navigate", row: r + 1, col: c });
          e.preventDefault();
          break;
        case "ArrowLeft":
          if (c > 0) dispatch({ type: "navigate", row: r, col: c - 1 });
          e.preventDefault();
          break;
        case "ArrowRight":
          if (c + 1 < state.totalCols)
            dispatch({ type: "navigate", row: r, col: c + 1 });
          e.preventDefault();
          break;
        case "Enter":
        case "F2":
          dispatch({ type: "editStart", row: r, col: c });
          e.preventDefault();
          break;
        default:
          // Any single printable character starts editing.
          if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
            dispatch({ type: "editStart", row: r, col: c });
          }
      }
    },
    [
      state.selectedRow,
      state.selectedCol,
      state.editRow,
      state.totalRows,
      state.totalCols,
    ],
  );

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100vh" }}>
      <FormulaBar
        cellAddress={cellLabel(state.selectedRow, state.selectedCol)}
        formula={formulaBarValue}
        readOnly={false}
        dispatch={dispatch}
      />
      <Grid
        columnHeaders={state.columnHeaders}
        viewportRows={viewportRows}
        columnWidths={state.columnWidths}
        // viewport scroll bound. With sticky-header: true in the .mll
        // (WA5, UI27 §6), the Grid wraps its `<table>` in a scroll div
        // and pins the `<thead>`. 600px keeps roughly 26 visible rows
        // at the 22px row height declared in Grid.dark.msl.
        //
        // UI28-1 / U29-D1 note: sticky-header is deferred per UI28-1
        // §2 constraint 5. `totalHeight` is still a Grid.mil slot for
        // forward compatibility, but the v0.2.0 Grid.mll doesn't
        // consume it — the header scrolls away with the body until
        // UI28-2 brings sticky-header back via HostScroll composition.
        totalHeight={600}
        selectedRow={state.selectedRow - state.viewportOffset}
        selectedCol={state.selectedCol}
        editRow={state.editRow - state.viewportOffset}
        editCol={state.editCol}
        // UI28-1 / U29-D1 — the live edit buffer Grid threads into the
        // inline <input value=...> when the user is editing a cell.
        editContent={state.editContent}
        dispatch={dispatch}
      />
    </div>
  );
}
