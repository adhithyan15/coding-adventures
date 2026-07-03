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

import {
  useReducer,
  useMemo,
  useEffect,
  useCallback,
  useRef,
  useState,
} from "react";
import { Grid } from "../components/Grid";
import { FormulaBar } from "../components/FormulaBar";
import { initialState, reducer, type AppAction } from "./state";
import { cellLabel } from "./util";
import { loadEngine, type Engine } from "./engine";

export function App() {
  const [state, dispatch] = useReducer(reducer, initialState);

  // The Rust spreadsheet-core engine, compiled to WASM (see engine.ts). It holds
  // the cells / dependency graph / recalc; the grid renders ITS computed values
  // and commits write through to it — unlike the reducer-only v0.1.0, whose cells
  // were raw strings with no formula evaluation (so the grid was empty).
  const engineRef = useRef<Engine | null>(null);
  const [ready, setReady] = useState(false);
  const [rev, setRev] = useState(0); // bump to re-derive after a mutation

  useEffect(() => {
    let live = true;
    loadEngine().then((e) => {
      if (!live) return;
      engineRef.current = e;
      setReady(true);
      setRev((r) => r + 1);
    });
    return () => {
      live = false;
    };
  }, []);

  // Derived: the 2-D viewport slice the Grid is given — the engine's *computed*
  // display strings for the visible window. Re-derived when the engine mutates
  // (rev) or the viewport moves.
  const viewportRows = useMemo(
    () =>
      ready && engineRef.current
        ? engineRef.current.window(
            state.viewportOffset,
            state.viewportSize,
            state.totalRows,
            state.totalCols,
          )
        : [],
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [ready, rev, state.viewportOffset, state.viewportSize, state.totalRows, state.totalCols],
  );

  // The formula bar shows the live edit buffer while editing, else the selected
  // cell's raw SOURCE (=SUM(A1:D1), not the computed 38) read from the engine.
  const isEditingSelected =
    state.editRow === state.selectedRow && state.editCol === state.selectedCol;
  const formulaBarValue = isEditingSelected
    ? state.editContent
    : ready && engineRef.current
      ? engineRef.current.raw(state.selectedRow, state.selectedCol)
      : "";

  // Intercept commits: write the buffered edit through to the engine (which
  // recomputes every dependent), then let the reducer update UI state and bump
  // `rev` so the grid re-reads the engine. `editCommit` is the grid's inline
  // edit; `commit` is the formula bar's. Both buffer into state.editContent.
  const engineDispatch = useCallback(
    (action: AppAction) => {
      // editStart: seed the edit buffer with the cell's SOURCE from the engine
      // (the reducer reads state.cells, which is empty now that the engine owns
      // the data — so an edit would otherwise start blank).
      if (action.type === "editStart" && engineRef.current) {
        dispatch(action);
        dispatch({
          type: "formulaChange",
          value: engineRef.current.raw(action.row, action.col),
        });
        return;
      }
      if (
        (action.type === "editCommit" || action.type === "commit") &&
        engineRef.current &&
        state.editRow !== -1
      ) {
        engineRef.current.setCell(
          state.editRow,
          state.editCol,
          state.editContent,
        );
        dispatch(action);
        setRev((r) => r + 1);
        return;
      }
      dispatch(action);
    },
    [state.editRow, state.editCol, state.editContent],
  );

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
          if (r > 0) engineDispatch({ type: "navigate", row: r - 1, col: c });
          e.preventDefault();
          break;
        case "ArrowDown":
          if (r + 1 < state.totalRows)
            engineDispatch({ type: "navigate", row: r + 1, col: c });
          e.preventDefault();
          break;
        case "ArrowLeft":
          if (c > 0) engineDispatch({ type: "navigate", row: r, col: c - 1 });
          e.preventDefault();
          break;
        case "ArrowRight":
          if (c + 1 < state.totalCols)
            engineDispatch({ type: "navigate", row: r, col: c + 1 });
          e.preventDefault();
          break;
        case "Enter":
        case "F2":
          engineDispatch({ type: "editStart", row: r, col: c });
          e.preventDefault();
          break;
        default:
          // Any single printable character starts editing.
          if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
            engineDispatch({ type: "editStart", row: r, col: c });
          }
      }
    },
    [
      state.selectedRow,
      state.selectedCol,
      state.editRow,
      state.totalRows,
      state.totalCols,
      engineDispatch,
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
        dispatch={engineDispatch}
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
        dispatch={engineDispatch}
      />
    </div>
  );
}
