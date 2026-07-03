# UI27 — Grid v2: Sub-Parts, Slot Overrides, and Visual Primitives

**Status:** Specification (draft)
**Layer:** UI / cross-cutting (mosstyle + moslayout + emitters)
**Depends on:** UI13 (mosmodel), UI14 (moslayout), UI15 (mosstyle), UI24 (emit→dispatch), UI26 (visicalc)

---

## 1. Purpose

The Grid primitive shipped in the first VisiCalc demo had three visible
gaps that block real-world spreadsheet use:

1. **No grid lines.** Mosstyle could style the `<table>` itself (the
   author-named `[sheet]` part) but had no way to reach into the cells
   the Grid emitter synthesises internally. The author has no
   `part_name` to target for `<td>` / `<th>` / `<tr>`, so cells render
   with no borders.
2. **No column widths or row heights.** Cells render at flex-default
   sizes; columns wobble as data widens; there is no way to declare
   "header row is 24 px tall" or "column A is 60 px wide".
3. **No alternating row stripes, no sticky header.** Both are
   table-of-stakes UX in any modern grid; both need a way for styling
   to participate in emitter-internal structure.

This spec resolves all three with one underlying mechanism — **mosstyle
sub-parts** — plus a small set of **slot-driven overrides** that let
the host pass styling at runtime (so a parent app's theme can flow into
a Grid without re-authoring its `.msl` file).

The same mechanism generalises to every other primitive that has
emitter-internal structure (`Input`, future `Combobox`, future
`DatePicker`, etc.). Grid is just the first user.

---

## 2. Two-layer cascade: defaults below, overrides above

Every styled property on a Grid (and any future primitive with
sub-parts) is resolved in this order, **first match wins**:

```
1. Slot value          (runtime, from the host)
2. Mosstyle sub-part   (design-time, from .msl)
3. Mosstyle parent part inheritance  (design-time, from the .msl that styles the wrapping part)
4. Built-in primitive default        (compiled into the emitter)
```

### Why this shape

- **Sub-part defaults** belong in mosstyle because they are *theme-time*
  decisions ("in the dark theme, cells have a 1 px `#3f3f46` border").
  They live with the rest of the style code, get versioned with the
  style file, and get swapped out when the host picks a different
  theme variant.
- **Slot overrides** exist for *application-time* dynamism. The host
  application may want all Grids in the app to share a single
  accent colour driven by user preference, or to flow per-cell colours
  from a heatmap computation. Those values aren't known at theme-build
  time — they need to ride in as slot data.
- **Slot wins over mosstyle** because slots are explicitly passed by
  the host — if the host passes a value, the host meant it. Mosstyle
  is the implicit fallback.

This is the same principle as CSS specificity inverted: explicit data
flow beats stylesheets, stylesheets beat built-ins.

---

## 3. Sub-parts in mosstyle

### 3.1 Syntax addition

A `part` block may declare sub-parts using a slash-separated path. The
slash means "the sub-part named on the right, of the parent part on
the left".

```mosstyle
style Grid {
  part sheet {
    background:  #1e1e1e ;
    color:       #cccccc ;
  }

  part sheet/cell {
    border-bottom-width: 1px ;
    border-bottom-style: solid ;
    border-bottom-color: #3f3f46 ;
    padding:             4px ;
  }

  part sheet/header-cell {
    background:  #2d2d30 ;
    font-weight: bold ;
    padding:     4px ;
  }

  part sheet/data-row {
    background:  #1e1e1e ;
  }
}
```

A sub-part is just a normal part block whose name happens to contain a
`/`. The grammar change is one rule:

```diff
- part_def = KW_PART NAME LBRACE { part_item } RBRACE ;
+ part_def = KW_PART part_path LBRACE { part_item } RBRACE ;
+ part_path = NAME ( SLASH NAME )* ;
```

The slash is a new token in the mosstyle tokens file.

### 3.2 Cascade between parent and sub-part

A property declared on a parent (`part sheet`) is **inherited** by all
its sub-parts unless the sub-part overrides it. This matches CSS
inheritance for `color` / `font-family` and is more general than the
CSS subset (here `background` inherits too, because we are not using
real CSS rules — we are resolving to a flat property bag per element
at emit time).

Inheritance order, lowest-precedence first:

1. Parent's `part X` properties
2. Sub-part's `part X/Y` properties
3. Slot override (see §4)

