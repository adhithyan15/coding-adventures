# UI26 — visicalc: VisiCalc Component Suite

**Status:** Specification (draft)
**Layer:** UI / application
**Depends on:** UI13 (mosmodel), UI14 (moslayout), UI15 (mosstyle), UI23 (mosaic-pipeline), UI24 (emit→dispatch), UI25 (Input primitive)

---

## 1. Purpose and architecture

This spec describes a working **VisiCalc-style spreadsheet UI** built end
to end on the Mosaic component system, targeting the React backend. It
is the capstone for the Mosaic UI work: every other spec in the UI series
(UI13 through UI25) is exercised by this one application.

The goal is to prove that the Mosaic compiler stack — three small
languages plus a pipeline manifest plus the React emitter — can produce
the entire view layer of a non-trivial reactive application without any
view-layer code being written by hand. The host application owns state
and event handling; every pixel is generated.

### System diagram

```
.mil + .mll + .msl  (per component, authored by hand)
        │
        ▼
mosaic-compile --pipeline visicalc-desktop-dark.mospipeline
        │
        ▼
src/components/Grid.tsx        ←  generated, do not edit
src/components/FormulaBar.tsx  ←  generated, do not edit

         ┌────────────────────────────────────┐
         │  Host React application            │
         │  (useReducer + AppState)           │
         │                                    │
         │   slots (data ↓)  dispatch (↑)     │
         ├─────────────────┬──────────────────┤
         ▼                 ▼
   FormulaBar         Grid
   (generated)        (generated)
```

### Three structural invariants

1. **Mosaic components are dumb renderers.** They know nothing about
   spreadsheets, formulas, or VisiCalc. They render the data they are
   handed and fire the events declared in their `.mil` interface.
2. **The host owns all state** in a single `useReducer`. Cell values,
   selection, edit state, viewport offset — all of it.
3. **Formula evaluation is out of scope** for this UI work. The host
   stores raw string values per cell and displays them as-is. Replacing
   that with a real formula engine is a separate parallel track and
   does not change anything specified here.

These invariants are what make the suite testable and replayable: events
are plain data, state is one object, and the components are functions
of props.

---

## 2. Component interfaces (`.mil`)

Two components make up the VisiCalc UI: `Grid` and `FormulaBar`.

### 2.1 Grid.mil

```mosmodel
component Grid version 1.0 {
  // Display data
  slot column-headers : list<text> ;
  slot column-widths  : list<number> ;
  slot viewport-rows  : list<list<text>> ;   // visible rows, display-ready
  slot total-rows     : number ;

  // Viewport (host owns scroll position)
  slot viewport-offset : number = 0 ;

  // Selection (host owns)
  slot selected-row : number = 0 ;
  slot selected-col : number = 0 ;

  // Edit state (host owns)
  slot edit-row     : number = -1 ;          // -1 means not editing
  slot edit-col     : number = -1 ;
  slot edit-content : text   = "" ;

  // Events
  emit onNavigate   ( row : number , col : number ) ;
  emit onEditStart  ( row : number , col : number ) ;
  emit onEditCommit ( value : text ) ;
  emit onEditCancel ;
  emit onScroll     ( offset : number ) ;
  emit onSelect     ( start-row : number , start-col : number ,
                      end-row   : number , end-col   : number ) ;
}
```

Design notes:

- All state slots are mirror-of-host: the host decides what is selected,
  what is being edited, where the viewport sits. Grid does not maintain
  its own copy.
- `viewport-rows` is a `list<list<text>>` of **already-resolved display
  strings**. Grid does not see formulas, just the strings to print. This
  is what makes Grid agnostic to whatever formula engine the host uses.
- `edit-row = -1` is the sentinel for "no cell is being edited." A
  separate `bool` slot would have worked equally well; the sentinel
  approach mirrors common spreadsheet code and avoids one extra slot.

### 2.2 FormulaBar.mil

```mosmodel
component FormulaBar version 1.0 {
  slot cell-address : text ;     // e.g. "A1", "B12"
  slot formula      : text ;     // raw value shown in the editor
  slot read-only    : bool = false ;

  emit onFormulaChange ( formula : text ) ;
  emit onCommit ;
  emit onCancel ;
}
```

Design notes:

- `formula` is whatever the user is typing. While editing, the host
  pushes the live edit content. While not editing, the host pushes the
  raw value of the selected cell.
