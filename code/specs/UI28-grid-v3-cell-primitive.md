# UI28 — Grid v3: Cell-Centric Composition

**Status:** Specification (draft)
**Layer:** UI / cross-cutting (moslayout primitive vocabulary + all backend emitters)
**Depends on:** UI14 (moslayout), UI24 (emit→dispatch), UI26 (visicalc), UI27 (Grid v2)
**Supersedes (partial):** the monolithic Grid in UI14 §11 and the sub-part defaults in UI27 §5

---

## 1. Purpose

Grid v1 (UI26 §6) and Grid v2 (UI27) shipped a **monolithic** primitive:
the Grid emitter on every backend synthesises the entire `<table>` /
`<th>` / `<td>` structure internally. Authors style cells via *sub-parts*
(`sheet/cell`, `sheet/header-cell`, etc.) — but a sub-part is a styling
target, not a primitive. Cells cannot be **substituted**, **extended**,
or **composed**.

This monolithic shape blocks every ag-grid-class feature:

| Feature | Why it's impossible today |
|---|---|
| Custom cell renderers (image, checkbox, button, sparkline) | Cell isn't a primitive; can't replace the body of a `<td>` |
| Column groups | No declarative column-metadata primitive |
| Per-column editability / type / format | Editing is baked into the Grid emitter, not per-column |
| Row headers (leftmost "1", "2", "3" column) | Currently requires forking the Grid emitter; should just be a Column with `editable: false` |
| Sortable / filterable headers | No HeaderCell primitive; the `<th>` content is hardcoded `{h}` |
| Pinned columns | Same — no first-class column primitive |
| Standalone Cell views (card view of one record) | Cell only exists *inside* a Grid; not reusable |

UI28 fixes the architecture by decomposing the Grid into three
first-class primitives:

1. **`Cell`** — one cell view. Stand-alone primitive; can be used
   anywhere a value needs to display (inside a Grid, in a record card,
   as the content of a list item, etc.).
2. **`Column`** — declarative column metadata. Lives only as a child
   of Grid; emits nothing by itself.
3. **`Grid` v3** — composition primitive. Reads its Column children
   for column metadata and clones a Cell template child per body cell.

Row headers, per-column editability, custom renderers, and a path to
ag-grid-class features fall out of the decomposition for free.

The decomposition is also what makes **"portable to every UI
platform"** real. With monolithic Grid, each backend re-implements the
table internally. With Cell + Column + Grid v3, each backend implements
Cell once and the Grid is a composition layer that maps cleanly onto
the host platform's table widget (DOM `<table>`, SwiftUI `Table`, Qt
`QTableView`, Compose `LazyVerticalGrid`, etc.).

---

## 2. New / changed primitives

### 2.1 `Cell` (new primitive)

A single editable value view. Stand-alone primitive — usable inside a
Grid or anywhere else.

#### Slots

| Slot | Type | Default | Effect |
|---|---|---|---|
| `value` | `text` | (required) | The display value |
| `editable` | `bool` | `true` | When `false`, the cell never renders an inline editor |
| `cell-type` | `text` | `"text"` | One of: `"text"`, `"number"`, `"image"`, `"checkbox"`, `"link"`. Drives the rendered element type. |
| `alignment` | `text` | `"left"` | One of: `"left"`, `"right"`, `"center"` |
| `is-editing` | `bool` | `false` | When `true`, the cell renders its inline editor (`<input>` in React; `TextField` in SwiftUI; `QLineEdit` in Qt). The host owns edit state and pushes it in via this slot. |

#### Emits

| Emit | Payload | When |
|---|---|---|
| `onClick` | (none) | User clicks the cell when not editing |
| `onCommit` | `value: text` | Inline editor's Enter / submit |
| `onCancel` | (none) | Inline editor's Escape |

#### Sub-parts

| Sub-part | Targets |
|---|---|
| `cell` | The cell's outer element (`<td>` inside Grid; `<div>` standalone) |
| `cell:selected` | When the cell is the selected one (host coordinates via Grid; standalone Cell ignores) |
| `cell:editing` | When `is-editing` is `true` |
| `cell:read-only` | When `editable` is `false` |

Following UI27, sub-part defaults live in mosstyle; slots override at
runtime. Same two-layer cascade.

#### Standalone behaviour

When `Cell` appears outside a Grid, it lowers to a non-table element
(`<div>` in DOM-flavoured backends, a `View` in SwiftUI, a `QWidget`
in Qt). It still supports editing, dispatch, and sub-part styling.
This makes Cell reusable in record cards, form rows, and other
non-grid layouts.

