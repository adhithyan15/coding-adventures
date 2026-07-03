# UI28-1 — Grid v3 (revised): a userland library Grid built only on kernel primitives

**Status:** Specification (draft)
**Layer:** UI / userland (`mosaic-pkg-grid` v0.2.0)
**Supersedes (partial):** UI28 §2.1 and §2.3 promote `Cell` / `Column` /
`Grid` as new **kernel** primitives. UI28-1 keeps the same architectural
shape (Cell + Column + Grid) but ships them as **userland components**
inside `mosaic-pkg-grid` — no new kernel primitives are added.
**Depends on:** UI14 (moslayout), UI24 (emit→dispatch), UI26 (visicalc
demo), UI27 (Grid v2 monolithic emitter), UI29 (kernel primitives:
`Box`, `If`, `Else`, `For`, `HostInput`, `HostTable*`, …), UI31
(HostTable family across all backends).
**Targets:** the **U29-X1 milestone** the moslayout-compiler comment
already promised: *"`Grid` is retained … for backwards-compatibility
with the pre-UI29 backends and will be removed by U29-X1 once the
userland `mosaic-pkg-grid` package proves the new architecture
end-to-end."*

---

## 1. Why this revision

UI28 was drafted before the **kernel-primitive moratorium** the user
formalised in the VisiCalc-Grid design sessions (see "User-imposed
constraints" below). It promotes three new kernel primitives. The
real-world constraint set since then has been the opposite: do not
grow the kernel; prove that what's already in the kernel is sufficient
to build a real grid.

UI28-1 takes UI28's shape (Cell-and-Column composition is the right
decomposition) and re-targets it at userland. Every primitive the
revised Grid uses is **already a kernel primitive every backend
lowers today**:

```
Box, Row, Column, Stack, Text, Image, Spacer, Divider, Icon,
If, Else, For,
HostInput, HostButton, HostTable, HostTableHead, HostTableBody,
HostTableFoot, HostTableColGroup, Col, HostScroll, HostDialog,
HostCheckbox, HostRadio, HostLink, HostTooltip, HostNumberInput
```

The 21-element kernel was frozen by UI29 §2.1. UI28-1's Grid v3 makes
no additions to it.

---

## 2. User-imposed constraints (verbatim)

These constraints come from the live design session that produced this
spec. They are not negotiable inside v0.2.0:

1. **No new kernel primitives.** Cell stays a userland component.
2. **Performance focus.** The chosen investment is **stable iteration
   keys**: every backend's `For` lowering must thread `For`'s `index:`
   binding into the framework-native list-key mechanism (React `key=`,
   SwiftUI `id:`, Flutter `Key()`, Qt `Repeater` index property).
3. **No implicit template bindings.** Grid v3 does *not* introduce
   UI28 §3.1's reserved names (`cell-value`, `cell-row`, `column-key`,
   …). Authors write explicit `For` loops; bindings flow through the
   normal slot mechanism.
4. **Encapsulation, no spilling.** The host pushes only its own
   already-existing state — selected-row, selected-col, edit-row,
   edit-col as plain numbers. The Grid component computes per-cell
   predicates internally. The host never learns about Grid's internal
   selection-mask or edit-mask shape.
5. **Sticky-header is optional / deferred.** Not in v0.2.0. Authors
   wanting a sticky header should compose `HostScroll` around `Grid`
   themselves, or wait for UI28-2.

---

## 3. The v0.2.0 component triple

`mosaic-pkg-grid` already ships v0.1.0 with this exact triple as a
scaffold; v0.2.0 fills it out.

### 3.1 `Cell` — single editable spreadsheet cell

#### Interface (`Cell.mil`)

```mosmodel
component Cell {
  // What the cell shows.
  slot value      : text ;

  // The cell's address inside its enclosing Grid. The host pushes the
  // SAME edit-row / edit-col / selected-row / selected-col it pushes
  // to the Grid; Cell computes whether THIS coordinate is the active
  // one. (Per §2 constraint 4: Grid encapsulates the comparison.)
  slot cell-row   : number ;
  slot cell-col   : number ;
  slot edit-row   : number ;
  slot edit-col   : number ;
  slot selected-row : number ;
  slot selected-col : number ;

  // Author affordances.
  slot editable   : bool ;
  slot alignment  : text ;
  slot cell-type  : text ;   // "text" | "number" — v0.2.0 treats both
                             // the same; cell-type is forwarded so the
                             // sub-part theme can style numeric cells
                             // differently. Formatting/validation is
                             // a v0.3.0 concern.

  emit onClick ;
  emit onCommit ( value : text ) ;
  emit onCancel ;
}
```

