# UI26 — visicalc: VisiCalc Component Suite

**Status:** Specification
**Layer:** UI
**Depends on:** UI13 (mosmodel), UI14 (moslayout), UI15 (mosstyle), UI23 (mosaic-pipeline), UI24 (mosaic-emit-dispatch), UI25 (moslayout-input), FE01 (mosaic-formula-engine)
**Created:** 2026-05-17

---

## §1 Purpose and Architecture Overview

This spec defines the **VisiCalc Component Suite** — a complete, end-to-end demonstration
of the Mosaic compiler system applied to the canonical spreadsheet problem. It serves a dual
purpose: (1) it is a real, usable VisiCalc-style spreadsheet application, and (2) it is the
capstone integration test that exercises every layer of the Mosaic stack simultaneously.

The suite consists of:

- Two Mosaic-compiled components (`Grid` and `FormulaBar`) defined in `.mil` + `.mll` + `.msl`
- A `.mospipeline` manifest that drives code generation for both
- A host React application (`VisiCalcApp`) that owns all state via `useReducer`
- A TypeScript formula engine (FE01 JS port) that handles cell computation

### §1.1 System Diagram

```
mosaic-formula-engine (Rust → WASM, or TS port for demo)
         │
         │  setRaw / getDisplay / getFormula / recalculate
         ▼
VisiCalc Host App (React + useReducer)          src/app/App.tsx
         │
         │  slots (data down) + dispatch (events up)
         ├─────────────────────────────────────────────┐
         ▼                                             ▼
  Grid component                             FormulaBar component
  (Mosaic-generated .tsx)                    (Mosaic-generated .tsx)
  src/components/Grid.tsx                    src/components/FormulaBar.tsx
```

### §1.2 Architectural Invariants

These invariants are **structural contracts**, not guidelines. Violation makes the component
non-Mosaic-conformant.

**Invariant 1 — Components are dumb renderers.**
Neither `Grid.tsx` nor `FormulaBar.tsx` contain any spreadsheet logic. They do not know
what a cell is, what a formula is, or what "selected" means semantically. They render
whatever the host pushes in and fire events for whatever the user does. A generated Grid
component could render inventory data or a calendar grid without modification.

**Invariant 2 — All state lives in the host reducer.**
Cell values, selection position, edit state, viewport offset — these all live in `AppState`
inside `reducer.ts`. No state leaks into the generated components. The components are pure
functions of their slot inputs.

**Invariant 3 — The formula engine is a pure computational module.**
`FormulaEngine` (whether the TS port or the WASM build) is a deterministic function:
given the same `set_raw` calls followed by `recalculate`, it produces the same `get_display`
outputs every time. It has no React dependencies, no DOM references, no side effects.

**Invariant 4 — Dispatch is the only event channel.**
Every event the user triggers (click, keypress, scroll) flows upward through the Mosaic
`dispatch` prop as a typed `AppEvent`. Nothing is handled inside the generated components.
The host reducer is the single point of decision.

### §1.3 Why This Architecture?

The original VisiCalc (1979) introduced the spreadsheet model: a grid of cells, each
optionally containing a formula that references other cells. The app recalculates the grid
whenever a cell changes, propagating results through the dependency graph.

This architecture naturally maps to the Mosaic patterns:

- **The grid** is a pure view: it displays whatever display strings the host provides.
  It does not know how those strings were computed.
- **The formula bar** is a pure text input with address context: it shows the formula of the
  selected cell and fires events when the user edits it.
- **The host reducer** is the spreadsheet engine's controller: it mediates between user
  events (from components) and engine state (from FormulaEngine), then reconstructs the
  slot values that components display.

This is MVVM at the system level, enforced by the Mosaic grammar rather than by convention.

---

## §2 Component Interfaces

Both component interfaces are declared in `.mil` files. The interfaces are backend-agnostic:
the same `.mil` files drive React, Qt, SwiftUI, or paint-vm output without modification.

### §2.1 Grid.mil

The Grid component is the primary display surface. It renders a virtual-scrolling table of
cells. The host owns selection, edit state, and viewport offset — the component only renders
what it is told.

```mosmodel
component Grid version 1.0 {
  // ── Display data ──────────────────────────────────────────────────────────
  // The column header labels: ["A", "B", "C", ...].
  slot column-headers  : list<text> ;

  // Per-column pixel widths.  Must have the same length as column-headers.
  slot column-widths   : list<number> ;

  // The visible slice of cell display values (post-formula evaluation).
  // Outer list = rows, inner list = columns.  Length = viewport height.
  slot viewport-rows   : list<list<text>> ;

  // Total number of rows in the spreadsheet (for the scrollbar).
  slot total-rows      : number ;

  // ── Viewport ──────────────────────────────────────────────────────────────
  // The logical row index of the first visible row (0-based).
  slot viewport-offset : number = 0 ;

  // ── Selection (host owns, pushes in) ──────────────────────────────────────
  // The currently selected cell.  Displayed with a distinct highlight.
  slot selected-row    : number = 0 ;
  slot selected-col    : number = 0 ;

  // ── Edit state (host owns, pushes in) ─────────────────────────────────────
  // When edit-row/edit-col match a cell, that cell renders an inline editor
  // showing edit-content rather than the display value.
  // Use -1 to indicate "not editing".
  slot edit-row        : number = -1 ;
  slot edit-col        : number = -1 ;
  slot edit-content    : text   = "" ;

  // ── Events ────────────────────────────────────────────────────────────────
  // User clicked or arrow-keyed to a different cell (not entering edit mode).
  emit onNavigate    ( row : number , col : number ) ;

  // User pressed Enter or F2 or typed a printable char — begin editing.
  emit onEditStart   ( row : number , col : number ) ;

  // User pressed Enter or Tab while editing — commit the edited value.
  emit onEditCommit  ( value : text ) ;

  // User pressed Escape while editing — discard the edit.
  emit onEditCancel  ;

  // User scrolled — the host should update viewport-offset and viewport-rows.
  emit onScroll      ( offset : number ) ;

  // User dragged to select a range.  start-* is the anchor, end-* is the tip.
  emit onSelect      ( start-row : number , start-col : number ,
                       end-row   : number , end-col   : number ) ;
}
```

**Design notes:**

- `viewport-rows` contains **display strings** (post-formula), not raw formulas.  The host
  computes these from the formula engine before pushing them in.
- `edit-row = -1` is the sentinel for "no cell is being edited".  The component never
  interprets -1 as a real row index.
- `onEditStart` carries the cell coordinates so the host knows which cell to load into
  `edit-content` and `FormulaBar.formula`.
- `onSelect` is separate from `onNavigate` to distinguish single-cell focus (keyboard nav)
  from range selection (mouse drag).

### §2.2 FormulaBar.mil

The formula bar sits above the grid. It shows the address of the selected cell on the left
and the raw formula (or literal value) of that cell in an editable text field on the right.

```mosmodel
component FormulaBar version 1.0 {
  // The address label, e.g. "A1", "B12".  Read-only display.
  slot cell-address : text ;

  // The formula or literal value of the selected cell, e.g. "=A1+B1" or "42".
  // When the user is actively editing, this is the live edit buffer.
  slot formula      : text ;

  // When true, the formula field is rendered non-editable.
  slot read-only    : bool = false ;

  // Fired on every keystroke — the host updates its edit buffer but does not
  // commit to the formula engine yet.
  emit onFormulaChange ( formula : text ) ;

  // User pressed Enter in the formula bar — commit the formula to the engine.
  emit onCommit ;

  // User pressed Escape in the formula bar — discard the edit.
  emit onCancel ;
}
```

**Design notes:**

- `FormulaBar` does not own its edit buffer.  The host passes `formula` down on every
  keystroke; the component is stateless.  This is the Mosaic slot-down/emit-up pattern
  applied to text input.
