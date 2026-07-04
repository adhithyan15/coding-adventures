//! The hierarchical **Document** layer (LTXDOC01 D2) — a pure fold over the flat [`Node`]
//! tree that LTX01 produces, giving callers a preamble/body split and a *block/inline*
//! document model instead of a presentation-shaped node stream.
//!
//! ## Where this sits
//!
//! LTX01's [`parse`](crate::parse) → `Vec<Node>` is flat and presentation-shaped: a
//! `\section` is a *sibling marker*, not a container; there is no preamble/body split and no
//! notion of block-vs-inline content. LTXDOC01 folds that flat stream **once** into a reusable
//! [`Document`] so every downstream consumer (renderers, format conversion, structure queries,
//! and the ADJ byte-provenance pipeline) reuses one model rather than re-deriving the hierarchy.
//! See `code/specs/LTXDOC01-latex-document-ast.md` §3–§5.
//!
//! ## What D2 does (and does *not*)
//!
//! D2 is the **skeleton** rung:
//!
//! 1. **Preamble / body split.** Everything before `\begin{document}` is the [`Preamble`]; the
//!    `document` environment's body is the document body. A *fragment* with no `document`
//!    environment is treated as all-preamble with an empty body (still valid).
//! 2. **Preamble classification.** `\documentclass[opts]{class}` → [`DocumentClass`];
//!    `\usepackage`/`\RequirePackage[opts]{name}` → [`Package`]s. Everything else in the
//!    preamble is kept untouched in [`Preamble::raw`]. We match on the already-folded
//!    [`Node::Preamble`] variant (LTX01's `recognize_structure` produces it), which is the
//!    cleaner path than re-matching raw `\cmd{…}` commands.
//! 3. **A *flat* body.** The body's flat nodes lower into a **flat** `Vec<Block>` — **no**
//!    sectioning nesting yet (that is D3): every heading becomes a **zero-body**
//!    [`Block::Section`]. Paragraphs are delimited by [`Node::Par`]; `Node::Tabular` →
//!    [`Block::Table`]; `Node::List` → [`Block::List`]; a display `Node::Math` →
//!    [`Block::DisplayMath`]; other environments recurse into [`Block::Environment`]; inline
//!    runs collect into a [`Block::Paragraph`].
//!
//! ## Span policy (D2 — coarse but honestly nested)
//!
//! Every [`Document`]/[`Preamble`]/[`Block`]/[`Inline`] node carries a [`Span`]. D2 does **not**
//! fabricate per-node byte ranges the flat [`Node`] cannot support. Instead:
//!
//! - `Document.span` = `0 .. src.len()` (the whole source).
//! - `Preamble.span` = `0 ..` the byte index of `\begin{document}` in the source (or `src.len()`
//!   if absent), located by a direct substring search.
//! - The *body region* span = end of `\begin{document}` .. start of `\end{document}` (or the
//!   preamble-end .. `src.len()` when there is no `document` environment).
//! - **Every block/inline span defaults to the enclosing region span** — the body region span for
//!   top-level blocks, the parent block's span for nested content.
//!
//! This is **coarse (region-granular)** but **honestly nested**: every child span ⊆ its parent
//! span ⊆ the `Document` span, so the containment invariant the spec (and the ADJ total-coverage
//! gate) rely on holds. **Precise per-node byte coverage is deferred to D6**, once the parser
//! threads token spans through `Node` — a *repo-standard-#9 divergence* from the spec's
//! per-node-span ideal. The type carries the `span` field now, so later rungs tighten the
//! *values* without an API break.
//!
//! ## Totality
//!
//! [`build_document`] is **total**: it never errors and never panics. Anything D2 does not model
//! is carried through as [`Block::Raw`] / [`Inline::Raw`] with its enclosing span — never
//! dropped. Recursion descends only into child node-lists LTX01 already produced (its depth is
//! bounded by the LTX01 tree depth, which the parser caps via `MAX_DEPTH`); D2 introduces no new
//! unbounded recursion.
//!
//! ## Round-trip
//!
//! [`Document::to_latex`] re-renders re-parseable source, and `parse_document(&doc.to_latex())`
//! yields a `Document` **structurally equal** to `doc`. Spans necessarily differ (offsets move
//! when surface spacing is normalized by the round-trip), so equality is compared **modulo
//! spans**: the round-trip test strips spans to a projection before comparing (see
//! [`Document::strip_spans`]).

use crate::ast::{ListItem, ListKind, Node, SectionLevel};
use crate::error::ParseError;
use crate::token::Span;
use crate::{document_to_latex, parse, recognize_accents, recognize_structure, recognize_tables};

// ---------------------------------------------------------------------------------------------
// The Document model (D2 types).
// ---------------------------------------------------------------------------------------------

/// A whole LaTeX document: a [`Preamble`] and a *flat* (D2) body of [`Block`]s.
///
/// `span` is the whole source (`0 .. src.len()`). Every child span is ⊆ this span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    /// Everything before `\begin{document}` (or the whole node stream for a fragment).
    pub preamble: Preamble,
    /// The document body, lowered to a **flat** `Vec<Block>` (no sectioning nesting in D2).
    pub body: Vec<Block>,
    /// The whole source: `Span { start: 0, end: src.len() }`.
    pub span: Span,
}

/// The preamble: the classified `\documentclass` / `\usepackage` directives plus every other
/// preamble node kept verbatim in [`Preamble::raw`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preamble {
    /// `\documentclass[opts]{class}`, if present.
    pub document_class: Option<DocumentClass>,
    /// `\usepackage[opts]{name}` / `\RequirePackage[opts]{name}` directives, in order.
    pub packages: Vec<Package>,
    /// Every other preamble node (macro definitions, custom setup, stray text), untouched.
    pub raw: Vec<Node>,
    /// `0 ..` the byte offset of `\begin{document}` (or `src.len()` if absent).
    pub span: Span,
}

/// A `\documentclass[options]{class}` directive.
///
/// `class` is the rendered class name (`"article"`, `"report"`, …); `options` is the rendered
/// bracketed option list (`"11pt,a4paper"`), or `None` if there was no `[…]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentClass {
    /// The class name, e.g. `"article"`.
    pub class: String,
    /// The `[options]` list rendered back to source, e.g. `Some("11pt,twocolumn")`.
    pub options: Option<String>,
    /// The span of the enclosing region (the preamble span in D2).
    pub span: Span,
}

/// A `\usepackage[options]{name}` (or `\RequirePackage`) directive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    /// The package name, e.g. `"amsmath"`.
    pub name: String,
    /// The `[options]` list rendered back to source, e.g. `Some("utf8")`.
    pub options: Option<String>,
    /// Which directive introduced it (`"usepackage"` or `"RequirePackage"`), preserved so the
    /// package round-trips to the exact command it came from.
    pub command: String,
    /// The span of the enclosing region (the preamble span in D2).
    pub span: Span,
}

