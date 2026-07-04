# LTXDOC01 — LaTeX → Document AST (a hierarchical, byte-provenanced document model)

> Status: **spec** (this document ships before any implementation, per repo standard #8).
> Companion to [`LTX01-full-latex-parser.md`](LTX01-full-latex-parser.md), which specifies the
> low-level `latex` crate (catcodes → tokens → a **flat** structural `Vec<Node>` + a math frontend).
> LTXDOC01 specifies the layer **above** that flat node stream: a faithful, reusable, hierarchical
> **`Document`** AST with a byte-span on every node — the thing the directive *"in the future we
> should be able to go from LaTeX to a Document AST"* names.

## 1. Why a second layer (and why not just `Vec<Node>`)

LTX01's `latex::parse(src) -> Vec<Node>` is already a real document parser: it lexes with a
text/math **mode stack**, matches `\begin{env}…\end{env}`, captures `\cmd[opt]{arg}`, and an opt-in
`recognize_structure` pass classifies generic commands into `Section`, `CrossRef`, `Preamble`,
`Styled`, `Accent`, … So ~80% of the *lexical* work is done. But `Vec<Node>` is deliberately **flat
and presentation-shaped**:

- `Node::Section` is a **sibling marker**, not a container. `\section{A} …text… \subsection{B} …`
  parses to `[Section(A), Text…, Section(B), Text…]` — the reader can't ask "what is inside section
  A?" without re-deriving the hierarchy every time.
- There is no **preamble / body** split, no document **metadata** (`\title`/`\author`/`\date`,
  `abstract`), and no notion of **block vs inline** content, **lists**, **tables**, or **floats**.
- Consumers that want *meaning* (render to HTML/Markdown, convert formats, diff two revisions, or —
  our north star — extract **byte-provenanced facts** for the ADJ→CPU reasoning pipeline) each have
  to re-implement the same fold. That violates repo principle *"generic engines over domain-specific
  point solutions"*: the fold should live **once**, in a reusable `Document` model.

LTXDOC01 is that fold: a **pure, allocation-only, side-effect-free** transformation
`Vec<Node> -> Document`, plus the `Document` type. It adds **no new parsing** of raw source — it
consumes LTX01's output — with the sole exception of the **document-mode environment gap** (tables &
lists) that LTX01 left as `L3b (later)`; LTXDOC01 owns closing that gap because a document AST is
meaningless if `\begin{tabular}` and `\begin{itemize}` still error.

### The design axis chosen: **generic + byte-provenance** (per the directive)
Every `Document` node carries a `Span { start: usize, end: usize }` (byte offsets into the ORIGINAL
source, half-open). The model is **generic** (Pandoc/DOM-shaped — usable for rendering, conversion,
structure queries, diffing) **and** provenance-complete (spans everywhere), so it serves both an
ordinary document consumer and the byte-provenance reasoning pipeline from ONE artifact. This is a
strict superset of a "generic only" or "provenance only" model — we build the superset once.

## 2. The honest limit (inherited from LTX01)

LTXDOC01 is a **document-structure** model, not a typesetter. It does **not** compute page layout,
resolve counters to printed numbers (section "3.2", figure "1"), run BibTeX, or execute Turing-
complete TeX (`\catcode` at runtime, `\csname`, arbitrary `\if…`). Anything LTX01 surfaces as
`Node::Unsupported { construct, span }` is carried through verbatim as a
`Block::Unsupported`/`Inline::Unsupported` with its span — **never dropped, never guessed**. The fold
is *total*: every input `Node` maps to exactly one output node or is attached to one (spans union),
so **no byte of source is silently lost** (a property the ADJ pipeline's total-coverage discipline
depends on — see the byte-coverage gate in the reasoning stack).

## 3. The `Document` AST

All nodes are `#[non_exhaustive]`-friendly in spirit (we may add variants; consumers match with a
wildcard arm). Spans are byte offsets into the original `&str`.