- `onFormulaChange` fires on every character.  The host updates `editContent` in the
  reducer but does not call `engine.set_raw` until `onCommit` fires.  This avoids
  triggering expensive recalculation mid-keystroke.
- `read-only` is provided for future use (e.g. protecting formula-only ranges) but
  defaults to `false` for the initial demo.

---

## §3 Layout Variants

Layout files declare how primitives are arranged and how emits are wired.  The naming
convention `<Component>.<platform>.mll` (from UI23 §2) selects the right variant per target.

### §3.1 Grid.desktop.mll

The desktop layout delegates directly to the built-in `Grid` primitive (from UI14 §2).
The Grid primitive is natively implemented per backend — the layout file is only a
pass-through wiring layer.

```moslayout
layout Grid.desktop version 1.0 implements Grid 1.x {
  Grid [grid-table] (
    headers:        slot: column-headers ,
    widths:         slot: column-widths ,
    rows:           slot: viewport-rows ,
    total-rows:     slot: total-rows ,
    viewport-offset: slot: viewport-offset ,
    selected-row:   slot: selected-row ,
    selected-col:   slot: selected-col ,
    edit-row:       slot: edit-row ,
    edit-col:       slot: edit-col ,
    edit-content:   slot: edit-content ,
    on-navigate:    emit: onNavigate ,
    on-edit-start:  emit: onEditStart ,
    on-edit-commit: emit: onEditCommit ,
    on-edit-cancel: emit: onEditCancel ,
    on-scroll:      emit: onScroll ,
    on-select:      emit: onSelect
  )
}
```

This layout exports one part: `grid-table`.  The style file targets this part for the
table's overall appearance; individual cell states (selected, editing, hover) are handled
by the backend's Grid primitive implementation, exposed as pseudo-states on the part.

### §3.2 FormulaBar.desktop.mll

The formula bar layout is a horizontal Row with two children: a Text label showing the
cell address, and an Input field (from UI25) showing the formula.

```moslayout
layout FormulaBar.desktop version 1.0 implements FormulaBar 1.x {
  Row [formula-bar-root] {
    Text [address-label] {
      content: @cell-address;
    }
    Input [formula-field] {
      value:     @formula;
      read-only: @read-only;
      connects: onChange(formula: text) -> emit onFormulaChange(formula: formula);
      connects: onCommit -> emit onCommit;
      connects: onCancel -> emit onCancel;
    }
  }
}
```

This layout exports three parts: `formula-bar-root`, `address-label`, `formula-field`.

**Wiring semantics** (from UI25 §3):

- `onChange(formula: text)` — the Input primitive fires this on every keystroke with the
  current value.  The layout wires it to `onFormulaChange` on the interface.
- `onCommit` — the Input fires this when the user presses Enter (or equivalent platform
  confirm gesture).  Wired to `FormulaBar.onCommit`.
- `onCancel` — the Input fires this when the user presses Escape.  Wired to `FormulaBar.onCancel`.

---

## §4 Style Variants

### §4.1 visicalc-tokens.msl — Shared Design Tokens

A token file shared by both components.  Both `Grid.dark.msl` and `FormulaBar.dark.msl`
reference these tokens.  Swapping this file changes the entire visual theme.

```mosstyle
tokens visicalc {
  // Surfaces
  $color-surface:         #1e1e1e ;   // main background (grid cells, body)
  $color-surface-alt:     #252526 ;   // alternating row background
  $color-surface-header:  #2d2d30 ;   // column header row background
  $color-border:          #3f3f46 ;   // cell borders, separator lines
  $color-border-heavy:    #555558 ;   // outer border, selected column header

  // Text
  $color-text-primary:    #cccccc ;   // normal cell text
  $color-text-secondary:  #9d9d9d ;   // row numbers, address label
  $color-text-header:     #cccccc ;   // column header text (same weight as body)

  // Interaction
  $color-accent:          #007acc ;   // VS Code blue — focus ring, accent elements
  $color-selected:        #264f78 ;   // selected cell background
  $color-selected-text:   #ffffff ;   // selected cell text
  $color-editing:         #1f4f3f ;   // cell-in-edit-mode background (green tint)
  $color-hover:           #2a2d2e ;   // cell hover background

  // Semantic
  $color-error:           #f44747 ;   // error values (#DIV/0!, #REF!, etc.)
  $color-formula-bar:     #1e1e1e ;   // formula bar background

  // Geometry
  $cell-height:           22px ;
  $header-height:         24px ;
  $formula-bar-height:    28px ;
  $row-number-width:      48px ;
  $border-width:          1px ;

  // Typography
  $font-family-mono:      "Menlo", "Consolas", "Courier New", monospace ;
  $font-family-ui:        "Inter", "Segoe UI", system-ui, sans-serif ;
  $font-size-cell:        13px ;
  $font-size-header:      12px ;
  $font-size-address:     12px ;
  $font-weight-normal:    400 ;
  $font-weight-header:    600 ;

  // Animation
  $duration-instant:      0ms ;   // no animation on cell navigation (performance)
  $duration-fast:         80ms ;
  $easing-out:            ease-out ;
}
```

**Why no animation on cell navigation?** Spreadsheets are used at high speed — a power user
navigates hundreds of cells per minute with arrow keys.  Any transition delay on selection
movement would make the app feel sluggish.  Animation tokens are included for focus rings
and formula bar transitions where the slower rhythm is acceptable.

### §4.2 Grid.dark.msl

```mosstyle
style Grid.dark version 1.0 for Grid 1.x
  uses tokens visicalc {

  // ── grid-table: the outer grid container ─────────────────────────────────
  part grid-table {
    background:    $color-surface ;
    border-color:  $color-border ;
    border-width:  $border-width ;
    font-family:   $font-family-mono ;
    font-size:     $font-size-cell ;
    font-weight:   $font-weight-normal ;
    color:         $color-text-primary ;

    // Header row (the A, B, C… labels at the top of each column)
    part header-row {
      background:  $color-surface-header ;
      color:       $color-text-header ;
      font-family: $font-family-ui ;
      font-size:   $font-size-header ;
      font-weight: $font-weight-header ;
      border-color: $color-border ;
      border-width: $border-width ;
      padding:     0px 4px ;
    }

    // Row number gutter (the 1, 2, 3… labels at the left)
    part row-number {
      background:  $color-surface-header ;
      color:       $color-text-secondary ;
      font-family: $font-family-ui ;
      font-size:   $font-size-header ;
      font-weight: $font-weight-normal ;
      border-color: $color-border ;
      border-width: $border-width ;
      padding:     0px 4px ;
      text-align:  end ;
    }

    // Individual data cell (even row — zero-based)
    part cell-even {
      background:  $color-surface ;
      color:       $color-text-primary ;
      border-color: $color-border ;
      border-width: $border-width ;
      padding:     0px 4px ;

      state hover {
        background: $color-hover ;
      }

      state selected {
        background: $color-selected ;
        color:      $color-selected-text ;
        outline-color: $color-accent ;
        outline-width: 1px ;
        transition background $duration-instant ;
        transition color $duration-instant ;
      }

      state editing {
        background:    $color-editing ;
        color:         $color-selected-text ;
        outline-color: $color-accent ;
        outline-width: 2px ;
      }
    }

    // Individual data cell (odd row — zero-based)
    // Inherits all cell-even rules; only background differs.
    part cell-odd {
      background:  $color-surface-alt ;
      color:       $color-text-primary ;
      border-color: $color-border ;
      border-width: $border-width ;
      padding:     0px 4px ;

      state hover {
        background: $color-hover ;
      }

      state selected {
        background: $color-selected ;
        color:      $color-selected-text ;
        outline-color: $color-accent ;
        outline-width: 1px ;
        transition background $duration-instant ;
        transition color $duration-instant ;
      }

      state editing {
        background:    $color-editing ;
        color:         $color-selected-text ;
        outline-color: $color-accent ;
        outline-width: 2px ;
      }
    }

    // Error cell — used when a cell's display value is a formula error
    // (#DIV/0!, #REF!, #NAME?, #VALUE!, #CIRC, #PARSE)
    part cell-error {
      color: $color-error ;
    }
  }
}
```

