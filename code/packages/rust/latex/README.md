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
| **D1 doc tables/lists** | document-mode `tabular`/`tabular*` grids (split on `&`/`\\`) → `NodeKind::Tabular` and `itemize`/`enumerate`/`description` (split on `\item`) → `NodeKind::List`, via the opt-in `recognize_tables` pass; total, round-trip | ✅ |
| **D2 Document skeleton** | hierarchical `Document` model: preamble/body split at `\begin{document}`, `\documentclass`/`\usepackage` classified, body lowered to a **flat** `Vec<Block>` (headings → zero-body `Block::Section`; paragraphs/lists/tables/display-math/environments; inline runs → `Vec<Inline>`); `parse_document`/`build_document` + `Document::to_latex` round-trip; body spans precise per node as of 0.34.0 (S3) | ✅ |
| **D3 sectioning fold** | folds the flat block stream into the **nested sectioning forest**: each heading OWNS the run of following blocks up to the next heading of same-or-higher level (`\part > \chapter > \section > … > \subparagraph`, via `rank(level)`); deeper headings nest. A trailing `\label{key}` is hoisted onto its section's new `label` field. Applies to every block-list (top-level + environment/list/table bodies). Total & panic-free; `to_latex` fixed point; `flatten(fold(flat)) == flat` property test; folded-section span = union of heading ∪ children spans | ✅ |
| **D4 metadata** | extracts `\title`/`\author{A \and B}`/`\date` and the `abstract` env into a typed `Metadata` record on `Document`, as an **additive projection** — the underlying nodes stay in `preamble`/`body`, so `to_latex` round-trips unchanged and re-parsing repopulates the same `Metadata` (fixed point). Both preamble and body scanned; first title/date wins; every `\author` (each `\and`-split) contributes; `\maketitle` is a metadata no-op. Total & panic-free; never fabricated. (Inline normalization — `\textbf`/`\emph`/`\texttt`/`$…$`/`\ref`/accents — already lands in D2/D3's `lower_inline`.) | ✅ |
| **D5 floats/code/display-math** | specializes the generic environment fold by name: `figure`/`figure*` → `Block::Figure` and `table`/`table*` → the inner `Block::Table`, each with `\caption{…}` → `Caption` + a hoisted `\label`; `verbatim`/`lstlisting` → `Block::CodeBlock` (raw text kept unparsed); `equation`/`align`/`gather`/… → `Block::DisplayMath` (source kept, delegated to the math frontend on demand); `quote`/`quotation` → `Block::Quote`; any other env → recursed `Block::Environment`. `to_latex` fixed point; total & panic-free; body spans precise per node as of 0.34.0 (S3) | ✅ |
| **D6 provenance API (capstone)** | the byte-provenance surface: `Document::walk()` — a pre-order, depth-first `impl Iterator<Item = NodeRef>` over every body block + nested inline; `Document::node_at(byte)` — the innermost walked node whose span contains a source byte, returned as `Provenance { node, span }`; `NodeRef::{span,kind}`. A capstone real-paper corpus (article + abstract + tabular + itemize + inline/display math + figure+caption+label + `\cite`) proves a `to_latex` fixed point, a non-panicking `walk()`, and **byte coverage**: every non-whitespace body-region byte is owned by ≥1 walked node. Panic-free (`saturating_sub`); spans region-coarse at D6, made precise by LTXDOC02 S3, and `node_at` resolves to the true body leaf as of S4 (below). | ✅ |
| **LTXDOC02 S1 — spanned L1 nodes** | `Node` restructured to `{ kind: NodeKind, span: Span }`; `parse()` threads each token's exact byte span onto the node it builds, so `&src[node.span()]` slices back to the node's own source (`\textbf{x}`, `{…}` incl. braces, `$…$` incl. delimiters, `\begin{env}…\end{env}`, a `Text` run's exact chars). Span is orthogonal to shape; `PartialEq` ignores it (round-trip = fixed point **modulo spans**); `Unsupported`'s bespoke tuple folded onto the uniform `Node.span`. `to_latex` unchanged. The one parser-level rung of the precise-spans arc (recognition-pass + Document-fold precision = S2/S3). | ✅ |
| **LTXDOC02 S2 — spanned recognition passes** | the opt-in recognition passes now give each *synthesised* node the exact union of its constituents' real S1 spans: `recognize_structure` (`Section`/`CrossRef`/`Preamble`/`Styled` = recognizing command ∪ each argument), `recognize_accents` (`Accent` = command ∪ argument), `recognize_tables` (`Tabular` = `\begin{tabular}…\end{tabular}` ∪ every cell; `List` = `\begin{env}…\end{env}` ∪ every item). `&src[node.span()]` slices back to the exact source extent for a Section (heading through owned body), an Accent, a Tabular, and an itemize List. Unions over real child spans (never substring search); `to_latex` + round-trip-modulo-spans unchanged. Document-fold precision = S3. | ✅ |
| **LTXDOC02 S3 — precise Document fold** | `build_document` reads each source `Node`'s carried, precise span instead of the coarse enclosing `region`, so **every body `Block`/`Inline` span is now the node's tight source range**: `&src[inline.span]` slices back to exactly a `Text` run's word, a `\textbf{…}`, an inline `$…$`, a `\cite{…}`. Composites union their children's real spans (`Paragraph` = ∪ its inlines; `Section` = heading ∪ owned body; `List`/`Tabular`/`Environment`/`Figure` = the `\begin…\end` extent; captioned `table` float = tabular ∪ float; `DocListItem` = term ∪ body; `Caption` = ∪ its content). The `region` parameter is deleted from the fold helpers (a fallback-only seed survives on `lower_blocks`). Preamble/`DocumentClass`/`Package` stay honestly preamble-region-coarse (classified, not walked). `to_latex` + round-trip-modulo-spans unchanged; precise `node_at` + coverage capstone = S4/S5. | ✅ |
| **LTXDOC02 S4 — precise `node_at`, region-coarse caveat retired** | with S3's tight body spans, `Document::node_at(byte)` **formally** resolves to the **true per-token leaf** — the narrowest node whose *precise* span contains the byte (ties → deepest in pre-order): a byte inside `widgets` → the `Text` run owning `widgets` (not the enclosing `Paragraph`/`Section`); a byte inside a `\section` title → the title inline (not the whole `Section`). Docs-and-tests rung (no `node_at`/`walk` logic change): retired the region-coarse hedging on `node_at`/`Provenance`/`walk`/module note for **body** nodes, kept the honest coarse note on `Preamble`/`DocumentClass`/`Package`. New leaf-resolution tests + an honest body byte-coverage test (every non-whitespace body byte resolves to a node whose precise span contains it, on a representative input; whole-corpus tightest-leaf capstone = S5). | ✅ |
| **LTXDOC02 S5 — precise byte-coverage capstone (arc COMPLETE)** | the capstone `capstone_every_body_byte_resolves_to_tightest_covering_node` proves, over the same LTXDOC01 D6 representative corpus, that **every** non-whitespace body byte (a) resolves (`node_at(b).is_some()`) AND (b) resolves to the **tightest-covering** walked node — no *other* walked node whose span is a strict subset also contains the byte. Honest, not overclaimed: the load-bearing gate is tightest-covering, **not** "always a `Text` leaf" — structural bytes (`\section`/`\item`/`\begin{…}` machinery, inter-child delimiters) legitimately resolve to their enclosing composite, which is the tightest cover there (a soft signal records that the *majority* of content bytes still land on leaves). No `node_at`/parser/fold logic change (S1–S4 already made spans precise + `node_at` leaf-resolving); pure test rung. Corpus fixed/bounded ⇒ O(len), not a DoS. **Completes the LTXDOC02 precise-per-token-spans arc.** | ✅ |
| **LTXDOC03 S1 — cross-reference resolution (label table + `\ref` binding)** | `Document::resolve_references() -> ReferenceResolution` — a pure, additive pass (no parser/fold/`walk`/`node_at`/span change) that binds each cross-reference to the `\label` that defines it, **with byte spans on both sides**. Collects the label table from hoisted section/table/figure labels (`Block::…{ label: Some(k) }`) and inline `\label{k}` (`Inline::CrossRef`); resolves the reference family `{ref, eqref, pageref}` against it → `ResolvedRef` (ref-span **and** target def-span + `LabelKind`) or `UnresolvedRef` (dangling key + ref-span); reports `Duplicate` keys with **first-def-wins**. `\cite` is deferred (bibliography = a separate table, a later rung). The static analogue of LaTeX's two-pass `.aux` machinery, binding *structure* not numbers/pages. Total & panic-free, reuses the bounded `walk` (no new recursion). | ✅ |

The low-level ladder is **complete** (L0–L6). 🎉 The hierarchical **Document** layer (LTXDOC01) is
now **complete too** — D1–D6 all shipped, taking LaTeX → `Document` AST **end-to-end**: source →
tables/lists (D1) → preamble/body skeleton (D2) → sectioning forest (D3) → metadata + inline
normalization (D4) → floats/code/display-math (D5) → provenance API + byte-coverage capstone (D6).
The **precise per-token spans** arc (LTXDOC02) is now **complete** 🎉: **S1 shipped spanned L1 nodes**
(`parse()` retains the exact byte range it already computed for each node), **S2 shipped spanned
recognition passes** (each synthesised `Section`/`CrossRef`/`Preamble`/`Styled`/`Accent`/`Tabular`/
`List` node carries the exact union of its constituents' spans), **S3 shipped the precise
Document fold** (every body `Block`/`Inline` span is now the node's tight source range, composites
union their children's real spans, and the coarse `region` plumbing is deleted), **S4 retired
the region-coarse caveat for body nodes** (`node_at(byte)` now formally resolves to the true
per-token leaf; preamble/metadata stay honestly coarse), and **S5 shipped the precise byte-coverage
capstone** — over the representative corpus, every non-whitespace body byte resolves to the
*tightest-covering* walked node (no strictly-narrower walked node also covers it), stated honestly as
tightest-covering rather than leaf-only.

The **document-feature** arc (LTXDOC03) now builds on that precise-span foundation: **S1 ships
cross-reference resolution** (`Document::resolve_references`) — a pure, additive pass that binds each
`\ref`/`\eqref`/`\pageref` to the `\label` that defines it, with byte spans on both sides, reporting
duplicate and dangling references. `\cite`/bibliography binding is a deferred later rung.

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
// D3: the heading OWNS the following blocks — its `body` is the nested sectioning forest.
if let Block::Section { body, .. } = &doc.body[0] {
    assert!(matches!(body[0], Block::Paragraph(..)));         // "Hello world." is owned by Intro
}
assert_eq!(doc.to_latex().is_empty(), false);                // round-trips (modulo spans)
```

### Document metadata (LTXDOC01 D4)

`Document::metadata` is a typed `Metadata { title, authors, date, abstract_ }` record — ask "what is
the title / who are the authors / what is the abstract?" without walking the tree:

```rust
use latex::parse_document;

let doc = parse_document(
    r"\title{Paper}\author{Alice \and Bob}\begin{document}\maketitle\begin{abstract}An abstract.\end{abstract}Body.\end{document}",
).unwrap();

assert_eq!(doc.metadata.authors.len(), 2);           // \and split → two entries
assert!(doc.metadata.title.is_some());
assert!(doc.metadata.abstract_.is_some());
```

Metadata is an **additive projection**: the `\title`/`\author`/`\date` commands and the `abstract`
environment are *not* removed — they still live in `preamble`/`body`, so `to_latex` round-trips
unchanged and re-parsing repopulates the identical `Metadata` (a fixed point). Absent directives
leave the fields `None`/empty — never fabricated.

### Floats, code & display math (LTXDOC01 D5)

D5 gives the semantic block kinds a real paper uses, by specializing the environment fold on the
`\begin{env}` name:

```rust
use latex::{parse_document, Block};

let doc = parse_document(concat!(
    r"\begin{document}",
    r"\begin{figure}\includegraphics{plot.png}\caption{A plot}\label{fig:p}\end{figure}",
    r"\begin{table}\begin{tabular}{lc}a & b \\ c & d\end{tabular}\caption{Grid}\label{tab:g}\end{table}",
    r"\begin{equation}E = mc^2\end{equation}",
    r"\end{document}",
)).unwrap();

// A `figure` float: caption + label lifted, the \includegraphics body preserved.
assert!(doc.body.iter().any(|b| matches!(b, Block::Figure { caption: Some(_), label: Some(_), .. })));
// A `table` float attaches its \caption/\label to the *inner* tabular's Block::Table.
assert!(doc.body.iter().any(|b| matches!(b, Block::Table { caption: Some(_), .. })));
// A named display-math environment keeps its source string (unparsed here).
assert!(doc.body.iter().any(|b| matches!(b, Block::DisplayMath { .. })));
```

- **`figure`/`figure*`** → `Block::Figure { content, caption, label }` — the `\caption{…}` becomes a
  `Caption`, a following `\label{…}` is hoisted, and everything else (e.g. `\includegraphics`) stays
  in `content`.
- **`table`/`table*`** → the inner `Block::Table` with the float's caption/label attached (a float
  with no tabular degrades to a `Block::Figure`, so nothing is lost).
- **`verbatim`/`lstlisting`** → `Block::CodeBlock` — the body is kept **unparsed** (code is source,
  not marked-up LaTeX).
- **`equation`/`align`/`gather`/`displaymath`/…** → `Block::DisplayMath` — the inner LaTeX is kept
  as a source string, delegated to the math frontend on demand (LTXDOC01 never parses math itself).
- **`quote`/`quotation`** → `Block::Quote`; any other environment stays a recursed
  `Block::Environment`.

Caption/label extraction mirrors D3's `\label` hoist and never drops float content; `to_latex`
re-emits the `figure`/`table` wrapper (with `\caption`/`\label`), `verbatim` fences, and `$$…$$`
so `parse(to_latex(d)).strip_spans() == d.strip_spans()` remains a fixed point.

**Body spans are precise as of 0.34.0 (LTXDOC02 S3), and `node_at` resolves to the true body leaf as
of 0.35.0 (S4):** every body block/inline span is the source
node's own tight byte range (`&src[node.span]` slices back to exactly its source), and a composite's
span is the **union** of its children's real spans — so every child span ⊆ its parent ⊆ the
`Document` span still holds, and now the leaves are tight. Only `Preamble` / `DocumentClass` /
`Package` remain honestly preamble-region-coarse (the preamble is classified out of directives, not
walked as per-node body content).

### The provenance API — walk / node_at (LTXDOC01 D6, arc complete)

D6 exposes the byte-provenance surface the ADJ reasoning pipeline consumes — "which node owns this
source byte?" and "visit every node in order":

```rust
use latex::parse_document;

let d = parse_document(r"\begin{document}\section{Intro}Body text.\end{document}").unwrap();

// walk(): pre-order, depth-first over every body Block + nested Inline.
let kinds: Vec<&str> = d.walk().map(|n| n.kind()).collect();
// e.g. ["Section", "Text", "Paragraph", "Text", …] — a parent precedes its children.

// node_at(byte): the true per-token leaf — the narrowest node whose precise span contains the byte.
if let Some(p) = d.node_at(35) {
    let _ = (p.node.kind(), p.span); // NodeRef + its span
}
assert!(d.node_at(usize::MAX).is_none()); // out of range → None, never panics
```

**Granularity.** As of 0.35.0 (S4) body spans are precise and `node_at(byte)` **formally resolves to
the true per-token leaf** — the narrowest node whose precise span contains the byte (ties → the
deepest node in pre-order). A byte inside `widgets` resolves to the `Text` run owning `widgets`, not
to the enclosing `Paragraph`/`Section`; a byte inside a `\section` title resolves to the title
inline, not the whole `Section`. This holds for **body** nodes (the ones `walk` visits);
`Preamble`/`DocumentClass`/`Package` stay honestly preamble-region-coarse (classified out of
directives, not walked, and `node_at` never resolves into them). The region-scoped capstone
byte-coverage test asserts every non-whitespace byte inside the document **body region** is owned by
≥1 walked node; and as of 0.36.0 (**S5**, arc complete) the whole-corpus capstone
`capstone_every_body_byte_resolves_to_tightest_covering_node` strengthens this to
**tightest-covering** — every non-whitespace body byte resolves to the innermost walked node, with
no strictly-narrower walked node also covering it (stated honestly as tightest-covering, since
structural bytes legitimately resolve to their enclosing composite rather than a `Text` leaf).

### Cross-reference resolution (LTXDOC03 S1)

Built on top of the precise spans, `Document::resolve_references()` binds each cross-reference to the
`\label` that defines it — the static, single-pass analogue of LaTeX's two-pass `.aux` machinery, but
carrying the defining node's **source bytes** rather than its number/page. It is pure analysis: it
changes nothing about the parser, the fold, `walk`, `node_at`, any span, or the `to_latex`
round-trip.

```rust
use latex::{parse_document, LabelKind};

let src = r"\begin{document}\section{Intro}\label{sec:intro}

See Section~\ref{sec:intro}, and also \ref{missing}.\end{document}";
let doc = parse_document(src).unwrap();
let res = doc.resolve_references();

// The `\label` is a definition (a section-kind label).
assert_eq!(res.definitions[0].kind, LabelKind::Section);

// The `\ref{sec:intro}` RESOLVES: both spans slice back to real source.
let r = &res.resolved[0];
assert_eq!(&src[r.ref_span.start..r.ref_span.end], r"\ref{sec:intro}");
assert!(src[r.target_span.start..r.target_span.end].starts_with(r"\section{Intro}"));

// The `\ref{missing}` is dangling (LaTeX's "Reference `missing' undefined").
assert_eq!(res.unresolved[0].key, "missing");
```

The reference family is `{ref, eqref, pageref}`; `\cite` is deferred (it resolves against a
bibliography — a separate table — in a later rung), and a multiply-defined key is reported as a
`Duplicate` with **first-def-wins** for resolution. A resolved `\ref` therefore points at the exact
defining node's source bytes — the source→source correlation the ADJ byte-provenance pipeline audits.

## Usage

```rust
use latex::{parse, NodeKind};

let doc = parse(r"Let $x$ be \textbf{bold}.").unwrap();
assert!(matches!(doc[0].kind, NodeKind::Text(_)));                       // "Let"
assert!(doc.iter().any(|n| matches!(n.kind, NodeKind::Math { .. })));    // $x$
// every node carries its exact source byte span (LTXDOC02 S1):
let src = r"Let $x$ be \textbf{bold}.";
let d = parse(src).unwrap();
assert_eq!(&src[d[0].span().start..d[0].span().end], "Let");
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
tokenizer suspends catcodes inside, so `{ } $ # \` are literal — producing a `NodeKind::Verb`
that round-trips:

```rust
use latex::{parse, Node};

let doc = parse(r"call \verb|x{y}$z| now").unwrap();
assert!(matches!(doc[1].kind, NodeKind::Verb { delim: '|', .. }));   // body "x{y}$z" kept verbatim
```

The **`verbatim` environment** (and `verbatim*`) reads its whole body raw — newlines included —
up to the matching `\end{verbatim}`, producing a `NodeKind::VerbatimEnv` that also round-trips:

```rust
use latex::{parse, Node};

let doc = parse("\\begin{verbatim}let x = {1};\n$y$\\end{verbatim}").unwrap();
assert!(matches!(doc[0].kind, NodeKind::VerbatimEnv { .. }));   // body kept literal, $/{} not special
```

Only `verbatim`/`verbatim*` divert to raw scanning; every other `\begin{…}` is parsed
structurally. An unterminated `\verb` (or a `*`/space delimiter, or a body past the line end)
and an unterminated `verbatim` environment are spanned errors — never a mis-parse.

### Text accents (L5c)

`recognize_accents` is an opt-in pass (like `expand`) that folds an accent control sequence
and the character it accents into a `NodeKind::Accent` — both spellings, `\'e` and `\'{e}`,
recognize to the same node and round-trip:

```rust
use latex::{parse, recognize_accents, Node};

let doc = recognize_accents(parse(r"caf\'e").unwrap());
assert!(matches!(doc[1].kind, NodeKind::Accent { .. }));   // é over `e`; "caf" stays text
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
assert!(matches!(doc[0].kind, NodeKind::Section { level: SectionLevel::Section, starred: true, .. }));
assert!(doc.iter().any(|n| matches!(n.kind, NodeKind::CrossRef { .. })));   // \ref{fig:1}
```

Recognized:

- **`NodeKind::Section`** — `\part`/`\chapter`/`\section`/`\subsection`/`\subsubsection`/
  `\paragraph`/`\subparagraph`, the starred `\section*{…}` form (the `*` sibling is folded),
  and the optional short TOC title `\section[Short]{Title}`;
- **`NodeKind::CrossRef`** — `\label`/`\ref`/`\eqref`/`\pageref`/`\autoref`/`\nameref`/`\cite`/
  `\citep`/`\citet` (the `\cite[note]{key}` optional is kept);
- **`NodeKind::Preamble`** — `\documentclass`/`\usepackage`/`\RequirePackage` with `[options]`;
- **`NodeKind::Styled`** — argument-form font commands (`\textbf`, `\textit`, `\texttt`, `\emph`,
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
assert!(matches!(table[0].kind, NodeKind::Tabular { .. }));   // 2×2 grid, col_spec = Some("lc")

let list = recognize_tables(parse(r"\begin{itemize}\item one\item two\end{itemize}").unwrap());
assert!(matches!(list[0].kind, NodeKind::List { kind: ListKind::Itemize, .. }));
```

Recognized:

- **`NodeKind::Tabular { col_spec, rows }`** — `tabular`/`tabular*`; `rows[r][c]` is the node
  sequence of cell `c` in row `r`; `col_spec` is the column spec captured verbatim (`None` if
  absent). A `tabular*` `{width}` argument is dropped, keeping the trailing `{colspec}`.
- **`NodeKind::List { kind, items }`** — `itemize`/`enumerate`/`description`; each `ListItem` carries
  its `\item[term]` optional `label` and the `body` up to the next `\item`.

The pass is **total and infallible**: ragged rows (differing cell counts) are preserved exactly,
and a list with stray content before its first `\item` is left as a generic `NodeKind::Environment`
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
