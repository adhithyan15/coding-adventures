// App.tsx — the VisiCalc host application (UI26 §7.5).
//
// Pulls together the two Mosaic-generated components (Grid and
// FormulaBar) and wires them to a single `useReducer` dispatch. The
// components themselves know nothing about spreadsheets — they just
// render the props they receive and fire events.
//
// ## Known limitation: viewport-rows nested-list typing
//
// The UI26 spec calls for `slot viewport-rows : list<list<text>>` so the
// Grid receives an `Array<Array<string>>`. The current mosmodel-compiler
// `ListInnerType` enum only models flat lists (no recursive `List`
// variant), so the generated Grid.tsx prop type is `Array<string>`
// instead of the spec's `Array<Array<string>>`. We use `// @ts-expect-error`
// below to acknowledge the mismatch — the architecture is correct, but
// the demo will not render row cells correctly until mosmodel grows
// nested-list support. See README for the follow-up note.

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
        // @ts-expect-error UI26 §11: Grid component expects Array<Array<string>>
        // per spec, but mosmodel-compiler can only represent `list<text>` (flat)
        // until ListInnerType gains a recursive `List` variant. Documented in
        // README. The demo still compiles and renders the FormulaBar correctly.
        viewportRows={viewportRows}
        selectedRow={state.selectedRow - state.viewportOffset}
        selectedCol={state.selectedCol}
        editRow={state.editRow - state.viewportOffset}
        editCol={state.editCol}
        dispatch={dispatch}
      />
    </div>
  );
}
