# P2D04 — FormatDoc Standard Templates

## Overview

`format-doc-std` is the first reusable template layer built on top of
`format-doc`.

The goal is simple:

- `format-doc` owns the primitive document algebra
- `format-doc-std` owns common syntax shapes that most languages reuse
- language-specific formatter packages compose these templates and override the
  remaining unusual constructs

This package is the "80% layer" in the formatter stack.

```text
language-specific AST printer
  -> format-doc-std templates
  -> Doc
  -> DocLayoutTree
  -> PaintScene
  -> paint-vm-ascii
```

---

## Scope of v1

The first standard templates are:

1. `delimitedList()`
2. `callLike()`
3. `blockLike()`
4. `infixChain()`

These four shapes cover a large percentage of real formatter output:

- arrays, tuples, parameter lists, argument lists, object fields
- function and constructor calls
- braces / begin-end / indented block bodies
- arithmetic, boolean, pipeline, and type-operator chains

---

## Design principles

### Build Docs, not strings

Every template returns a `Doc`. It never emits text directly.

### Flat by default, broken when needed

Templates rely on `group()`, `line()`, `softline()`, and `indent()` to let the
core width-fitting algorithm decide when to break.

### Keep policy parameters small

Templates accept the formatting choices that usually vary by language, such as:

- delimiters
- separators
- trailing separator behavior
- whether breaks happen before or after operators
- whether empty blocks include inner spacing

They do not try to encode every language-specific edge case.

### Escape hatches stay in language packages

If a language has unusual layout rules, the language package should build a
custom `Doc` directly or wrap the standard templates with its own conventions.

---

## Public API

### `delimitedList(options)`

Formats a list surrounded by opening and closing delimiters.

```typescript
interface DelimitedListOptions {
  open: Doc;
  close: Doc;
  items: Doc[];
  separator?: Doc;              // default: text(",")
  trailingSeparator?: "never" | "always" | "ifBreak";
  emptySpacing?: boolean;       // default: false
}
```

Behavior:

- empty list: `[]` by default, `[ ]` when `emptySpacing = true`
- flat list: `[a, b, c]`
- broken list:

```text
[
  a,
  b,
  c,
]
```

when `trailingSeparator = "always"` or `"ifBreak"`

### `callLike(callee, args, options)`

A convenience wrapper around `delimitedList()` for call expressions.

```typescript
interface CallLikeOptions {
  open?: Doc;                   // default: text("(")
  close?: Doc;                  // default: text(")")
  separator?: Doc;              // default: text(",")
  trailingSeparator?: "never" | "always" | "ifBreak";
}
```

Behavior:

- flat: `foo(a, b)`
- broken:

```text
foo(
  a,
  b,
)
```

### `blockLike(options)`

Formats a block with an opener, a body, and a closer.

```typescript
interface BlockLikeOptions {
  open: Doc;
  body: Doc;
  close: Doc;
  emptySpacing?: boolean;       // default: true
}
```

Behavior:

- empty block: `{ }` when `emptySpacing = true`, otherwise `{}`
- non-empty block:

```text
{
  body
}
```

### `infixChain(options)`

Formats a sequence of operands joined by operators.

```typescript
interface InfixChainOptions {
  operands: Doc[];
  operators: Doc[];
  breakBeforeOperators?: boolean;   // default: false
}
```

The number of operators must be exactly `operands.length - 1`.

Flat output:

```text
a + b + c
```

Broken output with `breakBeforeOperators = false`:

```text
a +
  b +
  c
```

Broken output with `breakBeforeOperators = true`:

```text
a
  + b
  + c
```

---

## Why these templates first?

These four helpers exercise the most important document-algebra ideas:

- grouped alternatives
- indentation after breaks
- conditional punctuation with `ifBreak()`
- structural reuse across languages

If these shapes feel ergonomic, the next layer can add richer templates like:

- member chains
- clause chains (`if/else if/else`, `match`, `case`)
- declaration-like forms
- assignment-like forms

---

## Future direction

`format-doc-std` should stay opinionated but small.

Its job is not to eliminate language-specific printers. Its job is to make
those printers thin by handling the recurring structure that appears in nearly
all grammars.
