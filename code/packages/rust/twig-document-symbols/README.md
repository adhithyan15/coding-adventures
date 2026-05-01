# twig-document-symbols

**LSP outline view for Twig.**  Walks a parsed
`twig_parser::Program` and returns a typed `Vec<DocumentSymbol>` —
the data feed for the editor's outline view (VS Code's "Outline"
pane, JetBrains' "Structure" tool window, every LSP-aware editor's
`textDocument/documentSymbol` response).

Together with [`twig-formatter`](../twig-formatter/) and
[`twig-semantic-tokens`](../twig-semantic-tokens/), this is the
**third piece of the Twig authoring-experience layer**.

---

## What's a document symbol

Per the LSP spec, `DocumentSymbol` is a hierarchical structure
describing the named "things" in a file — functions, classes,
variables — for navigation features (outline, breadcrumbs,
workspace symbol search, "Go to Symbol in File").

Twig's symbol vocabulary is small:

| Form | `SymbolKind` | `detail` |
|------|-------------|----------|
| `(define name (lambda params body))` | `Function` | `"(params)"` |
| `(define name expr)` (other expr)    | `Variable` | `None`   |

Bare top-level expressions are **not** symbols (they don't bind a
name).  Nested defines aren't a concern: the Twig grammar only
allows `(define …)` at the top level — the parser rejects them in
expression position before they reach this crate.

## Public API

```rust
pub fn document_symbols(source: &str) -> Result<Vec<DocumentSymbol>, TwigParseError>;
pub fn symbols_for_program(program: &Program) -> Vec<DocumentSymbol>;

pub struct DocumentSymbol {
    pub name:    String,
    pub kind:    SymbolKind,
    pub detail:  Option<String>,
    pub line:    u32,    // 1-based
    pub column:  u32,    // 1-based
}

pub enum SymbolKind {  // #[non_exhaustive]
    Function,
    Variable,
}
```

`SymbolKind::mnemonic() -> &'static str` returns lowercase strings
matching LSP's `SymbolKind` names where the meanings line up.

Symbols come back in **document order** (top to bottom) — what
LSP outline-view providers want.

## Position model

All positions are **1-based** `(line, column)` matching
`twig-parser`.  V1 returns the start position only; the LSP spec
wants both `range` (full extent of the symbol declaration
including body) and `selectionRange` (just the name).  Adding end
positions requires threading them through `twig-parser`, which is
filed as a follow-up.

## What this crate does NOT do

- **No `let`-binding symbols.**  Outline views typically don't list
  inner-scope bindings — they clutter the navigation pane.  Editors
  that want them can layer it on top.
- **No LSP wire encoding.**  Returns a typed `Vec<DocumentSymbol>`;
  the JSON `DocumentSymbol[]` shape is one level up so this crate
  stays usable from non-LSP consumers (e.g. CLI `twig outline foo.twig`).

## Caller responsibilities

Inherits the non-guarantees of `twig-parser`.

---

## Example

```rust
use twig_document_symbols::{document_symbols, SymbolKind};

let symbols = document_symbols(r#"
    (define greeting 'hello)
    (define (square x) (* x x))
    (define pi 3)
"#).unwrap();

assert_eq!(symbols.len(), 3);
assert_eq!(symbols[0].name, "greeting");
assert_eq!(symbols[0].kind, SymbolKind::Variable);
assert_eq!(symbols[1].name, "square");
assert_eq!(symbols[1].kind, SymbolKind::Function);
assert_eq!(symbols[1].detail.as_deref(), Some("(x)"));
assert_eq!(symbols[2].name, "pi");
```

---

## Dependencies

- [`twig-parser`](../twig-parser/) — Twig source → typed AST.

That's it.  No I/O, no FFI, no unsafe.  See `required_capabilities.json`.

---

## Tests

20 unit tests covering empty programs, bare top-level expressions
(no symbol), value bindings (`Variable`), lambda bindings
(`Function`) including the `(define (f x) body)` sugar form,
nullary and multi-param signatures, multiple top-level defines,
document-order sort, mixed top-level streams (skipping bare
exprs), correct multi-line positions, error path,
`symbols_for_program` direct path, a realistic four-symbol module
outline, and long-identifier passthrough in signatures.

```sh
cargo test -p twig-document-symbols
```

---

## Roadmap

- **End positions.**  Thread per-form `(end_line, end_column)`
  through `twig-parser` so each `DocumentSymbol` can carry an LSP
  `range` and `selectionRange` instead of a start-only position.
- **Workspace symbols.**  An aggregator over a file set so editors
  can answer `workspace/symbol` queries (`Cmd-T` go-to-anywhere).
- **`twig-lsp` wire encoding.**  Separate crate that consumes
  this one's typed Vec and produces LSP JSON.
- **`twig outline` CLI.**  Standalone binary using this crate.