/// A block-level element of the document body. In D2 the block stream is **flat** — a
/// [`Block::Section`] always has an empty `body` (the sectioning fold that fills it is D3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    /// A sectioning heading. **D3 fills `body`** with the run of blocks this heading owns (the
    /// blocks that follow it until the next heading of the same-or-higher level), recursively
    /// folded so deeper headings nest. A trailing `\label{…}` is hoisted onto `label`.
    Section {
        /// `\part` … `\subparagraph`.
        level: SectionLevel,
        /// `false` for the starred `\section*` (no-number) form.
        numbered: bool,
        /// The heading title, lowered to inlines.
        title: Vec<Inline>,
        /// The optional `[short]` TOC/running-head title, if present.
        short_title: Option<Vec<Inline>>,
        /// A `\label{key}` hoisted off the section's body (the key, without braces), if the block
        /// immediately following the heading was a lone `\label`. `None` otherwise. See the D3
        /// **label hoisting** note on [`fold_sections`].
        label: Option<String>,
        /// The blocks owned by this section. Empty until D3's [`fold_sections`] pass fills it.
        body: Vec<Block>,
        /// Enclosing-region span (D3: the union of the heading's region span and the owned
        /// children's spans — still ⊆ the body-region span).
        span: Span,
    },
    /// A run of inline content between paragraph breaks.
    Paragraph(Vec<Inline>, Span),
    /// An `itemize`/`enumerate`/`description` list (from [`Node::List`]).
    List {
        /// Which list flavour.
        kind: ListKind,
        /// The list items (each lowered to blocks; the term to inlines).
        items: Vec<DocListItem>,
        /// Enclosing-region span (D2 coarse).
        span: Span,
    },
    /// A `tabular`/`tabular*` grid (from [`Node::Tabular`]).
    Table {
        /// The column spec captured verbatim (`"lcr"`), or `None`.
        col_spec: Option<String>,
        /// `rows[r][c]` is cell `c` of row `r`, each lowered to blocks.
        rows: Vec<Vec<Vec<Block>>>,
        /// Enclosing-region span (D2 coarse).
        span: Span,
    },
    /// A display-math island (`\[…\]`, `$$…$$`) — kept as its source string (delegated to the
    /// math frontend on demand).
    DisplayMath {
        /// The exact inner math source.
        source: String,
        /// Enclosing-region span (D2 coarse).
        span: Span,
    },
    /// Any other `\begin{env}…\end{env}` block, recursed.
    Environment {
        /// The environment name.
        name: String,
        /// The environment body, recursively lowered to blocks.
        body: Vec<Block>,
        /// Enclosing-region span (D2 coarse).
        span: Span,
    },
    /// An LTX01 node with no block meaning of its own — carried through verbatim (never dropped).
    Raw(Node, Span),
}

/// One entry of a [`Block::List`] — the D2 analogue of [`crate::ListItem`], with the `\item[term]`
/// optional term lowered to inlines and the item body lowered to blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocListItem {
    /// The `\item[term]` optional term, lowered to inlines (`None` for a plain `\item`).
    pub term: Option<Vec<Inline>>,
    /// The item body, lowered to blocks.
    pub body: Vec<Block>,
    /// Enclosing-region span (D2 coarse).
    pub span: Span,
}

/// An inline (character-level) element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    /// A run of ordinary text.
    Text(String, Span),
    /// Significant inter-word space.
    Space(Span),
    /// `\textbf{…}` — strong (bold) emphasis.
    Strong(Vec<Inline>, Span),
    /// `\emph{…}` — emphasis.
    Emph(Vec<Inline>, Span),
    /// `\texttt{…}` — monospace/code.
    Code(String, Span),
    /// Any other argument-form font command (`\textsf{…}`, `\underline{…}`, …).
    Styled {
        /// The control-word verbatim (`"textsf"`, …).
        command: String,
        /// The wrapped content, lowered to inlines.
        content: Vec<Inline>,
        /// Enclosing-region span (D2 coarse).
        span: Span,
    },
    /// An inline math island (`$…$`, `\(…\)`) — kept as its source string.
    Math {
        /// The exact inner math source.
        source: String,
        /// Enclosing-region span (D2 coarse).
        span: Span,
    },
    /// A cross-reference / citation (`\ref{k}`, `\cite[note]{k}`, …).
    CrossRef {
        /// The control-word verbatim (`"ref"`, `"cite"`, …).
        command: String,
        /// The optional bracketed note (`\cite[p. 3]{…}`), lowered to inlines.
        note: Option<Vec<Inline>>,
        /// The mandatory target key rendered back to source (`"foo"`).
        target: String,
        /// Enclosing-region span (D2 coarse).
        span: Span,
    },
    /// A text accent (`\'e`, `\c{c}`) — the accent control word plus its base inline.
    Accent {
        /// The accent control-word verbatim (`"'"`, `"c"`, …).
        accent: String,
        /// The accented base, lowered to a single inline.
        base: Box<Inline>,
        /// Enclosing-region span (D2 coarse).
        span: Span,
    },
    /// An LTX01 node with no inline meaning of its own — carried through verbatim.
    Raw(Node, Span),
}

// ---------------------------------------------------------------------------------------------
// The public API.
// ---------------------------------------------------------------------------------------------

/// Parse `src` straight to a [`Document`], composing the shipped LTX01 passes then folding.
///
/// The pipeline is:
///
/// ```text
///   parse(src)?                    // L1 structural parse (the only fallible step)
///   |> recognize_structure         // fold \documentclass/\usepackage → Node::Preamble,
///   |                              //   \section → Node::Section, \ref → Node::CrossRef, …
///   |> recognize_accents           // fold \'e → Node::Accent
///   |> recognize_tables            // fold tabular/itemize → Node::Tabular / Node::List
///   |> build_document(_, src)      // D2 fold → Document  (total, never fails)
/// ```
///
/// **Ordering rationale.** `recognize_structure` runs **first** so the D2 fold sees the semantic
/// variants it matches on directly ([`Node::Preamble`] for the preamble classification,
/// [`Node::Section`]/[`Node::CrossRef`]/[`Node::Styled`] for the body lowering) rather than raw
/// `\cmd{…}` commands. `recognize_accents` and `recognize_tables` are independent of it (each
/// only touches its own construct and recurses through the rest), so their relative order does
/// not matter; `recognize_tables` runs last purely so that any list/table nested inside a
/// recognized section title is still folded. The only fallible stage is [`parse`] — once we hold
/// a node tree, the rest is infallible.
pub fn parse_document(src: &str) -> Result<Document, ParseError> {
    let nodes = recognize_tables(recognize_accents(recognize_structure(parse(src)?)));
    Ok(build_document(nodes, src))
}

/// Fold already-parsed LTX01 `nodes` into a [`Document`], using `src` to locate the `document`
/// environment by substring search (so spans point into the real source).
///
/// **Total**: never errors, never panics. This is the fold in isolation, for callers who already
/// hold LTX01 nodes (e.g. after macro expansion). `nodes` is expected to be the output of
/// `recognize_tables(recognize_accents(recognize_structure(parse(src)?)))` — but D2 degrades
/// gracefully on any node stream (unrecognized nodes become [`Block::Raw`]).
pub fn build_document(nodes: Vec<Node>, src: &str) -> Document {
    let doc_span = Span::new(0, src.len());

    // Locate the `document` environment purely by substring search over the *source string*, per
    // the D2 span policy. This is exact and reliable: `\begin{document}` / `\end{document}` are
    // literal, and the offsets we derive are honest byte positions into `src`.
    let begin_marker = r"\begin{document}";
    let end_marker = r"\end{document}";
    let begin_at = src.find(begin_marker);
    let preamble_end = begin_at.unwrap_or(src.len());
    let preamble_span = Span::new(0, preamble_end);

    // The body region: from just after `\begin{document}` to the start of `\end{document}` (or,
    // for a fragment with no document env, from preamble_end to end of source — an empty region).
    let body_region = match begin_at {
        Some(b) => {
            let body_start = b + begin_marker.len();
            // Search for `\end{document}` after the begin marker; fall back to src.len() if the
            // parser accepted something we can't locate (defensive — parse would have rejected a
            // truly unbalanced env upstream).
            let body_end = src[body_start..]
                .find(end_marker)
                .map(|rel| body_start + rel)
                .unwrap_or(src.len());
            Span::new(body_start, body_end.max(body_start))
        }
        None => Span::new(preamble_end, src.len()),
    };

    // Split the node stream at the `document` environment. Everything before it is the preamble;
    // the environment's own body is the document body. If there is no `document` env, the whole
    // stream is the preamble and the body is empty.
    let mut preamble_nodes: Vec<Node> = Vec::new();
    let mut body_nodes: Vec<Node> = Vec::new();
    let mut found_document = false;
    for node in nodes {
        if !found_document {
            if let Node::Environment { name, body, .. } = &node {
                if name == "document" {
                    body_nodes = body.clone();
                    found_document = true;
                    continue;
                }
            }
            preamble_nodes.push(node);
        } else {
            // Nodes *after* the document environment (rare — trailing content). Keep them in the
            // body region so nothing is dropped; they lower like any other body node.
            body_nodes.push(node);
        }
    }

    let preamble = classify_preamble(preamble_nodes, preamble_span);
    // `lower_blocks` produces the *flat* D2 block stream and then runs the D3 sectioning fold on
    // it (see `lower_blocks`), so `body` is already the nested sectioning forest. The same fold
    // runs on every nested block-list (environment/quote/figure bodies, list items, table cells)
    // because they all route through `lower_blocks` too — nesting therefore works everywhere.
    let body = lower_blocks(body_nodes, body_region);

    Document { preamble, body, span: doc_span }
}

