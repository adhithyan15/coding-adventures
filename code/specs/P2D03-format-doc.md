# P2D03 — FormatDoc: Document Algebra, Layout Tree, and Paint Bridge

## Overview

`format-doc` is the pretty-printing layer that sits above paint. It does not
emit strings directly. Instead it builds a backend-neutral `Doc` tree and
realizes that tree into a backend-neutral `DocLayoutTree`.

The concrete output path for v1 is:

```text
AST + trivia + formatter rules
  -> Doc
  -> DocLayoutTree
  -> PaintScene
  -> paint-vm-ascii
  -> terminal string
```

This is the important architectural rule:

- `Doc` is the stable semantic IR
- `DocLayoutTree` is the first realized layout form
- `PaintScene` is the first concrete rendering scene
- terminal text comes from `paint-vm-ascii`, not from a direct string backend

That keeps the formatter compatible with future output targets such as:

- canvas
- svg
- editor-native paint pipelines
- streamed render data over the network

---

## Package split

### `@coding-adventures/format-doc`

Owns:

- `Doc` node types
- combinators (`text`, `group`, `indent`, `line`, ...)
- the width-fitting realization algorithm
- the `DocLayoutTree` data structure

### `@coding-adventures/format-doc-to-paint`

Owns:

- `docLayoutToPaintScene(layout) -> PaintScene`
- `docToPaintScene(doc, layoutOptions, paintOptions) -> PaintScene`

This split mirrors the existing layout and paint stacks:

```text
layout-ir          -> layout-to-paint
format-doc         -> format-doc-to-paint
```

---

## Core Doc Types

The v1 `Doc` algebra is intentionally small.

```typescript
type Doc =
  | { kind: "nil" }
  | { kind: "text"; value: string }
  | { kind: "concat"; parts: Doc[] }
  | { kind: "group"; content: Doc }
  | { kind: "indent"; levels: number; content: Doc }
  | { kind: "line"; mode: "soft" | "normal" | "hard" }
  | { kind: "if_break"; broken: Doc; flat: Doc }
  | { kind: "annotate"; annotation: DocAnnotation; content: Doc }
```

### Meaning of the primitives

| Primitive | Meaning |
|---|---|
| `text("foo")` | Emit literal text |
| `concat([a, b, c])` | Emit child docs in sequence |
| `group(d)` | Try to print `d` flat; if it does not fit, print it broken |
| `indent(d)` | Increase indentation for broken lines inside `d` |
| `line()` | Space when flat, newline when broken |
| `softline()` | Empty when flat, newline when broken |
| `hardline()` | Always newline |
| `ifBreak(broken, flat)` | Emit `broken` in broken mode, otherwise `flat` |
| `annotate(meta, d)` | Attach metadata to emitted spans without changing layout |

The intentionally missing v1 features are:

- `fill`
- `lineSuffix`
- `breakParent`
- `align`

These can be added later without changing the execution model.

---

## DocLayoutTree

`format-doc` realizes a `Doc` into a line-oriented layout tree.

```typescript
interface DocLayoutSpan {
  column: number;
  text: string;
  annotations: DocAnnotation[];
}

interface DocLayoutLine {
  row: number;
  indentColumns: number;
  width: number;
  spans: DocLayoutSpan[];
}

interface DocLayoutTree {
  printWidth: number;
  indentWidth: number;
  lineHeight: number;
  width: number;
  height: number;
  lines: DocLayoutLine[];
}
```

This layout tree is intentionally simple:

- coordinates are in monospace cell units
- each line has a row index
- each span has a starting column
- width and height are concrete layout extents

This is enough to drive `paint-vm-ascii` today while still being a proper
layout stage that later backends can refine.

---

## Layout options

```typescript
interface LayoutOptions {
  printWidth: number;
  indentWidth?: number; // default 2
  lineHeight?: number;  // default 1
}
```

V1 intentionally keeps configuration small. Width, indentation, and line
height are enough to prove the pipeline.

---

## Flat vs broken mode

Each `group` is evaluated in one of two modes:

- **flat mode**: try to keep everything on one line
- **broken mode**: allow line breaks inside the group

Primitive behavior depends on the mode:

| Primitive | Flat mode | Broken mode |
|---|---|---|
| `line()` | `" "` | newline + indentation |
| `softline()` | `""` | newline + indentation |
| `hardline()` | newline | newline |
| `ifBreak(b, f)` | `f` | `b` |

---

## Realization algorithm

The realization algorithm is a structured, width-aware interpreter.

It walks a stack of commands:

```typescript
{
  indentLevels: number,
  mode: "flat" | "break",
  annotations: DocAnnotation[],
  doc: Doc
}
```

When it encounters a `group`, it asks:

> If this group's content were printed in flat mode from the current column,
> would it fit within the remaining width?

If yes:

- continue in flat mode

If no:

- continue in broken mode

### `fits()` contract

`fits(remainingWidth, pendingCommands)` is a look-ahead simulation.
It walks the pending docs without emitting output:

- `text` consumes character width
- `line` consumes one column in flat mode
- `softline` consumes zero columns in flat mode
- broken-mode lines terminate the fit check successfully because the current
  line may end there
- `hardline` fails a flat candidate immediately because a hard line cannot be
  flattened

This is the same family of algorithm used by Wadler-style pretty printers and
practical systems like Prettier.

---

## Paint bridge

`format-doc-to-paint` converts a `DocLayoutTree` into a `PaintScene`.

### Mapping rules

Each layout line maps to one or more `glyph_run` instructions:

- `x = span.column + glyph_offset`
- `y = line.row * lineHeight`
- `glyph_id = Unicode code point`

The v1 bridge is deliberately simple:

- font metrics are treated as already resolved by the monospace cell layout
- all glyphs use the same `font_ref` and `font_size`
- annotations stay on the layout tree for now; they are not yet projected into
  `PaintInstruction.metadata`

### Scene dimensions

```typescript
scene.width  = layout.width
scene.height = layout.height
```

Background defaults to `"transparent"`.

---

## Example

This doc:

```typescript
group(
  concat([
    text("foo("),
    indent(
      concat([
        softline(),
        text("bar,"),
        line(),
        text("baz"),
      ])
    ),
    softline(),
    text(")"),
  ])
)
```

realizes to this layout tree shape when the width is narrow:

```text
row 0: "foo("
row 1: "  bar,"
row 2: "  baz"
row 3: ")"
```

The paint bridge converts that to glyph runs, and `paint-vm-ascii` renders:

```text
foo(
  bar,
  baz
)
```

---

## Future extensions

The v1 design intentionally leaves room for:

- richer `Doc` combinators (`fill`, `align`, `lineSuffix`)
- paint projection that preserves annotations in metadata
- non-terminal paint backends
- language-specific formatter packages that compile AST nodes into `Doc`

The most important stability promise is:

`Doc` remains the semantic IR even as new backends appear.