- `read-only` lets the host gate editing (e.g., when no cell is selected
  or when the workbook is locked).

---

## 3. Layout variants (`.mll`)

### 3.1 Grid.desktop.mll

```moslayout
layout Grid.desktop version 1.0 implements Grid 1.x {
  Grid {
    column-headers:  @column-headers;
    column-widths:   @column-widths;
    viewport-rows:   @viewport-rows;
    total-rows:      @total-rows;
    viewport-offset: @viewport-offset;
    selected-row:    @selected-row;
    selected-col:    @selected-col;
    edit-row:        @edit-row;
    edit-col:        @edit-col;
    edit-content:    @edit-content;
    [grid-table]

    connects: onNavigate(row: number, col: number)
              -> emit onNavigate(row: row, col: col);
    connects: onEditStart(row: number, col: number)
              -> emit onEditStart(row: row, col: col);
    connects: onEditCommit(value: text)
              -> emit onEditCommit(value: value);
    connects: onEditCancel
              -> emit onEditCancel;
    connects: onScroll(offset: number)
              -> emit onScroll(offset: offset);
    connects: onSelect(startRow: number, startCol: number,
                       endRow:   number, endCol:   number)
              -> emit onSelect(start-row: startRow, start-col: startCol,
                                end-row:  endRow,  end-col:  endCol);
  }
}
```

This is a pass-through layout: the only primitive used is the built-in
`Grid` primitive (UI14 §11), every slot is forwarded by reference, and
every native Grid event is wired to the matching mosmodel emit.

### 3.2 FormulaBar.desktop.mll

```moslayout
layout FormulaBar.desktop version 1.0 implements FormulaBar 1.x {
  Row {
    Text {
      content: @cell-address;
      [address-label]
    }
    Input {
      value:     @formula;
      read-only: @read-only;
      [formula-field]

      connects: onChange(value: text) -> emit onFormulaChange(formula: value);
      connects: onCommit              -> emit onCommit;
      connects: onCancel              -> emit onCancel;
    }
  }
}
```

This layout uses the new `Input` primitive from UI25 to make the formula
text editable, and the existing `Row` and `Text` primitives from UI14
to lay out the address label and field side by side.

---

## 4. Style variants (`.msl`)

### 4.1 visicalc-tokens.msl

Shared design tokens used by both component styles. Lives at the
pipeline level via `[global-style]` (UI23 §4).

```mosstyle
tokens visicalc {
  $color-surface         : #1e1e1e ;
  $color-surface-alt     : #252526 ;
  $color-surface-header  : #2d2d30 ;
  $color-border          : #3f3f46 ;
  $color-text-primary    : #cccccc ;
  $color-text-secondary  : #9d9d9d ;
  $color-accent          : #007acc ;
  $color-selected        : #264f78 ;
  $color-selected-text   : #ffffff ;
  $color-editing         : #1f4f3f ;
  $color-error           : #f44747 ;
  $row-height            : 22px ;
  $cell-pad-x            : 8px ;
  $cell-pad-y            : 4px ;
  $header-height         : 24px ;
  $row-number-width      : 40px ;
  $monospace             : "SF Mono", "Cascadia Code", "Consolas", monospace ;
  $sans                  : -apple-system, "Segoe UI", system-ui, sans-serif ;
}
```

### 4.2 Grid.dark.msl

```mosstyle
style Grid.dark version 1.0 for Grid 1.x {
  .grid-table {
    font-family: $monospace;
    font-size:   12px;
    color:       $color-text-primary;
    background:  $color-surface;
    border-collapse: collapse;
    width: 100%;
  }

  .grid-header-row {
    background: $color-surface-header;
    height:     $header-height;
    color:      $color-text-secondary;
    font-weight: normal;
    text-align: center;
    border-bottom: 1px solid $color-border;
  }

  .grid-row-number {
    background: $color-surface-alt;
    color:      $color-text-secondary;
    width:      $row-number-width;
    text-align: right;
    padding:    $cell-pad-y $cell-pad-x;
    border-right: 1px solid $color-border;
  }

  .grid-cell {
    height:  $row-height;
    padding: $cell-pad-y $cell-pad-x;
    border:  1px solid $color-border;
    background: $color-surface;

    state hover {
      background: $color-surface-alt;
    }
    state selected {
      background: $color-selected;
      color:      $color-selected-text;
      outline:    1px solid $color-accent;
    }
    state editing {
      background: $color-editing;
      outline:    1px solid $color-accent;
    }
    state error {
      color: $color-error;
    }
  }
}
```