---

### 2.2 `Column` (new primitive)

Declarative column metadata. Has no rendered output on its own — it
lives only as a child of Grid and is consumed by the Grid emitter.

#### Slots

| Slot | Type | Default | Effect |
|---|---|---|---|
| `key` | `text` | (required) | Stable column identifier (e.g., `"A"`, `"name"`, `"price"`) |
| `header` | `text` | (required) | Header-row display label |
| `width` | `number` (px) | `auto` | Column width; the Grid emitter writes this into `<colgroup><col>` (or the platform equivalent) |
| `editable` | `bool` | `true` | When `false`, every body cell in this column is non-editable. Row-header columns set this to `false`. |
| `cell-type` | `text` | `"text"` | Default `cell-type` for every body cell in this column |
| `default-alignment` | `text` | `"left"` | Default `alignment` for every body cell in this column |

#### Emits

None. Column is purely metadata.

#### Where it lives

Inside a Grid's `children` block. Multiple Columns declare the column
list:

```moslayout
Grid [sheet] (...) {
  Column (key: "row-num", header: "", width: 40, editable: false, default-alignment: "right")
  Column (key: "A", header: "A", width: 80)
  Column (key: "B", header: "B", width: 80)
  // ...
}
```

A standalone `Column` (outside Grid) is a compile-time warning — the
mosaic-emit-* emitters discard it with a `// Column is metadata; use
inside Grid` comment in dev builds.

---

### 2.3 `Grid` v3 (refactored)

#### Slots (changed from v2)

The Grid's prop surface shrinks: properties that used to be Grid slots
move into per-Column slots. What remains is the **runtime state** that
varies per render.

| Slot | Type | Default | Effect | Inherited from v2? |
|---|---|---|---|---|
| `viewport-rows` | `list<list<text>>` | (required) | 2-D matrix of cell values; one inner list per visible row | Yes |
| `selected-row` | `number` | `-1` | Currently selected row (viewport-relative index, not absolute) | Yes |
| `selected-col` | `number` | `-1` | Currently selected column (column-index, 0-based) | Yes |
| `edit-row` | `number` | `-1` | Row whose cell is being edited; `-1` means no edit | Yes |
| `edit-col` | `number` | `-1` | Column being edited | Yes |
| `edit-content` | `text` | `""` | Live edit-buffer content the inline editor displays | Yes |
| `total-height` | `number` (px) | `auto` | Scroll viewport height; UI27 §6 | Yes |
| `sticky-header` | `bool` (literal) | `false` | Compile-time keyword; UI27 §6 | Yes |

#### Removed in v3

| v2 slot | v3 replacement |
|---|---|
| `column-headers : list<text>` | Each `Column { header }` declares its own |
| `column-widths : list<number>` | Each `Column { width }` declares its own |

#### Emits (unchanged)

| Emit | Payload |
|---|---|
| `onNavigate` | `row: number, col: number` |
| `onEditCommit` | `value: text` |
| `onEditCancel` | (none) |

#### Children

Grid v3 takes two kinds of children:

1. **Zero or more `Column` nodes** — column metadata, in display order.
2. **Up to one `Cell` template** — the body-cell renderer. Optional:
   if absent, Grid emits a default "value text only" cell.
3. **Up to one HeaderCell template** — the header-row renderer.
   Optional: if absent, Grid emits a non-editable Cell with the
   `column.header` as its value. *Implementation note:* in v3 there
   is no separate `HeaderCell` primitive — the header template is
   just a Cell node whose `part_name` differs (convention: `header-cell`
   vs `body-cell`). Backends can keep the distinction purely
   stylistic.

```moslayout
Grid [sheet] (
  viewport-rows: slot: viewport-rows,
  selected-row:  slot: selected-row,
  selected-col:  slot: selected-col,
  edit-row:      slot: edit-row,
  edit-col:      slot: edit-col,
  edit-content:  slot: edit-content,
  sticky-header: true,
  total-height:  slot: total-height,
  onNavigate:    emit: onNavigate
) {
  Column (key: "row-num", header: "",  width: 40, editable: false, default-alignment: "right")
  Column (key: "A",       header: "A", width: 80)
  Column (key: "B",       header: "B", width: 80)
  // ... 24 more columns ...

  Cell [header-cell] (
    value:    column-header,
    editable: false
  )

  Cell [body-cell] (
    value:      cell-value,
    editable:   column-editable,
    cell-type:  column-cell-type,
    alignment:  column-default-alignment,
    is-editing: cell-is-editing,
    onCommit:   emit: onEditCommit,
    onCancel:   emit: onEditCancel,
    onClick:    emit: onNavigate
  )
}
```