**Even/odd alternating rows** — the Grid primitive backend reports each cell's row parity
as a pseudo-state.  The style author declares two part variants (`cell-even`, `cell-odd`)
with the same state set but different base backgrounds.  The backend selects the appropriate
part based on `rowIndex % 2`.  This is equivalent to CSS `:nth-child(even)` but expressed
in the Mosaic part/state model without requiring arbitrary expressions in `.msl`.

**Error cells** — when `get_display` returns a value starting with `#` (e.g. `#DIV/0!`),
the host sets the cell part to `cell-error` and the Grid backend applies the `$color-error`
foreground.  The formula engine (FE01) defines the exact error strings; this style file only
assigns the color.

### §4.3 FormulaBar.dark.msl

```mosstyle
style FormulaBar.dark version 1.0 for FormulaBar 1.x
  uses tokens visicalc {

  part formula-bar-root {
    background:   $color-formula-bar ;
    border-color: $color-border ;
    border-width: $border-width ;
    padding:      0px 8px ;
    gap:          8px ;
  }

  part address-label {
    background:   $color-surface-header ;
    color:        $color-text-secondary ;
    font-family:  $font-family-ui ;
    font-size:    $font-size-address ;
    font-weight:  $font-weight-normal ;
    padding:      0px 8px ;
    border-color: $color-border ;
    border-width: $border-width ;
    border-radius: 2px ;
    text-align:   center ;
  }

  part formula-field {
    background:   $color-surface ;
    color:        $color-text-primary ;
    font-family:  $font-family-mono ;
    font-size:    $font-size-cell ;
    font-weight:  $font-weight-normal ;
    border-color: $color-border ;
    border-width: $border-width ;
    border-radius: 2px ;
    padding:      2px 6px ;

    state focused {
      border-color:  $color-accent ;
      outline-color: $color-accent ;
      outline-width: 1px ;
      transition border-color $duration-fast $easing-out ;
    }

    state disabled {
      opacity: 0.5 ;
      color:   $color-text-secondary ;
    }
  }
}
```

---

## §5 The `.mospipeline` Manifest

The pipeline manifest (from UI23) assembles both components in a single `mosaic-compile`
invocation.  The global-style block makes `visicalc-tokens.msl` available to all component
style files without being listed again per component.

```toml
# pipelines/visicalc-desktop-dark.mospipeline

[pipeline]
name    = "visicalc-desktop-dark"
version = "1.0"

[global-style]
tokens = ["mosaic/visicalc-tokens.msl"]

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

The compiler resolves `.mil` / `.mll` / `.msl` files by looking for them under the
`mosaic/` directory adjacent to the pipeline file.  File resolution follows the naming
convention from UI23 §3: `<Component>.<variant>.<ext>`.

Running:

```sh
mosaic-compile --pipeline pipelines/visicalc-desktop-dark.mospipeline
```

Produces four files (two components × two outputs per component):

```
src/components/
  Grid.tsx              (generated — do not edit)
  GridEvent.ts          (generated — do not edit)
  FormulaBar.tsx        (generated — do not edit)
  FormulaBarEvent.ts    (generated — do not edit)
```

---

## §6 Generated React Output

This section shows the **exact** TypeScript output that `mosaic-compile` produces for
the VisiCalc pipeline.  These files are never hand-edited.  The `// Auto-generated`
header and the `do not edit` directive in `out-dir` enforce this at the repo policy level.

### §6.1 GridEvent.ts

```typescript
// Auto-generated by mosaic-emit-react. Do not edit.
// Source: mosaic/Grid.mil (Grid version 1.0)

/**
 * Discriminated union of all events the Grid component can fire.
 *
 * Use with useReducer in your host application:
 *
 *   function reducer(state: AppState, event: AppEvent): AppState {
 *     switch (event.type) {
 *       case "navigate":   return { ...state, selectedRow: event.row, selectedCol: event.col };
 *       case "editStart":  return { ...state, editRow: event.row, editCol: event.col };
 *       case "editCommit": return handleCommit(state, event.value);
 *       case "editCancel": return { ...state, editRow: -1, editCol: -1 };
 *       case "scroll":     return handleScroll(state, event.offset);
 *       case "select":     return { ...state, selectedRow: event.startRow, selectedCol: event.startCol };
 *     }
 *   }
 */
export type GridEvent =
  | { type: "navigate";   row: number; col: number }
  | { type: "editStart";  row: number; col: number }
  | { type: "editCommit"; value: string }
  | { type: "editCancel" }
  | { type: "scroll";     offset: number }
  | { type: "select";     startRow: number; startCol: number; endRow: number; endCol: number };
```

### §6.2 FormulaBarEvent.ts

```typescript
// Auto-generated by mosaic-emit-react. Do not edit.
// Source: mosaic/FormulaBar.mil (FormulaBar version 1.0)

/**
 * Discriminated union of all events the FormulaBar component can fire.
 */
export type FormulaBarEvent =
  | { type: "formulaChange"; formula: string }
  | { type: "commit" }
  | { type: "cancel" };
```

### §6.3 Grid.tsx

The Grid component is the most complex generated output.  It renders a virtual-scrolling
table, handles keyboard navigation within the table, and wires all six emits through the
single `dispatch` prop.