### 4.3 FormulaBar.dark.msl

```mosstyle
style FormulaBar.dark version 1.0 for FormulaBar 1.x {
  .address-label {
    font-family: $monospace;
    font-size:   12px;
    color:       $color-text-secondary;
    background:  $color-surface-alt;
    min-width:   48px;
    padding:     $cell-pad-y $cell-pad-x;
    text-align:  right;
  }

  .formula-field {
    font-family:   $monospace;
    font-size:     13px;
    flex:          1;
    border:        none;
    border-bottom: 1px solid $color-border;
    background:    transparent;
    color:         $color-text-primary;
    padding:       $cell-pad-y $cell-pad-x;

    state focused {
      border-bottom-color: $color-accent;
    }
    state disabled {
      color: $color-text-secondary;
    }
  }
}
```

---

## 5. The pipeline manifest

```toml
# pipelines/visicalc-desktop-dark.mospipeline

[pipeline]
name    = "visicalc-desktop-dark"
version = "1.0"

[global-style]
tokens = ["mosaic/visicalc-tokens.msl"]

[search-path]
components = ["mosaic"]

[[component]]
interface = "Grid@1.x"
layout    = "Grid.desktop@1.x"
style     = "Grid.dark@1.x"
output    = "react"
out-dir   = "src/components"

[[component]]
interface = "FormulaBar@1.x"
layout    = "FormulaBar.desktop@1.x"
style     = "FormulaBar.dark@1.x"
output    = "react"
out-dir   = "src/components"
```

Running `mosaic-compile --pipeline pipelines/visicalc-desktop-dark.mospipeline`
produces:

```
src/components/
  Grid.tsx
  GridEvent.ts
  FormulaBar.tsx
  FormulaBarEvent.ts
```

The CSS for both components is inlined into the JSX via the React
emitter's existing inline-style mechanism (UI20 §4.1).

---

## 6. Generated React output

### 6.1 GridEvent.ts

```tsx
// Auto-generated by mosaic-emit-react. Do not edit.

export type GridEvent =
  | { type: "navigate";   row: number; col: number }
  | { type: "editStart";  row: number; col: number }
  | { type: "editCommit"; value: string }
  | { type: "editCancel" }
  | { type: "scroll";     offset: number }
  | { type: "select";     startRow: number; startCol: number;
                          endRow:   number; endCol:   number };
```

### 6.2 Grid.tsx (abbreviated)

```tsx
// Auto-generated by mosaic-emit-react. Do not edit.
import React from "react";
import type { GridEvent } from "./GridEvent";

interface GridProps {
  columnHeaders:  string[];
  columnWidths:   number[];
  viewportRows:   string[][];
  totalRows:      number;
  viewportOffset: number;
  selectedRow:    number;
  selectedCol:    number;
  editRow:        number;
  editCol:        number;
  editContent:    string;
  dispatch:       (event: GridEvent) => void;
}

export function Grid({
  columnHeaders, columnWidths, viewportRows, totalRows,
  viewportOffset, selectedRow, selectedCol,
  editRow, editCol, editContent, dispatch,
}: GridProps) {
  return (
    <table style={{ /* generated from Grid.dark.msl */ }}>
      <thead>
        <tr>
          <th style={{ /* row-number column */ }}></th>
          {columnHeaders.map((h, c) => (
            <th key={c} style={{ width: columnWidths[c] }}>{h}</th>
          ))}
        </tr>
      </thead>
      <tbody>
        {viewportRows.map((rowCells, r) => {
          const absoluteRow = viewportOffset + r;
          return (
            <tr key={absoluteRow}>
              <td style={{ /* row-number style */ }}>{absoluteRow + 1}</td>
              {rowCells.map((cellValue, c) => {
                const isSelected = absoluteRow === selectedRow && c === selectedCol;
                const isEditing  = absoluteRow === editRow     && c === editCol;
                return (
                  <td
                    key={c}
                    style={{ /* cell + selected/editing state styles */ }}
                    onClick={() => dispatch({ type: "navigate", row: absoluteRow, col: c })}
                    onDoubleClick={() => dispatch({ type: "editStart", row: absoluteRow, col: c })}
                  >
                    {isEditing ? (
                      <input
                        autoFocus
                        value={editContent}
                        onChange={e => {/* host updates editContent via separate event in §7 */}}
                        onKeyDown={e => {
                          if (e.key === "Enter")  dispatch({ type: "editCommit", value: (e.target as HTMLInputElement).value });
                          if (e.key === "Escape") dispatch({ type: "editCancel" });
                        }}
                      />
                    ) : cellValue}
                  </td>
                );
              })}
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}
```

