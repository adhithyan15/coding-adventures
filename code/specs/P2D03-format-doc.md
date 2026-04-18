# P2D03 — FormatDoc: Document Algebra IR and Text Backend

## Overview

`format-doc` is the pretty-printing layer that sits above layout and paint.
It does not print strings directly. Instead it builds a backend-neutral `Doc`
tree that describes:

- literal text
- optional break points
- grouping
- indentation
- conditional content that depends on whether a group breaks
- annotations that later backends can preserve

The first concrete backend is `format-doc-text`, which realizes a `Doc`
into lines and then serializes those lines to a plain string.

This keeps the pipeline open for future targets:

```text
AST + trivia + formatter rules
  -> Doc
  -> realized line/span layout
  -> text backend

AST + trivia + formatter rules
  -> Doc
  -> realized line/span layout
  -> layout-to-paint-doc   (future)
  -> PaintScene
  -> paint-vm-ascii / canvas / svg
```

The key design rule is simple:

- `Doc` is the stable semantic IR
- line breaking happens during realization
- plain text is only one backend, not the definition of the document

---

## Package split

### `@coding-adventures/format-doc`

Owns:

- `Doc` node types
- combinators (`text`, `group`, `indent`, `line`, ...)
- the width-fitting realization algorithm
- the realized line/span layout data structure

### `@coding-adventures/format-doc-text`

Owns:

- `renderLayoutToText(layout) -> string`
- `renderDocToText(doc, options) -> string`
- tabs/spaces serialization policy for indentation

This split mirrors the paint stack:

```text
paint-instructions  -> paint-vm-* backends
format-doc          -> format-doc-text backend
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

These can be added later without changing the core execution model.

---

## Realized layout

`format-doc` realizes a `Doc` into a line/span structure:

```typescript
interface LayoutSpan {
  text: string;
  annotations: DocAnnotation[];
}

interface LayoutLine {
  indentColumns: number;
  spans: LayoutSpan[];
}

interface LayoutDocument {
  printWidth: number;
  indentWidth: number;
  useTabs: boolean;
  maxColumn: number;
  lines: LayoutLine[];
}
```

This representation is more useful than a final string because it preserves:

- line boundaries
- indentation in columns
- text segments
- annotation stacks

That makes it suitable for future paint conversion and editor features.

---

## Layout options

```typescript
interface LayoutOptions {
  printWidth: number;
  indentWidth?: number;  // default 2
  useTabs?: boolean;     // default false
}
```

V1 intentionally keeps configuration small. The formatter pipeline can grow
more style controls later, but width and indentation are enough to prove the
core abstraction.

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
{ indent: number, mode: "flat" | "break", annotations: DocAnnotation[], doc: Doc }
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

## Text backend

`format-doc-text` consumes `LayoutDocument` and produces plain text.

### Indentation

Each line carries `indentColumns`, not literal indentation bytes.
The text backend chooses how to serialize that:

- spaces only when `useTabs = false`
- as many tabs as possible, then spaces, when `useTabs = true`

This is why the text backend belongs in a separate package.

### Final string rules

- concatenate indentation and span text for each line
- join lines with `\n`
- preserve internal blank lines
- do not trim annotation content

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

renders as:

```text
foo(bar, baz)
```

with a wide `printWidth`, and as:

```text
foo(
  bar,
  baz
)
```

with a narrow `printWidth`.

---

## Future extensions

The v1 design intentionally leaves room for:

- richer `Doc` combinators (`fill`, `align`, `lineSuffix`)
- a `Doc -> PaintScene` bridge through line/span layout
- syntax-aware formatter packages that compile AST nodes into `Doc`
- annotations that carry AST node ids, token classes, or source spans

The most important stability promise is:

`Doc` remains the semantic IR even as new backends appear.