#### Layout (`Cell.mll`)

```moslayout
layout Cell {
  Box [ cell ] {
    If ( when: ( cell-row == edit-row ) and ( cell-col == edit-col ) ) {
      HostInput (
        value:    slot: value ,
        onCommit: emit: onCommit ,
        onCancel: emit: onCancel
      )
    }
    Else {
      Text ( content: slot: value )
    }
  }
}
```

Two notes on the `If` predicate:

- Uses **expression-in-slot-binding** — the `==` and `and` operators
  parse into `LayoutPropValue::Expr` today (verified in this session
  by reading `moslayout-compiler` lines 1127-1129). The blocker is
  scoping: `cell-row`, `edit-row`, etc. must resolve to interface slot
  references. Today the parser handles bare-NAME-as-slot-ref via the
  `slot:` keyword; expressions need the same resolution. **UI29 §3.4
  is the enabling work** (see §6 below).
- Selection highlight (`cell-row == selected-row and cell-col ==
  selected-col`) is **sub-part-driven**, not `If`-driven: it's a CSS-
  class / view-modifier toggle on the same `Box [cell]`, not a
  different rendered tree. The Cell.dark.msl + Cell.light.msl ship the
  `cell:selected` sub-part rule. Implementation detail: emitters need
  to recognise the predicate-bound `cell:selected` part name. v0.1.0
  has the `cell` part wired; v0.2.0 adds `cell:selected`, `cell:editing`,
  `cell:read-only` matching UI28 §2.1.

### 3.2 `Column` — column metadata

Unchanged from v0.1.0 (already correct):

```mosmodel
component Column {
  slot key               : text ;
  slot header            : text ;
  slot width             : number ;
  slot editable          : bool ;
  slot cell-type         : text ;
  slot default-alignment : text ;
}
```

```moslayout
layout Column { Box [ column-marker ] }
```

The hollow Box is a quirk of moslayout requiring a single root node
per layout. Compilers strip it. A future moslayout extension that
accepts zero-root metadata-only components removes the marker; that's
a v0.3.0 concern.

### 3.3 `Grid` — the composition

#### Interface (`Grid.mil`)

```mosmodel
component Grid {
  // Per-row data the host has already sliced to the viewport.
  slot viewport-rows : list<list<text>> ;

  // Column metadata. v0.2.0 accepts a parallel-array shape until
  // mosmodel grows a record type:
  //   slot column-headers : list<text>  — header label per column
  //   slot column-widths  : list<number> — width per column in px
  // (column-editable / column-cell-type / column-alignment go into
  // parallel slots the same way, or stay defaults inside Cell.)
  slot column-headers : list<text> ;
  slot column-widths  : list<number> ;

  // Selection + edit state. Plain numbers; -1 = none.
  slot selected-row : number ;
  slot selected-col : number ;
  slot edit-row     : number ;
  slot edit-col     : number ;
  slot edit-content : text ;

  emit onNavigate    ( row : number , col : number ) ;
  emit onEditCommit  ( value : text ) ;
  emit onEditCancel ;
}
```

#### Layout (`Grid.mll`)

```moslayout
layout Grid {
  HostTable [ sheet ] {
    HostTableColGroup {
      For ( each: slot: column-widths , as: w , index: c ) {
        Col [ col ] ( width: w )
      }
    }
    HostTableHead {
      Row [ header-row ] {
        For ( each: slot: column-headers , as: header , index: c ) {
          Box [ header-cell ] { Text ( content: header ) }
        }
      }
    }
    HostTableBody {
      For ( each: slot: viewport-rows , as: row , index: r ) {
        Row [ data-row ] {
          For ( each: row , as: cell-value , index: c ) {
            Cell (
              value:         cell-value ,
              cell-row:      r ,
              cell-col:      c ,
              edit-row:      slot: edit-row ,
              edit-col:      slot: edit-col ,
              selected-row:  slot: selected-row ,
              selected-col:  slot: selected-col ,
              editable:      true ,
              alignment:     "left" ,
              cell-type:     "text" ,
              onClick:       emit: onNavigate ,
              onCommit:      emit: onEditCommit ,
              onCancel:      emit: onEditCancel
            )
          }
        }
      }
    }
  }
}
```

The inner `For (each: row, ...)` is **row-binding-as-iterable** — the
inner loop's `each:` value is the outer loop's `as:` binding. UI29 §3.4
(see §6 below) is what unblocks this.

---

## 4. Cross-backend lowering — what every backend already does