The abbreviation hides the inlined style objects for brevity. The actual
generated file inlines every style from `Grid.dark.msl` into the JSX,
including the cascade for `state selected` / `state editing` / `state hover`.

### 6.3 FormulaBarEvent.ts

```tsx
// Auto-generated by mosaic-emit-react. Do not edit.

export type FormulaBarEvent =
  | { type: "formulaChange"; formula: string }
  | { type: "commit" }
  | { type: "cancel" };
```

### 6.4 FormulaBar.tsx

```tsx
// Auto-generated by mosaic-emit-react. Do not edit.
import React from "react";
import type { FormulaBarEvent } from "./FormulaBarEvent";

interface FormulaBarProps {
  cellAddress: string;
  formula:     string;
  readOnly:    boolean;
  dispatch:    (event: FormulaBarEvent) => void;
}

export function FormulaBar({ cellAddress, formula, readOnly, dispatch }: FormulaBarProps) {
  return (
    <div style={{ display: "flex", flexDirection: "row" }}>
      <span style={{ /* inlined from .address-label */ }}>{cellAddress}</span>
      <input
        type="text"
        value={formula}
        readOnly={readOnly}
        style={{ /* inlined from .formula-field */ }}
        onChange={e => dispatch({ type: "formulaChange", formula: e.target.value })}
        onKeyDown={e => {
          if (e.key === "Enter")  dispatch({ type: "commit" });
          if (e.key === "Escape") dispatch({ type: "cancel" });
        }}
      />
    </div>
  );
}
```

---

## 7. The host application

### 7.1 State shape

```typescript
// src/app/state.ts

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
  viewportSize: number;          // number of rows visible

  /** Column metadata */
  columnHeaders: string[];       // ["A", "B", "C", ...]
  columnWidths: number[];        // px widths

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
```

### 7.2 The action union

The reducer accepts every Mosaic component's event union plus any
host-internal actions:

```typescript
import type { GridEvent } from "../components/GridEvent";
import type { FormulaBarEvent } from "../components/FormulaBarEvent";

export type AppAction =
  | GridEvent
  | FormulaBarEvent
  | { type: "loadData"; cells: Record<string, string> };
```

### 7.3 The reducer

The reducer is the only piece of "spreadsheet logic" in the entire suite,
and it deliberately contains **no formula evaluation** — cell values are
stored and displayed as raw strings. Replacing that with a real
evaluator is a separate parallel track (see §11).

```typescript
import { cellKey } from "./util";

export function reducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {

    case "navigate": {
      // Cancel any in-progress edit when navigating elsewhere.
      const exitEdit = state.editRow !== -1
        ? { editRow: -1, editCol: -1, editContent: "" }
        : {};
      return { ...state, ...exitEdit, selectedRow: action.row, selectedCol: action.col };
    }

    case "editStart": {
      const k = cellKey(action.row, action.col);
      return {
        ...state,
        editRow:     action.row,
        editCol:     action.col,
        editContent: state.cells[k] ?? "",
      };
    }

    case "editCommit": {
      const k = cellKey(state.editRow, state.editCol);
      const newCells = { ...state.cells, [k]: action.value };
      // Move selection down one row after commit (Excel convention).
      const nextRow = Math.min(state.editRow + 1, state.totalRows - 1);
      return {
        ...state,
        cells:       newCells,
        editRow:     -1,
        editCol:     -1,
        editContent: "",
        selectedRow: nextRow,
        selectedCol: state.editCol,
      };
    }

    case "editCancel":
      return { ...state, editRow: -1, editCol: -1, editContent: "" };

    case "scroll":
      return { ...state, viewportOffset: action.offset };

    case "select":
      // Phase 1: treat selection as moving the cursor to the range start.
      return { ...state, selectedRow: action.startRow, selectedCol: action.startCol };

    // FormulaBar events
    case "formulaChange":
      return { ...state, editContent: action.formula };

    case "commit": {
      if (state.editRow === -1) return state;
      const k = cellKey(state.editRow, state.editCol);
      return {
        ...state,
        cells:       { ...state.cells, [k]: state.editContent },
        editRow:     -1,
        editCol:     -1,
        editContent: "",
      };
    }

    case "cancel":
      return { ...state, editRow: -1, editCol: -1, editContent: "" };

    case "loadData":
      return { ...state, cells: { ...action.cells } };
  }
}
```