#### Backwards compatibility

When a Grid has **no Column children**, it falls back to v2 monolithic
behaviour: it reads `column-headers: slot: ...` (the old slot) and
synthesises cells internally. This lets existing demos compile
unchanged until they migrate.

When a Grid has **Column children but no Cell template**, it uses a
default Cell that just renders `{cellValue}` as text and dispatches
`onNavigate` on click.

---

## 3. Implicit template bindings

The Cell-template-inside-Grid pattern needs a way for the template's
slot bindings to reference loop variables (the row/col coordinate, the
column's metadata, etc.). UI28 introduces **implicit template
bindings**: NAME tokens with reserved meanings recognised by Grid
emitters when they appear inside a Cell template child of Grid.

### 3.1 Reserved names inside the body-cell template

| Binding | Type | Source |
|---|---|---|
| `cell-value` | `text` | `viewport-rows[r][c]` |
| `cell-row` | `number` | Viewport-relative row index (`r`) |
| `cell-col` | `number` | Column index (`c`) |
| `cell-is-editing` | `bool` | `r === editRow && c === editCol` |
| `cell-is-selected` | `bool` | `r === selectedRow && c === selectedCol` |
| `column-key` | `text` | `columns[c].key` |
| `column-header` | `text` | `columns[c].header` |
| `column-width` | `number` | `columns[c].width` |
| `column-editable` | `bool` | `columns[c].editable` |
| `column-cell-type` | `text` | `columns[c].cellType` |
| `column-default-alignment` | `text` | `columns[c].defaultAlignment` |

### 3.2 Reserved names inside the header-cell template

| Binding | Type | Source |
|---|---|---|
| `column-key` | `text` | `columns[c].key` |
| `column-header` | `text` | `columns[c].header` |
| `column-width` | `number` | `columns[c].width` |
| `column-editable` | `bool` | `columns[c].editable` (always false-style render expected) |
| `column-index` | `number` | `c` |

### 3.3 Grammar fit

These names are plain `NAME` tokens — no moslayout grammar change.
The interpretation is purely a backend-emitter rule: when a `Cell` is
a child of a `Grid`, the emitter recognises these names in prop
bindings and substitutes the appropriate loop variable. Outside a
Grid, the same names are just unbound identifiers and produce a
compile error.

### 3.4 Worked example

```moslayout
Cell [body-cell] (
  value:      cell-value,       // → row.cells[c] / viewportRows[r][c]
  editable:   column-editable,  // → columns[c].editable
  cell-type:  column-cell-type, // → columns[c].cellType
  is-editing: cell-is-editing,  // → r === editRow && c === editCol
  onCommit:   emit: onEditCommit
)
```

After lowering by the React backend, this becomes (sketched):

```tsx
{viewportRows.map((row, r) =>
  <tr key={r}>
    {columns.map((col, c) =>
      <Cell
        value={row[c]}
        editable={col.editable}
        cellType={col.cellType}
        isEditing={r === editRow && c === editCol}
        dispatch={dispatch}
      />
    )}
  </tr>
)}
```

(In practice the Grid emitter inlines the Cell's body directly rather
than emitting a separate component, but the semantics are identical.)

---

## 4. Backend lowering tables

Each backend implements all three primitives; the Grid emitter on each
backend composes them.

### 4.1 React (`mosaic-emit-react`)

| Primitive | Standalone | Inside Grid |
|---|---|---|
| Cell | `<div>` with conditional `<input>` (`editable && isEditing`) | `<td>` with the same body |
| Column | Discarded with `// Column is metadata` warning | Read for `<colgroup>`, header row, and per-cell defaults |
| Grid | n/a (Grid is always a Grid) | `<table><colgroup>...</colgroup><thead>...</thead><tbody>...</tbody></table>` |

### 4.2 WebComponent (`mosaic-emit-webcomponent`)

| Primitive | Lowering |
|---|---|
| Cell | A `<mos-cell>` Custom Element with `value` / `editable` / `is-editing` attributes; emits CustomEvents for `commit` / `cancel` / `click` |
| Column | Discarded; metadata consumed by Grid |
| Grid | `<mos-grid>` Custom Element whose shadow root mounts the `<table>` and embeds `<mos-cell>` instances |

### 4.3 HTML (static) (`mosaic-emit-html`)

| Primitive | Lowering |
|---|---|
| Cell | `<span>` (standalone) / `<td>` (inside Grid) with the value as plain text. No editor. No dispatch (static HTML has no JS). |
| Column | Discarded; metadata consumed by Grid for `<colgroup>` and `<th>` |
| Grid | Plain `<table>` snapshot. Selection / editing slot values are baked into the static markup if present. |

### 4.4 SwiftUI (`mosaic-emit-swiftui` — new crate)

| Primitive | Lowering |
|---|---|
| Cell | A struct conforming to `View`. Body uses `if isEditing { TextField(...) } else { Text(...) }`. Click dispatches via the `dispatch` closure prop. |
| Column | Discarded as a SwiftUI view; metadata builds `TableColumn` definitions |
| Grid | `SwiftUI.Table(viewportRows) { TableColumn(...) { row in CellView(...) } }` — one `TableColumn` per Column metadata entry; the Cell template's body becomes the column's row-builder closure |

### 4.5 Qt (`mosaic-emit-qt` — new crate)

| Primitive | Lowering |
|---|---|
| Cell | A `QStyledItemDelegate` subclass: `paint()` draws the value + sub-part styling; `createEditor()` returns a `QLineEdit` when `editable && isEditing`; commits via the model's `setData()` |
| Column | A `QAbstractTableModel::headerData()` / column-metadata entry |
| Grid | `QTableView` with a `QAbstractTableModel` populated from `viewport-rows` and the per-Column delegate registered via `setItemDelegateForColumn` |

### 4.6 Paint-VM (`mosaic-emit-paint`)

| Primitive | Lowering |
|---|---|
| Cell | A `PaintCellInstruction` widget descriptor (resolved by the Widget Runtime layer, future UI-WR1) |
| Column | A `PaintColumnMetadata` entry inside the Grid's `PaintGridInstruction` |
| Grid | One `PaintGridInstruction` carrying the column metadata and row matrix |

### 4.7 Future backends

The same pattern extends naturally:

- **AppKit**: `NSTableView` with column-specific `NSTableCellView` subclasses
- **Jetpack Compose**: `LazyVerticalGrid` with `items()` that render the Cell composable
- **GTK**: `GtkColumnView` with per-column factory functions
- **WinUI / XAML**: `DataGrid` with `DataGridTextColumn` / `DataGridTemplateColumn`
- **Win32**: `ListView` (`LVS_REPORT` style) with owner-drawn cells
- **Terminal (TUI)**: ratatui `Table` with one row per `viewport-rows` entry

The Mosaic value prop: **the .mll/.mil/.msl files are written once**.
Each backend's emitter implements Cell + Column + Grid lowering, and
all the above platforms render the same source.

---

## 5. Migration plan

### 5.1 What changes in mosaic-emit-react

Existing `emit_grid_jsx` keeps working for v2-style layouts (no Column
children). For v3-style layouts:

1. Iterate `node.children` for `Column` and `Cell` (with part_names
   `body-cell` and `header-cell`) nodes.
2. Build `columns: Vec<ColumnMeta>` from Column children.
3. Generate `<colgroup>` from `columns.map(c => c.width)`.
4. Generate `<thead>` by walking the header-cell template once per column.
5. Generate `<tbody>` by nested `.map()` — outer over `viewportRows`,
   inner over `columns`, with the body-cell template instantiated per
   intersection.

### 5.2 What stays the same

- Sub-part styling (UI27 §3) — still works; the Cell primitive declares
  its own sub-parts (`cell`, `cell:selected`, etc.) that authors target
  in mosstyle.
- Slot overrides on Grid (UI27 §6) — `sticky-header`, `total-height`
  still apply to the Grid as a whole.
- The `state even` / `state odd` mechanism for row stripes (WA4) —
  attaches to `sheet/data-row` exactly as before; row-level styling
  is independent of cell decomposition.

### 5.3 code/programs/typescript/visicalc migration

The VisiCalc demo (UI26) migrates to v3 as follows:

1. `Grid.mil` removes `column-headers` slot; gains nothing else
   (columns are declared in `.mll`).
2. `Grid.desktop.mll` adds 26 `Column` children (one per A-Z) plus a
   row-number Column with `editable: false`; declares one `Cell
   [body-cell]` template and one `Cell [header-cell]` template.
3. `Grid.dark.msl` adds `part body-cell` and `part header-cell` blocks
   (replacing the implicit `sheet/cell` and `sheet/header-cell`
   sub-parts of v2). The state blocks (`even`/`odd`/`selected`/`editing`)
   move under those parts.
4. `App.tsx` no longer passes `columnHeaders` — Grid reads them from
   its Column children.

The host's reducer and useReducer logic don't change at all. State
shape (cells map, selection, edit state) is unchanged.

---

## 6. Out of scope for UI28

- **Column groups** (two-level headers spanning multiple columns).
  Defer; the Column primitive can grow a `parent-group: text` slot
  later.
- **Pinned columns** (left/right). Defer.
- **Row virtualisation** (only render visible rows of a million-row
  dataset). Defer.
- **Sort / filter affordances on column headers.** Defer; they'll
  ride on top of the HeaderCell template via additional slot bindings
  for sort direction and filter expression.
- **Tree data (parent/child rows)**. Defer.
- **Custom cell renderers beyond the built-in `cell-type` values**.
  Defer; today's `cell-type` enum is closed. A future spec opens it
  via either (a) a `cell-renderer: node` slot, or (b) a registry
  pattern.

---

## 7. Test additions per backend

Each backend emitter must add tests covering:

1. Standalone Cell — value rendering, editable false, is-editing true with editor visible, sub-part styling
2. Standalone Cell — onClick, onCommit, onCancel dispatch
3. Cell-type variations (text, number, image, checkbox, link)
4. Grid with Column children + body-cell template — generates table-equivalent structure
5. Grid with Column children + header-cell template — header row uses the template
6. Grid with Column.editable=false — body cells in that column never enter edit mode
7. Grid implicit bindings — `cell-value`, `column-header`, `column-editable` resolve correctly
8. Backwards-compat: Grid with no Column children falls back to v2 monolithic behaviour
9. Standalone Column (outside Grid) emits the "Column is metadata" warning

---

## 8. Implementation roadmap (rolls up to the WB* loop)

| ID | Work | Owner |
|---|---|---|
| WB0 | This spec (UI28) | spec-only |
| WB1 | `mosaic-emit-react`: Cell + Column + Grid v3 + 9 tests | React backend |
| WB2 | `mosaic-emit-webcomponent`: Cell + Column + Grid v3 + tests | WebComponent backend |
| WB3 | `mosaic-emit-html`: Cell + Column + Grid v3 + tests | HTML backend |
| WB4 | `mosaic-emit-swiftui` (new crate): Cell + Column + Grid v3 + tests | SwiftUI backend |
| WB5 | `mosaic-emit-qt` (new crate): Cell + Column + Grid v3 + tests | Qt backend |
| WB6 | `mosaic-compile`: wire `--backend swiftui` and `--backend qt` flags | CLI |
| WB7 | `code/programs/typescript/visicalc`: migrate to Cell-centric shape | demo |
| WB8 | `code/programs/swift/visicalc-swiftui`: new SwiftPM demo using the same `.mil`/`.mll`/`.msl` | demo |
| WB9 | `code/programs/cpp/visicalc-qt`: new Qt demo using the same `.mil`/`.mll`/`.msl` | demo |

**WB1 through WB5 run in parallel** — each touches a different
backend crate; no file overlap. WB6 follows once WB4 and WB5 land.
WB7-WB9 follow once their respective backend lands.

---

## 9. Relationship to other specs

- **UI14 moslayout**: §11 (Grid primitive vocabulary) gains Cell and
  Column as siblings. The grammar doesn't change; both Cell and Column
  are valid NAME tokens under the existing `node` rule.
- **UI15 mosstyle**: no change. Sub-parts on Cell (`cell`,
  `cell:selected`, `cell:editing`, `cell:read-only`) follow the same
  path syntax already used for Grid sub-parts.
- **UI24 emit→dispatch**: Cell's emits (`onClick`, `onCommit`,
  `onCancel`) follow the same Flux pattern; payloads carry as union
  variants on the host-side `dispatch`.
- **UI27 Grid v2**: §5 (sub-parts) and §6 (slots) remain valid; the
  Cell primitive's sub-part vocabulary is the natural successor to
  Grid's `sheet/cell` sub-part.
- **UI26 visicalc**: WB7 updates the visicalc spec section §3.1 / §6.2
  to reflect the v3 shape; no other UI26 changes.
- **UI21 mosaic-emit-qt**: the existing Qt spec is now the parent
  document for WB5; UI28 fills in the Cell/Column/Grid lowering rows.