Per UI31 every backend lowers `HostTable + HostTableHead + HostTableBody
+ HostTableFoot + HostTableColGroup + Col + Row` to its native table
widget. Per UI29 every backend lowers `For`, `If`, `Else`, `Box`,
`Text`, `HostInput`. No backend needs new lowering work for the Grid
**shape** in v0.2.0. The three enabling dependencies in §6 are the
only per-backend touches.

| Backend | HostTable native widget | Per-cell click hook | Inline-edit input |
|---|---|---|---|
| React | `<table>` + `<thead>` + `<tbody>` + `<tr>` + `<td>` | `onClick` on the body `<td>` (UI29) | `<input>` from HostInput |
| HTML | same as React, static-rendered | static `data-on-click` attribute (UI24 dispatch) | static `<input>` |
| WebComponent | same in shadow DOM | event listener inside the component class | `<input>` in shadow DOM |
| Flutter | `DataTable` (or `Table` for non-stretchy) | `InkWell` wrap | `TextField` (HostInput) |
| Qt / QML | `TableView` or `Grid` + `Repeater` | `MouseArea` inside the delegate | `TextField` (HostInput) |
| SwiftUI | `Table` (macOS 12+) or `Grid` + `ForEach` | `.onTapGesture` | `TextField` (HostInput) |
| XAML | `<Grid>` + `<Grid.RowDefinitions>` + Border cells | `Tapped` event | `TextBox` (HostInput) |

---

## 5. Performance properties

v0.2.0's perf story is **stable keys plus host-owned virtualization**.