### 7.4 Building `viewportRows` from `cells`

`Grid` needs a 2-D array of display strings. The host derives this from
`AppState` immediately before render:

```typescript
function buildViewportRows(state: AppState): string[][] {
  const rows: string[][] = [];
  for (let r = 0; r < state.viewportSize; r++) {
    const absRow = state.viewportOffset + r;
    if (absRow >= state.totalRows) break;
    const row: string[] = [];
    for (let c = 0; c < state.totalCols; c++) {
      row.push(state.cells[cellKey(absRow, c)] ?? "");
    }
    rows.push(row);
  }
  return rows;
}
```

This is `O(viewportSize × totalCols)` per render, which is negligible
for any realistic viewport. When formula evaluation is added later (see
§11), this is the function that grows to consult the engine.

### 7.5 The App component

```tsx
// src/app/App.tsx
import React, { useReducer, useMemo } from "react";
import { Grid } from "../components/Grid";
import { FormulaBar } from "../components/FormulaBar";
import { initialState, reducer } from "./state";
import { buildViewportRows, cellLabel } from "./util";

export function App() {
  const [state, dispatch] = useReducer(reducer, initialState);

  // Recomputed each render; cheap for a 30-row viewport.
  const viewportRows = useMemo(() => buildViewportRows(state), [state]);

  // Formula bar shows either the live edit content or the raw value of the
  // selected cell — whichever is current.
  const isEditingSelected =
    state.editRow === state.selectedRow && state.editCol === state.selectedCol;
  const formulaBarValue = isEditingSelected
    ? state.editContent
    : (state.cells[cellKey(state.selectedRow, state.selectedCol)] ?? "");

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
        columnWidths={state.columnWidths}
        viewportRows={viewportRows}
        totalRows={state.totalRows}
        viewportOffset={state.viewportOffset}
        selectedRow={state.selectedRow}
        selectedCol={state.selectedCol}
        editRow={state.editRow}
        editCol={state.editCol}
        editContent={state.editContent}
        dispatch={dispatch}
      />
    </div>
  );
}
```

Both components share the same `dispatch` because the action union
includes both event types. This is the entire reason for the union-based
dispatch pattern (UI24).

---

## 8. Keyboard behaviour

| Key            | Not editing                                  | Editing                                  |
|---|---|---|
| Arrow Up       | dispatch `navigate(selectedRow-1, selectedCol)` | (default, moves caret within input)      |
| Arrow Down     | dispatch `navigate(selectedRow+1, selectedCol)` | (default)                                |
| Arrow Left     | dispatch `navigate(selectedRow, selectedCol-1)` | (default)                                |
| Arrow Right    | dispatch `navigate(selectedRow, selectedCol+1)` | (default)                                |
| Enter          | dispatch `editStart(selectedRow, selectedCol)` with empty buffer | dispatch `editCommit(value)` |
| F2             | dispatch `editStart(selectedRow, selectedCol)` with existing value | (n/a)                                    |
| Escape         | (no-op)                                       | dispatch `editCancel`                    |
| Tab            | dispatch `navigate(selectedRow, selectedCol+1)` | dispatch `editCommit` then move right    |
| Shift+Tab      | dispatch `navigate(selectedRow, selectedCol-1)` | dispatch `editCommit` then move left     |
| Any printable  | dispatch `editStart` and seed buffer with that character | (default text input behaviour)    |

The keyboard handler lives in `Grid.tsx` (generated). The "any printable
character starts editing" rule is implemented by binding `onKeyDown` on
the root `<table>` element with bounds checking.

---

## 9. Column label conventions

For the v1 spreadsheet, columns are A–Z (26 columns). Beyond Z requires
two-letter labels (AA, AB, …) which we treat as a future increment.

```typescript
// src/app/util.ts

export function colLabel(col: number): string {
  return String.fromCharCode(65 + col);   // 0→"A", 1→"B", ..., 25→"Z"
}

export function cellLabel(row: number, col: number): string {
  return `${colLabel(col)}${row + 1}`;     // (0,0)→"A1", (1,2)→"C2"
}

export function cellKey(row: number, col: number): string {
  return cellLabel(row, col);              // identity for now; matches Excel address format
}
```