/// Classify a preamble node stream into `\documentclass` / `\usepackage` directives plus the
/// untouched `raw` remainder. We match on the already-folded [`Node::Preamble`] variant.
fn classify_preamble(nodes: Vec<Node>, span: Span) -> Preamble {
    let mut document_class: Option<DocumentClass> = None;
    let mut packages: Vec<Package> = Vec::new();
    let mut raw: Vec<Node> = Vec::new();

    for node in nodes {
        match &node {
            Node::Preamble { command, options, name } if command == "documentclass" => {
                // Only the first `\documentclass` counts (LaTeX allows exactly one); a stray
                // second one is kept in `raw` so it is not silently lost.
                if document_class.is_none() {
                    document_class = Some(DocumentClass {
                        class: render_nodes(name),
                        options: options.as_ref().map(|o| render_nodes(o)),
                        span,
                    });
                } else {
                    raw.push(node);
                }
            }
            Node::Preamble { command, options, name }
                if command == "usepackage" || command == "RequirePackage" =>
            {
                packages.push(Package {
                    name: render_nodes(name),
                    options: options.as_ref().map(|o| render_nodes(o)),
                    command: command.clone(),
                    span,
                });
            }
            // Any other preamble node (macro defs, custom setup, comments, blank lines) is kept
            // verbatim — the fold is lossless.
            _ => raw.push(node),
        }
    }

    Preamble { document_class, packages, raw, span }
}

// ---------------------------------------------------------------------------------------------
// Body lowering: flat Vec<Node> -> flat Vec<Block>.
// ---------------------------------------------------------------------------------------------

/// Lower a flat node stream into a `Vec<Block>` and fold it into the **nested sectioning forest**
/// (D3). The walk first produces the flat D2 block stream (a run of *inline* nodes flushes into a
/// [`Block::Paragraph`]; each block-level node emits its block, with its own child block-lists
/// recursively lowered+folded by `lower_block`), then [`fold_sections`] folds that flat stream so
/// every [`Block::Section`] owns the run of blocks that follow it. `region` is the enclosing span
/// every emitted block inherits (coarse span policy); a folded section then unions in its
/// children's spans.
///
/// Because *every* block-list in the document (top-level body, environment/quote/figure bodies,
/// list-item bodies, table cells) routes through this one function, the sectioning fold applies
/// uniformly — a `\section` inside a `\begin{center}…\end{center}` nests just like a top-level one.
fn lower_blocks(nodes: Vec<Node>, region: Span) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    let mut pending: Vec<Inline> = Vec::new();

    // Flush the pending inline run into a Paragraph block, if non-empty.
    fn flush(pending: &mut Vec<Inline>, blocks: &mut Vec<Block>, region: Span) {
        if !pending.is_empty() {
            blocks.push(Block::Paragraph(std::mem::take(pending), region));
        }
    }

    for node in nodes {
        if is_block_node(&node) {
            flush(&mut pending, &mut blocks, region);
            blocks.push(lower_block(node, region));
        } else if matches!(node, Node::Par) {
            // A blank-line paragraph break closes the current paragraph.
            flush(&mut pending, &mut blocks, region);
        } else {
            pending.push(lower_inline(node, region));
        }
    }
    flush(&mut pending, &mut blocks, region);
    fold_sections(blocks)
}

// ---------------------------------------------------------------------------------------------
// D3 — the sectioning fold: flat Vec<Block> -> nested sectioning forest.
// ---------------------------------------------------------------------------------------------

/// The **rank** of a sectioning level: `0` for the coarsest (`\part`) up to `6` for the finest
/// (`\subparagraph`). A section *owns* every following block until it hits a heading whose rank is
/// **≤** its own — that heading begins a new sibling (equal rank) or ancestor (smaller rank)
/// section. Deeper headings (strictly greater rank) nest inside.
///
/// | level | rank |
/// |-------|------|
/// | `Part` | 0 |
/// | `Chapter` | 1 |
/// | `Section` | 2 |
/// | `Subsection` | 3 |
/// | `Subsubsection` | 4 |
/// | `Paragraph` | 5 |
/// | `Subparagraph` | 6 |
///
/// This mirrors the declaration order of [`SectionLevel`]; it is a pure lookup (no arithmetic on
/// the block list), so it cannot panic.
fn rank(level: SectionLevel) -> u8 {
    match level {
        SectionLevel::Part => 0,
        SectionLevel::Chapter => 1,
        SectionLevel::Section => 2,
        SectionLevel::Subsection => 3,
        SectionLevel::Subsubsection => 4,
        SectionLevel::Paragraph => 5,
        SectionLevel::Subparagraph => 6,
    }
}

/// If `block` is a heading, return its rank; otherwise `None` (a non-heading block is always owned
/// by whatever section it falls under — it never *starts* a new section).
fn heading_rank(block: &Block) -> Option<u8> {
    match block {
        Block::Section { level, .. } => Some(rank(*level)),
        _ => None,
    }
}

/// Fold a **flat** block stream into the nested sectioning forest (D3).
///
/// Algorithm (total, infallible):
///
/// - Leading non-`Section` blocks (before the first heading) stay at top level, in order.
/// - When a `Section` heading is met, it **owns** the maximal following run of blocks whose
///   heading-rank is strictly greater than its own (deeper headings) *or* which are non-headings.
///   The run stops at the first block whose heading-rank is ≤ this section's rank — that block
///   starts a new sibling/ancestor section and is not consumed here.
/// - The owned run is **recursively folded** (so deeper headings nest), then assigned as the
///   section's `body`. The section's `span` becomes the union of its heading region span and its
///   folded children's spans.
/// - **Label hoisting**: if, after folding, the first owned block is a lone `\label` (a
///   [`Block::Paragraph`] whose only non-space inline is an `Inline::CrossRef { command: "label" }`),
///   its key is hoisted onto the section's `label` and that block is dropped from `body` (the
///   `\label` is metadata about the section, not content). Any other `\label` position is **left in
///   place** in `body` (never dropped) — hoisting only the unambiguous immediately-following case
///   keeps the fold total; richer hoisting is a follow-on.
///
/// Termination: each recursive call folds a **strict sub-slice** of its input (the heading is
/// removed and its owned run is a proper suffix-prefix), so the recursion depth is bounded by the
/// number of blocks, which is itself bounded by the parser's `MAX_DEPTH`-capped tree. No `unwrap`,
/// no indexing that can go out of bounds.
fn fold_sections(blocks: Vec<Block>) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    let mut iter = blocks.into_iter().peekable();

    while let Some(block) = iter.next() {
        match block {
            Block::Section { level, numbered, title, short_title, label, body: _, span } => {
                let my_rank = rank(level);
                // Collect the run this heading owns: every subsequent block until one whose
                // heading-rank is ≤ my_rank. `peek` lets us stop *before* consuming that block.
                let mut owned: Vec<Block> = Vec::new();
                while let Some(next) = iter.peek() {
                    match heading_rank(next) {
                        Some(r) if r <= my_rank => break, // sibling/ancestor: not mine.
                        _ => {
                            // Deeper heading or a non-heading block: owned. `next()` cannot be
                            // `None` here because `peek()` just returned `Some`.
                            if let Some(owned_block) = iter.next() {
                                owned.push(owned_block);
                            }
                        }
                    }
                }
                // Recursively fold the owned run so deeper headings nest inside it.
                let mut folded = fold_sections(owned);
                // Label hoisting: pull a leading lone `\label{…}` onto the section.
                let hoisted = hoist_label(&mut folded);
                let label = label.or(hoisted);
                // Span union: heading region span ∪ each folded child's span.
                let span = folded.iter().fold(span, |acc, child| union(acc, block_span(child)));
                out.push(Block::Section {
                    level,
                    numbered,
                    title,
                    short_title,
                    label,
                    body: folded,
                    span,
                });
            }
            other => out.push(other),
        }
    }
    out
}

