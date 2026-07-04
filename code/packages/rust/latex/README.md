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
| **L2 math** | math AST (frac, binom, roots, scripts, big ops, functions, accents, `\left\right` fences, relations, `\overset`/`\underset`/`\stackrel` stacking, `\xrightarrow`-family extensible labelled arrows, `\overbrace`/`\underbrace` horizontal braces, and `\overrightarrow`/`\underrightarrow`-family stretchy over/under-arrow accents), precedence-climbing parser, `to_latex()` round-trip | ✅ |
| **L3 environments** | math env family — `matrix`/`pmatrix`/`bmatrix`/`vmatrix`/`cases`/`aligned`/`align` plus `array`/`subarray` (mandatory `{col-spec}`) split on `&` and `\\` → `MathNode::Matrix`, round-trip; nesting + scripts | ✅ |
| **L4 macros** | `\newcommand`/`\renewcommand`/`\providecommand` with positional `#1`..`#9`; bounded recursive expansion via `expand()` (L4a) | ✅ |
| **L5 text breadth** | `\verb`/`verbatim` raw (L5a/b) + text accents `\'e`/`\c{c}` via `recognize_accents` (L5c) + sectioning/refs/preamble/font via `recognize_structure` (L5d) | ✅ |
| **L6 frontend** | `LatexMath` implements `math-frontend::MathFrontend` — lifts `MathNode` → neutral `MathExpr`; LaTeX is plugin #1 via `registry()` (default-on `frontend` feature) | ✅ |
| **D1 doc tables/lists** | document-mode `tabular`/`tabular*` grids (split on `&`/`\\`) → `Node::Tabular` and `itemize`/`enumerate`/`description` (split on `\item`) → `Node::List`, via the opt-in `recognize_tables` pass; total, round-trip | ✅ |
| **D2 Document skeleton** | hierarchical `Document` model: preamble/body split at `\begin{document}`, `\documentclass`/`\usepackage` classified, body lowered to a **flat** `Vec<Block>` (headings → zero-body `Block::Section`; paragraphs/lists/tables/display-math/environments; inline runs → `Vec<Inline>`); `parse_document`/`build_document` + `Document::to_latex` round-trip; coarse (region-granular) spans | ✅ |

The low-level ladder is **complete** (L0–L6). 🎉 The hierarchical **Document** layer (LTXDOC01)
is building on top: D1–D2 shipped.

## The Document layer (LTXDOC01)

Above the flat `Vec<Node>` sits a reusable, hierarchical `Document` AST — the write-once fold every
consumer (renderers, format conversion, structure queries, the ADJ byte-provenance pipeline) shares
instead of re-deriving the hierarchy. `parse_document` composes the shipped LTX01 passes then folds:

```rust
use latex::{parse_document, Block, Inline};

let doc = parse_document(
    r"\documentclass{article}\begin{document}\section{Intro}Hello \textbf{world}.\end{document}",
).unwrap();

assert_eq!(doc.preamble.document_class.unwrap().class, "article");
assert!(matches!(doc.body[0], Block::Section { .. }));        // zero-body heading (D2)
assert_eq!(doc.to_latex().is_empty(), false);                // round-trips (modulo spans)
```

**D2 spans are coarse (region-granular)**: every block/inline span defaults to its enclosing region
span, guaranteeing every child span ⊆ its parent ⊆ the `Document` span. Precise per-node byte
coverage is deferred to D6 (once the parser threads token spans through `Node`); the `span` field
exists now so later rungs tighten the *values* without an API break.

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

The math environment family parses into `MathNode::Matrix { env, col_spec, rows }` — `&`
splits columns, `\\` splits rows. Supported: `matrix`/`pmatrix`/`bmatrix`/`Bmatrix`/`vmatrix`/
`Vmatrix`/`smallmatrix`, `cases`, the alignment environments (`aligned`/`align`/…), and the
general `array`/`subarray` grids. Cells hold arbitrary math, environments nest, and a matrix
is an atom (so `…^2` attaches):

```rust
use latex::{parse_math, MathNode};

let m = parse_math(r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}").unwrap();
if let MathNode::Matrix { env, col_spec, rows } = &m {
    assert_eq!(env, "pmatrix");
    assert_eq!(*col_spec, None);        // pmatrix takes no column-spec argument
    assert_eq!(rows.len(), 2);          // two rows
    assert_eq!(rows[0].len(), 2);       // two columns
}
assert_eq!(parse_math(&m.to_latex()).unwrap(), m);   // round-trips
```