| `(row, col)` | `cellLabel` | `cellKey` |
|---|---|---|
| (0, 0)   | `"A1"`  | `"A1"`  |
| (0, 1)   | `"B1"`  | `"B1"`  |
| (5, 2)   | `"C6"`  | `"C6"`  |
| (99, 25) | `"Z100"`| `"Z100"`|

---

## 10. Demo app directory structure

```
demo/visicalc/
  package.json
  tsconfig.json
  vite.config.ts
  index.html
  mosaic/
    Grid.mil
    Grid.desktop.mll
    Grid.dark.msl
    FormulaBar.mil
    FormulaBar.desktop.mll
    FormulaBar.dark.msl
    visicalc-tokens.msl
  pipelines/
    visicalc-desktop-dark.mospipeline
  src/
    components/             ← generated by mosaic-compile; do not edit
      Grid.tsx
      GridEvent.ts
      FormulaBar.tsx
      FormulaBarEvent.ts
    app/
      state.ts              ← AppState + initialState + reducer
      util.ts               ← colLabel, cellLabel, cellKey, buildViewportRows
      App.tsx               ← VisiCalcApp component
      main.tsx              ← React 18 root mount
```

| File                       | Owner   | Notes |
|---|---|---|
| `mosaic/*.mil`             | author  | Component interfaces |
| `mosaic/*.mll`             | author  | Layouts (desktop variant) |
| `mosaic/*.msl`             | author  | Styles (dark variant) |
| `pipelines/*.mospipeline`  | author  | Wires interfaces+layouts+styles into a build |
| `src/components/*.tsx`     | generator | Output of `mosaic-compile`. Do not edit. Regenerated each build. |
| `src/app/*.ts(x)`          | author  | Host application logic |

The build step is `mosaic-compile --pipeline pipelines/visicalc-desktop-dark.mospipeline`,
run before `vite build` (or via a Vite plugin).

---

## 11. Out of scope

The following are deliberately deferred from this spec so that the UI
work can land cleanly without coupling to unrelated systems:

- **Formula evaluation** — cell values are stored and displayed as raw
  strings. Adding a real expression evaluator (`=A1+B1` resolving to
  `42`) is being developed on a separate parallel track. When that work
  lands, only `buildViewportRows` and the `editCommit` case of the
  reducer need to change to consult the engine; nothing else in this
  spec is affected.
- **Multi-sheet workbooks** — one sheet only in v1.
- **Cell formatting** — bold, italic, alignment, number formats, font.
- **Column resize via mouse drag**, row height customisation.
- **Copy / cut / paste**, undo / redo.
- **Range selection** — `onSelect` is wired but the reducer collapses
  it to a single-cell selection. A future version handles multi-cell
  ranges with a `selection` slot of richer type.
- **Mobile layout variant** (`Grid.mobile.mll` and friends). The
  pipeline system from UI23 makes adding a second layout variant a
  matter of writing one more `.mll` and one more `.mospipeline` — no
  changes to the components or interfaces.
- **Two-letter column labels** (AA, AB, …) beyond Z.
- **Persistence** — saving and loading cell data.
- **Collaboration / CRDT / real-time editing**.

---

## 12. Relationship to other specs

| Spec | Role in this suite |
|---|---|
| **UI13 mosmodel** | Interface declaration language; defines the `slot` / `emit` syntax used in §2. |
| **UI14 moslayout** | Layout primitive vocabulary; §3 uses `Row`, `Text`, and the `Grid` primitive. |
| **UI15 mosstyle** | Style declaration language; §4 uses tokens, states, and the cascade. |
| **UI20 mosaic-emit-react** | React backend; produces §6's TSX. |
| **UI22 mosaic-emit-paint** | Not used here; preview-only backend. |
| **UI23 mosaic-pipeline** | Versioned named pipelines; §5 is a concrete example. |
| **UI24 mosaic-emit-dispatch** | The Flux dispatch pattern used by both generated components. |
| **UI25 moslayout-input** | The `Input` primitive used by FormulaBar in §3.2. |

Once UI24 and UI25 land in code, building this VisiCalc demo is a
matter of writing the seven source files in §2–§4, the manifest in §5,
and the host code in §7. Everything in `src/components/` is generated.