1. **Stable iteration keys.** Every `For` in `Grid.mll` binds `index:`
   to a fresh name (`r`, `c`). Each backend's For lowering threads
   that into its native list-key:
   - **React** — `key={r}` on each `<tr>`, `key={c}` on each `<td>`.
     (Pre-v0.2.0 the React emitter does NOT auto-emit keys from
     `For`'s `index:` — §6.3 fixes this.)
   - **SwiftUI** — `ForEach(rows.indices, id: \.self)`. (Already
     emitted today: SwiftUI's For lowering uses `id: \.offset` when
     `index:` is bound.)
   - **Flutter** — `Key(r.toString())` on each row widget. (Pre-v0.2.0
     Flutter doesn't emit For at all — §6.2 fixes this.)
   - **Qt / QML** — `Repeater { model: rows; delegate: Row { property
     int rowIndex: index; … } }`. (Already emitted today.)
   - **HTML / WebComponent / XAML** — static-rendered; no diffing,
     no keys needed.
2. **Host-owned virtualization.** The host slices the spreadsheet
   to the viewport BEFORE pushing `viewport-rows`. The Grid renders
   only what the host sends. A 1,000,000-row sheet with a 30-row
   viewport pushes a 30-row `viewport-rows` slot per render.
3. **No allocations in the Grid hot path.** Cell's `Box [cell]` does
   one stable wrapper element per cell. The `If/Else` is statically
   one of two children — no dynamic list construction.
4. **Cell sub-part styling stays in CSS / view-modifier land.** Per-
   cell appearance changes (`cell:selected`, `cell:editing`,
   `cell:read-only`) toggle classes / modifiers; they do not re-render
   the cell tree.

UI28-2 will add **list virtualization inside the Grid** for cases
where the host pushes a large `viewport-rows` (e.g., 10k rows visible
in a scrollable container). That requires a `HostVirtualList` kernel
primitive — out of scope for v0.2.0.

---

## 6. Enabling work (separately-shipped dependencies)

UI28-1 names three dependencies that ship as their own PRs **before**
Grid v0.2.0 can use them. Each dependency is independently useful
beyond Grid.

### 6.1 UI29 §3.4 — For-loop binding scope

**Today** the moslayout validator (`validate_for_node` at
`code/packages/rust/moslayout-compiler/src/lib.rs:546-634`) accepts
only `LayoutPropValue::SlotRef` or `LayoutPropValue::Expr` for `For`'s
`each:`. A bare NAME like `row` parses as `LayoutPropValue::Keyword`
and is rejected.

Expressions (`(cell-row == edit-row) and (cell-col == edit-col)`)
parse into `Expr(String)` because the operator productions descend
through `or_expr → and_expr → eq_expr → rel_expr → unary → postfix →
primary` (lib.rs lines 1127-1129). But the Expr is stored as opaque
source text, not a resolved AST. Outer For-loop `as:` / `index:`
bindings are not in any scope the expression evaluator can see.

**Work:**
- Extend `LayoutPropValue` with a `LoopBinding(String)` variant (or
  enrich `Expr` to carry a resolved binding-scope), so the IR
  distinguishes "this NAME is bound by an enclosing For" from "this
  NAME is something else".
- Make `validate_for_node` and `validate_layout` scope-aware: walk the
  tree carrying a stack of bindings introduced by ancestor For nodes.
- Per-emitter: when lowering a For whose `each:` is a LoopBinding,
  emit the target-language variable reference (`row`, `cells`, etc.)
  unchanged from the binding name. For Expr cases that mention
  loop-bound names, the per-backend expression substituter must
  recognise loop-bound names and emit them as the target-language
  loop variable.

PR slug: `feat/u29-3-4-for-binding-scope`.

### 6.2 Flutter For / If / Else lowering

**Today** `code/packages/rust/mosaic-emit-flutter/src/pipeline.rs`
line 593 returns a placeholder comment for `For` / `If` / `Else`.

**Work:**
- `For` → `<source>.asMap().entries.map((entry) { final r =
  entry.key; final row = entry.value; return …; }).toList()`. The
  emitted block is the body of a Column / ListView children
  expression.
- `If` → conditional expression: `if (predicate) <thenBody> else
  <elseBody>` inside the children-list context (Dart's
  collection-if). `Else` consumes the following sibling's body.
- Stable key: when `index:` is bound, the emitted widget receives
  `key: ValueKey(r)` (or `Key(r.toString())`).

PR slug: `feat/mosaic-emit-flutter-control-flow`.

### 6.3 React For auto-key from `index:`

**Today** `code/packages/rust/mosaic-emit-react/src/pipeline.rs` line
1008 comments out auto-keying: *"React's reconciler will warn the host
about a missing `key={...}` — that warning belongs in a follow-up
keying PR."*

**Work:**
- When a For node has `index: <name>` bound, the emitted JSX child
  receives `key={<name>}` automatically. The author does not have to
  spell it.
- When only `as: <name>` is bound (no `index:`), emit `key={<name>}`
  if the binding shape can serve as a key (string / number); else
  emit a synthetic numeric key from the iteration index and log a
  compile-time warning telling the author to add `index:` for a
  meaningful key.

PR slug: `feat/mosaic-emit-react-for-key-from-index`.

---

## 7. Out of scope for v0.2.0 (deferred to UI28-2)

These items appeared in UI28 or earlier Grid discussions but are
explicitly NOT in v0.2.0:

- **Sticky header.** Per §2 constraint 5. Authors compose
  `HostScroll { Grid {…} }` themselves or wait.
- **Custom cell renderers** (image cell, button cell, checkbox cell,
  sparkline cell). v0.3.0 can extend Cell with a `cell-type` that
  switches on `"image"` / `"checkbox"` / `"link"` — the slot already
  exists for forward-compat.
- **Column groups, sortable headers, pinned columns.** UI28's §1
  "ag-grid-class features" table. Each is its own design problem.
- **List virtualization inside Grid** (rendering only the visible
  rows of a large `viewport-rows`). Needs a `HostVirtualList` kernel
  primitive.
- **Mosmodel record type** (so `columns` can be `list<column-meta>`
  instead of parallel `list<text>` + `list<number>` arrays). v0.3.0
  language work.
- **Removing the legacy `Grid` built-in primitive** (the U29-X1
  milestone proper). Becomes safe once VisiCalc is rewired onto v0.2.0
  AND every other consumer is audited. Tracked as a follow-up PR.

---

## 8. Verification

UI28-1 is a spec. Verification of the spec itself is a peer-review
sign-off plus the cross-references it makes lining up with the actual
files (lib.rs line numbers, v0.1.0 component shape, the 21-element
kernel set). No code changes ship with this PR.

The implementation PRs (Phases 1-5 of the larger plan) each carry
their own verification plans — see the plan file
`/Users/adhithya/.claude/plans/async-wondering-hedgehog.md`.

---

## 9. Open questions

- **§3.3 `columns` shape.** Parallel arrays are a v0.2.0 pragmatism.
  Whether the spec should commit to that shape OR leave it as
  "implementation choice" pending the mosmodel record type — picked
  the former because the spec needs concrete slot names so the host
  can wire them. v0.3.0 migrates to records.
- **§3.1 Cell `editable` / `alignment` / `cell-type`.** v0.2.0 ships
  them as constants from the Grid call site (`editable: true`,
  `alignment: "left"`, `cell-type: "text"`). A future revision lets
  the Column metadata drive these per-column — requires the same
  scope/binding work as §6.1 to thread `columns[c].editable` into the
  Cell call.