/// Hoist a `\label{key}` off the front of a section's owned body, if one is there.
///
/// LaTeX practice is `\section{Title}\label{sec:title}…` — the `\label` comes right after the
/// heading. After D2 lowering, that `\label` is the **leading `label` cross-reference of the first
/// paragraph** (it fuses with any following text into one `Block::Paragraph`, since there is no
/// `Node::Par` between them). So we look at the first block:
///
/// - If it is a [`Block::Paragraph`] whose leading non-space inlines contain **exactly one**
///   `label` [`Inline::CrossRef`] (with no other content before it), we take its key, strip that
///   label inline out of the paragraph, and — if the paragraph is now empty (the label stood
///   alone) — drop the paragraph entirely. The rest of the paragraph text is preserved.
/// - Anything else (first block is not a paragraph, or its leading content is not a lone label)
///   leaves `body` untouched and returns `None` — no content is ever dropped.
fn hoist_label(body: &mut Vec<Block>) -> Option<String> {
    let Some(Block::Paragraph(inlines, span)) = body.first_mut() else {
        return None;
    };
    // Find the first non-space inline; it must be the label, and there must be no non-space
    // inline before it (there can't be — it's the first — but we scan defensively).
    let mut label_idx: Option<usize> = None;
    for (i, inline) in inlines.iter().enumerate() {
        match inline {
            Inline::Space(_) => continue,
            Inline::CrossRef { command, .. } if command == "label" => {
                label_idx = Some(i);
                break;
            }
            _ => return None, // leading content is not a label → do not hoist.
        }
    }
    let idx = label_idx?;
    let key = match &inlines[idx] {
        Inline::CrossRef { target, .. } => target.clone(),
        _ => return None, // unreachable given the scan above, but keep it total.
    };
    // Remove the label inline (and any spaces up to it) from the paragraph.
    inlines.drain(..=idx);
    // Drop a now-empty (or whitespace-only) paragraph so the hoisted label leaves no husk.
    let empty = inlines.iter().all(|i| matches!(i, Inline::Space(_)));
    let span = *span;
    if empty {
        body.remove(0);
    } else if let Some(Block::Paragraph(_, s)) = body.first_mut() {
        *s = span; // keep the (coarse) span; unchanged, but explicit.
    }
    Some(key)
}

/// The union of two spans: the smallest span covering both.
fn union(a: Span, b: Span) -> Span {
    Span::new(a.start.min(b.start), a.end.max(b.end))
}

/// The span of a block (mirror of the test helper, needed by the D3 span union in `fold_sections`).
fn block_span(b: &Block) -> Span {
    match b {
        Block::Section { span, .. }
        | Block::List { span, .. }
        | Block::Table { span, .. }
        | Block::DisplayMath { span, .. }
        | Block::Environment { span, .. } => *span,
        Block::Paragraph(_, span) | Block::Raw(_, span) => *span,
    }
}

/// Does this node lower to a [`Block`] of its own (rather than joining an inline run)?
fn is_block_node(node: &Node) -> bool {
    matches!(
        node,
        Node::Section { .. }
            | Node::List { .. }
            | Node::Tabular { .. }
            | Node::Math { display: true, .. }
            | Node::Environment { .. }
            | Node::VerbatimEnv { .. }
    )
}

/// Lower a single block-level node into its [`Block`], recursing into child node-lists. `region`
/// is the enclosing span the block (and, coarsely, its descendants) inherit.
fn lower_block(node: Node, region: Span) -> Block {
    match node {
        Node::Section { level, starred, short, title } => Block::Section {
            level,
            numbered: !starred,
            title: lower_inlines(title, region),
            short_title: short.map(|s| lower_inlines(s, region)),
            label: None,      // filled by fold_sections (D3) if a \label follows the heading.
            body: Vec::new(), // filled by fold_sections (D3): the run of blocks this heading owns.
            span: region,
        },
        Node::List { kind, items } => Block::List {
            kind,
            items: items.into_iter().map(|it| lower_list_item(it, region)).collect(),
            span: region,
        },
        Node::Tabular { col_spec, rows } => Block::Table {
            col_spec,
            rows: rows
                .into_iter()
                .map(|row| row.into_iter().map(|cell| lower_blocks(cell, region)).collect())
                .collect(),
            span: region,
        },
        Node::Math { display: true, content } => Block::DisplayMath { source: content, span: region },
        Node::Environment { name, body, .. } => {
            Block::Environment { name, body: lower_blocks(body, region), span: region }
        }
        // A verbatim environment has no structured block model in D2 — carry it through verbatim
        // (never dropped) so a later rung (D5 CodeBlock) can refine it without data loss.
        other => Block::Raw(other, region),
    }
}

/// Lower one [`ListItem`] into a [`DocListItem`].
fn lower_list_item(item: ListItem, region: Span) -> DocListItem {
    DocListItem {
        term: item.label.map(|t| lower_inlines(t, region)),
        body: lower_blocks(item.body, region),
        span: region,
    }
}

/// Lower a flat node run into a `Vec<Inline>` (used for headings, list terms, styled content).
fn lower_inlines(nodes: Vec<Node>, region: Span) -> Vec<Inline> {
    nodes.into_iter().map(|n| lower_inline(n, region)).collect()
}

/// Lower a single node into its [`Inline`]. Anything without an inline meaning becomes
/// [`Inline::Raw`] — never dropped, never a panic. `region` is the enclosing span.
fn lower_inline(node: Node, region: Span) -> Inline {
    match node {
        Node::Text(t) => Inline::Text(t, region),
        Node::Space => Inline::Space(region),
        Node::Styled { command, content } => match command.as_str() {
            "textbf" => Inline::Strong(lower_inlines(content, region), region),
            "emph" | "textit" => Inline::Emph(lower_inlines(content, region), region),
            "texttt" => Inline::Code(render_nodes(&content), region),
            _ => Inline::Styled { command, content: lower_inlines(content, region), span: region },
        },
        Node::Math { display: false, content } => Inline::Math { source: content, span: region },
        Node::CrossRef { command, note, target } => Inline::CrossRef {
            command,
            note: note.map(|n| lower_inlines(n, region)),
            target: render_nodes(&target),
            span: region,
        },
        Node::Accent { accent, arg } => Inline::Accent {
            accent,
            base: Box::new(lower_accent_base(arg, region)),
            span: region,
        },
        // Everything else (a display Math slipped into an inline run, an unhandled command, …)
        // is carried through verbatim.
        other => Inline::Raw(other, region),
    }
}

/// Lower an accent's base argument (a node list) into a single [`Inline`]. Accents apply to one
/// base; if the argument is a single node we lower it directly, otherwise we wrap the run so no
/// content is lost.
fn lower_accent_base(arg: Vec<Node>, region: Span) -> Inline {
    let mut it = arg.into_iter();
    match (it.next(), it.next()) {
        (Some(single), None) => lower_inline(single, region),
        (Some(first), Some(second)) => {
            // Multi-node base (rare) — keep it faithfully as a Styled group so nothing is dropped.
            let mut rest: Vec<Node> = vec![first, second];
            rest.extend(it);
            Inline::Styled { command: String::new(), content: lower_inlines(rest, region), span: region }
        }
        (None, _) => Inline::Text(String::new(), region),
    }
}

/// Render a node list back to source (used to capture class/package names, math targets, etc.).
fn render_nodes(nodes: &[Node]) -> String {
    document_to_latex(nodes)
}

// ---------------------------------------------------------------------------------------------
// to_latex — round-tripping back to re-parseable source.
// ---------------------------------------------------------------------------------------------