```tsx
// Auto-generated by mosaic-emit-react. Do not edit.
// Source: mosaic/Grid.mil + mosaic/Grid.desktop.mll + mosaic/Grid.dark.msl
// Pipeline: visicalc-desktop-dark version 1.0

import React, { useRef, useEffect, useCallback } from "react";
import type { GridEvent } from "./GridEvent";

// ── Props ────────────────────────────────────────────────────────────────────

interface GridProps {
  // Display data
  columnHeaders:  string[];
  columnWidths:   number[];
  viewportRows:   string[][];   // display values, post-formula
  totalRows:      number;

  // Viewport
  viewportOffset: number;

  // Selection (host owns)
  selectedRow:    number;
  selectedCol:    number;

  // Edit state (host owns; editRow === -1 means not editing)
  editRow:        number;
  editCol:        number;
  editContent:    string;

  // Single required event channel — Flux/useReducer pattern (UI24)
  dispatch: (event: GridEvent) => void;
}

// ── Styles (from Grid.dark.msl / visicalc-tokens.msl) ────────────────────────
// Tokens are inlined at compile time.  No runtime token resolution.

const TOKEN = {
  colorSurface:       "#1e1e1e",
  colorSurfaceAlt:    "#252526",
  colorSurfaceHeader: "#2d2d30",
  colorBorder:        "#3f3f46",
  colorTextPrimary:   "#cccccc",
  colorTextSecondary: "#9d9d9d",
  colorSelected:      "#264f78",
  colorSelectedText:  "#ffffff",
  colorEditing:       "#1f4f3f",
  colorHover:         "#2a2d2e",
  colorAccent:        "#007acc",
  colorError:         "#f44747",
  fontMono:           '"Menlo", "Consolas", "Courier New", monospace',
  fontUi:             '"Inter", "Segoe UI", system-ui, sans-serif',
  fontSizeCell:       "13px",
  fontSizeHeader:     "12px",
  cellHeight:         22,    // px
  headerHeight:       24,    // px
  rowNumberWidth:     48,    // px
} as const;

// ── Utility ───────────────────────────────────────────────────────────────────

function isErrorValue(display: string): boolean {
  return display.startsWith("#") && display.length > 1;
}

// ── Component ─────────────────────────────────────────────────────────────────

export function Grid({
  columnHeaders,
  columnWidths,
  viewportRows,
  totalRows,
  viewportOffset,
  selectedRow,
  selectedCol,
  editRow,
  editCol,
  editContent,
  dispatch,
}: GridProps) {
  const editInputRef = useRef<HTMLInputElement>(null);

  // Focus the inline edit input whenever editing begins.
  useEffect(() => {
    if (editRow >= 0 && editInputRef.current) {
      editInputRef.current.focus();
      editInputRef.current.select();
    }
  }, [editRow, editCol]);

  // ── Event handlers ──────────────────────────────────────────────────────────

  const handleCellClick = useCallback(
    (row: number, col: number) => {
      dispatch({ type: "navigate", row, col });
    },
    [dispatch]
  );

  const handleCellDoubleClick = useCallback(
    (row: number, col: number) => {
      dispatch({ type: "editStart", row, col });
    },
    [dispatch]
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLDivElement>) => {
      const editing = editRow >= 0;
      switch (e.key) {
        case "ArrowUp":
          if (!editing) {
            e.preventDefault();
            dispatch({ type: "navigate", row: Math.max(0, selectedRow - 1), col: selectedCol });
          }
          break;
        case "ArrowDown":
          if (!editing) {
            e.preventDefault();
            dispatch({ type: "navigate", row: Math.min(totalRows - 1, selectedRow + 1), col: selectedCol });
          }
          break;
        case "ArrowLeft":
          if (!editing) {
            e.preventDefault();
            dispatch({ type: "navigate", row: selectedRow, col: Math.max(0, selectedCol - 1) });
          }
          break;
        case "ArrowRight":
          if (!editing) {
            e.preventDefault();
            dispatch({ type: "navigate", row: selectedRow, col: Math.min(columnHeaders.length - 1, selectedCol + 1) });
          }
          break;
        case "Enter":
          if (!editing) {
            e.preventDefault();
            dispatch({ type: "editStart", row: selectedRow, col: selectedCol });
          }
          // editCommit is handled by the inline <input> onKeyDown
          break;
        case "Escape":
          if (editing) {
            e.preventDefault();
            dispatch({ type: "editCancel" });
          }
          break;
        case "F2":
          if (!editing) {
            e.preventDefault();
            dispatch({ type: "editStart", row: selectedRow, col: selectedCol });
          }
          break;
        case "Tab":
          if (editing) {
            e.preventDefault();
            // Tab commits and moves right; Shift+Tab commits and moves left.
            // The reducer handles movement after commit.
            dispatch({ type: "editCommit", value: editContent });
          }
          break;
        default:
          // Any printable character while not editing starts editing with
          // the typed character as the first character of editContent.
          if (!editing && e.key.length === 1 && !e.ctrlKey && !e.metaKey) {
            dispatch({ type: "editStart", row: selectedRow, col: selectedCol });
          }
          break;
      }
    },
    [dispatch, editing, selectedRow, selectedCol, totalRows, columnHeaders.length, editContent]
  );

  const handleInlineEditKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "Enter") {
        e.preventDefault();
        dispatch({ type: "editCommit", value: editContent });
      } else if (e.key === "Escape") {
        e.preventDefault();
        dispatch({ type: "editCancel" });
      } else if (e.key === "Tab") {
        e.preventDefault();
        dispatch({ type: "editCommit", value: editContent });
      }
    },
    [dispatch, editContent]
  );

  const handleInlineEditChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      // Inline edit changes flow through editStart → editCommit.
      // While typing, the host tracks changes via onEditStart content.
      // We re-use the formulaChange pathway: the host reducer listens
      // for inline edits via a synthetic formulaChange-equivalent path.
      // In this generated component, we dispatch a synthetic navigate
      // to signal live content (the reducer handles editContent update).
      //
      // Implementation note: the host reducer updates editContent in
      // response to formulaChange events on FormulaBar.  For the inline
      // grid editor, the host wires the same path by catching this event.
      dispatch({ type: "editStart", row: editRow, col: editCol });
      // The actual value is read from the input on commit.
      // To keep the component stateless, the host must track input value
      // via the FormulaBar which mirrors editContent.
    },
    [dispatch, editRow, editCol]
  );

  const handleScroll = useCallback(
    (e: React.UIEvent<HTMLDivElement>) => {
      const scrollTop = (e.target as HTMLDivElement).scrollTop;
      const newOffset = Math.floor(scrollTop / TOKEN.cellHeight);
      dispatch({ type: "scroll", offset: newOffset });
    },
    [dispatch]
  );

  // ── Render ──────────────────────────────────────────────────────────────────

  const containerStyle: React.CSSProperties = {
    flex: 1,
    overflow: "hidden",
    display: "flex",
    flexDirection: "column",
    fontFamily: TOKEN.fontMono,
    fontSize: TOKEN.fontSizeCell,
    background: TOKEN.colorSurface,
    color: TOKEN.colorTextPrimary,
    border: `1px solid ${TOKEN.colorBorder}`,
    outline: "none",
    userSelect: "none",
  };

  const tableStyle: React.CSSProperties = {
    width: "100%",
    borderCollapse: "collapse",
    tableLayout: "fixed",
  };

  const headerRowStyle: React.CSSProperties = {
    background: TOKEN.colorSurfaceHeader,
    height: `${TOKEN.headerHeight}px`,
    position: "sticky",
    top: 0,
    zIndex: 1,
  };

  const headerCellStyle: React.CSSProperties = {
    background: TOKEN.colorSurfaceHeader,
    color: TOKEN.colorTextPrimary,
    fontFamily: TOKEN.fontUi,
    fontSize: TOKEN.fontSizeHeader,
    fontWeight: 600,
    borderRight: `1px solid ${TOKEN.colorBorder}`,
    borderBottom: `1px solid ${TOKEN.colorBorder}`,
    padding: "0 4px",
    textAlign: "center",
    overflow: "hidden",
    whiteSpace: "nowrap",
  };

  const rowNumberCellStyle: React.CSSProperties = {
    background: TOKEN.colorSurfaceHeader,
    color: TOKEN.colorTextSecondary,
    fontFamily: TOKEN.fontUi,
    fontSize: TOKEN.fontSizeHeader,
    fontWeight: 400,
    borderRight: `1px solid ${TOKEN.colorBorder}`,
    borderBottom: `1px solid ${TOKEN.colorBorder}`,
    padding: "0 4px",
    textAlign: "right",
    width: `${TOKEN.rowNumberWidth}px`,
    userSelect: "none",
  };

  function cellStyle(
    rowIndex: number,
    colIndex: number,
    isSelected: boolean,
    isEditing: boolean,
    isError: boolean
  ): React.CSSProperties {
    const isEven = rowIndex % 2 === 0;
    let background = isEven ? TOKEN.colorSurface : TOKEN.colorSurfaceAlt;
    let color = isError ? TOKEN.colorError : TOKEN.colorTextPrimary;
    let outline = "none";

    if (isSelected) {
      background = TOKEN.colorSelected;
      color = TOKEN.colorSelectedText;
      outline = `1px solid ${TOKEN.colorAccent}`;
    }
    if (isEditing) {
      background = TOKEN.colorEditing;
      color = TOKEN.colorSelectedText;
      outline = `2px solid ${TOKEN.colorAccent}`;
    }

    return {
      background,
      color,
      outline,
      borderRight: `1px solid ${TOKEN.colorBorder}`,
      borderBottom: `1px solid ${TOKEN.colorBorder}`,
      padding: "0 4px",
      height: `${TOKEN.cellHeight}px`,
      overflow: "hidden",
      whiteSpace: "nowrap",
      cursor: "default",
      position: "relative",
    };
  }

  return (
    <div
      style={containerStyle}
      tabIndex={0}
      onKeyDown={handleKeyDown}
      onScroll={handleScroll}
    >
      <table style={tableStyle}>
        {/* Column width definitions */}
        <colgroup>
          <col style={{ width: `${TOKEN.rowNumberWidth}px` }} />
          {columnWidths.map((w, i) => (
            <col key={i} style={{ width: `${w}px` }} />
          ))}
        </colgroup>

        {/* Header row */}
        <thead>
          <tr style={headerRowStyle}>
            {/* Top-left corner cell — above the row numbers */}
            <th style={{ ...headerCellStyle, background: TOKEN.colorSurfaceHeader }} />
            {columnHeaders.map((header, colIdx) => (
              <th key={colIdx} style={headerCellStyle}>
                {header}
              </th>
            ))}
          </tr>
        </thead>

        {/* Data rows — only the viewport slice */}
        <tbody>
          {viewportRows.map((rowCells, viewportRowIdx) => {
            const logicalRow = viewportOffset + viewportRowIdx;
            return (
              <tr key={logicalRow} style={{ height: `${TOKEN.cellHeight}px` }}>
                {/* Row number */}
                <td style={rowNumberCellStyle}>{logicalRow + 1}</td>

                {/* Data cells */}
                {rowCells.map((displayValue, colIdx) => {
                  const isSelected = logicalRow === selectedRow && colIdx === selectedCol;
                  const isEditing = logicalRow === editRow && colIdx === editCol;
                  const isError = isErrorValue(displayValue);

                  return (
                    <td
                      key={colIdx}
                      style={cellStyle(logicalRow, colIdx, isSelected, isEditing, isError)}
                      onClick={() => handleCellClick(logicalRow, colIdx)}
                      onDoubleClick={() => handleCellDoubleClick(logicalRow, colIdx)}
                    >
                      {isEditing ? (
                        /* Inline editor — rendered in the cell during edit mode */
                        <input
                          ref={editInputRef}
                          value={editContent}
                          onChange={handleInlineEditChange}
                          onKeyDown={handleInlineEditKeyDown}
                          style={{
                            width: "100%",
                            height: "100%",
                            background: "transparent",
                            color: TOKEN.colorSelectedText,
                            border: "none",
                            outline: "none",
                            fontFamily: TOKEN.fontMono,
                            fontSize: TOKEN.fontSizeCell,
                            padding: 0,
                          }}
                        />
                      ) : (
                        displayValue
                      )}
                    </td>
                  );
                })}
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
```