### 3.3 Sub-part validation

A sub-part path is only valid if every leaf segment is declared by the
primitive whose top-level part it targets. Each Mosaic primitive
declares its sub-part vocabulary as part of its layout spec; the
mosstyle compiler validates against this list and rejects unknown
sub-parts with a clear error:

```
mosstyle: unknown sub-part 'foo' of primitive Grid at FormulaBar.dark.msl:9
  Grid exposes: cell, header-cell, header-row, data-row, selected-cell, editing-cell
  did you mean 'header-cell'?
```

### 3.4 Backend agnosticism

The mosstyle compiler emits a *resolved style map* per part-path. Each
backend emitter consumes the map and inlines the properties wherever
that sub-part lives in its native representation:

| Backend | How sub-parts apply |
|---|---|
| React (TSX) | Inlined into each `<td>` / `<th>` / `<tr>` `style={{...}}` attribute |
| WebComponent (JS) | Same, into the shadow-root template |
| HTML (static) | Inlined `style="..."` attributes |
| SwiftUI (future) | Each cell `View` receives `.background()` / `.foregroundColor()` modifiers |
| Qt (future) | Per-cell `QStyleOption` overrides, or a delegate that paints the resolved style |
| paint-vm | Each cell's `PaintRect` instruction carries the resolved fill / border |

Authors **do not write** platform-specific selectors (`:hover`,
`:nth-child(even)`). Sub-parts that depend on dynamic state are
declared at the primitive level (e.g., `selected-cell`,
`editing-cell`) and the emitter chooses how to apply them on each
platform.

---

## 4. Slot overrides on Grid

The Grid primitive gains a set of *optional* styling-related slots
that, when set, override the mosstyle sub-part value for the
corresponding property. All are nullable / defaulted; if the slot is
not bound by the host, the mosstyle value (or built-in default) wins.

| Slot | Type | Default | Maps to |
|---|---|---|---|
| `cell-border-color` | `color` | (from mosstyle) | `part sheet/cell.border-color` |
| `cell-border-width` | `number` (px) | (from mosstyle) | `part sheet/cell.border-width` |
| `cell-padding` | `number` (px) | (from mosstyle) | `part sheet/cell.padding` |
| `header-bg-color` | `color` | (from mosstyle) | `part sheet/header-cell.background` |
| `header-fg-color` | `color` | (from mosstyle) | `part sheet/header-cell.color` |
| `row-stripe-color` | `color` | (from mosstyle) | `part sheet/data-row:even.background` |
| `selected-bg-color` | `color` | (from mosstyle) | `part sheet/selected-cell.background` |
| `editing-bg-color` | `color` | (from mosstyle) | `part sheet/editing-cell.background` |

All are typed slots in mosmodel terms, so the compiler enforces colour
strings vs numbers. None is required.

---

## 5. Grid v2 sub-part vocabulary

The Grid primitive (UI14 §11) is amended to expose the following
sub-parts. The Grid emitter is responsible for applying the resolved
style for each at the right place in the generated output.

| Sub-part path | Applies to | Notes |
|---|---|---|
| `<grid>/cell` | every body `<td>` | Default body cell |
| `<grid>/header-cell` | every `<th>` in the header row | Author can style independently of body cells |
| `<grid>/header-row` | the `<tr>` in `<thead>` | Sets row-level background, height, sticky behaviour |
| `<grid>/data-row` | every `<tr>` in `<tbody>` | Sets default body row background and height |
| `<grid>/data-row:even` | every other body `<tr>` (zero-indexed) | The alternating-row sub-part. Authors write `part sheet/data-row:even { background: #252526 }` |
| `<grid>/data-row:odd` | the others | Symmetric counterpart |
| `<grid>/selected-cell` | the body `<td>` matching `selected-row` + `selected-col` | Replaces today's hard-coded `#264f78` highlight |
| `<grid>/editing-cell` | the body `<td>` matching `edit-row` + `edit-col` | Replaces today's hard-coded `#1f4f3f` highlight |

The `:even` / `:odd` syntax is a small extension to sub-part paths —
treated as a state suffix, similar to mosstyle's existing `state hover`
blocks. Implementation detail: the emitter expands them inline when
walking `viewportRows.map((row, r) => ...)` based on `r % 2`.

---

## 6. New Grid v2 slots for layout dimensions