```rust
pub struct Span { pub start: usize, pub end: usize }   // half-open [start, end)

pub struct Document {
    pub preamble: Preamble,          // everything before \begin{document} (or whole doc if none)
    pub metadata: Metadata,          // \title/\author/\date/abstract, extracted from preamble+body
    pub body: Vec<Block>,            // the sectioning forest (top-level blocks + section trees)
    pub span: Span,                  // whole source
}

pub struct Preamble {
    pub document_class: Option<DocumentClass>,   // \documentclass[opts]{article}
    pub packages: Vec<Package>,                  // \usepackage[opts]{amsmath}, …
    pub raw: Vec<Node>,                          // untouched LTX01 nodes (macro defs, custom setup)
    pub span: Span,
}

pub struct Metadata {                            // None fields when absent — never fabricated
    pub title:  Option<Vec<Inline>>,
    pub authors: Vec<Vec<Inline>>,               // \author{A \and B} → two entries
    pub date:   Option<Vec<Inline>>,
    pub abstract_: Option<Vec<Block>>,
}

pub enum Block {
    // Sectioning — the container the flat Node::Section could not be:
    Section { level: SectionLevel, numbered: bool, title: Vec<Inline>,
              short_title: Option<Vec<Inline>>, label: Option<String>,
              body: Vec<Block>, span: Span },
    Paragraph(Vec<Inline>, Span),
    List { kind: ListKind, items: Vec<ListItem>, span: Span },        // itemize/enumerate/description
    Table { spec: ColumnSpec, rows: Vec<Row>, caption: Option<Caption>,   // tabular (+ table float)
            label: Option<String>, span: Span },
    Figure { content: Vec<Block>, caption: Option<Caption>, label: Option<String>, span: Span },
    CodeBlock { verbatim: String, span: Span },                       // verbatim env / lstlisting
    DisplayMath { source: String, span: Span },                       // \[..\], equation, align…
    Quote(Vec<Block>, Span),                                          // quote/quotation
    Environment { name: String, body: Vec<Block>, span: Span },       // any other known env, recursed
    Raw(Node, Span),                                                  // an LTX01 node with no block meaning
    Unsupported { construct: String, span: Span },
}

pub enum Inline {
    Text(String, Span),
    Emph(Vec<Inline>, Span),  Strong(Vec<Inline>, Span),  Code(String, Span),  // \emph \textbf \texttt
    Styled { command: String, content: Vec<Inline>, span: Span },   // other \text.. font commands
    Math { source: String, span: Span },                            // inline $..$ / \(..\)
    CrossRef { kind: RefKind, target: String, note: Option<Vec<Inline>>, span: Span }, // \ref \cite …
    Accent { accent: String, base: Box<Inline>, span: Span },       // \'e etc. (from LTX01 recognize_accents)
    LineBreak(Span),                                                 // \\ in text mode
    Space(Span),
    Raw(Node, Span),
    Unsupported { construct: String, span: Span },
}

pub enum ListKind { Itemize, Enumerate, Description }
pub struct ListItem { pub term: Option<Vec<Inline>>, pub body: Vec<Block>, pub span: Span } // term = \item[..]
pub struct Row { pub cells: Vec<Vec<Block>>, pub span: Span }        // \\-separated; & splits cells
pub struct ColumnSpec { pub raw: String, pub columns: Vec<ColumnAlign> } // {l|c|r p{3cm}} → aligns
pub struct Caption { pub content: Vec<Inline>, pub span: Span }
```

Notes that keep the model faithful, not lossy:
- **Math is kept as its source string + span**, not eagerly parsed — a Document consumer that wants
  the math tree calls the existing `latex` math frontend / `math-frontend::MathExpr` on `source`
  (the PFE01 pipeline). LTXDOC01 does not re-implement math; it points at the island. (An optional
  `parse_math: bool` builder flag MAY eagerly attach a `MathExpr`, but the default keeps the two
  layers decoupled.)
- **Unknown-but-well-formed** commands/environments survive as `Raw`/`Environment`, so the fold is
  lossless and forward-compatible; only genuinely malformed input (already `Node::Unsupported`)
  becomes `Unsupported`.
- **`numbered`** distinguishes `\section` from `\section*`; **`label`** hoists a following
  `\label{…}` onto its section/table/figure so cross-references resolve structurally.

## 4. Public API

```rust
// The one entry point most callers use — source straight to Document:
pub fn parse_document(src: &str) -> Result<Document, ParseError>;

// The fold in isolation, for callers who already hold LTX01 nodes (e.g. after macro expansion):
pub fn build_document(nodes: Vec<Node>, src_len: usize) -> Document;   // total; never errors

impl Document {
    pub fn to_latex(&self) -> String;                 // round-trips to re-parseable source
    pub fn node_at(&self, byte: usize) -> Option<Provenance>;  // provenance query: which node owns a byte
    pub fn walk(&self) -> impl Iterator<Item = NodeRef<'_>>;   // pre-order, span-annotated
}
```

`parse_document(src)` = `build_document(recognize_structure(recognize_accents(parse(src)?)),
src.len())` after the L3b table/list recognition runs — i.e. it composes the shipped LTX01 passes and
then folds. Macro expansion (`expand`) is opt-in and, when desired, runs before the fold.

## 5. Conformance ladder = PR staging

Each rung is a small, reviewable, fully-tested PR (repo principle: 20–25 small PRs, mark blocking
deps). D1 is the only rung that touches the low-level parser; D2–D6 are pure folds over its output.
Home crate: the existing **`latex`** crate (a new `document.rs` module + additions to `ast.rs`),
gated so the dependency-free L0–L5 core is unaffected. **Every rung: spec-sync → tests → impl →
CHANGELOG → README → `/security-review` → babysit.** No `unsafe`; `cargo clippy -p latex -- -D
warnings` clean; a byte-span on every node asserted in tests.

- **D1 — close the LTX01 L3b gap: document-mode tables & lists (blocking prerequisite).**
  In the `latex` crate proper (parser + ast): text-mode `tabular`/`tabular*`/`array`-in-text and the
  list environments `itemize`/`enumerate`/`description` with `\item` (and `\item[term]`). New
  `Node::Tabular { col_spec, rows }` and `Node::List { kind, items }` with `&`/`\\` cell/row
  splitting and spanned errors on malformed grids. `to_latex` round-trip. *Until D1, `parse_document`
  cannot see a real paper without erroring — so D1 ships first.*
