# twig-completion

**LSP code-completion items for Twig.**  Walks a parsed
`twig_parser::Program` plus an optional partial-typed prefix and
returns the editor's autocomplete menu — defined symbols + built-
in keywords + constant literals.  Drives
`textDocument/completion`.

The **sixth piece of the Twig authoring-experience layer**
(alongside [`twig-formatter`](../twig-formatter/),
[`twig-semantic-tokens`](../twig-semantic-tokens/),
[`twig-document-symbols`](../twig-document-symbols/),
[`twig-folding-ranges`](../twig-folding-ranges/),
[`twig-hover`](../twig-hover/)).

---

## What's surfaced

| Source                                           | `CompletionKind` | `detail`               |
|--------------------------------------------------|------------------|------------------------|
| Top-level `(define name (lambda params body))`   | `Function`       | `"(params)"`           |
| Top-level `(define name expr)` (any other)       | `Variable`       | `None`                 |
| Built-in keyword (`if`/`let`/`lambda`/`begin`/`define`/`quote`) | `Keyword` | `None`        |
| Constant literal (`#t`/`#f`/`nil`)               | `Constant`       | `None`                 |

## Public API

```rust
pub fn completions(
    source: &str,
    prefix: Option<&str>,
) -> Result<Vec<CompletionItem>, TwigParseError>;

pub fn completions_for_program(
    program: &Program,
    prefix: Option<&str>,
) -> Vec<CompletionItem>;

pub struct CompletionItem {
    pub label:  String,
    pub kind:   CompletionKind,
    pub detail: Option<String>,
}

pub enum CompletionKind {  // #[non_exhaustive]
    Function, Variable, Keyword, Constant,
}
```

`CompletionKind::mnemonic() -> &'static str` returns lowercase
strings (`"function"`, `"variable"`, `"keyword"`, `"constant"`).

## Output order

Items come back sorted: **keywords first** (in declaration order
— `define`, `if`, `let`, `lambda`, `begin`, `quote`), **constants
next** (`#t`, `#f`, `nil`), then **user-defined symbols in name
order** (alphabetical).  Matches what most editor completion menus
expect when no score-based ranking is supplied.

## Prefix filtering

When `prefix` is `Some`, items whose `label` doesn't start with
it are filtered out.  When `None`, every item is returned.
Editors typically pass the substring already typed before the
cursor (for `(squa|` the prefix is `"squa"`).

Exact-prefix and case-sensitive — editors that want fuzzy
matching pass `prefix = None` and run their own client-side
fuzzy matcher.

## What this crate does NOT do

- **No fuzzy match.**  Pass `None` for client-side fuzzy.
- **No snippets.**  V1 emits `label` only; no `insert_text`
  templates like `(if ${1:cond} ${2:then} ${3:else})`.
- **No scope-aware suggestions.**  Parameters and let-bindings
  aren't surfaced (the parser doesn't thread per-binding
  positions).  Tracks alongside `twig-hover`'s scope-resolution
  follow-up.
- **No LSP wire encoding.**  Returns a typed
  `Vec<CompletionItem>`; the JSON `CompletionItem[]` shape is
  one level up.

## Caller responsibilities

Inherits the non-guarantees of `twig-parser` and
`twig-document-symbols`.

---

## Example

```rust
use twig_completion::{completions, CompletionKind};

let src = "\
(define greeting 'hello)
(define (square x) (* x x))
(define pi 3)";

// Unfiltered — full menu.
let items = completions(src, None).unwrap();
assert_eq!(items.len(), 6 + 3 + 3);  // 6 kw + 3 const + 3 user

// Prefix-filtered — only items starting with "sq".
let items = completions(src, Some("sq")).unwrap();
assert_eq!(items.len(), 1);
assert_eq!(items[0].label, "square");
assert_eq!(items[0].kind, CompletionKind::Function);
assert_eq!(items[0].detail.as_deref(), Some("(x)"));
```

---

## Dependencies

- [`twig-parser`](../twig-parser/) — Twig source → typed AST.
- [`twig-document-symbols`](../twig-document-symbols/) — top-level
  define table.

That's it.  No I/O, no FFI, no unsafe.  See `required_capabilities.json`.

---

## Tests

21 unit tests covering `CompletionKind` mnemonics, builtin items
always-present (keywords + constants), kind classification, sort
order (keywords → constants → symbols), keyword declaration
order, alphabetical user-symbol order, function detail = signature,
variable detail = none, prefix filtering (empty / matching /
keyword / constant / no-match / case-sensitive), error path,
`completions_for_program` direct path, a realistic four-symbol
menu, and determinism across calls.

```sh
cargo test -p twig-completion
```

---

## Roadmap

- **Snippets.**  Add `insert_text: Option<String>` with snippet
  templates (`(if ${1:cond} ${2:then} ${3:else})`) so editors
  can offer parameter placeholders for compound forms.
- **Scope-aware suggestions.**  Once `twig-parser` threads per-
  binding positions, surface parameters and let-bindings in
  scope at the cursor.
- **Documentation comments.**  Once the lexer grows a trivia
  channel, populate a `documentation: Option<String>` field.
- **LSP wire encoding.**  `twig-lsp` consumes this typed Vec and
  maps to LSP's `CompletionList` JSON shape.