Sub-parts cover *appearance* but not *dimensions* (which depend on
data — number of columns, density of content). These come in as
additional Grid slots:

| Slot | Type | Default | Effect |
|---|---|---|---|
| `column-widths` | `list<number>` | (auto) | One number per column, in pixels; emitter inlines as `<col style="width:N">` or platform equivalent |
| `row-height` | `number` (px) | (auto) | Uniform body row height; emitter sets `<tr style="height:N">` |
| `header-height` | `number` (px) | (auto) | Header row height |
| `sticky-header` | `bool` | `false` | When `true`, header row remains visible while body scrolls |
| `total-width` | `number` (px) | (auto) | Optional max table width; otherwise table fills its parent |
| `total-height` | `number` (px) | (auto) | Optional max table height — required for `sticky-header` to work (scroll requires a bounded container) |

When `sticky-header: true` is bound, the emitter:
- React: wraps `<table>` in a scroll container, applies `position: sticky; top: 0` to `<thead>`
- SwiftUI: uses `Table` with `.pinnedViews([.sectionHeaders])` equivalent
- Compose: uses `LazyVerticalGrid` with `stickyHeader` block
- paint-vm: emits a scroll-clipped region with the header drawn last

---

## 7. Grid emitter changes (mosaic-emit-react)

The pipeline emitter's `emit_grid_jsx` function changes as follows.

### 7.1 Resolve sub-part styles per element kind

A new helper `resolve_subpart_style(sheet_part_name, "cell", style_def)`
walks the mosstyle `StyleDef` to find a part block named
`sheet_part_name + "/cell"` and returns its property bag (inheriting
properties from the parent `sheet_part_name` block).

### 7.2 Apply per-cell

The existing per-cell `style={{...}}` expression grows to include the
sub-part defaults, plus the slot-override layer:

```tsx
<td key={c}
    style={{
      ...{ /* sheet/cell defaults */ },
      ...(r % 2 === 0 ? { /* sheet/data-row:even */ } : { /* sheet/data-row:odd */ }),
      ...(r === selectedRow && c === selectedCol ? { /* sheet/selected-cell */ } : {}),
      ...(r === editRow && c === editCol ? { /* sheet/editing-cell */ } : {}),
      // Slot overrides come last so they win
      ...(cellBorderColor != null ? { borderColor: cellBorderColor } : {}),
      ...(cellPadding != null ? { padding: `${cellPadding}px` } : {}),
    }}
    onClick={...}>{cell}</td>
```

### 7.3 Apply at the `<table>` and `<thead>` level

`<thead>` styles inline from `sheet/header-row`; each `<th>` inlines
from `sheet/header-cell`. When `sticky-header` is true, `<thead>`
additionally gets `{ position: "sticky", top: 0 }`.

When `total-width` / `total-height` are bound, the wrapping container
gets `max-width` / `max-height`. Without `total-height`, sticky-header
has no effect — emit a developer-visible console warning in dev mode
(`console.warn` inside the component).

### 7.4 Apply `column-widths`

Emit a `<colgroup>` inside the `<table>`, one `<col>` per width entry.
Missing entries (when `column-widths.length < columnHeaders.length`)
fall back to `auto`.

---

## 8. Mosstyle compiler changes

| Change | Where |
|---|---|
| Add `SLASH` and `:` tokens | `mosstyle.tokens` |
| Update `part_def` to allow `part_path` (`NAME (SLASH NAME)*` followed by optional `:state-suffix`) | `mosstyle.grammar` |
| Extend `PartStyle` IR to carry the full path as a list of segments | `mosstyle-compiler/src/lib.rs` |
| Add primitive sub-part vocabulary table — for now hard-coded for Grid / Input, generalised later | `mosstyle-compiler/src/lib.rs` |
| Validate every sub-part path against the vocabulary | `mosstyle-compiler/src/lib.rs` |
| Surface unknown-sub-part errors with `did you mean` suggestions | `mosstyle-compiler/src/lib.rs` |

### Backward compatibility

Existing `.msl` files that don't use sub-parts continue to compile
unchanged — they just declare bare `part name { ... }` blocks. The
slash-path syntax is purely additive.

---

## 9. mosaic-emit-react `pipeline.rs` changes

