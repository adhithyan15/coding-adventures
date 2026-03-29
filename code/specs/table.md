# Table

## Overview

The Table component renders tabular data with two interchangeable rendering backends:
an **HTML backend** built on semantic `<table>` elements, and a **Canvas backend** that
draws directly to a 2D canvas context. Both backends consume the same data model and
props, so switching between them is a single prop change.

Why two backends? HTML tables are accessible, semantic, and simple. But they hit a wall
at a few thousand rows — the browser has to create a DOM node for every cell, and
layout recalculation becomes expensive. Canvas draws pixels directly, bypassing the DOM
entirely. A Canvas table with virtual scrolling can handle millions of rows at 60fps
because it only draws what's visible.

This spec covers V1: a static, read-only table. Future versions will add virtual
scrolling, cell editing, sorting, filtering, selection, and copy/paste — evolving
toward a spreadsheet-like interface.

## Position in the Stack

```
[Packages]  ui-components
                └── Table (this component)
                      ├── DataTable (HTML backend)
                      └── CanvasTable (Canvas backend)
```

**Input from:** Application data — arrays of objects, column definitions.
**Output to:** The browser — either DOM elements or canvas pixels.

## Concepts

### The Data Model

A table has two dimensions: **columns** (the schema) and **rows** (the data). The
column definitions describe _what_ to show. The row data describes _what values_ exist.

```
┌──────────────────────────────────────────────┐
│  Column 1     Column 2     Column 3          │  ← ColumnDef[]
├──────────────────────────────────────────────┤
│  row[0].a     row[0].b     row[0].c          │  ← data[0]
│  row[1].a     row[1].b     row[1].c          │  ← data[1]
│  row[2].a     row[2].b     row[2].c          │  ← data[2]
│  ...          ...          ...               │
└──────────────────────────────────────────────┘
```

A **ColumnDef** tells the table:
1. What text to display in the header (`header`)
2. How to extract the cell value from a row (`accessor`)
3. How wide the column should be (`width`)
4. How to align the cell text (`align`)
5. A stable identity for the column (`id`)

The `accessor` can be either a property key or a function. Property keys are simpler:

```typescript
// Property key accessor — reads row["name"]
{ id: "name", header: "Name", accessor: "name" }

// Function accessor — computes a derived value
{ id: "fullName", header: "Full Name", accessor: (row) => `${row.first} ${row.last}` }
```

Function accessors enable computed columns (concatenation, formatting, conditional
display) without requiring the source data to be pre-transformed.

### Cell Value Resolution

Both backends need to convert a `(column, row)` pair into a display string. This is
the `resolveCellValue` utility:

```
resolveCellValue(column, row, rowIndex)
  │
  ├─ accessor is a function?
  │     → call accessor(row, rowIndex)
  │     → String(result ?? "")
  │
  └─ accessor is a property key?
        → read row[accessor]
        → String(result ?? "")
```

The `?? ""` handles null/undefined values gracefully — they render as empty cells
rather than the string "undefined".

### Row Identity

Each row needs a stable identity for React reconciliation. The `rowKey` function maps a
row to a unique string or number:

```typescript
// Default: use the array index (fragile if data is reordered)
rowKey = (row, index) => index

// Better: use a unique field from the data
rowKey = (row) => row.id
```

V1 defaults to index-based keys. When sorting and filtering are added in future
versions, stable row keys become critical — they prevent React from re-mounting rows
that merely changed position.

## HTML Backend (DataTable)

The HTML backend renders a standard `<table>` element with full semantic markup. This
is the accessible, SEO-friendly, and default choice.

### DOM Structure

```
<div class="table" role="region" aria-label="..." tabindex="0">
  <table class="table__grid">
    <caption class="table__caption">Optional caption</caption>
    <thead class="table__head">
      <tr class="table__row table__row--header">
        <th class="table__cell table__cell--header table__cell--align-left"
            scope="col" style="width: 200px">
          Name
        </th>
        <th class="table__cell table__cell--header table__cell--align-right"
            scope="col">
          Age
        </th>
      </tr>
    </thead>
    <tbody class="table__body">
      <tr class="table__row" key={rowKey(row, 0)}>
        <td class="table__cell table__cell--align-left">Alice</td>
        <td class="table__cell table__cell--align-right">30</td>
      </tr>
    </tbody>
  </table>
</div>
```

### Why the Wrapping `<div>`?

The outer `<div role="region" tabindex="0">` serves two purposes:

1. **Scrollable container** — when the table overflows horizontally, this div scrolls
   independently. Without it, the entire page would scroll.

2. **Keyboard scrollable** — `tabindex="0"` makes the container focusable, so keyboard
   users can scroll with arrow keys when the table overflows.

### BEM Class Naming

Following the existing ui-components convention:

| Class                           | Element    | Purpose                          |
|---------------------------------|------------|----------------------------------|
| `.table`                        | wrapper    | Outer region container           |
| `.table__grid`                  | `<table>`  | The actual table element         |
| `.table__caption`               | `<caption>`| Optional table caption           |
| `.table__head`                  | `<thead>`  | Header row group                 |
| `.table__body`                  | `<tbody>`  | Body row group                   |
| `.table__row`                   | `<tr>`     | A table row                      |
| `.table__row--header`           | `<tr>`     | Header row modifier              |
| `.table__cell`                  | `<th>/<td>`| A table cell                     |
| `.table__cell--header`          | `<th>`     | Header cell modifier             |
| `.table__cell--align-left`      | cell       | Left-aligned text (default)      |
| `.table__cell--align-center`    | cell       | Center-aligned text              |
| `.table__cell--align-right`     | cell       | Right-aligned text               |

### Accessibility

The HTML backend is inherently accessible because it uses native `<table>` semantics.
Screen readers already understand `<thead>`, `<th scope="col">`, `<tbody>`, `<td>`.
No additional ARIA markup is needed beyond `role="region"` and `aria-label` on the
scrollable container.

## Canvas Backend (CanvasTable)

The Canvas backend draws the table directly to a `<canvas>` element using the 2D
rendering context. This is the performance-optimized path for large datasets.

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│ <div class="table table--canvas" role="grid">           │
│                                                         │
│   ┌──────────────────────────────────────────────┐      │
│   │ <canvas aria-hidden="true">                  │      │  Visual layer
│   │   Draws: header bg, text, grid lines,        │      │  (what sighted
│   │          cell text, alternating row bg        │      │   users see)
│   └──────────────────────────────────────────────┘      │
│                                                         │
│   ┌──────────────────────────────────────────────┐      │
│   │ <div class="table__a11y-overlay">            │      │  Accessibility
│   │   ARIA grid: role="row", role="gridcell"     │      │  layer (what
│   │   Positioned to match canvas cells           │      │  screen readers
│   │   Keyboard navigation target                 │      │  and keyboards
│   └──────────────────────────────────────────────┘      │  traverse)
│                                                         │
└─────────────────────────────────────────────────────────┘
```

The canvas is `aria-hidden="true"` — it is purely visual. All accessibility comes from
the ARIA grid overlay.

### Canvas Rendering Pipeline

The rendering pipeline runs in a `useEffect` that depends on data, columns, theme, and
container dimensions:

```
1. Clear canvas
2. Compute layout
   ├── Column widths: explicit widths first, then distribute remaining space equally
   ├── Row height: fixed value derived from font size (e.g., fontSize * 2)
   └── Total height: headerHeight + (rowCount * rowHeight)
3. Scale for DPR (devicePixelRatio)
4. Draw header
   ├── Fill header background rect
   └── Draw header text (bold, clipped to column width)
5. Draw body rows
   ├── Fill alternating row backgrounds
   └── Draw cell text (clipped to column width, aligned per column)
6. Draw grid lines
   ├── Vertical lines at column boundaries
   └── Horizontal lines at row boundaries
```

### Device Pixel Ratio (DPR)

On high-DPI displays (retina), a CSS pixel maps to 2 or 3 physical pixels. If the
canvas is not scaled, text and lines appear blurry. The fix:

```
Canvas element:  width=800  height=600   (CSS size — what the layout engine uses)
Canvas buffer:   width=1600 height=1200  (physical size — actual pixel buffer)
2D context:      scale(2, 2)             (draw at 2x, so logical coordinates stay same)
```

This way, `ctx.fillText("Hello", 10, 20)` draws at logical position (10, 20) but
renders at physical pixel (20, 40), giving crisp text on retina displays.

### Text Clipping

Cell text that overflows its column must be clipped, not allowed to bleed into adjacent
columns. The Canvas API provides path-based clipping:

```
ctx.save();
ctx.beginPath();
ctx.rect(cellX, cellY, columnWidth, rowHeight);
ctx.clip();
ctx.fillText(text, textX, textY);
ctx.restore();
```

### Theme Bridge (useCanvasTheme)

The Canvas API doesn't read CSS. The `useCanvasTheme` hook bridges this gap by reading
CSS custom properties from a mounted DOM element via `getComputedStyle`:

```
Container <div> mounts
  → useEffect fires
    → getComputedStyle(container)
      → Read --body-bg, --body-text, --panel-bg, --panel-border, etc.
        → Return typed CanvasTheme object
          → Canvas useEffect draws with these colors