impl Document {
    /// Render this document back to re-parseable LaTeX source. `parse_document(&d.to_latex())`
    /// yields a `Document` structurally equal to `d` (equal **modulo spans** — see
    /// [`Document::strip_spans`]).
    pub fn to_latex(&self) -> String {
        let mut out = String::new();
        self.preamble.write_latex(&mut out);
        // Only wrap the body in a `document` environment if there was one (a fragment with an
        // empty body round-trips as pure preamble — matching how it was folded).
        if !self.body.is_empty() || self.preamble_has_document_env() {
            out.push_str("\\begin{document}");
            write_blocks(&self.body, &mut out);
            out.push_str("\\end{document}");
        }
        out
    }

    /// A fragment (no `document` env, empty body) must not sprout an empty `\begin{document}` on
    /// round-trip. We only emit the wrapper when there is body content. (`preamble_has_document_env`
    /// is always `false` in D2 — the `document` env is consumed into the body split — but the hook
    /// keeps `to_latex` explicit about the invariant.)
    fn preamble_has_document_env(&self) -> bool {
        false
    }

    /// A span-stripped structural projection, for round-trip equality that ignores byte offsets
    /// (which necessarily move when surface spacing is normalized). Two documents are
    /// "equal modulo spans" iff their `strip_spans()` projections are `==`.
    pub fn strip_spans(&self) -> Document {
        let z = Span::new(0, 0);
        Document {
            preamble: self.preamble.strip_spans(z),
            body: self.body.iter().map(|b| b.strip_spans(z)).collect(),
            span: z,
        }
    }
}

impl Preamble {
    fn write_latex(&self, out: &mut String) {
        if let Some(dc) = &self.document_class {
            out.push_str("\\documentclass");
            if let Some(opts) = &dc.options {
                out.push('[');
                out.push_str(opts);
                out.push(']');
            }
            out.push('{');
            out.push_str(&dc.class);
            out.push('}');
        }
        for pkg in &self.packages {
            out.push('\\');
            out.push_str(&pkg.command);
            if let Some(opts) = &pkg.options {
                out.push('[');
                out.push_str(opts);
                out.push(']');
            }
            out.push('{');
            out.push_str(&pkg.name);
            out.push('}');
        }
        for node in &self.raw {
            out.push_str(&node.to_latex());
        }
    }

    fn strip_spans(&self, z: Span) -> Preamble {
        Preamble {
            document_class: self.document_class.as_ref().map(|dc| DocumentClass { span: z, ..dc.clone() }),
            packages: self.packages.iter().map(|p| Package { span: z, ..p.clone() }).collect(),
            raw: self.raw.clone(),
            span: z,
        }
    }
}

impl Block {
    fn write_latex(&self, out: &mut String) {
        match self {
            Block::Section { level, numbered, title, short_title, label, body, .. } => {
                out.push('\\');
                out.push_str(level.command());
                if !numbered {
                    out.push('*');
                }
                if let Some(short) = short_title {
                    out.push('[');
                    write_inlines(short, out);
                    out.push(']');
                }
                out.push('{');
                write_inlines(title, out);
                out.push('}');
                // Re-emit a hoisted `\label{key}` immediately after the heading, so re-parsing +
                // re-folding hoists it back onto this section (round-trip fixed point). A `\par`
                // (blank line) after it keeps the label a **lone** paragraph on re-parse, so
                // `hoist_label` recognizes it again rather than fusing it with following text.
                if let Some(key) = label {
                    out.push_str("\\label{");
                    out.push_str(key);
                    out.push('}');
                    if !body.is_empty() {
                        out.push_str("\n\n");
                    }
                }
                // D3: render the owned child blocks inline after the heading.
                write_blocks(body, out);
            }
            Block::Paragraph(inlines, _) => {
                // No trailing paragraph break here — [`write_blocks`] inserts the `\n\n`
                // *between* adjacent blocks so a paragraph that is the sole content of a table
                // cell or a list item does not sprout a spurious `Par` on re-parse.
                write_inlines(inlines, out);
            }
            Block::List { kind, items, .. } => {
                out.push_str("\\begin{");
                out.push_str(kind.env());
                out.push('}');
                for item in items {
                    out.push_str("\\item");
                    if let Some(term) = &item.term {
                        out.push('[');
                        write_inlines(term, out);
                        out.push(']');
                    } else {
                        out.push(' ');
                    }
                    write_blocks(&item.body, out);
                }
                out.push_str("\\end{");
                out.push_str(kind.env());
                out.push('}');
            }
            Block::Table { col_spec, rows, .. } => {
                out.push_str("\\begin{tabular}");
                if let Some(spec) = col_spec {
                    out.push('{');
                    out.push_str(spec);
                    out.push('}');
                }
                for (r, row) in rows.iter().enumerate() {
                    if r > 0 {
                        out.push_str(" \\\\ ");
                    }
                    for (c, cell) in row.iter().enumerate() {
                        if c > 0 {
                            out.push_str(" & ");
                        }
                        write_blocks(cell, out);
                    }
                }
                out.push_str("\\end{tabular}");
            }
            Block::DisplayMath { source, .. } => {
                out.push_str("$$");
                out.push_str(source);
                out.push_str("$$");
            }
            Block::Environment { name, body, .. } => {
                out.push_str("\\begin{");
                out.push_str(name);
                out.push('}');
                write_blocks(body, out);
                out.push_str("\\end{");
                out.push_str(name);
                out.push('}');
            }
            Block::Raw(node, _) => out.push_str(&node.to_latex()),
        }
    }

    fn strip_spans(&self, z: Span) -> Block {
        match self {
            Block::Section { level, numbered, title, short_title, label, body, .. } => Block::Section {
                level: *level,
                numbered: *numbered,
                title: strip_inlines(title, z),
                short_title: short_title.as_ref().map(|s| strip_inlines(s, z)),
                label: label.clone(),
                body: body.iter().map(|b| b.strip_spans(z)).collect(),
                span: z,
            },
            Block::Paragraph(inlines, _) => Block::Paragraph(strip_inlines(inlines, z), z),
            Block::List { kind, items, .. } => Block::List {
                kind: *kind,
                items: items
                    .iter()
                    .map(|it| DocListItem {
                        term: it.term.as_ref().map(|t| strip_inlines(t, z)),
                        body: it.body.iter().map(|b| b.strip_spans(z)).collect(),
                        span: z,
                    })
                    .collect(),
                span: z,
            },
            Block::Table { col_spec, rows, .. } => Block::Table {
                col_spec: col_spec.clone(),
                rows: rows
                    .iter()
                    .map(|row| row.iter().map(|cell| cell.iter().map(|b| b.strip_spans(z)).collect()).collect())
                    .collect(),
                span: z,
            },
            Block::DisplayMath { source, .. } => Block::DisplayMath { source: source.clone(), span: z },
            Block::Environment { name, body, .. } => Block::Environment {
                name: name.clone(),
                body: body.iter().map(|b| b.strip_spans(z)).collect(),
                span: z,
            },
            Block::Raw(node, _) => Block::Raw(node.clone(), z),
        }
    }
}

impl Inline {
    fn write_latex(&self, out: &mut String) {
        match self {
            Inline::Text(t, _) => out.push_str(t),
            Inline::Space(_) => out.push(' '),
            Inline::Strong(content, _) => {
                out.push_str("\\textbf{");
                write_inlines(content, out);
                out.push('}');
            }
            Inline::Emph(content, _) => {
                out.push_str("\\emph{");
                write_inlines(content, out);
                out.push('}');
            }
            Inline::Code(text, _) => {
                out.push_str("\\texttt{");
                out.push_str(text);
                out.push('}');
            }
            Inline::Styled { command, content, .. } => {
                out.push('\\');
                out.push_str(command);
                out.push('{');
                write_inlines(content, out);
                out.push('}');
            }
            Inline::Math { source, .. } => {
                out.push('$');
                out.push_str(source);
                out.push('$');
            }
            Inline::CrossRef { command, note, target, .. } => {
                out.push('\\');
                out.push_str(command);
                if let Some(note) = note {
                    out.push('[');
                    write_inlines(note, out);
                    out.push(']');
                }
                out.push('{');
                out.push_str(target);
                out.push('}');
            }
            Inline::Accent { accent, base, .. } => {
                out.push('\\');
                out.push_str(accent);
                out.push('{');
                base.write_latex(out);
                out.push('}');
            }
            Inline::Raw(node, _) => out.push_str(&node.to_latex()),
        }
    }

