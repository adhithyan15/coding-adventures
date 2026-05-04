# twig-semantic-tokens

**Semantic-token extraction for Twig.**  Walks a parsed
[`twig_parser::Program`] and emits a typed token stream — keywords
/ identifiers / numbers / booleans / nil / quoted symbols —
suitable for **LSP semantic-tokens**, syntax highlighters, and
editor extensions.

Together with [`twig-formatter`](../twig-formatter/), this is the
second piece of the **Twig authoring-experience layer**.  Editors
pull semantic highlighting from this crate's typed token list
instead of relying on regex-based syntax patterns (which can't
tell e.g. a variable reference from a function-position
application head).

---

## Why semantic over regex

Regex highlighters can colour `(if cond ...)` because `if` is a
keyword in a fixed position — but they can't colour `cons` as a
function and `xs` as a variable in `(cons x xs)`, because they
don't know the grammar.  Semantic tokens drive richer themes:

- **Keyword vs identifier**
- **Function-position identifier vs variable-position identifier**
- **Constants** (`#t`, `#f`, `nil`) styled as constants
- **Quoted symbols** (`'foo`) styled as data literals
- **Parameter names** (the binder side of `define`/`let`/`lambda`)

---

## Public API

```rust
pub fn semantic_tokens(source: &str) -> Result<Vec<SemanticToken>, TwigParseError>;
pub fn tokens_for_program(program: &Program) -> Vec<SemanticToken>;

pub struct SemanticToken {
    pub line:   u32,    // 1-based
    pub column: u32,    // 1-based
    pub length: u32,    // monospace cells
    pub kind:   TokenKind,
}

pub enum TokenKind {       // #[non_exhaustive]
    Keyword,    // if / let / lambda / begin / define / quote
    Boolean,    // #t / #f
    Nil,        // nil
    Number,     // integer literal
    Symbol,     // 'foo  (quoted symbol; length includes the apostrophe)
    Function,   // head of (fn args) when fn is a VarRef
    Variable,   // VarRef outside function position
    Parameter,  // define name
}
```

`TokenKind::mnemonic() -> &'static str` returns lowercase strings
matching LSP semantic-token type names where the meanings line up.

Tokens come back in **document order** (top-to-bottom, left-to-
right within a line) — what LSP semantic-token providers want.

## Position model

All positions are **1-based** `(line, column)` in monospace cell
units, matching `twig-parser`.  `length` is the visible width of
the token in cells (char count for ASCII source — Twig
identifiers are ASCII).

## What this crate does NOT do

- **No punctuation tokens.**  Open / close parens are dropped: the
  parser AST doesn't preserve their positions independently.
  Editors that want paren highlighting can layer it on top.
- **No comment tokens.**  The Twig lexer is comment-stripping;
  comments don't survive into the AST.  Lands when the lexer
  grows a trivia channel.
- **No LSP wire encoding.**  Returns a typed `Vec<SemanticToken>`;
  conversion to LSP's delta-encoded format is one level up (so
  this crate stays usable from non-LSP consumers).
- **No `let`-binding / `lambda`-param positions** — the AST
  stores binding names as bare `String`s without per-name
  positions.  Future versions can thread positions through
  `twig-parser` to fix this; usages within the body are still
  coloured correctly via `VarRef`.

---

## Hardening

- All `usize → u32` conversions go through `u32_of` (saturating to
  `u32::MAX`) and `len_u32` — no truncation, no debug-mode panic
  on adversarial positions.
- All column arithmetic uses `saturating_add`.
- The `column == 0` / `line == 0` / `length == 0` sentinel makes
  `push_token` drop AST-derived positions that the parser
  couldn't fix (e.g. binding-name placeholders).

Security review: clean, no findings.  Recursion bounded by
twig-parser's `MAX_AST_DEPTH`; no panic surface; capability-empty.

---

## Example

```rust
use twig_semantic_tokens::{semantic_tokens, TokenKind};

let tokens = semantic_tokens("(cons 'foo nil)").unwrap();

let kinds: Vec<TokenKind> = tokens.iter().map(|t| t.kind).collect();
assert_eq!(kinds, vec![
    TokenKind::Function,  // cons
    TokenKind::Symbol,    // 'foo
    TokenKind::Nil,       // nil
]);
```

---

## Dependencies

- [`twig-parser`](../twig-parser/) — Twig source → typed AST.

That's it.  No I/O, no FFI, no unsafe.  See `required_capabilities.json`.

---

## Tests

24 unit tests covering atoms (Number / Boolean / Nil / Symbol /
Variable), each compound form (`if`/`let`/`lambda`/`begin`/`define`),
function-position re-classification, document-order sort,
multi-line input, keyword position correctness, error path,
`tokens_for_program` direct path, and a realistic factorial
example.

```sh
cargo test -p twig-semantic-tokens
```

---

## Roadmap

- **Position-preserving binding names** — thread per-name
  `(line, column)` through `twig-parser` so let bindings,
  lambda params, and define names get proper Parameter tokens
  with real positions.
- **LSP wire encoding** — separate `twig-lsp` crate that consumes
  this crate's typed tokens and produces LSP semantic-tokens
  delta encoding.
- **Comment tokens** — once the lexer grows a trivia channel.
- **Token modifiers** — LSP supports `definition`, `readonly`,
  `static`, etc. as modifiers on top of token kinds.  Useful
  when we add type-checked Twig variants (e.g. tagging
  immutable bindings).
- **Operator tokens** — Twig has no operator tokens today (it's
  pure Lisp), but if the language gains infix syntax, those
  classify as `Operator`.
