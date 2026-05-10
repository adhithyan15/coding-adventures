# twig-folding-ranges

**LSP folding-range extraction for Twig.**  Walks a parsed
`twig_parser::Program` and returns a typed `Vec<FoldingRange>` —
the data feed for the editor's `textDocument/foldingRange`
response.  Drives "fold all defines" / "collapse this `let` body"
/ "show outline only" commands across every LSP-aware editor.

The **fourth piece of the Twig authoring-experience layer**
(alongside [`twig-formatter`](../twig-formatter/),
[`twig-semantic-tokens`](../twig-semantic-tokens/), and
[`twig-document-symbols`](../twig-document-symbols/)).

---

## What folds

Any compound form that **spans more than one source line**:

- `(define name expr)`
- `(let ((bindings)) body)`
- `(begin e1 e2 …)`
- `(lambda (params) body)`
- `(if cond then else)`
- `(fn arg1 arg2 …)` — function application

Single-line forms (`(define x 42)` on one line) are **not** folded
— there's nothing to collapse.

## Public API

```rust
pub fn folding_ranges(source: &str) -> Result<Vec<FoldingRange>, TwigParseError>;
pub fn ranges_for_program(program: &Program) -> Vec<FoldingRange>;

pub struct FoldingRange {
    pub start_line: u32,    // 1-based
    pub end_line:   u32,    // 1-based; > start_line
    pub kind:       FoldingRangeKind,
}

pub enum FoldingRangeKind {  // #[non_exhaustive]
    Region,                   // generic foldable region
}
```

`FoldingRangeKind::mnemonic() -> &'static str` returns lowercase
strings matching LSP's `FoldingRangeKind` values where the
meanings line up.

Ranges come back in **document order** (start line ascending,
then end line ascending for ties).

## Position model

All positions are **1-based** lines matching `twig-parser`.  V1
is line-based (no columns) — sufficient for every LSP folding-
range consumer.

End lines are **derived** from the maximum line of any position
in the form's subtree.  Approximate (it doesn't see the closing
paren if it's on a line past every atom), but tracks the visible
region the user wants to collapse — which is what folding-range
consumers care about.

## What this crate does NOT do

- **No comment regions.**  The Twig lexer is comment-stripping.
- **No `#region` / `#endregion` markers.**  Twig has no such convention.
- **No LSP wire encoding.**  Returns a typed `Vec<FoldingRange>`;
  the JSON `FoldingRange[]` shape is one level up.

---

## Example

```rust
use twig_folding_ranges::{folding_ranges, FoldingRangeKind};

let src = r#"
(define x 42)

(define (factorial n)
  (if (= n 0)
      1
      (* n (factorial (- n 1)))))

(define result
  (factorial 10))
"#;

let ranges = folding_ranges(src).unwrap();
// factorial define folds, the inner `if` folds, result define folds.
// (Single-line `x` define doesn't fold.)
assert!(ranges.iter().any(|r| r.kind == FoldingRangeKind::Region));
```

---

## Dependencies

- [`twig-parser`](../twig-parser/)

That's it.  No I/O, no FFI, no unsafe.  See `required_capabilities.json`.

---

## Tests

20 unit tests covering empty programs, single-line non-folding
(define / if / apply / atoms), multi-line folding (define / if /
lambda / let / begin), nested multi-line forms, document-order
sort, mixed top-level streams, deeply-nested apply, single-line
filtering, error path, `ranges_for_program` direct path, and a
realistic module example.

```sh
cargo test -p twig-folding-ranges
```

---

## Roadmap

- **End columns.**  Add column to `FoldingRange` once the parser
  threads end positions.
- **Comment regions.**  Once the lexer grows a trivia channel.
- **`twig-lsp` wire encoding.**  Separate crate consuming this
  typed Vec.
- **Configurable thresholds.**  E.g. "only fold regions > N
  lines".