```

This keeps the Canvas backend visually consistent with the HTML backend and the rest
of the dark theme without duplicating color values.

### Responsive Sizing (ResizeObserver)

The canvas must resize when its container changes dimensions (window resize, sidebar
toggle, etc.). A `ResizeObserver` watches the container `<div>` and updates the canvas
dimensions on change.

In jsdom (test environment), `ResizeObserver` is not available. The component guards
with `typeof ResizeObserver !== "undefined"` and falls back to a fixed size.

## Canvas Accessibility: ARIA Grid Overlay

Canvas is inherently inaccessible — it's a bitmap. Screen readers cannot traverse
individual cells, and keyboard navigation doesn't exist. The ARIA grid overlay solves
both problems.

### Overlay Structure

```html
<div class="table__a11y-overlay">
  <div role="rowgroup">                              <!-- header group -->
    <div role="row" aria-rowindex="1">
      <div role="columnheader" aria-colindex="1">Name</div>
      <div role="columnheader" aria-colindex="2">Age</div>
    </div>
  </div>
  <div role="rowgroup">                              <!-- body group -->
    <div role="row" aria-rowindex="2">
      <div role="gridcell" aria-colindex="1" tabindex="-1">Alice</div>
      <div role="gridcell" aria-colindex="2" tabindex="-1">30</div>
    </div>
    <div role="row" aria-rowindex="3">
      <div role="gridcell" aria-colindex="1" tabindex="-1">Bob</div>
      <div role="gridcell" aria-colindex="2" tabindex="-1">25</div>
    </div>
  </div>
</div>
```

### How It Works

1. **Positioned to match canvas cells** — each overlay div is absolutely positioned at
   the same (x, y, width, height) as its corresponding canvas cell. The overlay is
   transparent so the canvas shows through.

2. **Screen reader traversal** — screen readers treat `role="grid"` with
   `role="row"` / `role="columnheader"` / `role="gridcell"` as a native table. Users
   can navigate with table commands (Ctrl+Alt+Arrow on NVDA, VO+Arrow on VoiceOver).

3. **Keyboard navigation** — the `useGridKeyboard` hook handles:
   - Arrow keys: move focus between cells
   - Home / End: first / last cell in the current row
   - Ctrl+Home / Ctrl+End: first cell in table / last cell in table
   - Tab: exit the grid entirely (standard grid pattern)

4. **Focus management** — exactly one gridcell has `tabindex="0"` (the focused cell);
   all others have `tabindex="-1"`. This is the same roving tabindex pattern used by
   the existing `TabList` component.

5. **Dimension declarations** — `aria-rowcount` and `aria-colcount` on the grid root
   declare the total dimensions. `aria-rowindex` and `aria-colindex` on each row/cell
   give the absolute position. This is critical for future virtualization: when only a
   window of rows is in the DOM, `aria-rowcount` tells assistive tech the true total.

### Why Not a Hidden `<table>`?

A hidden `<table>` inside the `<canvas>` would provide read-only screen reader support,
but it cannot support keyboard navigation or future interactive features (cell
selection, editing). The ARIA grid overlay is more work up front, but it provides a
foundation that grows with the component.

## Unified Entry Point (Table)

The `Table` component is a thin router:

```typescript
function Table<T>({ renderer = "html", ...props }: TableProps<T>) {
  if (renderer === "canvas") return <CanvasTable {...props} />;
  return <DataTable {...props} />;
}
```

Consumers can also import `DataTable` or `CanvasTable` directly to bypass the router.
This is useful when the renderer is known at compile time and tree-shaking should
eliminate the unused backend.

## V1 Scope

What V1 includes:
- Static, read-only table rendering
- HTML and Canvas backends
- Column definitions with id, header, accessor, width, alignment
- Row key function for React reconciliation
- Full accessibility (semantic HTML for DataTable, ARIA grid overlay for CanvasTable)
- Keyboard navigation for Canvas backend
- Dark theme styling (CSS for HTML, theme bridge for Canvas)
- DPR-aware Canvas rendering

What V1 does NOT include (future versions):
- Virtual scrolling (render only visible rows)
- Column sorting (click header to sort)
- Column filtering
- Cell selection (single, range, multi)
- Cell editing (inline input overlay)
- Column resizing (drag header border)
- Column reordering (drag header)
- Copy/paste (Ctrl+C from selection)
- Row grouping / collapsing
- Frozen columns / rows (sticky headers)
- Custom cell renderers (render React nodes in cells)

## Implementation Notes

### TypeScript Generics

The `Table`, `DataTable`, and `CanvasTable` components are all generic over `T`, the
row type. This means column accessors are type-checked against the actual data shape:

```typescript
interface Person { name: string; age: number; }

// This compiles:
<Table<Person> columns={[{ id: "name", header: "Name", accessor: "name" }]} data={people} />

// This is a type error — "email" is not keyof Person:
<Table<Person> columns={[{ id: "email", header: "Email", accessor: "email" }]} data={people} />
```

### Performance Considerations for Future Versions

The V1 architecture is designed so that virtualization can be added without breaking
changes:

- `ColumnDef.id` provides stable column identity for sort/filter state
- `rowKey` provides stable row identity for selection state
- `aria-rowcount` / `aria-colcount` already declare total dimensions
- The Canvas rendering pipeline is structured as a series of draw calls that can be
  windowed (only draw rows in the visible range)
- The ARIA grid overlay can be virtualized in parallel (only create divs for visible
  rows)
