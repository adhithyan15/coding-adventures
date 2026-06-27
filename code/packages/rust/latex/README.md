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
| **L0 tokenizer** | catcode state machine → flat `Token` stream w/ byte spans | ✅ this release |
| L1 structural | groups, `\cmd[opt]{arg}`, `\begin{env}…\end{env}`, text runs | ⏳ |
| L2 math | full math AST (frac, scripts, big ops, accents, `\left\right`, …) | ⏳ |
| L3 environments | matrices / tabular / lists (`&`, `\\`) | ⏳ |
| L4 macros | `\newcommand`/`\def` + expansion (bounded) | ⏳ |
| L5 text breadth | sectioning, fonts, accents, `\verb`, refs | ⏳ |
| L6 frontend | implement `math-frontend::MathFrontend` (LaTeX becomes plugin #1) | ⏳ |

## Usage

```rust
use latex::{tokenize, TokenKind};

let toks = tokenize(r"Let $x$ be.").unwrap();
assert_eq!(toks[0].kind, TokenKind::Char('L'));
assert!(toks.iter().any(|t| matches!(t.kind, TokenKind::MathOn { .. })));
```

Every token carries a half-open byte `Span`; `tokenize` returns a spanned `LexError` on
malformed input rather than panicking.

## Tests

```
cargo test -p latex
cargo clippy -p latex -- -D warnings
```