| Function | Change |
|---|---|
| `build_part_style_map` | Build a *tree* of sub-part fragments keyed by the full slash-path. |
| `merge_styles` | Already exists. Reuse for layering sub-part + slot-override fragments. |
| `emit_grid_jsx` | Read `cell`, `header-cell`, `header-row`, `data-row[:even/:odd]`, `selected-cell`, `editing-cell` sub-parts. Inline at the right element. |
| `emit_grid_jsx` | Read new dimensional slots (`column-widths`, `row-height`, `header-height`, `sticky-header`, `total-width`, `total-height`); emit `<colgroup>`, set heights, wrap in scroll container as appropriate. |
| `emit_grid_jsx` | Read new override slots (`cell-border-color`, etc.); emit conditional spread expressions in the style. |

---

## 10. Backward compatibility for existing demos

The current VisiCalc demo's `Grid.dark.msl` still works — it only uses
the top-level `part sheet { ... }` block. The Grid will continue to
render without grid lines until the `.msl` file is updated to declare
`part sheet/cell` and friends. WA6 in the implementation queue is
exactly that update.

The current Grid `selected-cell` and `editing-cell` hard-coded colours
(`#264f78` / `#1f4f3f`) remain as built-in defaults when neither the
mosstyle nor a slot overrides them. This means existing host code
that just passes selection slots continues to see selection
highlighting even without an updated `.msl`.

---

## 11. Out of scope for UI27

- **General CSS pseudo-class support** (`:hover`, `:focus`, `:active`).
  Already partially handled by mosstyle's `state` blocks (UI15 §3);
  not changed here.
- **Container queries / responsive sub-parts**. Future.
- **Slot-driven sub-part *injection*** (the host passing arbitrary
  styles for un-declared sub-parts). Slots can only override
  *declared* properties. Adding new sub-parts requires a primitive
  spec change.
- **Virtualisation** (only-render-visible rows). UI27 lays the
  groundwork by introducing `total-height` and `sticky-header`, but
  actual virtual scrolling — emit `onViewportChange(firstRow, lastRow)`,
  let host return only visible rows — is a follow-up (tracked as
  UI28 in a future spec).
- **Column resize / reorder / sort**. ag-grid-class interactions —
  tracked as the C* items in the post-UI27 roadmap.

---

## 12. Test additions

For `mosstyle-compiler`:
- Parse `part X/Y { ... }`
- Parse `part X/Y:even { ... }`
- Reject `part X/UnknownSubPart { ... }` with helpful error
- Parent properties inherit into sub-part lookup
- Sub-part overrides parent on collision

For `mosaic-emit-react::pipeline`:
- Grid emits `<colgroup>` when `column-widths` slot is bound
- Grid emits `height: Npx` on `<tr>` when `row-height` is bound
- Grid emits scroll wrapper + `position: sticky` `<thead>` when `sticky-header: true`
- Per-cell style merges `sheet/cell` defaults
- Per-row style alternates between `data-row:even` / `data-row:odd`
- `cellBorderColor` slot wins over mosstyle `sheet/cell.border-color`
- `selected-bg-color` slot wins over mosstyle `sheet/selected-cell.background`
- Missing `total-height` with `sticky-header: true` emits a runtime warning

For `code/programs/typescript/visicalc/`:
- Updated `Grid.dark.msl` declares all the sub-parts
- Updated `App.tsx` removes the `@ts-expect-error` (depends on WA1)
- Vite dev server smoke-test renders the grid with visible borders

---

## 13. Implementation order (rolls up to the WA* loop)

| ID | Work | Spec? | Implementation |
|---|---|---|---|
| WA1 | `list<list<T>>` in mosmodel-compiler | No — covered by UI13 already | mosmodel-compiler + mosaic-emit-react |
| WA2 | Mosstyle sub-part parsing + Grid sub-part lowering | This spec (§3, §5, §7, §8, §9) | mosstyle-compiler + mosaic-emit-react |
| WA3 | `column-widths` + `row-height` + `header-height` slots | This spec (§6) | mosaic-emit-react |
| WA4 | Row stripes (`data-row:even` sub-part) | This spec (§5 row) | mosstyle-compiler + mosaic-emit-react |
| WA5 | Sticky header + scroll container | This spec (§6, §7.3) | mosaic-emit-react |
| WA6 | Update `code/programs/typescript/visicalc/` to use everything | n/a | code/programs/typescript/visicalc/** |

Each WA item lands as a single focused PR; the autonomous loop
sequences them.