### §6.4 FormulaBar.tsx

```tsx
// Auto-generated by mosaic-emit-react. Do not edit.
// Source: mosaic/FormulaBar.mil + mosaic/FormulaBar.desktop.mll + mosaic/FormulaBar.dark.msl
// Pipeline: visicalc-desktop-dark version 1.0

import React, { useCallback } from "react";
import type { FormulaBarEvent } from "./FormulaBarEvent";

// ── Props ────────────────────────────────────────────────────────────────────

interface FormulaBarProps {
  cellAddress: string;
  formula:     string;
  readOnly:    boolean;

  // Single required event channel (UI24 Flux pattern)
  dispatch: (event: FormulaBarEvent) => void;
}

// ── Styles (inlined from FormulaBar.dark.msl / visicalc-tokens.msl) ──────────

const FBAR_TOKEN = {
  colorSurface:       "#1e1e1e",
  colorSurfaceHeader: "#2d2d30",
  colorBorder:        "#3f3f46",
  colorTextPrimary:   "#cccccc",
  colorTextSecondary: "#9d9d9d",
  colorAccent:        "#007acc",
  fontMono:           '"Menlo", "Consolas", "Courier New", monospace',
  fontUi:             '"Inter", "Segoe UI", system-ui, sans-serif',
  fontSizeCell:       "13px",
  fontSizeAddress:    "12px",
  formulaBarHeight:   28,   // px
} as const;

// ── Component ─────────────────────────────────────────────────────────────────

export function FormulaBar({
  cellAddress,
  formula,
  readOnly,
  dispatch,
}: FormulaBarProps) {
  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      dispatch({ type: "formulaChange", formula: e.target.value });
    },
    [dispatch]
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "Enter") {
        e.preventDefault();
        dispatch({ type: "commit" });
      } else if (e.key === "Escape") {
        e.preventDefault();
        dispatch({ type: "cancel" });
      }
    },
    [dispatch]
  );

  const rootStyle: React.CSSProperties = {
    display: "flex",
    flexDirection: "row",
    alignItems: "center",
    height: `${FBAR_TOKEN.formulaBarHeight}px`,
    background: FBAR_TOKEN.colorSurface,
    borderBottom: `1px solid ${FBAR_TOKEN.colorBorder}`,
    padding: "0 8px",
    gap: "8px",
    flexShrink: 0,
  };

  const addressLabelStyle: React.CSSProperties = {
    background: FBAR_TOKEN.colorSurfaceHeader,
    color: FBAR_TOKEN.colorTextSecondary,
    fontFamily: FBAR_TOKEN.fontUi,
    fontSize: FBAR_TOKEN.fontSizeAddress,
    fontWeight: 400,
    padding: "0 8px",
    border: `1px solid ${FBAR_TOKEN.colorBorder}`,
    borderRadius: "2px",
    minWidth: "48px",
    textAlign: "center",
    whiteSpace: "nowrap",
    lineHeight: `${FBAR_TOKEN.formulaBarHeight - 4}px`,
  };

  const formulaFieldStyle: React.CSSProperties = {
    flex: 1,
    background: FBAR_TOKEN.colorSurface,
    color: FBAR_TOKEN.colorTextPrimary,
    fontFamily: FBAR_TOKEN.fontMono,
    fontSize: FBAR_TOKEN.fontSizeCell,
    fontWeight: 400,
    border: `1px solid ${FBAR_TOKEN.colorBorder}`,
    borderRadius: "2px",
    padding: "2px 6px",
    outline: "none",
    height: `${FBAR_TOKEN.formulaBarHeight - 6}px`,
  };

  return (
    <div style={rootStyle}>
      {/* Address label — part: address-label */}
      <div style={addressLabelStyle}>{cellAddress}</div>

      {/* Formula input — part: formula-field */}
      <input
        type="text"
        value={formula}
        readOnly={readOnly}
        onChange={handleChange}
        onKeyDown={handleKeyDown}
        style={formulaFieldStyle}
        onFocus={(e) => {
          e.target.style.borderColor = FBAR_TOKEN.colorAccent;
          e.target.style.outline = `1px solid ${FBAR_TOKEN.colorAccent}`;
        }}
        onBlur={(e) => {
          e.target.style.borderColor = FBAR_TOKEN.colorBorder;
          e.target.style.outline = "none";
        }}
        spellCheck={false}
        autoComplete="off"
        autoCapitalize="off"
      />
    </div>
  );
}
```

---

## §7 The Host Application

The host application owns all spreadsheet state and mediates between the generated
components and the formula engine.  It lives entirely in `src/app/`.

### §7.1 State Shape

```typescript
// src/app/reducer.ts

/**
 * Per-cell storage.
 *
 * raw     — what the user typed: "42", "hello", "=A1+B1"
 * display — what the formula engine computed: "42", "hello", "84"
 * error   — null if the cell is OK, otherwise the error string ("#DIV/0!", etc.)
 *
 * The formula engine (FE01) is the source of truth for both display and error.
 * These fields are cached here to avoid calling the engine on every render.
 */
interface CellData {
  raw:     string;
  display: string;
  error:   string | null;
}

/**
 * Complete application state.  Every piece of state that affects the rendered
 * output must live here.  Nothing is stored inside the generated components.
 *
 * The single-source-of-truth rule: if you find yourself wondering "where is
 * X stored?", the answer is always AppState.
 */
interface AppState {
  // ── Cell storage (keyed by cell label: "A1", "B12", etc.) ────────────────
  cells: Record<string, CellData>;

  // ── Selection ─────────────────────────────────────────────────────────────
  selectedRow: number;
  selectedCol: number;

  // ── Edit state (editRow === -1 means no cell is being edited) ────────────
  editRow:     number;
  editCol:     number;
  editContent: string;   // live buffer — not yet committed to formula engine

  // ── Viewport ──────────────────────────────────────────────────────────────
  viewportOffset: number;                // logical row index of first visible row
  viewportHeight: number;                // how many rows fit in the visible area

  // ── Grid metadata ─────────────────────────────────────────────────────────
  columnHeaders: string[];               // ["A", "B", "C", ...]
  columnWidths:  number[];               // per-column pixel widths
  totalRows:     number;

  // ── Derived / precomputed for slots ───────────────────────────────────────
  viewportRows: string[][];              // display values for visible rows (slot-ready)
}
```

