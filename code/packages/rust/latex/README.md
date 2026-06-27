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
| **L1 structural** | groups, `\cmd[opt]{arg}`, `\begin{env}…\end{env}`, text runs, raw math islands, `to_latex()` round-trip | ✅ |
| **L2 math** | math AST (frac, binom, roots, scripts, big ops, functions, accents, `\left\right` fences, relations), precedence-climbing parser, `to_latex()` round-trip | ✅ |
| **L3 environments** | math env family — `matrix`/`pmatrix`/`bmatrix`/`vmatrix`/`cases`/`aligned`/`align` split on `&` and `\\` → `MathNode::Matrix`, round-trip; nesting + scripts | ✅ |
| **L4 macros** | `\newcommand`/`\renewcommand`/`\providecommand` with positional `#1`..`#9`; bounded recursive expansion via `expand()` (L4a) | ✅ |
| **L5 text breadth** | inline `\verb`/`\verb*` raw verbatim (L5a); accents, sectioning, refs, `verbatim` env to follow | 🚧 this release (L5a) |
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

### Math (L2)

Each `$…$` island keeps its **raw** inner source at L1; the math grammar parses it on
demand into a `MathNode` tree with full operator precedence:

```rust
use latex::{parse_math, MathNode};

// fractions, big operators with bounds, roots, scripts, fences — all supported
let m = parse_math(r"\sum_{i=1}^{n} i").unwrap();
assert!(matches!(m, MathNode::BigOp { .. }));

// precedence-aware round-trip: re-parsing the rendered AST yields the same AST
let e = parse_math(r"\left(\frac{a}{b}\right)^2").unwrap();
assert_eq!(parse_math(&e.to_latex()).unwrap(), e);

// parse an island found in a document directly
let doc = parse(r"area is $\pi r^2$").unwrap();
let area = doc.iter().find_map(|n| n.parsed_math()).unwrap().unwrap();
assert!(matches!(area, MathNode::Bin(..)));   // π · r²  (implicit multiplication)
```

### Environments (L3)

The math environment family parses into `MathNode::Matrix { env, rows }` — `&` splits
columns, `\\` splits rows. Supported: `matrix`/`pmatrix`/`bmatrix`/`Bmatrix`/`vmatrix`/
`Vmatrix`/`smallmatrix`, `cases`, and the alignment environments (`aligned`/`align`/…).
Cells hold arbitrary math, environments nest, and a matrix is an atom (so `…^2` attaches):

```rust
use latex::{parse_math, MathNode};

let m = parse_math(r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}").unwrap();
if let MathNode::Matrix { env, rows } = &m {
    assert_eq!(env, "pmatrix");
    assert_eq!(rows.len(), 2);          // two rows
    assert_eq!(rows[0].len(), 2);       // two columns
}
assert_eq!(parse_math(&m.to_latex()).unwrap(), m);   // round-trips
```

`array`/`tabular` (which take a column-spec argument) and document-mode list environments
are a later layer; an unknown `\begin{…}` is rejected with a spanned error, never mis-parsed.

### Macros (L4a)

`parse` stays purely structural; `expand` is an **opt-in pass** over the document tree that
registers `\newcommand`/`\renewcommand`/`\providecommand` (positional `#1`..`#9`) and replaces
later uses by their substituted, recursively-expanded bodies. Definitions vanish from the
output, just like in LaTeX:

```rust
use latex::{parse, expand, document_to_latex};

let doc = parse(r"\newcommand{\sq}[1]{#1^2} area \sq{r}").unwrap();
let expanded = expand(doc).unwrap();
assert_eq!(document_to_latex(&expanded), "area r^2");
```

Expansion is **bounded** — a recursive macro (`\newcommand{\a}{\a}\a`) or an expansion bomb
errors via a depth + work-budget guard rather than hanging or overflowing. Deferred to later
sub-rungs: optional arguments with a default (`[n][default]`), TeX-style `\def`, and a
built-in starter set; `#n` inside a math island is not substituted in L4a.

### Verbatim (L5a)

`\verb<delim>…<delim>` (and the `\verb*` visible-space variant) read their body **raw** — the
tokenizer suspends catcodes inside, so `{ } $ # \` are literal — producing a `Node::Verb`
that round-trips:

```rust
use latex::{parse, Node};

let doc = parse(r"call \verb|x{y}$z| now").unwrap();
assert!(matches!(doc[1], Node::Verb { delim: '|', .. }));   // body "x{y}$z" kept verbatim
```

An unterminated `\verb`, a body running past the end of the line, or a `*`/space delimiter is
a spanned error — never a mis-parse. (The `verbatim` environment, text accents, sectioning,
and cross-refs arrive in later L5 sub-rungs.)

The low-level `tokenize` is also public. Tokens and errors carry half-open byte `Span`s;
all of `parse`, `parse_math`, and `tokenize` return spanned errors rather than panicking,
and recursion is depth-guarded so adversarial nesting errors instead of overflowing.

## Tests

```
cargo test -p latex
cargo clippy -p latex -- -D warnings
```
