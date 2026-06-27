# latex

A **full-fidelity LaTeX parser** (documents *and* math, not a math subset), built as a
standalone, reusable Rust crate. It turns LaTeX source into a faithful AST that any
consumer can use — a reasoning engine, a computer-algebra system, a renderer. It is the
first frontend of the pluggable [`math-frontend`](../math-frontend) framework (it will
implement `MathFrontend` in a later layer) and is useful on its own.

Spec: [`code/specs/LTX01-full-latex-parser.md`](../../../specs/LTX01-full-latex-parser.md);
framework: [`code/specs/PFE01-pluggable-parser-frontends.md`](../../../specs/PFE01-pluggable-parser-frontends.md).

## Honest scope

LaTeX rests on TeX, whose macro layer is Turing-complete. This crate parses the full LaTeX
**surface** and supports the macro mechanisms authors actually use (`\newcommand`/`\def`).
The programmable TeX tail — runtime `\catcode` reassignment, `\expandafter`/`\csname`,
`\if…` programming, external `\input` — is the **documented asymptote**, surfaced as an
explicit "unsupported" node rather than mis-parsed.

## Why a catcode state machine

A character's meaning in LaTeX depends on its **category code** and the scanner's state:
`\` begins a control sequence, `%` skips to end of line, `$` toggles math, a blank line is
a paragraph break. The tokenizer is therefore a hand-written, catcode-driven state machine
with a **text-mode-primary** mode stack (LaTeX starts in text mode; math is entered by
`$`/`\(`/`\[`/`$$`) — the inverse of a math-only tokenizer. (This mirrors how
`grammar-tools` hand-writes its own `.tokens` scanner; the pattern is established here.)

## Status / roadmap (conformance ladder)

| Layer | Contents | Status |
|-------|----------|--------|
| **L0 tokenizer** | catcode state machine → flat `Token` stream w/ byte spans | ✅ |
| **L1 structural** | groups, `\cmd[opt]{arg}`, `\begin{env}…\end{env}`, text runs, raw math islands, `to_latex()` round-trip | ✅ this release |
| L2 math | full math AST (frac, scripts, big ops, accents, `\left\right`, …) | ⏳ |
| L3 environments | matrices / tabular / lists (`&`, `\\`) | ⏳ |
| L4 macros | `\newcommand`/`\def` + expansion (bounded) | ⏳ |
| L5 text breadth | sectioning, fonts, accents, `\verb`, refs | ⏳ |
| L6 frontend | implement `math-frontend::MathFrontend` (LaTeX becomes plugin #1) | ⏳ |

## Usage

```rust
use latex::{parse, Node};

let doc = parse(r"Let $x$ be \textbf{bold}.").unwrap();
assert!(matches!(doc[0], Node::Text(_)));                       // "Let"
assert!(doc.iter().any(|n| matches!(n, Node::Math { .. })));    // $x$
// round-trips: parsing the rendered AST yields the same AST
assert_eq!(parse(&latex::document_to_latex(&doc)).unwrap(), doc);
```

The low-level `tokenize` is also public. Tokens and errors carry half-open byte `Span`s;
both `parse` and `tokenize` return spanned errors rather than panicking.

## Tests

```
cargo test -p latex
cargo clippy -p latex -- -D warnings
```