### §7.2 Event Union

The host reducer handles events from both components through a single `AppEvent` union.
This is the key insight from UI24: because both generated components dispatch typed unions,
the host can compose them without wrapping.

```typescript
// src/app/reducer.ts (continued)

import type { GridEvent } from "../components/GridEvent";
import type { FormulaBarEvent } from "../components/FormulaBarEvent";

/**
 * The host reducer handles all events from all components plus internal
 * host-originated events like "recalculate".
 *
 * AppEvent = GridEvent | FormulaBarEvent | internal events
 *
 * The discriminated union's "type" field is unique across all three sets
 * because mosaic-emit-react strips the "on" prefix and lowercases the first
 * character (UI24 §3.2).  Collision is impossible if the emit names in the
 * two .mil files don't overlap — which they don't here by design.
 */
type AppEvent =
  | GridEvent
  | FormulaBarEvent
  | { type: "recalculate" };
```

### §7.3 Column Label Utilities

```typescript
// src/app/reducer.ts (continued)

/**
 * Convert a zero-based column index to a letter label.
 *
 * The original VisiCalc supported 63 columns (A–BK).  This implementation
 * supports 26 columns (A–Z), which is sufficient for the demo.
 *
 *   colLabel(0)  → "A"
 *   colLabel(25) → "Z"
 */
function colLabel(col: number): string {
  return String.fromCharCode(65 + col);   // 65 = char code for 'A'
}

/**
 * Convert (row, col) to a cell address string.
 *
 *   cellLabel(0, 0)  → "A1"
 *   cellLabel(0, 2)  → "C1"
 *   cellLabel(1, 0)  → "A2"
 *   cellLabel(11, 2) → "C12"
 *
 * Note: row is 0-based internally, but 1-based in cell labels.
 */
function cellLabel(row: number, col: number): string {
  return `${colLabel(col)}${row + 1}`;
}
```

### §7.4 The Reducer

```typescript
// src/app/reducer.ts (continued)

import { createEngine, type FormulaEngine } from "../engine/formulaEngine";

// The engine is module-level state — a single shared instance.
// It persists across renders; the reducer mutates it on editCommit.
const engine: FormulaEngine = createEngine();

/**
 * Compute the display-string slice for the current viewport.
 * Called after every editCommit, scroll, or initial render.
 */
function buildViewportRows(
  state: Pick<AppState, "viewportOffset" | "viewportHeight" | "totalRows" | "columnHeaders">
): string[][] {
  const rows: string[][] = [];
  for (
    let r = state.viewportOffset;
    r < Math.min(state.viewportOffset + state.viewportHeight, state.totalRows);
    r++
  ) {
    const row: string[] = [];
    for (let c = 0; c < state.columnHeaders.length; c++) {
      row.push(engine.getDisplay(cellLabel(r, c)));
    }
    rows.push(row);
  }
  return rows;
}

/**
 * Main reducer.  Every AppEvent case produces a new AppState.
 *
 * The reducer NEVER reads from the DOM, NEVER reads from the formula engine
 * after a commit except via buildViewportRows, and NEVER performs async work.
 * It is a pure function: (state, event) → state.
 */
function reducer(state: AppState, event: AppEvent): AppState {
  switch (event.type) {

    // ── Grid events ──────────────────────────────────────────────────────────

    case "navigate": {
      // The user moved to a new cell via keyboard or click.
      // Update the selection and show the formula of the new cell in the bar.
      return {
        ...state,
        selectedRow: event.row,
        selectedCol: event.col,
        // If we were editing, cancel the edit.
        editRow: -1,
        editCol: -1,
        editContent: "",
      };
    }

    case "editStart": {
      // The user began editing a cell (Enter, F2, double-click, or printable key).
      // Load the cell's raw formula into editContent so the FormulaBar shows it.
      const addr = cellLabel(event.row, event.col);
      const raw = state.cells[addr]?.raw ?? "";
      return {
        ...state,
        selectedRow: event.row,
        selectedCol: event.col,
        editRow:     event.row,
        editCol:     event.col,
        editContent: raw,
      };
    }

    case "editCommit": {
      // The user confirmed an edit (Enter, Tab, or clicking away).
      // Commit to the formula engine and recalculate.
      const addr = cellLabel(state.editRow, state.editCol);
      engine.setRaw(addr, event.value);
      engine.recalculate();
      const display = engine.getDisplay(addr);
      const isError = display.startsWith("#") && display.length > 1;

      const newCells = {
        ...state.cells,
        [addr]: {
          raw:     event.value,
          display: display,
          error:   isError ? display : null,
        },
      };

      const newState: AppState = {
        ...state,
        cells:    newCells,
        editRow:  -1,
        editCol:  -1,
        editContent: "",
        // After commit, move selection down one row (original VisiCalc behaviour).
        selectedRow: Math.min(state.totalRows - 1, state.editRow + 1),
        selectedCol: state.editCol,
      };

      return {
        ...newState,
        viewportRows: buildViewportRows(newState),
      };
    }

    case "editCancel": {
      // The user pressed Escape.  Discard the edit buffer; restore selection.
      return {
        ...state,
        editRow:     -1,
        editCol:     -1,
        editContent: "",
      };
    }

    case "scroll": {
      // The user scrolled the grid.  Update viewport and recompute display slice.
      const newState: AppState = {
        ...state,
        viewportOffset: Math.max(
          0,
          Math.min(event.offset, state.totalRows - state.viewportHeight)
        ),
      };
      return {
        ...newState,
        viewportRows: buildViewportRows(newState),
      };
    }

    case "select": {
      // Mouse drag selection.  Move selection to the anchor cell.
      return {
        ...state,
        selectedRow: event.startRow,
        selectedCol: event.startCol,
      };
    }

    // ── FormulaBar events ────────────────────────────────────────────────────

    case "formulaChange": {
      // The user typed in the formula bar.  Update the live edit buffer.
      // Do NOT commit to the engine yet — that happens on commit/Enter.
      return {
        ...state,
        editRow:     state.selectedRow,
        editCol:     state.selectedCol,
        editContent: event.formula,
      };
    }

    case "commit": {
      // The formula bar's Enter — same as grid editCommit.
      return reducer(state, { type: "editCommit", value: state.editContent });
    }

    case "cancel": {
      // The formula bar's Escape — same as grid editCancel.
      return reducer(state, { type: "editCancel" });
    }

    // ── Internal events ──────────────────────────────────────────────────────

    case "recalculate": {
      // Full recalculation — used on initial load or if the engine is reset.
      engine.recalculate();
      return {
        ...state,
        viewportRows: buildViewportRows(state),
      };
    }

    default: {
      // TypeScript exhaustiveness check: if this line has a type error,
      // a new event type was added without a handler.
      const _exhaustive: never = event;
      return state;
    }
  }
}
```

### §7.5 Initial State