- **D2 — `Document` skeleton + preamble/body split.** The `Document`/`Preamble`/`Block`/`Inline`
  types (spans on all); `build_document` that splits at `\begin{document}…\end{document}` (whole doc
  is preamble if absent), classifies `\documentclass`/`\usepackage` into `Preamble`, and lowers the
  body's flat nodes into a *flat* `Vec<Block>`/`Vec<Inline>` (no sectioning nesting yet — every
  heading is a zero-body `Section`). `to_latex` round-trip; span-coverage test (union of children ==
  parent).
- **D3 — sectioning fold.** Fold the flat block stream into the **nested** sectioning forest: each
  heading owns the blocks that follow until a heading of the same-or-higher level; `\part >
  \chapter > \section > …`. Hoist a trailing `\label` onto its section. Property test: flattening the
  tree reproduces D2's order.
- **D4 — inline normalization + metadata.** Paragraphs from blank-line (`Node::Par`) boundaries;
  `\textbf`/`\emph`/`\texttt` → `Strong`/`Emph`/`Code`; inline `$…$` → `Inline::Math`; `\ref`/`\cite`
  → `CrossRef`; accents → `Inline::Accent`. Extract `Metadata` (`\title`/`\author{A\and B}`/`\date`,
  `abstract` env → `Vec<Block>`). `\maketitle` becomes a no-op marker (metadata already captured).
- **D5 — floats, captions, code & display math.** `figure`/`table` floats → `Block::Figure`/`Table`
  with `\caption{…}` → `Caption` and a hoisted `\label`; `verbatim`/`lstlisting` → `CodeBlock`;
  `equation`/`align`/`\[..\]` → `DisplayMath` (source kept, delegated to the math frontend on demand);
  `quote`/`quotation` → `Quote`; any other `\begin{env}` → recursed `Block::Environment`.
- **D6 — provenance API + round-trip corpus + capstone.** `Document::node_at(byte)` (which node owns
  a source byte), `walk()` pre-order iterator, and `to_latex()` fixed-point over a corpus of real
  papers (a sectioned article with abstract, a `tabular`, an `itemize`, inline + display math, a
  figure with caption+label, a `\cite`). Assert **total byte coverage**: every non-whitespace source
  byte is owned by exactly one leaf node — the provenance guarantee the ADJ pipeline consumes.

**Explicitly out of scope (documented, not built):** counter resolution / printed numbers, ToC/index
generation, BibTeX resolution (we keep `\cite{key}` as a `CrossRef`, not the formatted citation),
page layout, and Turing-complete TeX — all inherited limits from LTX01 §1.

## 6. Downstream consumers (why this is the reusable substrate)

One `Document` model, many consumers — the write-once-use-many pattern already proven for the ADJ
recall libraries:
1. **Byte-provenanced fact extraction** (north star): `node_at(byte)` + `walk()` give the ADJ
   pipeline exact source spans for every claim, feeding the LLM→ADJ→CPU→solve/abstain path with
   auditable provenance (see the byte-provenance thesis).
2. **Format conversion**: `Document` → HTML/Markdown/`Node`-tree renderers (a thin visitor each).
3. **Structure queries / diffing**: "sections changed between two revisions", "all tables", "all
   `\cite` targets" — pure `walk()` filters.
4. **The math frontends (PFE01)** remain the leaf handler for every `Math`/`DisplayMath` island — the
   two layers compose, they don't overlap.

## 7. Crates & files

- Additions to the existing **`latex`** crate: `src/document.rs` (the `Document` model + fold),
  additions to `src/ast.rs` (`Node::Tabular`/`Node::List` in D1), and parser support in D1. No new
  crate. Public re-exports from `lib.rs` (`parse_document`, `build_document`, `Document`, `Block`,
  `Inline`, …).
- No new third-party dependency (repo policy: zero-dep core; the math frontend link is the existing
  opt-in `frontend` feature).
- CHANGELOG + README updated each rung; `latex` version bumped per rung.

## 8. Verification

- `cargo test -p latex` and `cargo clippy -p latex -- -D warnings` green at every rung; no `unsafe`.
- **Span integrity** (every rung): a test asserts each node's span is within its parent's and that a
  parent's span is the union of its children's; leaf spans are non-overlapping and cover every
  non-whitespace source byte (D6 makes this a hard gate).
- **Round-trip corpus**: `parse_document(s).to_latex()` re-parses to an equal `Document` for a set of
  real LaTeX papers (sectioned article + abstract + tabular + itemize + inline/display math + figure).
- **Totality**: `build_document` never panics and never errors — malformed source is already an
  `Unsupported` node from LTX01 and is carried through with its span.
- **Provenance**: for a labelled corpus, `node_at(byte)` returns the expected node kind for sampled
  offsets (heading vs paragraph vs table cell vs math island).