`array` and `subarray` carry a **mandatory column-spec argument** — `\begin{array}{l|cr}` —
captured verbatim on `col_spec` (`Some("l|cr")`) so the node round-trips. Alignment is
presentation, so the neutral `MathExpr` lowering **drops** `col_spec`: an `array` and the
equivalent `pmatrix` lower to the same `MathExpr::Matrix`. The text-mode `tabular` family and
document-mode list environments are a later layer; an unknown `\begin{…}` (or an `array`
missing its column-spec) is rejected with a spanned error, never mis-parsed.

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

### Verbatim (L5a/L5b)

`\verb<delim>…<delim>` (and the `\verb*` visible-space variant) read their body **raw** — the
tokenizer suspends catcodes inside, so `{ } $ # \` are literal — producing a `Node::Verb`
that round-trips:

```rust
use latex::{parse, Node};

let doc = parse(r"call \verb|x{y}$z| now").unwrap();
assert!(matches!(doc[1], Node::Verb { delim: '|', .. }));   // body "x{y}$z" kept verbatim
```

The **`verbatim` environment** (and `verbatim*`) reads its whole body raw — newlines included —
up to the matching `\end{verbatim}`, producing a `Node::VerbatimEnv` that also round-trips:

```rust
use latex::{parse, Node};

let doc = parse("\\begin{verbatim}let x = {1};\n$y$\\end{verbatim}").unwrap();
assert!(matches!(doc[0], Node::VerbatimEnv { .. }));   // body kept literal, $/{} not special
```

Only `verbatim`/`verbatim*` divert to raw scanning; every other `\begin{…}` is parsed
structurally. An unterminated `\verb` (or a `*`/space delimiter, or a body past the line end)
and an unterminated `verbatim` environment are spanned errors — never a mis-parse.

### Text accents (L5c)

`recognize_accents` is an opt-in pass (like `expand`) that folds an accent control sequence
and the character it accents into a `Node::Accent` — both spellings, `\'e` and `\'{e}`,
recognize to the same node and round-trip:

```rust
use latex::{parse, recognize_accents, Node};

let doc = recognize_accents(parse(r"caf\'e").unwrap());
assert!(matches!(doc[1], Node::Accent { .. }));   // é over `e`; "caf" stays text
```

Recognized: `\'  \`  \^  \"  \~  \=  \.` and `\u \v \H \c \d \b \r \t`. A dangling accent (no
accent-able char after it) is left as a plain command — never dropped.

### Document structure (L5d)

`recognize_structure` is the second opt-in classification pass (like `recognize_accents`). It
turns the *generic* commands L1 produces into **semantic** structure nodes — headings,
cross-references, preamble directives, and argument-form font commands — while leaving L1's
round-trip intact:

```rust
use latex::{parse, recognize_structure, Node, SectionLevel};

let doc = recognize_structure(parse(r"\section*{Intro} see \ref{fig:1}").unwrap());
assert!(matches!(doc[0], Node::Section { level: SectionLevel::Section, starred: true, .. }));
assert!(doc.iter().any(|n| matches!(n, Node::CrossRef { .. })));   // \ref{fig:1}
```

Recognized:

- **`Node::Section`** — `\part`/`\chapter`/`\section`/`\subsection`/`\subsubsection`/
  `\paragraph`/`\subparagraph`, the starred `\section*{…}` form (the `*` sibling is folded),
  and the optional short TOC title `\section[Short]{Title}`;
- **`Node::CrossRef`** — `\label`/`\ref`/`\eqref`/`\pageref`/`\autoref`/`\nameref`/`\cite`/
  `\citep`/`\citet` (the `\cite[note]{key}` optional is kept);
- **`Node::Preamble`** — `\documentclass`/`\usepackage`/`\RequirePackage` with `[options]`;
- **`Node::Styled`** — argument-form font commands (`\textbf`, `\textit`, `\texttt`, `\emph`,
  `\underline`, …).

A command that does not match its expected shape (a sectioning command with no title, a
cross-ref with no key) is left as a plain command — never dropped or mis-folded. Font
*declarations* (`\bfseries`, `\itshape`, `\large`, …) also stay plain commands: their effect is
positional (until end of group), so wrapping them in an argument node would misrepresent them.
The pass is idempotent and round-trips: `recognize_structure(parse(&n.to_latex())) == [n]`.
(The two passes — `recognize_accents` and `recognize_structure` — are independent and compose.)

### Document-mode tables & lists (D1)

`recognize_tables` is the third opt-in classification pass (like the two above). It folds the
*generic* environments L1 produces for document-mode `tabular`/`tabular*` grids and the
`itemize`/`enumerate`/`description` list environments into structured nodes — splitting a table
body on the `&` alignment tab and the `\\` row break, and a list body on `\item`:

```rust
use latex::{parse, recognize_tables, Node, ListKind};

let table = recognize_tables(parse(r"\begin{tabular}{lc}a & b \\ c & d\end{tabular}").unwrap());
assert!(matches!(table[0], Node::Tabular { .. }));   // 2×2 grid, col_spec = Some("lc")

let list = recognize_tables(parse(r"\begin{itemize}\item one\item two\end{itemize}").unwrap());
assert!(matches!(list[0], Node::List { kind: ListKind::Itemize, .. }));
```

Recognized:

- **`Node::Tabular { col_spec, rows }`** — `tabular`/`tabular*`; `rows[r][c]` is the node
  sequence of cell `c` in row `r`; `col_spec` is the column spec captured verbatim (`None` if
  absent). A `tabular*` `{width}` argument is dropped, keeping the trailing `{colspec}`.
- **`Node::List { kind, items }`** — `itemize`/`enumerate`/`description`; each `ListItem` carries
  its `\item[term]` optional `label` and the `body` up to the next `\item`.

The pass is **total and infallible**: ragged rows (differing cell counts) are preserved exactly,
and a list with stray content before its first `\item` is left as a generic `Node::Environment`
— never an error here (truly malformed input — unbalanced braces, `\begin`/`\end` mismatch — is
already rejected by the L1 parser with a spanned error, upstream of this pass). It is idempotent
and round-trips: `recognize_tables(parse(&n.to_latex())) == [n]`. All three recognition passes
are independent and compose.

### Pluggable frontend (L6)

The capstone: `LatexMath` implements the [`math-frontend`](../math-frontend) `MathFrontend`
trait, so LaTeX math plugs into the shared, notation-agnostic registry. `parse` runs the math
grammar and **lowers** the LaTeX-shaped `MathNode` into the neutral `MathExpr` — two source
strings that mean the same math produce the same tree, so a consumer lowers *one* AST and gets
every notation for free:

```rust
use latex::registry;                       // a FrontendRegistry with LaTeX installed
use math_frontend::{MathExpr, BinOp};

let reg = registry();
assert_eq!(reg.names(), ["latex"]);

// \times, \cdot, and juxtaposition all normalize to the same neutral Mul:
let a = reg.parse("latex", r"a \times b").unwrap();
assert_eq!(a, reg.parse("latex", "ab").unwrap());
assert!(matches!(a, MathExpr::Bin(BinOp::Mul, _, _)));
```

Lowering drops *presentation* and keeps *meaning*: fence style → `Group`, matrix delimiter →
`Matrix`, `a^n` → `Pow`, `a_i` → `Subscript`, accents → `Call`; numbers stay **exact**
(`MathExpr::Number`, never `f64`). `\pm`/`\mp` lower to `BinOp::PlusMinus`/`MinusPlus` (the ± / ∓
pair operators) and `\binom{n}{k}` to `MathExpr::Binom` — every LaTeX math construct the grammar
parses now has a faithful neutral counterpart (the two former gaps were closed by extending the
`math-frontend` neutral AST, not by faking them here). The adapter sits behind the default-on
**`frontend`** feature; build with `--no-default-features` for the zero-dependency L0–L5 parser
alone.

The low-level `tokenize` is also public. Tokens and errors carry half-open byte `Span`s;
all of `parse`, `parse_math`, and `tokenize` return spanned errors rather than panicking,
and recursion is depth-guarded so adversarial nesting errors instead of overflowing.

### Deep-tree drop safety

`MathNode` is a recursive `Box`-owning tree, so a naive (compiler-derived) destructor would
recurse once per level. The parser's `MAX_DEPTH` bounds *nesting*, but left-associative
chains — `1+1+1+…`, juxtaposition `aaa…` — are built in loops with no per-term depth charge,
so they produce O(n)-deep trees that `parse_math` happily returns (it builds iteratively).
Dropping such a tree would overflow the stack: an **uncatchable abort**. To prevent it,
`MathNode` implements `Drop` explicitly, dismantling the tree with a **heap worklist** (each
boxed child is moved onto a `Vec`, replaced in place by a cheap leaf, then popped) so the
generated destructor recurses at most one trivial level — O(1) stack depth at any size. The
neutral `math_frontend::MathExpr` does the same. (Consequence: because `MathNode: Drop`, you
cannot move fields out of an owned `MathNode` in a by-value `match` — borrow with `match &node`
and lift children via `mem::replace`/`Option::take`.)

## Tests

```
cargo test -p latex
cargo clippy -p latex -- -D warnings
```
