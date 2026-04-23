# NIB03 — Nib Formatter

## Overview

`@coding-adventures/nib-formatter` is the first language-specific formatter
package built on top of the shared document algebra stack:

```text
Nib source
  -> nib-parser AST
  -> Doc
  -> DocLayoutTree
  -> PaintScene
  -> paint-vm-ascii
  -> formatted text
```

The goal of v1 is not perfect source preservation. The goal is to make valid,
badly-formatted Nib code print into one stable canonical form with very little
language-specific machinery while still preserving the most important source
trivia that users expect a formatter to keep: line comments and intentional
blank lines.

This package is the first proof that:

- `format-doc` is a useful semantic IR
- `format-doc-std` covers most ordinary syntax shapes out of the box
- a language package can wrap the generic formatter stack and still expose an
  easy, high-level `formatNib()` API

---

## Scope

V1 supports canonical formatting for the full Nib v1 grammar:

- top-level `const`, `static`, and `fn` declarations
- parameter lists and argument lists
- blocks and statements
- `let`, assignment, `return`, `for`, and `if`
- unary and infix expressions
- parenthesized expressions
- call expressions

V1 now preserves comment and blank-line trivia through the source-based
formatting path:

- line comments from the Nib lexer skip stream
- blank lines between top-level declarations
- blank lines between statements inside blocks
- comments before a closing `}`
- comments at end of file

The formatter does **not** aim for perfect token-by-token trivia fidelity.
Instead it normalizes comment placement into a stable canonical form using the
preserved lexer/parser trivia stream.

---

## Package

### `@coding-adventures/nib-formatter`

Owns:

- lowering the generic Nib parser AST into `Doc`
- Nib-specific printing rules and operator precedence handling
- high-level entry points for formatting Nib source or a parsed Nib AST
- end-to-end ASCII string output through `format-doc-to-paint` and
  `paint-vm-ascii`

Depends on:

- `@coding-adventures/nib-parser`
- `@coding-adventures/format-doc`
- `@coding-adventures/format-doc-std`
- `@coding-adventures/format-doc-to-paint`
- `@coding-adventures/paint-vm-ascii`

---

## Public API

```typescript
import type { ASTNode } from "@coding-adventures/parser";
import type { Doc, LayoutOptions } from "@coding-adventures/format-doc";
import type { DocPaintOptions } from "@coding-adventures/format-doc-to-paint";

export interface NibFormatOptions extends LayoutOptions {
  paint?: DocPaintOptions;
}

export function printNibDoc(ast: ASTNode): Doc;
export function printNibSourceToDoc(source: string): Doc;
export function formatNibAst(ast: ASTNode, options: NibFormatOptions): string;
export function formatNib(source: string, options?: Partial<NibFormatOptions>): string;
```

Default options:

- `printWidth`: `80`
- `indentWidth`: `2`
- `lineHeight`: `1`

`formatNib()` is the main convenience API. It parses source, lowers to `Doc`,
runs layout, converts the layout to paint, and finally renders through
`paint-vm-ascii`.

Source-based entry points preserve EOF trivia by parsing through a formatter
oriented Nib parser path that keeps the original token stream. AST-only entry
points preserve node-attached trivia, but cannot recover comments that live
only on the synthetic EOF token.

---

## Formatting Rules

### Top-level declarations

- Each top-level declaration starts on its own line.
- A single blank line separates adjacent top-level declarations.
- Function signatures stay on one line when they fit.
- Function bodies always print as blocks.

### Blocks

- Empty blocks print as `{ }`.
- Non-empty blocks print with one statement per line.
- Statements inside blocks are indented one level.

### Statements

- `let`, assignment, `return`, and expression statements print on one line when
  they fit, otherwise the expression part may break.
- `for` prints as:

```nib
for i: u8 in 0..10 {
  body();
}
```

- `if` prints as:

```nib
if condition {
  yes();
} else {
  no();
}
```

### Lists

- Parameter lists and call arguments use shared delimited-list formatting.
- Short lists remain inline.
- Long lists break one item per line inside the delimiters.

### Expressions

- Operator precedence comes from the Nib grammar.
- Nested expressions only gain explicit parentheses when the AST already
  contains them.
- Unary operators print tight to their operand: `!flag`, `~mask`
- Infix operators print with a single space on both sides.
- Broken infix chains indent continuation lines.

### Canonical spacing

- One space after keywords such as `fn`, `let`, `if`, `for`, `return`, and
  `else`
- No spaces just inside delimiters such as `(`, `)`, `{`, `}`, and `,`
- A single space around `=`, `->`, and infix operators
- No trailing whitespace

### Comments and blank lines

- Leading line comments are preserved before the declaration, statement, or
  closing brace they precede.
- Line comments written after a declaration or statement stay trailing on that
  line when the preserved trivia shows the comment began before the next
  newline.
- Blank lines between top-level declarations and block statements are preserved
  in canonical form.
- End-of-file comments are preserved.
- Comment text is emitted exactly as written, including the original `//`
  prefix.

---

## Lowering strategy

The formatter walks the grammar-driven AST by `ruleName`.

It relies on the shared shape helpers whenever possible:

- `delimitedList()` for parameters and arguments
- `blockLike()` for braces and statement bodies
- `callLike()` for calls
- `infixChain()` for precedence levels that repeat binary operators

Nib-specific logic only handles:

- extracting identifiers and child rules from the generic parser AST
- distinguishing statement and declaration forms
- preserving explicit parenthesized expressions
- stitching the expression precedence ladder together
- interpreting preserved lexer/parser trivia as leading comments, trailing
  comments, and blank lines

To preserve comments, the source-based formatter path parses Nib with
`preserveSourceInfo: true` and retains the original token list long enough to
recover trivia attached to the synthetic EOF token and to structural tokens
such as `RBRACE` and `"else"`.

This is the central design goal of the package: most formatting should come
from the shared document algebra standard library, not from hand-written
language-specific layout code.

---

## Test strategy

The package must cover:

- smoke formatting of each declaration and statement form
- ugly input normalization for complete Nib programs
- line-wrapping behavior for long parameter lists, argument lists, and infix
  chains
- comment preservation for top-level declarations, statements, blocks, `else`
  boundaries, and EOF
- blank-line preservation for declarations and statements
- idempotence: formatting already-formatted code should not change it again
- error handling for unsupported AST shapes and malformed nodes

Coverage target: well above 80%, ideally 95%+ because this is a small library
package.