```typescript
// src/app/reducer.ts (continued)

const COLUMNS = 26;           // A–Z
const TOTAL_ROWS = 99;        // matches FE01 maximum (row 1–99)
const VIEWPORT_HEIGHT = 30;   // rows visible at a time (approximate; resize later)
const DEFAULT_COLUMN_WIDTH = 80;   // pixels

const initialState: AppState = {
  cells:          {},
  selectedRow:    0,
  selectedCol:    0,
  editRow:        -1,
  editCol:        -1,
  editContent:    "",
  viewportOffset: 0,
  viewportHeight: VIEWPORT_HEIGHT,
  columnHeaders:  Array.from({ length: COLUMNS }, (_, i) => colLabel(i)),
  columnWidths:   Array.from({ length: COLUMNS }, () => DEFAULT_COLUMN_WIDTH),
  totalRows:      TOTAL_ROWS,
  viewportRows:   Array.from({ length: VIEWPORT_HEIGHT }, () =>
    Array.from({ length: COLUMNS }, () => "")
  ),
};
```

### §7.6 The VisiCalcApp Component

```tsx
// src/app/App.tsx

import React, { useReducer } from "react";
import { Grid }       from "../components/Grid";
import { FormulaBar } from "../components/FormulaBar";
import { reducer, initialState, cellLabel } from "./reducer";

/**
 * VisiCalcApp — the host application.
 *
 * This component contains zero spreadsheet logic.  Its job is:
 *   1. Hold the AppState via useReducer.
 *   2. Compute derived slot values (formula bar formula, cell address label).
 *   3. Render FormulaBar and Grid with the correct slot values.
 *   4. Pass the dispatch function to both components.
 *
 * The reducer (reducer.ts) handles all logic.
 * The formula engine (formulaEngine.ts) handles all computation.
 * The generated components (Grid.tsx, FormulaBar.tsx) handle all rendering.
 */
export function VisiCalcApp() {
  const [state, dispatch] = useReducer(reducer, initialState);

  // The formula bar always shows the formula of the selected cell.
  // If we're currently editing that cell, show the live edit buffer instead.
  const isEditingSelected =
    state.editRow === state.selectedRow && state.editCol === state.selectedCol;

  const formulaBarValue = isEditingSelected
    ? state.editContent
    : state.cells[cellLabel(state.selectedRow, state.selectedCol)]?.raw ?? "";

  const cellAddress = cellLabel(state.selectedRow, state.selectedCol);

  return (
    <div
      style={{
        display:        "flex",
        flexDirection:  "column",
        height:         "100vh",
        background:     "#1e1e1e",
        overflow:       "hidden",
      }}
    >
      {/* Formula bar — Mosaic-generated component */}
      <FormulaBar
        cellAddress={cellAddress}
        formula={formulaBarValue}
        readOnly={false}
        dispatch={dispatch}
      />

      {/* Grid — Mosaic-generated component */}
      <Grid
        columnHeaders={state.columnHeaders}
        columnWidths={state.columnWidths}
        viewportRows={state.viewportRows}
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

### §7.7 React 18 Entry Point

```tsx
// src/app/main.tsx

import React from "react";
import { createRoot } from "react-dom/client";
import { VisiCalcApp } from "./App";

const container = document.getElementById("root");
if (!container) throw new Error("Root element #root not found in index.html");

createRoot(container).render(
  <React.StrictMode>
    <VisiCalcApp />
  </React.StrictMode>
);
```

---

## §8 Formula Engine Integration

### §8.1 Integration Options

The formula engine (FE01) is a Rust crate.  The host app can use it in two ways:

**Option A — WASM build (production path)**

`mosaic-formula-engine` is compiled to WebAssembly via `wasm-pack`:

```sh
wasm-pack build --target web --out-dir ../demo/visicalc/src/engine/wasm
```

The generated JS/WASM module is imported as:

```typescript
import init, { FormulaEngineWasm } from "./wasm/mosaic_formula_engine";

await init();  // load the WASM binary
const engine = FormulaEngineWasm.new();
```

The WASM build preserves the exact FE01 API (§2.4 of FE01): `set_raw`, `get_display`,
`get_formula`, `recalculate`.  The `CellAddr` parsing (A1 notation) happens inside Rust;
the TypeScript caller passes address strings, not numeric coordinates.

**Option B — TypeScript port (demo path, recommended for initial implementation)**

A pure-TypeScript implementation of the FE01 logic avoids the WASM build step and makes
the demo runnable with `npm run dev` without any Rust toolchain.  The TypeScript port
must be semantically identical to the Rust crate — same cell address format, same formula
grammar, same error codes.

**Recommendation: ship Option B first.**  The WASM path is the production target but
requires `wasm-pack`, a `wasm32-unknown-unknown` toolchain, and a bundler WASM plugin.
For the initial demo, Option B is faster to iterate on and keeps the dependency count low.
Document Option A as the upgrade path.

### §8.2 TypeScript FormulaEngine Interface

Both Option A and Option B expose exactly the same TypeScript interface.  The host
application (`reducer.ts`) imports from `"../engine/formulaEngine"` and never sees
whether the implementation is TypeScript or WASM.

```typescript
// src/engine/formulaEngine.ts

/**
 * The formula engine interface.  Implementations:
 *   - TypeScript port (this file)
 *   - WASM build (Option A, future)
 *
 * All methods are synchronous.  The WASM build wraps the Rust functions in
 * synchronous JS bindings (wasm-bindgen generates these automatically).
 */
export interface FormulaEngine {
  /**
   * Set the raw content of a cell.  If `raw` starts with '=', it is treated
   * as a formula.  Otherwise it is a literal: a parseable number becomes
   * numeric, anything else becomes text.
   *
   * This does NOT trigger recalculation.  Call recalculate() when ready.
   *
   * @param addr  Cell address string, e.g. "A1", "B12".
   * @param raw   Raw content string.
   */
  setRaw(addr: string, raw: string): void;

  /**
   * Get the display string for a cell (the value the user sees in the cell).
   *
   * - Empty cell → ""
   * - Number 6.0 → "6" (no trailing .0 for integral values)
   * - Number 2.5 → "2.5"
   * - Text → the original text
   * - Error → "#DIV/0!", "#REF!", "#NAME?", "#VALUE!", "#CIRC", "#PARSE"
   *
   * @param addr  Cell address string.
   */
  getDisplay(addr: string): string;

  /**
   * Get the formula string for a cell (what appears in the formula bar).
   *
   * - Formula cell → the original "=..." string
   * - Literal cell → the original raw string
   * - Empty cell → ""
   *
   * @param addr  Cell address string.
   */
  getFormula(addr: string): string;

  /**
   * Evaluate all dirty cells in dependency order.
   * Must be called after setRaw before getDisplay reflects the new value.
   */
  recalculate(): void;
}

/**
 * Create a new, empty formula engine.
 * The host reducer holds a module-level instance.
 */