    fn strip_spans(&self, z: Span) -> Inline {
        match self {
            Inline::Text(t, _) => Inline::Text(t.clone(), z),
            Inline::Space(_) => Inline::Space(z),
            Inline::Strong(c, _) => Inline::Strong(strip_inlines(c, z), z),
            Inline::Emph(c, _) => Inline::Emph(strip_inlines(c, z), z),
            Inline::Code(t, _) => Inline::Code(t.clone(), z),
            Inline::Styled { command, content, .. } => {
                Inline::Styled { command: command.clone(), content: strip_inlines(content, z), span: z }
            }
            Inline::Math { source, .. } => Inline::Math { source: source.clone(), span: z },
            Inline::CrossRef { command, note, target, .. } => Inline::CrossRef {
                command: command.clone(),
                note: note.as_ref().map(|n| strip_inlines(n, z)),
                target: target.clone(),
                span: z,
            },
            Inline::Accent { accent, base, .. } => {
                Inline::Accent { accent: accent.clone(), base: Box::new(base.strip_spans(z)), span: z }
            }
            Inline::Raw(node, _) => Inline::Raw(node.clone(), z),
        }
    }
}

/// Render a sequence of blocks, inserting a `\n\n` paragraph break **between** two adjacent
/// blocks when either side is a [`Block::Paragraph`], so re-parsing recovers the `Node::Par`
/// boundary. Because the break is emitted between blocks (not after each one), a paragraph that
/// is the sole content of a table cell or list item stays break-free and round-trips cleanly.
fn write_blocks(blocks: &[Block], out: &mut String) {
    for (i, block) in blocks.iter().enumerate() {
        if i > 0 {
            let prev_para = matches!(blocks[i - 1], Block::Paragraph(..));
            let this_para = matches!(block, Block::Paragraph(..));
            if prev_para || this_para {
                out.push_str("\n\n");
            }
        }
        block.write_latex(out);
    }
}

/// Render a `Vec<Inline>` back to LaTeX.
fn write_inlines(inlines: &[Inline], out: &mut String) {
    for inl in inlines {
        inl.write_latex(out);
    }
}

