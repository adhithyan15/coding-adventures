# twig-hover

**LSP hover-info extraction for Twig.**  Given a parsed
`twig_parser::Program` and a `(line, column)` cursor position,
returns the symbol under the cursor — name, kind, and signature
(for functions).  Drives the editor's "what's this?" tooltip
(LSP `textDocument/hover`).

The **fifth piece of the Twig authoring-experience layer**
(alongside [`twig-formatter`](../twig-formatter/),
[`twig-semantic-tokens`](../twig-semantic-tokens/),
[`twig-document-symbols`](../twig-document-symbols/),
[`twig-folding-ranges`](../twig-folding-ranges/)).

---

## What surfaces in hover

| Cursor on…                     | Hover shows                                             |
|--------------------------------|---------------------------------------------------------|
| A `VarRef` whose name binds a top-level `(define …)` | The define's signature (function or value) |
| A `VarRef` with no matching define | "UnresolvedVariable" + name (parameter / let-binding / typo) |
| `BoolLit` / `NilLit` / `IntLit` / `SymLit` | The literal kind + value                       |
| A keyword (`if`/`let`/`lambda`/`begin`/`define`) | "Keyword" + the form name                |
| A `(define name …)` name       | The binding kind (Function with signature, or Variable) |
| Anywhere else                  | `None`                                                  |

## Public API

```rust
pub fn hover_at(source: &str, line: u32, column: u32) -> Result<Option<Hover>, TwigParseError>;
pub fn hover_for_program(program: &Program, line: u32, column: u32) -> Option<Hover>;

pub struct Hover {
    pub kind:      HoverKind,
    pub name:      String,
    pub signature: Option<String>,
    pub line:      u32,    // 1-based start
    pub column:    u32,    // 1-based start
    pub length:    u32,
}

pub enum HoverKind {  // #[non_exhaustive]
    Function, Variable, UnresolvedVariable,
    Boolean, Nil, Number, Symbol,
    Keyword,
}
```

`HoverKind::mnemonic() -> &'static str` returns lowercase strings
(`"function"`, `"variable"`, `"unresolved-variable"`, …).

## Position model

All positions are **1-based** `(line, column)` matching
`twig-parser`.  A token at `(line, col)` with length `len`
"contains" cursor `(L, C)` iff `L == line` and `col <= C <= col +
len`.  The trailing `=` is intentional — a cursor sitting just
past the last character of an identifier still counts as "on" it
(prettier convention).  Cursors past `col + len` produce `None`.

---

## What this crate does NOT do

- **No type information.**  Twig has no type-checker yet.  When
  one ships, hover gains an inferred-type field.
- **No documentation comments.**  The Twig lexer is comment-
  stripping; doc comments don't survive into the AST.  Lands
  when the lexer grows a trivia channel.
- **No LSP wire encoding.**  Returns a typed `Option<Hover>`;
  the JSON `Hover` shape is one level up.
- **No `let`-binding / `lambda`-param resolution.**  An
  unresolved `VarRef` could be a parameter, a let-binding, or a
  typo — surfaces as `UnresolvedVariable`.  Smarter scope
  analysis lands when the parser threads per-binding positions.

## Caller responsibilities

Inherits the non-guarantees of `twig-parser` and
`twig-document-symbols`.

---

## Example

```rust
use twig_hover::{hover_at, HoverKind};

let src = "\
(define (square x) (* x x))
(square 5)
";

// Cursor on `square` in the call on line 2, column 2.
let h = hover_at(src, 2, 2).unwrap().unwrap();
assert_eq!(h.kind, HoverKind::Function);
assert_eq!(h.name, "square");
assert_eq!(h.signature.as_deref(), Some("(x)"));
```

---

## Dependencies

- [`twig-parser`](../twig-parser/) — Twig source → typed AST.
- [`twig-document-symbols`](../twig-document-symbols/) — top-level
  define table for VarRef resolution.

That's it.  No I/O, no FFI, no unsafe.  See `required_capabilities.json`.

---

## Tests

26 unit tests covering the empty program, out-of-range cursor,
zero-position sentinel, every atom kind (Number / negative /
Boolean / Nil / Symbol), unresolved VarRef, function-resolved
VarRef, variable-resolved VarRef, every keyword (if / let /
lambda / begin / define), cursor-at-token-end boundary, define
name (Function with signature, Variable without), innermost-token
selection in nested forms, multi-line input, error path,
`hover_for_program` direct path.

```sh
cargo test -p twig-hover
```

---

## Roadmap

- **Type info** — once Twig grows a type-checker, populate a new
  `inferred_type: Option<String>` field on `Hover`.
- **Scope-aware resolution** — once `twig-parser` threads per-
  binding positions, resolve `let` and `lambda` parameter VarRefs
  to their binding sites instead of marking them
  `UnresolvedVariable`.
- **Doc comments** — once the lexer grows a trivia channel,
  populate `documentation: Option<String>` from `;;` comments
  immediately preceding a `define`.
- **LSP wire encoding** — `twig-lsp` consumes this typed
  `Option<Hover>` and maps to LSP's `Hover { contents,
  range }` JSON shape.