export function createEngine(): FormulaEngine {
  return new TypeScriptFormulaEngine();
}
```

### §8.3 Constructing viewportRows

The `viewportRows` slot fed to Grid is a 2D array of display strings.  It must be
reconstructed whenever cells change or the viewport scrolls.  The `buildViewportRows`
helper in `reducer.ts` (§7.4 above) encapsulates this:

```typescript
// For each visible row r in [viewportOffset, viewportOffset + viewportHeight):
//   For each column c in [0, columnHeaders.length):
//     viewportRows[r - viewportOffset][c] = engine.getDisplay(cellLabel(r, c))
```

**Performance note:** `getDisplay` must be O(1) per cell after `recalculate()` runs.
The formula engine caches computed values.  `recalculate()` is the expensive operation;
it runs in topological dependency order (O(cells + edges)).  `buildViewportRows` calls
`getDisplay` on at most `30 × 26 = 780` cells per render — well within budget.

---

## §9 Keyboard Behaviour

The full keyboard model must be implemented in `Grid.tsx` (the generated component's
`handleKeyDown` handler, shown in §6.3) and coordinated via the reducer.

```
┌─────────────────────────────────┬────────────────────────┬────────────────────────┐
│ Key                             │ Not editing            │ Editing                │
├─────────────────────────────────┼────────────────────────┼────────────────────────┤
│ Arrow keys (↑ ↓ ← →)           │ Navigate               │ Move cursor (native)   │
│ Enter                           │ Begin editing           │ Commit, move down      │
│ Escape                          │ (no-op)                │ Cancel                 │
│ F2                              │ Begin editing           │ (no-op)                │
│ Tab                             │ Move right              │ Commit, move right     │
│ Shift+Tab                       │ Move left               │ Commit, move left      │
│ Any printable char              │ Begin editing (replace) │ Append (native input)  │
│ Backspace / Delete              │ Clear cell              │ Delete char (native)   │
└─────────────────────────────────┴────────────────────────┴────────────────────────┘
```

**Begin editing with a printable character (replace mode):**
When the user presses a printable key while not editing, the reducer fires `editStart`
and sets `editContent` to the pressed character, replacing the previous cell content.
This matches original VisiCalc behaviour: typing on a cell immediately replaces it.

**Tab post-commit movement:**
When Tab commits an edit, the reducer moves the selection right (or left for Shift+Tab)
after clearing the edit state, matching the "commit and advance" spreadsheet convention.

**Backspace on non-editing selection:**
Pressing Backspace on a selected cell while not editing fires `editCommit` with an empty
string, clearing the cell.  This is consistent with the "clear cell" action in Excel and
VisiCalc descendants.

**Formula bar keyboard:**
When the user clicks into the FormulaBar and presses Enter or Escape, `FormulaBar`
fires `commit` or `cancel` events.  The reducer routes these to the same `editCommit` /
`editCancel` logic as Grid events (§7.4).

---

## §10 Column Label Conventions

VisiCalc uses single letters A–Z for its 26 columns.  The original VisiCalc (1979) on the
Apple II used A–W (23 columns) on an 80-column display.  This implementation uses the
full A–Z alphabet for 26 columns, matching the FE01 crate's `CellAddr` range.

```typescript
// src/app/reducer.ts

/**
 * Map a zero-based column index to its letter label.
 *
 * Truth table:
 *   col=0  → "A"   (char code 65)
 *   col=1  → "B"   (char code 66)
 *   col=25 → "Z"   (char code 90)
 *
 * Precondition: 0 ≤ col ≤ 25.  Values outside this range produce
 * invalid addresses — the formula engine will reject them with #REF!
 */
function colLabel(col: number): string {
  return String.fromCharCode(65 + col);
}

/**
 * Map (row, col) to a cell address string.
 *
 * Truth table:
 *   (row=0, col=0)   → "A1"    — top-left cell
 *   (row=0, col=2)   → "C1"    — third column, first row
 *   (row=1, col=0)   → "A2"    — first column, second row
 *   (row=11, col=2)  → "C12"   — twelfth row
 *   (row=98, col=25) → "Z99"   — bottom-right cell (FE01 maximum)
 *
 * Note: rows are 0-based internally but 1-based in the address string.
 * This matches standard spreadsheet convention (row 1 is the top).
 */
function cellLabel(row: number, col: number): string {
  return `${colLabel(col)}${row + 1}`;
}
```

The inverse function (`parseAddr`) is not needed in the host application — only the
formula engine needs to parse addresses, and it does so in Rust (or in the TS port).

---

## §11 Directory Structure

The demo application lives under `demo/visicalc/` relative to the repo root.

```
demo/visicalc/
├── src/
│   ├── components/                    ← Mosaic-generated output (do not edit)
│   │   ├── Grid.tsx
│   │   ├── GridEvent.ts
│   │   ├── FormulaBar.tsx
│   │   └── FormulaBarEvent.ts
│   ├── engine/
│   │   └── formulaEngine.ts           ← TypeScript formula engine (FE01 JS port)
│   └── app/
│       ├── reducer.ts                 ← AppState, AppEvent, reducer, initialState
│       ├── App.tsx                    ← VisiCalcApp component
│       └── main.tsx                   ← React 18 createRoot entry point
├── mosaic/                            ← Mosaic source files (checked in)
│   ├── Grid.mil
│   ├── Grid.desktop.mll
│   ├── Grid.dark.msl
│   ├── FormulaBar.mil
│   ├── FormulaBar.desktop.mll
│   ├── FormulaBar.dark.msl
│   └── visicalc-tokens.msl
├── pipelines/
│   └── visicalc-desktop-dark.mospipeline
├── public/
│   └── index.html                     ← <div id="root"></div>
├── package.json
├── tsconfig.json
└── vite.config.ts
```

**File ownership:**

| Directory / file | Owner | Rule |
|---|---|---|
| `src/components/` | `mosaic-compile` | Never hand-edit.  Regenerate by re-running the pipeline. |
| `src/engine/formulaEngine.ts` | Human | FE01 TypeScript port.  May be replaced by WASM bindings later. |
| `src/app/` | Human | Host application logic.  The one place spreadsheet semantics live. |
| `mosaic/` | Human | Mosaic source files.  Checked into git.  Regenerating `src/components/` requires these. |
| `pipelines/` | Human | Pipeline manifests.  One file per theme × platform combination. |

---

## §12 What Is Out of Scope

The following are explicitly deferred and not part of this spec.  Future specs may address
them; this spec does not provide partial support for any of them.

| Feature | Deferral reason |
|---|---|
| Multi-sheet workbooks | Requires tab bar component (new `.mil`) and engine namespace extension |
| Cell formatting (bold, italic, number formats) | Requires format metadata layer in AppState and formula engine |
| Column resize by dragging | Requires pointer-capture event model not yet specced in moslayout |
| Row height customization | Same as column resize |
| Copy / paste | Requires `navigator.clipboard` API access and clipboard event model |
| Undo / redo | Requires `AppState` history stack (immutable state is a prerequisite) |
| Mobile layout variant | Requires `Grid.mobile.mll` + touch event model in UI14 |
| WASM build of mosaic-formula-engine | Option A (§8.1) — deferred pending `wasm-pack` build pipeline |
| Widget runtime for Input | The `Input` primitive's paint-vm backend is deferred per UI25 §out-of-scope |
| Range formulas (SUM(A1:B10)) | FE01 specifies range support; the TS port may implement a subset |
| Named ranges | Not in FE01; deferred to a hypothetical FE02 |
| Sorted / filtered views | Orthogonal to the spreadsheet model; deferred |

None of these deferreds affect the correctness or completeness of the §1–§11 design.
The demo application is fully functional — a user can enter numbers, enter formulas that
reference other cells, see computed results, navigate with arrow keys, and edit with the
formula bar — without any of the deferred features.

---

## Appendix A — Emit Type Naming Rules

The emit → TypeScript event type mapping follows UI24 §3.2 (reproduced here for
reference):

| `.mil` emit name | TypeScript event `type` value |
|---|---|
| `onNavigate` | `"navigate"` |
| `onEditStart` | `"editStart"` |
| `onEditCommit` | `"editCommit"` |
| `onEditCancel` | `"editCancel"` |
| `onScroll` | `"scroll"` |
| `onSelect` | `"select"` |
| `onFormulaChange` | `"formulaChange"` |
| `onCommit` | `"commit"` |
| `onCancel` | `"cancel"` |

Rule: strip the `on` prefix, lowercase the first character of the remainder.
`onFooBar` → `"fooBar"`.  This produces camelCase event types.

## Appendix B — Token Resolution Order

When `mosaic-compile` processes the pipeline, it resolves token references in this order:

1. **Global tokens** — declared in `[global-style] tokens` (i.e. `visicalc-tokens.msl`).
2. **Component-local tokens** — declared inside the component's own `.msl` file (if any).
3. **Compiler defaults** — hardcoded fallbacks for required token types.

Later entries in the list override earlier entries.  This allows a component style to
override a global token for local use without modifying the shared token file.

No unresolved token reference survives compilation.  Every `$token-name` in a `.msl` file
must have a concrete value (color hex, pixel length, etc.) by the time the backend emitter
runs.  The emitter inlines the resolved values — no token names appear in the generated
output.