/// Strip spans from a `Vec<Inline>`.
fn strip_inlines(inlines: &[Inline], z: Span) -> Vec<Inline> {
    inlines.iter().map(|i| i.strip_spans(z)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- span-integrity helpers ---------------------------------------------------------------

    /// Assert `child ⊆ parent` (start ≥ parent.start, end ≤ parent.end).
    fn assert_within(child: Span, parent: Span, what: &str) {
        assert!(
            child.start >= parent.start && child.end <= parent.end,
            "{what}: child span {child:?} not within parent {parent:?}"
        );
    }

    /// Walk a block, asserting every span is within `parent`, recursing.
    fn check_block(block: &Block, parent: Span) {
        let span = block_span(block);
        assert_within(span, parent, "block");
        match block {
            Block::Section { title, short_title, body, .. } => {
                for i in title {
                    check_inline(i, span);
                }
                if let Some(s) = short_title {
                    for i in s {
                        check_inline(i, span);
                    }
                }
                for b in body {
                    check_block(b, span);
                }
            }
            Block::Paragraph(inlines, _) => {
                for i in inlines {
                    check_inline(i, span);
                }
            }
            Block::List { items, .. } => {
                for it in items {
                    assert_within(it.span, span, "list item");
                    if let Some(t) = &it.term {
                        for i in t {
                            check_inline(i, it.span);
                        }
                    }
                    for b in &it.body {
                        check_block(b, it.span);
                    }
                }
            }
            Block::Table { rows, .. } => {
                for row in rows {
                    for cell in row {
                        for b in cell {
                            check_block(b, span);
                        }
                    }
                }
            }
            Block::DisplayMath { .. } | Block::Raw(..) => {}
            Block::Environment { body, .. } => {
                for b in body {
                    check_block(b, span);
                }
            }
        }
    }

    fn check_inline(inline: &Inline, parent: Span) {
        let span = inline_span(inline);
        assert_within(span, parent, "inline");
        match inline {
            Inline::Strong(c, _) | Inline::Emph(c, _) => {
                for i in c {
                    check_inline(i, span);
                }
            }
            Inline::Styled { content, .. } => {
                for i in content {
                    check_inline(i, span);
                }
            }
            Inline::CrossRef { note: Some(n), .. } => {
                for i in n {
                    check_inline(i, span);
                }
            }
            Inline::Accent { base, .. } => check_inline(base, span),
            _ => {}
        }
    }

    fn block_span(b: &Block) -> Span {
        match b {
            Block::Section { span, .. }
            | Block::List { span, .. }
            | Block::Table { span, .. }
            | Block::DisplayMath { span, .. }
            | Block::Environment { span, .. } => *span,
            Block::Paragraph(_, span) | Block::Raw(_, span) => *span,
        }
    }

    fn inline_span(i: &Inline) -> Span {
        match i {
            Inline::Text(_, span)
            | Inline::Space(span)
            | Inline::Strong(_, span)
            | Inline::Emph(_, span)
            | Inline::Code(_, span)
            | Inline::Raw(_, span) => *span,
            Inline::Styled { span, .. }
            | Inline::Math { span, .. }
            | Inline::CrossRef { span, .. }
            | Inline::Accent { span, .. } => *span,
        }
    }

    // -- tests --------------------------------------------------------------------------------

    #[test]
    fn preamble_body_split_real_document() {
        let src = r"\documentclass{article}\begin{document}Hello.\end{document}";
        let doc = parse_document(src).expect("parse");
        // Class captured.
        assert_eq!(doc.preamble.document_class.as_ref().unwrap().class, "article");
        // Body has the greeting as a paragraph.
        assert_eq!(doc.body.len(), 1);
        assert!(matches!(doc.body[0], Block::Paragraph(..)));
        // Preamble span ends at `\begin{document}`; document span is the whole source.
        assert_eq!(doc.span, Span::new(0, src.len()));
        assert_eq!(doc.preamble.span.start, 0);
        assert_eq!(doc.preamble.span.end, src.find(r"\begin{document}").unwrap());
    }

    #[test]
    fn fragment_with_no_document_env_is_all_preamble() {
        // No \begin{document}: the whole stream is preamble, body empty (valid fragment).
        let src = r"\documentclass{article}\usepackage{amsmath}";
        let doc = parse_document(src).expect("parse");
        assert!(doc.body.is_empty(), "fragment body must be empty");
        assert_eq!(doc.preamble.document_class.as_ref().unwrap().class, "article");
        assert_eq!(doc.preamble.packages.len(), 1);
        // Preamble span covers the whole source when there is no document env.
        assert_eq!(doc.preamble.span, Span::new(0, src.len()));
    }

    #[test]
    fn documentclass_and_usepackage_classified() {
        let src = r"\documentclass[11pt,a4paper]{report}\usepackage[utf8]{inputenc}\RequirePackage{amsmath}\begin{document}x\end{document}";
        let doc = parse_document(src).expect("parse");
        let dc = doc.preamble.document_class.as_ref().unwrap();
        assert_eq!(dc.class, "report");
        assert_eq!(dc.options.as_deref(), Some("11pt,a4paper"));
        assert_eq!(doc.preamble.packages.len(), 2);
        assert_eq!(doc.preamble.packages[0].name, "inputenc");
        assert_eq!(doc.preamble.packages[0].options.as_deref(), Some("utf8"));
        assert_eq!(doc.preamble.packages[0].command, "usepackage");
        assert_eq!(doc.preamble.packages[1].name, "amsmath");
        assert_eq!(doc.preamble.packages[1].command, "RequirePackage");
        assert_eq!(doc.preamble.packages[1].options, None);
    }

    #[test]
    fn body_lowering_all_block_and_inline_kinds() {
        let src = concat!(
            r"\begin{document}",
            r"\section{Intro}",
            r"Some \textbf{bold} and \emph{it} text with $x^2$ inline.",
            r"\begin{itemize}\item one\item two\end{itemize}",
            r"\begin{tabular}{lc}a & b \\ c & d\end{tabular}",
            r"\[ E = mc^2 \]",
            r"\ref{eq:one}",
            r"\end{document}",
        );
        let doc = parse_document(src).expect("parse");

        // D3: the heading now OWNS every following block (all deeper-or-non-heading), so the
        // section sits at top level and the paragraph/list/table/math/ref are inside its body.
        assert_eq!(doc.body.len(), 1, "the single \\section owns the whole body");
        let sec = &doc.body[0];
        let owned: &[Block] = if let Block::Section { level, numbered, title, body, .. } = sec {
            assert_eq!(*level, SectionLevel::Section);
            assert!(*numbered);
            assert!(!body.is_empty(), "D3 sections own their following blocks");
            assert!(title.iter().any(|i| matches!(i, Inline::Text(t, _) if t == "Intro")));
            body
        } else {
            panic!("expected a Section at top level");
        };

        // A paragraph with Strong / Emph / inline Math — now inside the section body.
        let para = owned
            .iter()
            .find(|b| matches!(b, Block::Paragraph(inls, _)
                if inls.iter().any(|i| matches!(i, Inline::Strong(..)))))
            .expect("paragraph with strong");
        if let Block::Paragraph(inls, _) = para {
            assert!(inls.iter().any(|i| matches!(i, Inline::Strong(..))));
            assert!(inls.iter().any(|i| matches!(i, Inline::Emph(..))));
            assert!(inls.iter().any(|i| matches!(i, Inline::Math { source, .. } if source.contains("x^2"))));
        }

        // List, Table, DisplayMath, CrossRef — all owned by the section.
        assert!(owned.iter().any(|b| matches!(b, Block::List { .. })));
        assert!(owned.iter().any(|b| matches!(b, Block::Table { .. })));
        assert!(owned.iter().any(|b| matches!(b, Block::DisplayMath { source, .. } if source.contains("E = mc^2"))));
        // The \ref lowered to a CrossRef inline inside its paragraph.
        assert!(owned.iter().any(|b| matches!(b, Block::Paragraph(inls, _)
            if inls.iter().any(|i| matches!(i, Inline::CrossRef { command, target, .. }
                if command == "ref" && target == "eq:one")))));
    }

    #[test]
    fn span_integrity_child_within_parent_within_document() {
        let src = concat!(
            r"\documentclass{article}\begin{document}",
            r"\section{H}Text with \textbf{bold} and $x$.",
            r"\begin{itemize}\item a\item b\end{itemize}",
            r"\begin{tabular}{c}p & q\end{tabular}",
            r"\end{document}",
        );
        let doc = parse_document(src).expect("parse");
        // Preamble ⊆ Document.
        assert_within(doc.preamble.span, doc.span, "preamble");
        // Every block (and its inlines/nested content) ⊆ its parent ⊆ Document.
        for block in &doc.body {
            check_block(block, doc.span);
        }
    }

    #[test]
    fn round_trip_modulo_spans() {
        for src in [
            r"\documentclass{article}\begin{document}Hello \textbf{world}.\end{document}",
            r"\documentclass[11pt]{report}\usepackage[utf8]{inputenc}\begin{document}\section{A}Body $x^2$ here.\begin{itemize}\item one\item two\end{itemize}\end{document}",
            r"\documentclass{article}\usepackage{amsmath}", // fragment, empty body
            r"\begin{document}Just a body with \emph{emphasis} and \ref{k}.\end{document}",
            r"\begin{document}\[ E=mc^2 \]\begin{tabular}{lc}a & b \\ c & d\end{tabular}\end{document}",
        ] {
            let doc = parse_document(src).expect("parse");
            let rendered = doc.to_latex();
            let redoc = parse_document(&rendered).expect("re-parse");
            assert_eq!(
                doc.strip_spans(),
                redoc.strip_spans(),
                "round-trip modulo spans failed:\n src = {src:?}\n rendered = {rendered:?}"
            );
        }
    }

    #[test]
    fn build_document_is_total_on_junk() {
        // A stream with no document env and unmodelled nodes must not panic; junk → Raw.
        let src = r"\weird{cmd}~plain";
        let doc = parse_document(src).expect("parse");
        assert!(doc.body.is_empty());
        // The weird command is kept in the preamble raw (nothing dropped).
        assert!(!doc.preamble.raw.is_empty());
    }

    #[test]
    fn other_environment_recurses_to_block_environment() {
        let src = r"\begin{document}\begin{center}centered \textbf{text}\end{center}\end{document}";
        let doc = parse_document(src).expect("parse");
        let env = doc.body.iter().find(|b| matches!(b, Block::Environment { .. })).expect("env");
        if let Block::Environment { name, body, .. } = env {
            assert_eq!(name, "center");
            assert!(body.iter().any(|b| matches!(b, Block::Paragraph(inls, _)
                if inls.iter().any(|i| matches!(i, Inline::Strong(..))))));
        }
    }

    // -- D3: sectioning-fold tests ------------------------------------------------------------

    /// A flat, span-stripped **linearization** of the folded forest: a heading followed by the
    /// depth-first linearization of the blocks it owns. This is the inverse of `fold_sections`:
    /// `flatten(fold(flat)) == flat` (modulo section bodies, which are empty pre-fold and populated
    /// post-fold — so we compare with each `Section`'s own `body` emptied). All spans are stripped
    /// to `z` so ordering is compared without the D3 span-union differences.
    fn flatten(blocks: &[Block]) -> Vec<Block> {
        let z = Span::new(0, 0);
        let mut out = Vec::new();
        for b in blocks {
            match b {
                Block::Section { level, numbered, title, short_title, label, body, .. } => {
                    // Emit the heading with an EMPTY body (matching the pre-fold shape), then the
                    // depth-first linearization of the blocks it owned.
                    out.push(Block::Section {
                        level: *level,
                        numbered: *numbered,
                        title: strip_inlines(title, z),
                        short_title: short_title.as_ref().map(|s| strip_inlines(s, z)),
                        label: label.clone(),
                        body: Vec::new(),
                        span: z,
                    });
                    out.extend(flatten(body));
                }
                other => out.push(other.strip_spans(z)),
            }
        }
        out
    }

    #[test]
    fn fold_nests_and_flatten_reproduces_order() {
        // \section{A} p1 \subsection{B} p2 \section{C} p3
        // → A owns {p1, B{p2}}, C owns {p3}; A and C are top-level siblings; B nests in A.
        let src = concat!(
            r"\begin{document}",
            r"\section{A}p1",
            r"\subsection{B}p2",
            r"\section{C}p3",
            r"\end{document}",
        );
        let doc = parse_document(src).expect("parse");

        // Two top-level sections: A and C.
        assert_eq!(doc.body.len(), 2);
        let (a_body, c_body) = match (&doc.body[0], &doc.body[1]) {
            (
                Block::Section { title: ta, body: ba, level: SectionLevel::Section, .. },
                Block::Section { title: tc, body: bc, level: SectionLevel::Section, .. },
            ) => {
                assert!(ta.iter().any(|i| matches!(i, Inline::Text(t, _) if t == "A")));
                assert!(tc.iter().any(|i| matches!(i, Inline::Text(t, _) if t == "C")));
                (ba, bc)
            }
            other => panic!("expected two top-level Sections, got {other:?}"),
        };

        // A owns p1 then subsection B (which nests p2).
        assert!(a_body.iter().any(|b| matches!(b, Block::Paragraph(inls, _)
            if inls.iter().any(|i| matches!(i, Inline::Text(t, _) if t == "p1")))));
        let b_sec = a_body
            .iter()
            .find(|b| matches!(b, Block::Section { level: SectionLevel::Subsection, .. }))
            .expect("subsection B nested in A");
        if let Block::Section { title, body, .. } = b_sec {
            assert!(title.iter().any(|i| matches!(i, Inline::Text(t, _) if t == "B")));
            assert!(body.iter().any(|b| matches!(b, Block::Paragraph(inls, _)
                if inls.iter().any(|i| matches!(i, Inline::Text(t, _) if t == "p2")))));
        }
        // C owns only p3 (no nested section).
        assert!(c_body.iter().any(|b| matches!(b, Block::Paragraph(inls, _)
            if inls.iter().any(|i| matches!(i, Inline::Text(t, _) if t == "p3")))));
        assert!(!c_body.iter().any(|b| matches!(b, Block::Section { .. })));

        // Property: flattening the folded forest reproduces the pre-fold flat block order.
        // The pre-fold shape = zero-body Section markers interleaved with the owned blocks, in
        // source order: [Sec(A), p1, Sec(B), p2, Sec(C), p3].
        let flat = flatten(&doc.body);
        let z = Span::new(0, 0);
        let sec = |title: &str, level: SectionLevel| Block::Section {
            level,
            numbered: true,
            title: vec![Inline::Text(title.into(), z)],
            short_title: None,
            label: None,
            body: Vec::new(),
            span: z,
        };
        let para = |t: &str| Block::Paragraph(vec![Inline::Text(t.into(), z)], z);
        assert_eq!(
            flat,
            vec![
                sec("A", SectionLevel::Section),
                para("p1"),
                sec("B", SectionLevel::Subsection),
                para("p2"),
                sec("C", SectionLevel::Section),
                para("p3"),
            ]
        );
    }

    #[test]
    fn deeper_then_shallower_closes_the_right_sections() {
        // \section{A} \subsection{B} \subsubsection{C} \section{D}
        // C nests in B nests in A; D is a top-level sibling of A (rank ≤ A closes A, B, C).
        let src = concat!(
            r"\begin{document}",
            r"\section{A}\subsection{B}\subsubsection{C}x\section{D}y",
            r"\end{document}",
        );
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.body.len(), 2, "A and D are top-level siblings");
        // A → B → C chain.
        if let Block::Section { body: a, .. } = &doc.body[0] {
            let b = a.iter().find_map(|blk| match blk {
                Block::Section { level: SectionLevel::Subsection, body, .. } => Some(body),
                _ => None,
            });
            let b = b.expect("B nested in A");
            let c = b.iter().find_map(|blk| match blk {
                Block::Section { level: SectionLevel::Subsubsection, body, .. } => Some(body),
                _ => None,
            });
            let c = c.expect("C nested in B");
            assert!(c.iter().any(|blk| matches!(blk, Block::Paragraph(inls, _)
                if inls.iter().any(|i| matches!(i, Inline::Text(t, _) if t == "x")))));
        } else {
            panic!("body[0] must be section A");
        }
        // D owns "y".
        if let Block::Section { title, body, .. } = &doc.body[1] {
            assert!(title.iter().any(|i| matches!(i, Inline::Text(t, _) if t == "D")));
            assert!(body.iter().any(|blk| matches!(blk, Block::Paragraph(inls, _)
                if inls.iter().any(|i| matches!(i, Inline::Text(t, _) if t == "y")))));
        }
    }

    #[test]
    fn leading_blocks_before_first_heading_stay_top_level() {
        let src = r"\begin{document}intro text\section{A}body\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.body.len(), 2, "the intro paragraph + section A");
        assert!(matches!(doc.body[0], Block::Paragraph(..)), "leading text stays at top level");
        assert!(matches!(doc.body[1], Block::Section { .. }));
    }

    #[test]
    fn label_hoisted_onto_section() {
        let src = r"\begin{document}\section{Intro}\label{sec:intro}First paragraph.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.body.len(), 1);
        if let Block::Section { label, body, .. } = &doc.body[0] {
            assert_eq!(label.as_deref(), Some("sec:intro"), "the \\label is hoisted onto the section");
            // The label block is removed from body; the paragraph remains.
            assert!(!body.iter().any(|b| matches!(b, Block::Paragraph(inls, _)
                if inls.iter().any(|i| matches!(i, Inline::CrossRef { command, .. } if command == "label")))),
                "the hoisted label is no longer a body block");
            assert!(body.iter().any(|b| matches!(b, Block::Paragraph(inls, _)
                if inls.iter().any(|i| matches!(i, Inline::Text(t, _) if t.contains("First"))))));
        } else {
            panic!("expected section");
        }
    }

    #[test]
    fn non_lone_label_is_not_hoisted() {
        // A `\ref` (not `\label`), or a label fused with text, must NOT be hoisted — kept in body.
        let src = r"\begin{document}\section{A}See \ref{k} here.\end{document}";
        let doc = parse_document(src).expect("parse");
        if let Block::Section { label, body, .. } = &doc.body[0] {
            assert_eq!(*label, None, "a \\ref is not a section label");
            assert!(body.iter().any(|b| matches!(b, Block::Paragraph(inls, _)
                if inls.iter().any(|i| matches!(i, Inline::CrossRef { command, .. } if command == "ref")))),
                "the \\ref is preserved in the body");
        }
    }

    #[test]
    fn label_round_trips_through_to_latex() {
        let src = r"\begin{document}\section{Intro}\label{sec:intro}Body text here.\end{document}";
        let doc = parse_document(src).expect("parse");
        let rendered = doc.to_latex();
        let redoc = parse_document(&rendered).expect("re-parse");
        assert_eq!(
            doc.strip_spans(),
            redoc.strip_spans(),
            "label round-trip failed; rendered = {rendered:?}"
        );
        // And the label survived the round-trip.
        if let Block::Section { label, .. } = &redoc.body[0] {
            assert_eq!(label.as_deref(), Some("sec:intro"));
        }
    }

    #[test]
    fn round_trip_with_nested_sections() {
        // The folded forest must be a to_latex fixed point (modulo spans) for nested sections.
        for src in [
            r"\begin{document}\section{A}p1\subsection{B}p2\section{C}p3\end{document}",
            r"\begin{document}\part{P}\chapter{C}\section{S}deep text\end{document}",
            r"\begin{document}intro\section{A}\label{sec:a}owned\subsection{B}more\end{document}",
        ] {
            let doc = parse_document(src).expect("parse");
            let rendered = doc.to_latex();
            let redoc = parse_document(&rendered).expect("re-parse");
            assert_eq!(
                doc.strip_spans(),
                redoc.strip_spans(),
                "nested round-trip failed:\n src = {src:?}\n rendered = {rendered:?}"
            );
        }
    }

    #[test]
    fn span_union_containment_for_nested_sections() {
        // D3 span-union: a section's span ⊇ its children's spans, and still ⊆ the body region.
        let src = concat!(
            r"\documentclass{article}\begin{document}",
            r"\section{A}p1\subsection{B}p2\section{C}p3",
            r"\end{document}",
        );
        let doc = parse_document(src).expect("parse");
        for block in &doc.body {
            check_block(block, doc.span);
        }
        // Explicitly: section A's span contains subsection B's span.
        if let Block::Section { span: a_span, body, .. } = &doc.body[0] {
            let b = body
                .iter()
                .find(|b| matches!(b, Block::Section { level: SectionLevel::Subsection, .. }))
                .expect("B");
            let b_span = block_span(b);
            assert!(
                b_span.start >= a_span.start && b_span.end <= a_span.end,
                "B span {b_span:?} not within A span {a_span:?}"
            );
        }
    }

    #[test]
    fn fold_is_total_on_headings_inside_environment() {
        // A section inside a `center` environment nests via the same fold — no panic, and the
        // environment body is a folded forest too.
        let src = r"\begin{document}\begin{center}\section{Inner}owned text\end{center}\end{document}";
        let doc = parse_document(src).expect("parse");
        let env = doc.body.iter().find(|b| matches!(b, Block::Environment { .. })).expect("env");
        if let Block::Environment { body, .. } = env {
            let sec = body.iter().find(|b| matches!(b, Block::Section { .. })).expect("inner section");
            if let Block::Section { body, .. } = sec {
                assert!(body.iter().any(|b| matches!(b, Block::Paragraph(..))), "section owns its text");
            }
        }
    }
}
