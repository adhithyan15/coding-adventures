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
//! ## Span policy (LTXDOC02 S3/S4 — precise body spans, precise `node_at`)
//!
//! Every [`Document`]/[`Preamble`]/[`Block`]/[`Inline`] node carries a [`Span`]. As of **LTXDOC02
//! S3**, the fold reads each source [`Node`]'s **carried, precise byte [`Span`]** (S1 threaded
//! exact token spans onto every `Node`; S2 unioned them onto the recognition-pass nodes), so:
//!
//! - `Document.span` = `0 .. src.len()` (the whole source).
//! - `Preamble.span` = `0 ..` the byte index of `\begin{document}` in the source (or `src.len()`
//!   if absent), located by a direct substring search.
//! - **Every body [`Block`]/[`Inline`] span is now the source node's own tight byte range** —
//!   `&src[block.span]` / `&src[inline.span]` slices back to *exactly* that node's source text
//!   (`\textbf{x}`, a `Text` run's word, a `\section` heading, …). A **composite** block that owns
//!   children (a `Section` after the D3 fold, a `Paragraph`, a `List`, a `Table`/`Figure` float, a
//!   list item, a caption) carries the **union** of its constituents' real spans (min start …
//!   max end), folded from the real child spans — never re-derived by substring search.
//!
//! Every child span ⊆ its parent span ⊆ the `Document` span still holds (the containment invariant
//! the spec and the ADJ total-coverage gate rely on), and now the leaves are *tight*: a body byte
//! resolves to the single node whose source it is.
//!
//! ### What stays region-coarse (honestly)
//!
//! - **`Preamble.span`** and the `span` fields on [`DocumentClass`]/[`Package`] remain the
//!   preamble-region span: the preamble is classified out of `\documentclass`/`\usepackage`
//!   *directives*, not walked as per-node body content, so a preamble-region span is the honest
//!   granularity there (these nodes are not visited by [`Document::walk`]).
//! - **[`Metadata`]** inline/block *content* is precise (it lowers through the same span-precise
//!   fold), but `Metadata` is an additive index over preamble/body nodes and is likewise not
//!   walked.
//!
//! As of **LTXDOC02 S4**, [`Document::node_at`] is formally the precise counterpart: because body
//! spans are now tight, `node_at(byte)` returns the **true per-token leaf** — the narrowest node
//! whose precise span contains the byte (ties → deepest in pre-order). The old "region-coarse"
//! hedging on `node_at`/[`Provenance`] is **retired** for body nodes; only the preamble/metadata
//! spans below stay honestly coarse.
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

use crate::ast::{ListItem, ListKind, Node, NodeKind, SectionLevel};
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
    /// Document metadata (`\title`/`\author`/`\date`/`abstract`), extracted as an **additive
    /// projection** over the preamble + body nodes — see [`Metadata`]. Never fabricated; the
    /// underlying nodes stay in `preamble`/`body`, so `to_latex` round-trips unchanged.
    pub metadata: Metadata,
    /// The document body, the nested sectioning forest of [`Block`]s.
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

/// Document **metadata** (LTXDOC01 D4): the `\title` / `\author` / `\date` directives and the
/// `abstract` environment, lifted into a small typed record so a consumer can ask "what is the
/// title?" without walking the block/inline tree.
///
/// ## Additive projection — the underlying nodes are **not** removed
///
/// `Metadata` is an **additive, non-destructive projection**. The `\title{…}` / `\author{…}` /
/// `\date{…}` commands still lower into [`Preamble::raw`] (or a body [`Block`]) exactly as they
/// did before D4, and the `abstract` environment still becomes a [`Block::Environment`] in the
/// body. `Metadata` is just a typed *index over* those nodes — nothing is moved or deleted. Two
/// consequences the tests pin down:
///
/// - **`to_latex` round-trips unchanged.** Because no node was removed, rendering back to source
///   is byte-for-byte the same as it was pre-D4 (the metadata field contributes nothing to
///   `to_latex`; the nodes it points at do the rendering, as before).
/// - **Re-parsing repopulates the same `Metadata` (a fixed point).**
///   `parse_document(&doc.to_latex())` yields a `Document` whose `metadata` equals `doc.metadata`
///   modulo spans — the projection is stable under the round-trip.
///
/// ## Never fabricated
///
/// Every field is `None`/empty when the corresponding directive is absent — D4 never invents a
/// title, author, or date. Extraction is **total and panic-free**: it scans the already-parsed
/// node stream (no new parsing, no unchecked indexing, no unbounded recursion — the `abstract`
/// body lowers through the same bounded [`lower_blocks`] every other environment body uses).
///
/// ## Where the directives may appear
///
/// LaTeX allows `\title` / `\author` / `\date` in **either** the preamble (the common case) *or*
/// the body (after `\begin{document}`, before `\maketitle`). D4 therefore scans **both** node
/// streams; the first `\title` (and first `\date`) wins, and every `\author` contributes. The
/// `abstract` environment is a body construct, so only the body is scanned for it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Metadata {
    /// `\title{…}`, lowered to inlines. `None` when there is no `\title`. First `\title` wins.
    pub title: Option<Vec<Inline>>,
    /// `\author{A \and B}` → one entry per `\and`-separated author group, each lowered to inlines.
    /// Every `\author` command contributes (a paper may split authors across several `\author`s).
    pub authors: Vec<Vec<Inline>>,
    /// `\date{…}`, lowered to inlines. `None` when there is no `\date`. First `\date` wins.
    pub date: Option<Vec<Inline>>,
    /// The `abstract` environment's body, lowered to blocks. `None` when there is no `abstract`.
    pub abstract_: Option<Vec<Block>>,
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
    /// The preamble-region span. **Honestly region-coarse** (and stays so past S4, which only made
    /// *body* nodes precise): the preamble is classified out of directives rather than walked as
    /// per-node body content, so a preamble-region span is the right granularity here (this node is
    /// not visited by [`Document::walk`], and `node_at` never resolves into it).
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
    /// The preamble-region span. **Honestly region-coarse** (and stays so past S4, which only made
    /// *body* nodes precise): the preamble is classified out of directives rather than walked as
    /// per-node body content, so a preamble-region span is the right granularity here (this node is
    /// not visited by [`Document::walk`], and `node_at` never resolves into it).
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
        /// Precise span (S3): the union of the heading command's real span and the owned
        /// children's real spans — the section's exact source extent.
        span: Span,
    },
    /// A run of inline content between paragraph breaks. The span is the union of the run's
    /// inlines' real spans (S3) — the paragraph's tight source extent.
    Paragraph(Vec<Inline>, Span),
    /// An `itemize`/`enumerate`/`description` list (from [`Node::List`]).
    List {
        /// Which list flavour.
        kind: ListKind,
        /// The list items (each lowered to blocks; the term to inlines).
        items: Vec<DocListItem>,
        /// Precise span (S3): the `\begin{…}`…`\end{…}` extent of the list environment.
        span: Span,
    },
    /// A `tabular`/`tabular*` grid (from [`Node::Tabular`]) — optionally wrapped by a `table`
    /// float that contributes a `\caption` and/or `\label` (D5).
    Table {
        /// The column spec captured verbatim (`"lcr"`), or `None`.
        col_spec: Option<String>,
        /// `rows[r][c]` is cell `c` of row `r`, each lowered to blocks.
        rows: Vec<Vec<Vec<Block>>>,
        /// The `\caption{…}` of the enclosing `table` float, if any (D5). `None` for a bare
        /// `tabular` with no float wrapper.
        caption: Option<Caption>,
        /// A `\label{…}` hoisted off the enclosing `table` float (the key, no braces), if any (D5).
        label: Option<String>,
        /// Precise span (S3): the `\begin{tabular}`…`\end{tabular}` extent — unioned with the
        /// enclosing `\begin{table}`…`\end{table}` float extent when a float wraps it, so a
        /// captioned table's span covers its caption/label bytes too.
        span: Span,
    },
    /// A `figure`/`figure*` float (D5). `content` is the float body (e.g. an `\includegraphics`
    /// command carried through as a paragraph) minus the `\caption`/`\label` markers, which are
    /// lifted into `caption`/`label`.
    Figure {
        /// The float body, lowered to blocks, with the caption/label markers removed.
        content: Vec<Block>,
        /// The `\caption{…}`, lowered to inlines, if any.
        caption: Option<Caption>,
        /// A `\label{…}` hoisted off the float (the key, no braces), if any.
        label: Option<String>,
        /// Precise span (S3): the `\begin{figure}`…`\end{figure}` float extent.
        span: Span,
    },
    /// A verbatim code block (`verbatim`/`verbatim*`/`lstlisting`, D5). `verbatim` is the raw inner
    /// text kept **unparsed** — a code listing is source, not marked-up LaTeX.
    CodeBlock {
        /// The raw inner text of the verbatim environment.
        verbatim: String,
        /// Precise span (S3): the `\begin{verbatim}`…`\end{verbatim}` extent.
        span: Span,
    },
    /// A display-math island (`\[…\]`, `$$…$$`, or a named display-math environment such as
    /// `equation`/`align`/`gather`, D5) — kept as its source string (delegated to the math frontend
    /// on demand, never parsed here).
    DisplayMath {
        /// The exact inner math source.
        source: String,
        /// Precise span (S3): the display-math island's extent (`\[…\]`, `$$…$$`, or the named
        /// `\begin{equation}`…`\end{equation}`).
        span: Span,
    },
    /// A `quote`/`quotation` block quotation (D5), body recursively lowered to blocks. The span is
    /// the `\begin{quote}`…`\end{quote}` extent (S3).
    Quote(Vec<Block>, Span),
    /// Any other `\begin{env}…\end{env}` block, recursed.
    Environment {
        /// The environment name.
        name: String,
        /// The environment body, recursively lowered to blocks.
        body: Vec<Block>,
        /// Precise span (S3): the `\begin{name}`…`\end{name}` extent.
        span: Span,
    },
    /// An LTX01 node with no block meaning of its own — carried through verbatim (never dropped).
    /// The span is the underlying node's own real span (S3).
    Raw(Node, Span),
}

/// A `\caption{…}` on a float (D5): the caption content lowered to inlines, plus its span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Caption {
    /// The caption content, lowered to inlines.
    pub content: Vec<Inline>,
    /// Precise span (S3): the union of the caption's content inlines' real spans (falling back to
    /// the `\caption` command's own span for an empty `\caption{}`).
    pub span: Span,
}

impl Caption {
    /// Span-stripped projection of the caption (its content inlines zeroed), for round-trip
    /// equality that ignores byte offsets.
    fn strip_spans(&self, z: Span) -> Caption {
        Caption { content: strip_inlines(&self.content, z), span: z }
    }
}

/// One entry of a [`Block::List`] — the D2 analogue of [`crate::ListItem`], with the `\item[term]`
/// optional term lowered to inlines and the item body lowered to blocks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocListItem {
    /// The `\item[term]` optional term, lowered to inlines (`None` for a plain `\item`).
    pub term: Option<Vec<Inline>>,
    /// The item body, lowered to blocks.
    pub body: Vec<Block>,
    /// Precise span (S3): the union of the item's term-inline spans and body-block spans — its
    /// tight source extent. ([`crate::ListItem`] carries no span of its own, so we fold it here.)
    pub span: Span,
}

/// An inline (character-level) element. As of **S3** every span is the source node's own precise
/// byte range (`&src[inline.span()]` slices back to exactly the inline's source); a composite
/// inline (`Strong`/`Emph`/`Styled`/`CrossRef`/`Accent`) carries the whole-construct span S2
/// computed (e.g. `\textbf`…closing `}`), which ⊇ its children's spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Inline {
    /// A run of ordinary text; span = the run's exact bytes (S3).
    Text(String, Span),
    /// Significant inter-word space; span = the space's exact bytes (S3).
    Space(Span),
    /// `\textbf{…}` — strong (bold) emphasis; span = `\textbf`…closing `}` (S3).
    Strong(Vec<Inline>, Span),
    /// `\emph{…}` — emphasis; span = `\emph`…closing `}` (S3).
    Emph(Vec<Inline>, Span),
    /// `\texttt{…}` — monospace/code; span = `\texttt`…closing `}` (S3).
    Code(String, Span),
    /// Any other argument-form font command (`\textsf{…}`, `\underline{…}`, …).
    Styled {
        /// The control-word verbatim (`"textsf"`, …).
        command: String,
        /// The wrapped content, lowered to inlines.
        content: Vec<Inline>,
        /// Precise span (S3): the whole `\command{…}` construct.
        span: Span,
    },
    /// An inline math island (`$…$`, `\(…\)`) — kept as its source string.
    Math {
        /// The exact inner math source.
        source: String,
        /// Precise span (S3): the whole `$…$` / `\(…\)` island.
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
        /// Precise span (S3): `\command`…closing `}` (spanning the optional `[note]` too).
        span: Span,
    },
    /// A text accent (`\'e`, `\c{c}`) — the accent control word plus its base inline.
    Accent {
        /// The accent control-word verbatim (`"'"`, `"c"`, …).
        accent: String,
        /// The accented base, lowered to a single inline.
        base: Box<Inline>,
        /// Precise span (S3): `\accent`…the base's closing `}` — the whole accent construct.
        span: Span,
    },
    /// An LTX01 node with no inline meaning of its own — carried through verbatim; span = the
    /// node's own real span (S3).
    Raw(Node, Span),
}

// ---------------------------------------------------------------------------------------------
// D6 — the provenance API: `walk` (pre-order, span-annotated) and `node_at` (byte → node).
//
// The `Document` AST already carries a `Span` on every node. D6 exposes two views over those
// spans so a consumer (the ADJ byte-provenance pipeline being the north-star one) can ask, of any
// source byte, *which document node owns it* — the exact source→node correlation the reasoning
// stack audits against.
//
// ## Span granularity (S3/S4: body Block/Inline spans are precise; `node_at` resolves to the leaf)
//
// As of **LTXDOC02 S3**, the D2 fold reads each source node's carried, precise byte span (S1/S2)
// instead of the old enclosing-region parameter. So a body `Block`/`Inline` span is the node's
// *tight* source range — a `Section`'s title inline, its child paragraphs, and their `Text` runs
// no longer share one region span; each slices back to exactly its own source. `walk()` visits
// every body node in structural pre-order (unchanged), and every yielded span is precise.
//
// **S4 retires the region-coarse caveat for body nodes.** Because the spans are tight,
// [`Document::node_at`] returns the **true per-token leaf**: the narrowest node whose precise span
// contains the queried byte (ties → the deepest node in pre-order). A byte inside `widgets`
// resolves to the `Text` node owning `widgets`, not to the enclosing `Paragraph`/`Section`. The
// dedicated leaf-resolution tests (`node_at_resolves_to_text_leaf_not_paragraph`,
// `node_at_in_section_title_resolves_to_heading_inline`,
// `node_at_in_textbf_resolves_to_inner_leaf`) pin this down, and the honest body byte-coverage test
// (`body_bytes_resolve_to_containing_node`) asserts every non-whitespace body byte resolves to a
// node whose precise span actually contains it. What stays coarse is **only** the preamble/metadata
// (classified out of directives, not walked). The full tightest-covering-leaf capstone over the
// whole LTXDOC01 corpus (no strictly-narrower node exists) is **S5**.
// ---------------------------------------------------------------------------------------------

/// A borrowed reference to one node visited by [`Document::walk`]: either a body [`Block`] or an
/// [`Inline`], tagged so the caller can read its [`kind`](NodeRef::kind) and
/// [`span`](NodeRef::span) uniformly without matching every variant.
///
/// This is a lightweight *view* — it borrows the node in place (no clone), so a `walk()` over a
/// large document allocates only the traversal's `Vec<NodeRef>`, not a copy of the tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeRef<'a> {
    /// A block-level node (`Section`, `Paragraph`, `Table`, `Figure`, …).
    Block(&'a Block),
    /// An inline node (`Text`, `Strong`, `Math`, `CrossRef`, …).
    Inline(&'a Inline),
}

impl<'a> NodeRef<'a> {
    /// This node's byte [`Span`] — precise (tight source range) for body nodes (see the module span
    /// note); `node_at` uses it to resolve a byte to the true per-token leaf (S4).
    pub fn span(&self) -> Span {
        match self {
            NodeRef::Block(b) => block_span(b),
            NodeRef::Inline(i) => inline_span(i),
        }
    }

    /// A stable, human-readable kind string for this node's variant (`"Section"`, `"Paragraph"`,
    /// `"Text"`, `"Math"`, …). Useful for structure queries and test assertions that want to name
    /// what `walk()` yielded without matching the full enum.
    pub fn kind(&self) -> &'static str {
        match self {
            NodeRef::Block(b) => match b {
                Block::Section { .. } => "Section",
                Block::Paragraph(..) => "Paragraph",
                Block::List { .. } => "List",
                Block::Table { .. } => "Table",
                Block::Figure { .. } => "Figure",
                Block::CodeBlock { .. } => "CodeBlock",
                Block::DisplayMath { .. } => "DisplayMath",
                Block::Quote(..) => "Quote",
                Block::Environment { .. } => "Environment",
                Block::Raw(..) => "Raw",
            },
            NodeRef::Inline(i) => match i {
                Inline::Text(..) => "Text",
                Inline::Space(..) => "Space",
                Inline::Strong(..) => "Strong",
                Inline::Emph(..) => "Emph",
                Inline::Code(..) => "Code",
                Inline::Styled { .. } => "Styled",
                Inline::Math { .. } => "Math",
                Inline::CrossRef { .. } => "CrossRef",
                Inline::Accent { .. } => "Accent",
                Inline::Raw(..) => "Raw",
            },
        }
    }
}

/// The result of a [`Document::node_at`] provenance query: the narrowest walked [`NodeRef`] whose
/// span contains the queried byte, plus that node's [`Span`] (surfaced directly so the caller need
/// not re-derive it).
///
/// The `span` is the returned node's own byte range — **precise** for body nodes (see the module
/// span note). `node_at` returns the node with the *narrowest* such span, which as of **S4** is the
/// true per-token leaf: a byte inside a word resolves to the `Text` run owning that word, not to an
/// enclosing `Paragraph`/`Section`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Provenance<'a> {
    /// The narrowest node owning the queried byte — the true per-token leaf for body nodes (S4).
    pub node: NodeRef<'a>,
    /// That node's precise span (for body nodes).
    pub span: Span,
}

/// The production analogue of the test-module helper: this node's [`Span`], mirroring
/// [`block_span`] for the [`Inline`] side. Used by [`NodeRef::span`] and the walk/`node_at` logic.
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
            if let NodeKind::Environment { name, body, .. } = &node.kind {
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

    // D4 metadata extraction runs **before** we consume the two node streams, as a read-only
    // scan over *borrowed* nodes: it copies out the `\title`/`\author`/`\date` arguments and the
    // `abstract` body, but leaves every node in place so the subsequent `classify_preamble` /
    // `lower_blocks` folds still see them (additive projection — see [`Metadata`]).
    let metadata = extract_metadata(&preamble_nodes, &body_nodes);

    let preamble = classify_preamble(preamble_nodes, preamble_span);
    // `lower_blocks` produces the *flat* D2 block stream and then runs the D3 sectioning fold on
    // it (see `lower_blocks`), so `body` is already the nested sectioning forest. The same fold
    // runs on every nested block-list (environment/quote/figure bodies, list items, table cells)
    // because they all route through `lower_blocks` too — nesting therefore works everywhere.
    let body = lower_blocks(body_nodes, body_region);

    Document { preamble, metadata, body, span: doc_span }
}

// ---------------------------------------------------------------------------------------------
// D4 — metadata extraction (an additive projection over the preamble + body node streams).
// ---------------------------------------------------------------------------------------------

/// Build the [`Metadata`] projection by scanning the preamble and body node streams for the
/// `\title` / `\author` / `\date` commands and the `abstract` environment.
///
/// **Non-destructive**: this takes the node streams *by reference* and mutates nothing — the same
/// nodes are folded into `preamble.raw` / `body` afterwards, so nothing is moved or dropped. The
/// scan is a single linear pass over each top-level stream (it does **not** descend into arbitrary
/// nested groups — `\title` in real documents is a top-level directive, and descending would risk
/// double-capturing a `\title` that appears verbatim inside, say, a `verbatim` example). This
/// keeps the pass total and O(n).
///
/// Precedence: the **first** `\title` and the **first** `\date` win (LaTeX honours the last
/// definition, but for a faithful *structure* model the first occurrence is the stable choice and
/// matches how `\maketitle` would render a well-formed single-`\title` document). Every `\author`
/// contributes — a document may issue several `\author` commands, and each `\and` inside one
/// splits it into multiple author entries.
///
/// **S3 (precise spans).** The metadata's inline runs (`\title`/`\author`/`\date` arguments) and
/// the `abstract` body lower through the span-precise [`lower_inlines`]/[`lower_blocks_precise`],
/// so each metadata inline/block carries its own tight source range — no enclosing region is
/// threaded here anymore.
fn extract_metadata(preamble_nodes: &[Node], body_nodes: &[Node]) -> Metadata {
    let mut meta = Metadata::default();

    // `\title`/`\author`/`\date` may live in either stream. Scan the preamble first (the common
    // home), then the body, so a preamble `\title` takes precedence over a stray body one.
    scan_title_author_date(preamble_nodes, &mut meta);
    scan_title_author_date(body_nodes, &mut meta);

    // The `abstract` environment is a body construct only.
    for node in body_nodes {
        if let NodeKind::Environment { name, body, .. } = &node.kind {
            if name == "abstract" && meta.abstract_.is_none() {
                meta.abstract_ = Some(lower_blocks_precise(body.clone()));
            }
        }
    }

    meta
}

/// Scan one node stream for `\title` / `\author` / `\date` commands, folding each into `meta`.
/// Each lowered inline keeps its own real span (S3). Total and allocation-only.
fn scan_title_author_date(nodes: &[Node], meta: &mut Metadata) {
    for node in nodes {
        // These directives survive D2 lowering as plain `Node::Command`s (they are not among the
        // constructs `recognize_structure` folds), so we match the raw command with exactly one
        // mandatory argument and no optional argument.
        if let NodeKind::Command { name, optional, arguments } = &node.kind {
            if !optional.is_empty() {
                continue; // A bracketed form isn't the plain \title{…}/\author{…}/\date{…} we mean.
            }
            match name.as_str() {
                "title" => {
                    if meta.title.is_none() {
                        if let Some(arg) = arguments.first() {
                            meta.title = Some(lower_inlines(arg.clone()));
                        }
                    }
                }
                "date" => {
                    if meta.date.is_none() {
                        if let Some(arg) = arguments.first() {
                            meta.date = Some(lower_inlines(arg.clone()));
                        }
                    }
                }
                "author" => {
                    if let Some(arg) = arguments.first() {
                        for group in split_on_and(arg) {
                            meta.authors.push(lower_inlines(group));
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// Split an `\author` argument's node list on the `\and` separator (`\and` parses to a zero-arg
/// `Node::Command { name: "and", … }`). `\author{Alice \and Bob}` → `[[Alice, Space], [Space, Bob]]`
/// (two groups). A leading/trailing `\and` yields an empty group, which still becomes an (empty)
/// author entry — we do **not** silently drop it, keeping the split faithful; a single-author
/// argument with no `\and` yields exactly one group.
fn split_on_and(arg: &[Node]) -> Vec<Vec<Node>> {
    let mut groups: Vec<Vec<Node>> = Vec::new();
    let mut current: Vec<Node> = Vec::new();
    for node in arg {
        if matches!(&node.kind, NodeKind::Command { name, arguments, .. } if name == "and" && arguments.is_empty())
        {
            groups.push(std::mem::take(&mut current));
        } else {
            current.push(node.clone());
        }
    }
    groups.push(current);
    groups
}

/// Classify a preamble node stream into `\documentclass` / `\usepackage` directives plus the
/// untouched `raw` remainder. We match on the already-folded [`Node::Preamble`] variant.
fn classify_preamble(nodes: Vec<Node>, span: Span) -> Preamble {
    let mut document_class: Option<DocumentClass> = None;
    let mut packages: Vec<Package> = Vec::new();
    let mut raw: Vec<Node> = Vec::new();

    for node in nodes {
        match &node.kind {
            NodeKind::Preamble { command, options, name } if command == "documentclass" => {
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
            NodeKind::Preamble { command, options, name }
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
/// every [`Block::Section`] owns the run of blocks that follow it.
///
/// **S3 (precise spans).** Each emitted block carries its source node's own tight span (via
/// `lower_block`); a flushed [`Block::Paragraph`] carries the union of its inlines' real spans. The
/// `region` parameter is now only a **fallback** for the degenerate empty-paragraph case (which
/// `flush` never actually reaches) and for the top-level body-region seed — every real block/inline
/// span is precise. A folded section then unions in its owned children's real spans.
///
/// Because *every* block-list in the document (top-level body, environment/quote/figure bodies,
/// list-item bodies, table cells) routes through this one function, the sectioning fold applies
/// uniformly — a `\section` inside a `\begin{center}…\end{center}` nests just like a top-level one.
fn lower_blocks(nodes: Vec<Node>, region: Span) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    let mut pending: Vec<Inline> = Vec::new();

    // Flush the pending inline run into a Paragraph block, if non-empty. The paragraph's span is
    // the **union of its inlines' real spans** (S3): the byte range from the first inline's start
    // to the last inline's end. `region` is the fallback for the degenerate empty-inlines case
    // (which `flush` never actually reaches, since it only runs when `pending` is non-empty), so
    // the paragraph span is precise — `&src[span]` slices back to exactly the run's source text.
    fn flush(pending: &mut Vec<Inline>, blocks: &mut Vec<Block>, region: Span) {
        if !pending.is_empty() {
            let span = span_of_inlines(pending, region);
            blocks.push(Block::Paragraph(std::mem::take(pending), span));
        }
    }

    for node in nodes {
        if is_block_node(&node) {
            flush(&mut pending, &mut blocks, region);
            blocks.push(lower_block(node));
        } else if matches!(node.kind, NodeKind::Par) {
            // A blank-line paragraph break closes the current paragraph.
            flush(&mut pending, &mut blocks, region);
        } else {
            pending.push(lower_inline(node));
        }
    }
    flush(&mut pending, &mut blocks, region);
    fold_sections(blocks)
}

/// The union of an inline run's real spans (S3): min start … max end over every inline's carried
/// span. Used to give a [`Block::Paragraph`] (and a float caption) the tight byte range of its
/// content, rather than the coarse enclosing region. We *seed the fold from the first inline's own
/// span* (via [`Iterator::reduce`]) so a non-empty run's span is exactly its content extent — a
/// plain `fold(fallback, …)` would only ever *widen* past `fallback` and never shrink to the true
/// start. An empty run has no inlines, so `reduce` yields `None` and we fall back to `fallback`
/// (never a panic).
fn span_of_inlines(inlines: &[Inline], fallback: Span) -> Span {
    inlines.iter().map(inline_span).reduce(union).unwrap_or(fallback)
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
///   section's `body`. The section's `span` becomes the union of its heading's real span and its
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
                // Span union: heading's real span ∪ each folded child's real span (S3).
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
        *s = span; // keep the paragraph's span; unchanged, but explicit.
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
        | Block::Figure { span, .. }
        | Block::CodeBlock { span, .. }
        | Block::DisplayMath { span, .. }
        | Block::Environment { span, .. } => *span,
        Block::Paragraph(_, span) | Block::Quote(_, span) | Block::Raw(_, span) => *span,
    }
}

// ---------------------------------------------------------------------------------------------
// D6 walk collectors — pre-order, depth-first accumulation into a `Vec<NodeRef>`.
//
// These mirror the shape of the test-module's `check_block`/`check_inline` traversals exactly, so
// "what `walk()` visits" and "what the span-integrity check descends into" stay in lockstep. Each
// function pushes the node itself FIRST (pre-order), then recurses into its children in source
// order. Recursion depth is bounded by the parser's `MAX_DEPTH` cap on body nesting.
// ---------------------------------------------------------------------------------------------

/// Push `block` and, in pre-order, every descendant block/inline into `out`.
fn walk_block<'a>(block: &'a Block, out: &mut Vec<NodeRef<'a>>) {
    out.push(NodeRef::Block(block));
    match block {
        Block::Section { title, short_title, body, .. } => {
            for i in title {
                walk_inline(i, out);
            }
            if let Some(s) = short_title {
                for i in s {
                    walk_inline(i, out);
                }
            }
            for b in body {
                walk_block(b, out);
            }
        }
        Block::Paragraph(inlines, _) => {
            for i in inlines {
                walk_inline(i, out);
            }
        }
        Block::List { items, .. } => {
            for it in items {
                if let Some(t) = &it.term {
                    for i in t {
                        walk_inline(i, out);
                    }
                }
                for b in &it.body {
                    walk_block(b, out);
                }
            }
        }
        Block::Table { rows, caption, .. } => {
            if let Some(cap) = caption {
                for i in &cap.content {
                    walk_inline(i, out);
                }
            }
            for row in rows {
                for cell in row {
                    for b in cell {
                        walk_block(b, out);
                    }
                }
            }
        }
        Block::Figure { content, caption, .. } => {
            if let Some(cap) = caption {
                for i in &cap.content {
                    walk_inline(i, out);
                }
            }
            for b in content {
                walk_block(b, out);
            }
        }
        Block::Environment { body, .. } | Block::Quote(body, _) => {
            for b in body {
                walk_block(b, out);
            }
        }
        // Leaves with no walkable children.
        Block::DisplayMath { .. } | Block::CodeBlock { .. } | Block::Raw(..) => {}
    }
}

/// Push `inline` and, in pre-order, every descendant inline into `out`.
fn walk_inline<'a>(inline: &'a Inline, out: &mut Vec<NodeRef<'a>>) {
    out.push(NodeRef::Inline(inline));
    match inline {
        Inline::Strong(c, _) | Inline::Emph(c, _) => {
            for i in c {
                walk_inline(i, out);
            }
        }
        Inline::Styled { content, .. } => {
            for i in content {
                walk_inline(i, out);
            }
        }
        Inline::CrossRef { note: Some(n), .. } => {
            for i in n {
                walk_inline(i, out);
            }
        }
        Inline::Accent { base, .. } => walk_inline(base, out),
        // Leaves (and `CrossRef` with no note).
        _ => {}
    }
}

/// Does this node lower to a [`Block`] of its own (rather than joining an inline run)?
fn is_block_node(node: &Node) -> bool {
    matches!(
        node.kind,
        NodeKind::Section { .. }
            | NodeKind::List { .. }
            | NodeKind::Tabular { .. }
            | NodeKind::Math { display: true, .. }
            | NodeKind::Environment { .. }
            | NodeKind::VerbatimEnv { .. }
    )
}

/// Lower a single block-level node into its [`Block`], recursing into child node-lists.
///
/// **S3 (precise spans).** The resulting block is stamped with the source node's own carried
/// [`Node::span`] — its exact byte range — not an enclosing region. For a `Section` this is the
/// heading command's span (S2: `\section`…title); [`fold_sections`] later unions in the owned
/// children's real spans. For a `List`/`Tabular`/`Environment` it is the `\begin`…`\end` extent
/// S2 computed. Child node-lists (list items, table cells, environment bodies) recurse through the
/// span-precise [`lower_blocks`]/[`lower_inline`], so *their* spans are precise too.
fn lower_block(node: Node) -> Block {
    // Destructure on the kind; several arms need the node's own span, and the fallback needs the
    // whole node, so bind `span` up front.
    let Node { kind, span } = node;
    match kind {
        NodeKind::Section { level, starred, short, title } => Block::Section {
            level,
            numbered: !starred,
            title: lower_inlines(title),
            short_title: short.map(lower_inlines),
            label: None,      // filled by fold_sections (D3) if a \label follows the heading.
            body: Vec::new(), // filled by fold_sections (D3): the run of blocks this heading owns.
            span, // the heading command's real span; fold_sections unions in the owned children.
        },
        NodeKind::List { kind, items } => Block::List {
            kind,
            items: items.into_iter().map(lower_list_item).collect(),
            span, // the `\begin{…}`…`\end{…}` extent (S2).
        },
        NodeKind::Tabular { col_spec, rows } => Block::Table {
            col_spec,
            rows: rows
                .into_iter()
                .map(|row| row.into_iter().map(lower_blocks_precise).collect())
                .collect(),
            caption: None, // set only when a `table` float wraps this tabular (see `lower_environment`).
            label: None,
            span, // the `\begin{tabular}`…`\end{tabular}` extent (S2).
        },
        NodeKind::Math { display: true, content } => Block::DisplayMath { source: content, span },
        // A `verbatim`/`verbatim*` environment is lexed raw (catcodes suspended) into a
        // `VerbatimEnv` node; its content is source code, not marked-up LaTeX, so we keep it
        // **unparsed** as a `CodeBlock` (D5).
        NodeKind::VerbatimEnv { content, .. } => Block::CodeBlock { verbatim: content, span },
        // Every `\begin{env}…\end{env}` (floats, quotes, display-math envs, code listings, or an
        // unknown env) is classified by name in `lower_environment` (D5), carrying the env's span.
        NodeKind::Environment { name, body, .. } => lower_environment(name, body, span),
        // Anything else with no block model of its own — carried through verbatim (never dropped).
        other => {
            let raw = Node::new(other, span);
            Block::Raw(raw, span)
        }
    }
}

/// A cell/body lowering that lowers a node-list to blocks and unions its blocks' real spans into
/// the cell's fallback region — used for `tabular` cells, whose enclosing region span we no longer
/// thread. Cells contain fully-spanned blocks, so this just forwards to [`lower_blocks`] with the
/// cell's own union as the fallback region: the union of the cell's node spans (empty cell → an
/// empty span at the tabular start, which never surfaces because empty cells produce no blocks).
fn lower_blocks_precise(nodes: Vec<Node>) -> Vec<Block> {
    // The cell's fallback region: the union of its constituent node spans (precise), so any
    // paragraph flushed from an all-inline cell gets a tight span even without a threaded region.
    let region = nodes.iter().map(|n| n.span).reduce(union).unwrap_or(Span::new(0, 0));
    lower_blocks(nodes, region)
}

// ---------------------------------------------------------------------------------------------
// D5 — environment classification: figures, table floats, code, display-math, quotes.
// ---------------------------------------------------------------------------------------------

/// The named display-math **environments** (`equation`, `align`, …). These are display math whose
/// inner LaTeX is kept as a **source string** (delegated to the math frontend on demand — LTXDOC01
/// never parses math itself). `\[…\]` / `$$…$$` already route through [`Node::Math`]; this closes
/// the *named-environment* form. `align`/`gather`/`multline` carry `\\`-separated lines and `&`
/// alignment points — all preserved verbatim in the source string, since we do not tokenize them.
fn is_display_math_env(name: &str) -> bool {
    matches!(
        name,
        "equation"
            | "equation*"
            | "align"
            | "align*"
            | "displaymath"
            | "gather"
            | "gather*"
            | "multline"
            | "multline*"
            | "eqnarray"
            | "eqnarray*"
    )
}

/// Classify a `\begin{name}…\end{name}` environment into its D5 [`Block`], recursing the body
/// through the same bounded [`lower_blocks`] fold every other block-list uses (so nesting and
/// sectioning work uniformly inside floats/quotes). **Total & panic-free**: no `unwrap`/`expect`,
/// no unchecked indexing; anything that does not classify cleanly falls back to a lossless
/// [`Block::Environment`] / [`Block::Figure`] so no content is ever dropped.
///
/// | environment | → block |
/// |-------------|---------|
/// | `figure`, `figure*` | [`Block::Figure`] (body minus `\caption`/`\label`) |
/// | `table`, `table*` | the inner [`Block::Table`] with the float's `\caption`/`\label` attached (or [`Block::Figure`] if no tabular inside) |
/// | `quote`, `quotation` | [`Block::Quote`] |
/// | display-math envs (see [`is_display_math_env`]) | [`Block::DisplayMath`] (source kept, unparsed) |
/// | `lstlisting` | [`Block::CodeBlock`] (body rendered back to source text) |
/// | any other | [`Block::Environment`] (recursed) — unchanged from D2 |
fn lower_environment(name: String, body: Vec<Node>, env_span: Span) -> Block {
    // `env_span` is the environment node's own real span (S2/S3): `\begin{name}`…the closing `}`
    // of `\end{name}`. It is the precise byte extent of the whole float/quote/env, so every block
    // below stamps it directly. The body's *inner* blocks recurse through span-precise
    // `lower_blocks_precise`, so their spans are the tight per-node ranges — only the wrapper
    // block carries the `\begin…\end` extent (which correctly ⊇ its children).

    // A display-math environment: keep the inner LaTeX as a source string. We render the body's
    // nodes back to source (the parser accepted the math tokens as ordinary nodes, since these
    // are *text-mode* `\begin{equation}` wrappers) and trim the outer whitespace the wrapper adds.
    if is_display_math_env(&name) {
        return Block::DisplayMath { source: render_nodes(&body).trim().to_string(), span: env_span };
    }
    // `lstlisting` is *not* lexed raw (only `verbatim`/`verbatim*` are), so its body parsed as
    // ordinary nodes; render it back to source to recover the listing text. (A perfectly faithful
    // capture of `lstlisting` would need raw-lexing support in a later rung; rendering the parsed
    // body back is lossless for the common case of plain code and keeps the fold total.)
    if name == "lstlisting" {
        return Block::CodeBlock { verbatim: render_nodes(&body), span: env_span };
    }
    if name == "quote" || name == "quotation" {
        return Block::Quote(lower_blocks_precise(body), env_span);
    }

    // Floats: lower the body first, then lift the `\caption`/`\label` markers out of it. The
    // float wrapper carries the `\begin…\end` extent; its inner content blocks are span-precise.
    if name == "figure" || name == "figure*" {
        let mut content = lower_blocks_precise(body);
        let (caption, label) = extract_caption_label(&mut content);
        return Block::Figure { content, caption, label, span: env_span };
    }
    if name == "table" || name == "table*" {
        let mut content = lower_blocks_precise(body);
        let (caption, label) = extract_caption_label(&mut content);
        // Attach the float's caption/label to the inner `tabular` if there is one — the common,
        // faithful shape (`\begin{table}…\begin{tabular}…\end{tabular}\caption{…}\end{table}`).
        // The inner `tabular` was lowered to a `Block::Table` with `caption: None, label: None`;
        // we rebuild it with the float's caption/label attached. Its span becomes the **union** of
        // the inner tabular's real span and the enclosing float extent (so the captioned table's
        // span covers the whole `\begin{table}…\end{table}`, which owns the caption/label bytes).
        if let Some(idx) = content.iter().position(|b| matches!(b, Block::Table { .. })) {
            if let Block::Table { col_spec, rows, span, .. } = content.remove(idx) {
                return Block::Table { col_spec, rows, caption, label, span: union(span, env_span) };
            }
        }
        // No inner tabular — do not lose the float; treat it as a figure-shaped float so the
        // caption/label are still attached and the body survives.
        return Block::Figure { content, caption, label, span: env_span };
    }

    // Any other environment: recurse, carrying the env's real `\begin…\end` span.
    Block::Environment { name, body: lower_blocks_precise(body), span: env_span }
}

/// Lift a `\caption{…}` and a `\label{…}` out of a float's lowered body, returning them and
/// **removing** their marker inlines from `content` (so they are not double-counted). Everything
/// that is *not* a caption or label stays in `content` — the fold never drops float content.
///
/// After [`lower_blocks`], a float's `\caption{X}` is an [`Inline::Raw`] wrapping a
/// `Node::Command { name: "caption", … }` and its `\label{k}` is an [`Inline::CrossRef`] with
/// `command == "label"`, both living inside the float's [`Block::Paragraph`]s (there is usually no
/// `\par` between `\includegraphics`, `\caption`, and `\label`, so they fuse into one paragraph).
/// We scan every paragraph, pull the **first** caption and **first** label out, and drop any
/// paragraph left empty (whitespace-only) once its markers are removed. Total & panic-free.
fn extract_caption_label(content: &mut Vec<Block>) -> (Option<Caption>, Option<String>) {
    let mut caption: Option<Caption> = None;
    let mut label: Option<String> = None;

    for block in content.iter_mut() {
        if let Block::Paragraph(inlines, _) = block {
            // Walk the paragraph, keeping non-marker inlines and lifting the first caption/label.
            let mut kept: Vec<Inline> = Vec::with_capacity(inlines.len());
            for inline in std::mem::take(inlines) {
                match &inline {
                    Inline::Raw(Node { kind: NodeKind::Command { name, arguments, optional }, span: cmd_span }, _)
                        if name == "caption" && optional.is_empty() && caption.is_none() =>
                    {
                        // `\caption{X}` — lower its mandatory argument to inlines. The caption's
                        // span is the union of those inlines' real spans (S3), falling back to the
                        // `\caption` command's own span for an empty `\caption{}`.
                        let content_inlines = arguments
                            .first()
                            .map(|arg| lower_inlines(arg.clone()))
                            .unwrap_or_default();
                        let cap_span = span_of_inlines(&content_inlines, *cmd_span);
                        caption = Some(Caption { content: content_inlines, span: cap_span });
                    }
                    Inline::CrossRef { command, target, .. }
                        if command == "label" && label.is_none() =>
                    {
                        label = Some(target.clone());
                    }
                    _ => kept.push(inline),
                }
            }
            *inlines = kept;
        }
    }
    // Drop any paragraph that is now empty or whitespace-only (its only content was a marker).
    content.retain(|b| match b {
        Block::Paragraph(inlines, _) => !inlines.iter().all(|i| matches!(i, Inline::Space(_))),
        _ => true,
    });

    (caption, label)
}

/// Lower one [`ListItem`] into a [`DocListItem`].
///
/// **S3 (precise spans).** [`ListItem`] itself carries no span (it is a pure regrouping of the
/// list body — see `ast.rs`), so the item's span is the **union of its constituents' real spans**:
/// the term inlines' spans folded with the body blocks' spans. That tight range slices back to the
/// item's source extent (from the `\item` argument/first body node to the last body node), rather
/// than the whole list region. An item with neither term nor body (rare) falls back to an empty
/// span at the origin — it produces no walkable children, so the fallback never surfaces.
fn lower_list_item(item: ListItem) -> DocListItem {
    let term = item.label.map(lower_inlines);
    let body = lower_blocks_precise(item.body);
    // Union the term inlines' spans and the body blocks' spans into the item's precise extent.
    let mut span: Option<Span> = None;
    if let Some(t) = &term {
        if let Some(s) = t.iter().map(inline_span).reduce(union) {
            span = Some(span.map_or(s, |acc| union(acc, s)));
        }
    }
    if let Some(s) = body.iter().map(block_span).reduce(union) {
        span = Some(span.map_or(s, |acc| union(acc, s)));
    }
    DocListItem { term, body, span: span.unwrap_or_else(|| Span::new(0, 0)) }
}

/// Lower a flat node run into a `Vec<Inline>` (used for headings, list terms, styled content).
/// Each node keeps its own real span (S3) — no enclosing region is threaded.
fn lower_inlines(nodes: Vec<Node>) -> Vec<Inline> {
    nodes.into_iter().map(lower_inline).collect()
}

/// Lower a single node into its [`Inline`]. Anything without an inline meaning becomes
/// [`Inline::Raw`] — never dropped, never a panic.
///
/// **S3 (precise spans).** Every resulting inline is stamped with the source node's own carried
/// [`Node::span`] — its exact byte range — not an enclosing region. For a composite inline
/// (`Strong`/`Emph`/`Styled`/`CrossRef`/`Accent`) S2 gave the node a span covering the whole
/// construct (`\textbf`…closing `}`, `\cite[note]`…`{key}`, `\'`…`{e}`), so that one span is both
/// the composite's tight extent *and* ⊇ its lowered children's spans. `&src[inline.span]` slices
/// back to exactly the inline's source.
fn lower_inline(node: Node) -> Inline {
    // Destructure on the kind; every arm stamps the node's own real `span`.
    let Node { kind, span } = node;
    match kind {
        NodeKind::Text(t) => Inline::Text(t, span),
        NodeKind::Space => Inline::Space(span),
        NodeKind::Styled { command, content } => match command.as_str() {
            "textbf" => Inline::Strong(lower_inlines(content), span),
            "emph" | "textit" => Inline::Emph(lower_inlines(content), span),
            "texttt" => Inline::Code(render_nodes(&content), span),
            _ => Inline::Styled { command, content: lower_inlines(content), span },
        },
        NodeKind::Math { display: false, content } => Inline::Math { source: content, span },
        NodeKind::CrossRef { command, note, target } => Inline::CrossRef {
            command,
            note: note.map(lower_inlines),
            target: render_nodes(&target),
            span,
        },
        NodeKind::Accent { accent, arg } => Inline::Accent {
            accent,
            base: Box::new(lower_accent_base(arg, span)),
            span,
        },
        // Everything else (a display Math slipped into an inline run, an unhandled command, …)
        // is carried through verbatim, keeping its own span.
        other => {
            let raw = Node::new(other, span);
            Inline::Raw(raw, span)
        }
    }
}

/// Lower an accent's base argument (a node list) into a single [`Inline`]. Accents apply to one
/// base; if the argument is a single node we lower it directly (keeping its real span), otherwise
/// we wrap the run in a `Styled` group whose span is the **union of the run's node spans** (S3) so
/// nothing is dropped and the wrapper's span is the true extent of the base. An empty base falls
/// back to `accent_span` (the accent command's own span) so the synthesised empty `Text` still sits
/// inside the accent's byte range.
fn lower_accent_base(arg: Vec<Node>, accent_span: Span) -> Inline {
    let run_span = arg.iter().map(|n| n.span).reduce(union).unwrap_or(accent_span);
    let mut it = arg.into_iter();
    match (it.next(), it.next()) {
        (Some(single), None) => lower_inline(single),
        (Some(first), Some(second)) => {
            // Multi-node base (rare) — keep it faithfully as a Styled group so nothing is dropped.
            let mut rest: Vec<Node> = vec![first, second];
            rest.extend(it);
            Inline::Styled { command: String::new(), content: lower_inlines(rest), span: run_span }
        }
        (None, _) => Inline::Text(String::new(), accent_span),
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

    /// Walk every body node in **pre-order, depth-first**, yielding a [`NodeRef`] for each.
    ///
    /// Order: a parent is yielded **before** its children, and children in source order — so a
    /// `Section` precedes its title inlines and then its child blocks; a `Figure` precedes its
    /// caption inlines and its content blocks; a `List` precedes each item's term inlines then body
    /// blocks; a `Table` precedes its cell blocks; a `Paragraph` precedes its inlines; and a
    /// composite inline (`Strong`/`Emph`/`Styled`/`CrossRef` note/`Accent` base) precedes its
    /// children. This mirrors the traversal a renderer or a diff would use.
    ///
    /// The traversal covers the **document body** (the core provenance surface). Preamble and
    /// metadata nodes are *not* walked — the preamble is classified out of directives and carries a
    /// preamble-region span rather than per-node body spans, so including it would add nodes whose
    /// spans overlap the whole preamble region without improving byte→node resolution; the spec §4
    /// names "body Blocks + their Inlines" as the core requirement and that is what we yield.
    ///
    /// **Precise spans (S3):** each yielded body span is the node's own tight source range (S1/S2
    /// threaded, S3 propagated). `walk()` is *total* — it visits every structural node exactly once
    /// — and *bounded*: body depth is capped upstream by the parser's `MAX_DEPTH`, so the recursion
    /// cannot blow the stack.
    ///
    /// Returned as a materialized `std::vec::IntoIter<NodeRef>` (the simplest total realization of
    /// the `impl Iterator` signature); the whole forest is small relative to the source.
    pub fn walk(&self) -> impl Iterator<Item = NodeRef<'_>> {
        let mut out: Vec<NodeRef<'_>> = Vec::new();
        for b in &self.body {
            walk_block(b, &mut out);
        }
        out.into_iter()
    }

    /// Provenance query: which document node owns source `byte`?
    ///
    /// Returns the **innermost** (narrowest-span) walked node whose half-open span *contains* the
    /// byte (`span.start <= byte < span.end`), or `None` if no walked node covers it. "Narrowest"
    /// is measured by span width `end.saturating_sub(start)` (saturating so the subtraction is
    /// panic-free even on a degenerate `end < start`), with ties broken toward the node visited
    /// *later* in pre-order — i.e. the deepest descendant, the tightest fit when a parent and child
    /// share a span.
    ///
    /// **Resolution (S4 — precise, region-coarse caveat retired):** body spans are the nodes' tight
    /// source ranges (S3 propagated the real per-node spans), so the narrowest-covering node this
    /// returns **is the true per-token leaf** — the narrowest node whose precise span contains the
    /// byte. A byte inside `widgets` resolves to the `Text` node owning `widgets`, not to the
    /// enclosing `Paragraph`/`Section`; a byte inside a `\section` title resolves to the heading's
    /// title inline, not to the whole `Section` block. This holds for **body** nodes (the ones
    /// `walk` visits); the preamble/metadata are classified out of directives rather than walked, so
    /// their spans stay honestly region-coarse and `node_at` does not resolve into them. The S5 rung
    /// adds the whole-corpus tightest-covering-leaf capstone (proving no strictly-narrower node
    /// exists for every body byte). Totally panic-free: no `unwrap`/`expect`, no unchecked indexing,
    /// guarded subtraction.
    pub fn node_at(&self, byte: usize) -> Option<Provenance<'_>> {
        let mut best: Option<NodeRef<'_>> = None;
        let mut best_width: usize = usize::MAX;
        for node in self.walk() {
            let span = node.span();
            if span.start <= byte && byte < span.end {
                let width = span.end.saturating_sub(span.start);
                // `<=` so a later (deeper, in pre-order) node of equal width wins the tie: the
                // deepest node sharing a span is the most specific answer.
                if best.is_none() || width <= best_width {
                    best = Some(node);
                    best_width = width;
                }
            }
        }
        best.map(|node| Provenance { node, span: node.span() })
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
            metadata: self.metadata.strip_spans(z),
            body: self.body.iter().map(|b| b.strip_spans(z)).collect(),
            span: z,
        }
    }
}

impl Metadata {
    /// Span-stripped projection of the metadata, so the round-trip fixed-point test can compare
    /// `\title`/`\author`/`\date`/`abstract` modulo byte offsets (which move under the round-trip).
    fn strip_spans(&self, z: Span) -> Metadata {
        Metadata {
            title: self.title.as_ref().map(|t| strip_inlines(t, z)),
            authors: self.authors.iter().map(|a| strip_inlines(a, z)).collect(),
            date: self.date.as_ref().map(|d| strip_inlines(d, z)),
            abstract_: self
                .abstract_
                .as_ref()
                .map(|blocks| blocks.iter().map(|b| b.strip_spans(z)).collect()),
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
            Block::Table { col_spec, rows, caption, label, .. } => {
                // When the tabular carries a float's caption/label, wrap it back in a `table`
                // float so re-parsing re-attaches them (round-trip fixed point).
                let floated = caption.is_some() || label.is_some();
                if floated {
                    out.push_str("\\begin{table}");
                }
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
                if floated {
                    write_caption_label(caption, label, out);
                    out.push_str("\\end{table}");
                }
            }
            Block::Figure { content, caption, label, .. } => {
                out.push_str("\\begin{figure}");
                write_blocks(content, out);
                write_caption_label(caption, label, out);
                out.push_str("\\end{figure}");
            }
            Block::CodeBlock { verbatim, .. } => {
                // Re-emit as a `verbatim` environment (which re-lexes raw to a `VerbatimEnv` →
                // `CodeBlock`, a fixed point). `lstlisting`-sourced blocks round-trip *through*
                // `verbatim` — the text is preserved, only the fence name normalizes.
                out.push_str("\\begin{verbatim}");
                out.push_str(verbatim);
                out.push_str("\\end{verbatim}");
            }
            Block::DisplayMath { source, .. } => {
                out.push_str("$$");
                out.push_str(source);
                out.push_str("$$");
            }
            Block::Quote(body, _) => {
                out.push_str("\\begin{quote}");
                write_blocks(body, out);
                out.push_str("\\end{quote}");
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
            Block::Table { col_spec, rows, caption, label, .. } => Block::Table {
                col_spec: col_spec.clone(),
                rows: rows
                    .iter()
                    .map(|row| row.iter().map(|cell| cell.iter().map(|b| b.strip_spans(z)).collect()).collect())
                    .collect(),
                caption: caption.as_ref().map(|c| c.strip_spans(z)),
                label: label.clone(),
                span: z,
            },
            Block::Figure { content, caption, label, .. } => Block::Figure {
                content: content.iter().map(|b| b.strip_spans(z)).collect(),
                caption: caption.as_ref().map(|c| c.strip_spans(z)),
                label: label.clone(),
                span: z,
            },
            Block::CodeBlock { verbatim, .. } => Block::CodeBlock { verbatim: verbatim.clone(), span: z },
            Block::Quote(body, _) => Block::Quote(body.iter().map(|b| b.strip_spans(z)).collect(), z),
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

/// Render a float's `\caption{…}` and hoisted `\label{…}` back to source (D5), in that order —
/// the shape `extract_caption_label` re-recognizes on the round-trip.
fn write_caption_label(caption: &Option<Caption>, label: &Option<String>, out: &mut String) {
    if let Some(cap) = caption {
        out.push_str("\\caption{");
        write_inlines(&cap.content, out);
        out.push('}');
    }
    if let Some(key) = label {
        out.push_str("\\label{");
        out.push_str(key);
        out.push('}');
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
            Block::DisplayMath { .. } | Block::CodeBlock { .. } | Block::Raw(..) => {}
            Block::Figure { content, caption, .. } => {
                if let Some(cap) = caption {
                    for i in &cap.content {
                        check_inline(i, span);
                    }
                }
                for b in content {
                    check_block(b, span);
                }
            }
            Block::Environment { body, .. } | Block::Quote(body, _) => {
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

    // `block_span` and `inline_span` are the production helpers, imported via `use super::*`.

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
        // S3 tightening: the containment above is no longer merely region-coarse — the "bold" Text
        // leaf slices back to EXACTLY its own source, proving the leaf span is the node source
        // range, not the enclosing region.
        let bold_leaf = doc
            .walk()
            .find_map(|n| match n {
                NodeRef::Inline(Inline::Text(t, sp)) if t == "bold" => Some(*sp),
                _ => None,
            })
            .expect("a 'bold' Text leaf");
        assert_eq!(&src[bold_leaf.start..bold_leaf.end], "bold", "S3: leaf span == exact node source");
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

    // -- D4: metadata-extraction tests --------------------------------------------------------

    /// Concatenate the plain-text of an inline run (for asserting a metadata field's text), joining
    /// [`Inline::Text`] and [`Inline::Space`] and recursing through emphasis/style wrappers. Spaces
    /// are rendered as a single ` `, and the result is trimmed so a leading/trailing `\and`-adjacent
    /// space (kept faithfully in the inlines) does not clutter the equality check.
    fn inline_text(inlines: &[Inline]) -> String {
        fn go(inlines: &[Inline], out: &mut String) {
            for i in inlines {
                match i {
                    Inline::Text(t, _) => out.push_str(t),
                    Inline::Space(_) => out.push(' '),
                    Inline::Strong(c, _) | Inline::Emph(c, _) => go(c, out),
                    Inline::Styled { content, .. } => go(content, out),
                    _ => {}
                }
            }
        }
        let mut s = String::new();
        go(inlines, &mut s);
        s.trim().to_string()
    }

    #[test]
    fn title_in_preamble_captured() {
        let src = r"\documentclass{article}\title{T}\begin{document}x\end{document}";
        let doc = parse_document(src).expect("parse");
        let title = doc.metadata.title.as_ref().expect("title present");
        assert_eq!(inline_text(title), "T");
        // Additive projection: the `\title` node is NOT removed — it survives in preamble.raw.
        assert!(
            doc.preamble.raw.iter().any(|n| matches!(&n.kind, NodeKind::Command { name, .. } if name == "title")),
            "the \\title node stays in preamble.raw (additive projection)"
        );
    }

    #[test]
    fn author_splits_on_and() {
        let src = r"\documentclass{article}\author{Alice \and Bob}\begin{document}x\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.metadata.authors.len(), 2, "\\and splits into two authors");
        assert_eq!(inline_text(&doc.metadata.authors[0]), "Alice");
        assert_eq!(inline_text(&doc.metadata.authors[1]), "Bob");
    }

    #[test]
    fn multiple_author_commands_all_contribute() {
        // A paper may issue several `\author` commands; each contributes its (possibly \and-split)
        // entries in order.
        let src = r"\author{Alice}\author{Bob \and Carol}\begin{document}x\end{document}";
        let doc = parse_document(src).expect("parse");
        let names: Vec<String> = doc.metadata.authors.iter().map(|a| inline_text(a)).collect();
        assert_eq!(names, vec!["Alice", "Bob", "Carol"]);
    }

    #[test]
    fn date_captured() {
        let src = r"\documentclass{article}\date{2026}\begin{document}x\end{document}";
        let doc = parse_document(src).expect("parse");
        let date = doc.metadata.date.as_ref().expect("date present");
        assert_eq!(inline_text(date), "2026");
    }

    #[test]
    fn abstract_env_captured_as_blocks() {
        let src = r"\begin{document}\begin{abstract}This is the abstract.\end{abstract}\end{document}";
        let doc = parse_document(src).expect("parse");
        let abs = doc.metadata.abstract_.as_ref().expect("abstract present");
        assert!(
            abs.iter().any(|b| matches!(b, Block::Paragraph(inls, _)
                if inls.iter().any(|i| matches!(i, Inline::Text(t, _) if t.contains("abstract"))))),
            "abstract body contains a paragraph with its text"
        );
        // Additive projection: the abstract environment ALSO stays as a body Block::Environment.
        assert!(
            doc.body.iter().any(|b| matches!(b, Block::Environment { name, .. } if name == "abstract")),
            "the abstract env stays in the body (additive projection)"
        );
    }

    #[test]
    fn title_in_body_captured() {
        // `\title` after `\begin{document}` (before `\maketitle`) is still captured.
        let src = r"\begin{document}\title{Body Title}\maketitle Hi.\end{document}";
        let doc = parse_document(src).expect("parse");
        let title = doc.metadata.title.as_ref().expect("body \\title present");
        assert_eq!(inline_text(title), "Body Title");
        // `\maketitle` is a no-op for metadata (nothing to capture) but is carried through the body.
        assert!(
            doc.body.iter().any(|b| matches!(b, Block::Paragraph(inls, _)
                if inls.iter().any(|i| matches!(i, Inline::Raw(Node { kind: NodeKind::Command { name, .. }, .. }, _)
                    if name == "maketitle")))),
            "\\maketitle is carried through as a Raw inline (no metadata side effect)"
        );
    }

    #[test]
    fn preamble_title_wins_over_body_title() {
        // First \title wins; the preamble is scanned before the body.
        let src = r"\title{Preamble}\begin{document}\title{Body}\maketitle\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(inline_text(doc.metadata.title.as_ref().unwrap()), "Preamble");
    }

    #[test]
    fn no_metadata_is_all_none_empty() {
        let src = r"\documentclass{article}\begin{document}Just body text.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.metadata, Metadata::default(), "no directives → default (all None/empty)");
        assert!(doc.metadata.title.is_none());
        assert!(doc.metadata.authors.is_empty());
        assert!(doc.metadata.date.is_none());
        assert!(doc.metadata.abstract_.is_none());
    }

    #[test]
    fn metadata_round_trip_fixed_point() {
        // Re-parsing `to_latex()` repopulates the SAME metadata (modulo spans) — the projection is
        // a fixed point because the underlying \title/\author/\date/abstract nodes are never removed.
        for src in [
            r"\documentclass{article}\title{Paper}\author{Alice \and Bob}\date{2026}\begin{document}\maketitle\begin{abstract}An abstract.\end{abstract}Body.\end{document}",
            r"\begin{document}\title{Only Title}\maketitle Text.\end{document}",
            r"\documentclass{article}\begin{document}No metadata here.\end{document}",
        ] {
            let doc = parse_document(src).expect("parse");
            let rendered = doc.to_latex();
            let redoc = parse_document(&rendered).expect("re-parse");
            assert_eq!(
                doc.strip_spans().metadata,
                redoc.strip_spans().metadata,
                "metadata is not a round-trip fixed point:\n src = {src:?}\n rendered = {rendered:?}"
            );
            // And the whole document still round-trips modulo spans (to_latex unaffected by D4).
            assert_eq!(
                doc.strip_spans(),
                redoc.strip_spans(),
                "D4 broke the document round-trip:\n src = {src:?}\n rendered = {rendered:?}"
            );
        }
    }

    // -- D5: floats, captions, code & display-math tests --------------------------------------

    #[test]
    fn figure_with_caption_and_label() {
        // A `figure` float with an \includegraphics-ish body, a \caption, and a \label.
        let src = concat!(
            r"\begin{document}",
            r"\begin{figure}\includegraphics{plot.png}\caption{A plot}\label{fig:plot}\end{figure}",
            r"\end{document}",
        );
        let doc = parse_document(src).expect("parse");
        let fig = doc.body.iter().find(|b| matches!(b, Block::Figure { .. })).expect("figure");
        if let Block::Figure { content, caption, label, .. } = fig {
            // Caption text is "A plot"; label is "fig:plot".
            let cap = caption.as_ref().expect("caption present");
            assert_eq!(inline_text(&cap.content), "A plot");
            assert_eq!(label.as_deref(), Some("fig:plot"));
            // The \includegraphics command survives in the body (nothing dropped), and neither the
            // caption nor the label lingers as body content.
            assert!(
                content.iter().any(|b| matches!(b, Block::Paragraph(inls, _)
                    if inls.iter().any(|i| matches!(i, Inline::Raw(Node { kind: NodeKind::Command { name, .. }, .. }, _)
                        if name == "includegraphics")))),
                "the \\includegraphics body is preserved"
            );
            assert!(
                !content.iter().any(|b| matches!(b, Block::Paragraph(inls, _)
                    if inls.iter().any(|i| matches!(i, Inline::Raw(Node { kind: NodeKind::Command { name, .. }, .. }, _)
                        if name == "caption")))),
                "the \\caption marker is lifted out of the body"
            );
        }
    }

    #[test]
    fn table_float_attaches_caption_to_inner_tabular() {
        // A `table` float wrapping a `tabular`, with a \caption and \label — the caption/label
        // attach to the inner Block::Table.
        let src = concat!(
            r"\begin{document}",
            r"\begin{table}\begin{tabular}{lc}a & b \\ c & d\end{tabular}\caption{Grid}\label{tab:g}\end{table}",
            r"\end{document}",
        );
        let doc = parse_document(src).expect("parse");
        let tbl = doc.body.iter().find(|b| matches!(b, Block::Table { .. })).expect("table");
        if let Block::Table { rows, caption, label, .. } = tbl {
            assert_eq!(rows.len(), 2, "two rows survive");
            let cap = caption.as_ref().expect("float caption attached to tabular");
            assert_eq!(inline_text(&cap.content), "Grid");
            assert_eq!(label.as_deref(), Some("tab:g"));
        }
        // A bare tabular (no float) has no caption/label.
        let bare = parse_document(r"\begin{document}\begin{tabular}{c}x\end{tabular}\end{document}")
            .expect("parse");
        assert!(bare.body.iter().any(|b| matches!(b, Block::Table { caption: None, label: None, .. })));
    }

    #[test]
    fn verbatim_becomes_codeblock_with_raw_text() {
        // The verbatim body is captured raw — `{b}`, `$x$`, and the newline are all literal.
        let src = "\\begin{document}\\begin{verbatim}fn main() { $x }\ncode\\end{verbatim}\\end{document}";
        let doc = parse_document(src).expect("parse");
        let code = doc.body.iter().find(|b| matches!(b, Block::CodeBlock { .. })).expect("codeblock");
        if let Block::CodeBlock { verbatim, .. } = code {
            assert_eq!(verbatim, "fn main() { $x }\ncode");
        }
    }

    #[test]
    fn quote_env_becomes_quote_block() {
        let src = r"\begin{document}\begin{quote}A quoted \emph{passage}.\end{quote}\end{document}";
        let doc = parse_document(src).expect("parse");
        let q = doc.body.iter().find(|b| matches!(b, Block::Quote(..))).expect("quote");
        if let Block::Quote(body, _) = q {
            assert!(body.iter().any(|b| matches!(b, Block::Paragraph(inls, _)
                if inls.iter().any(|i| matches!(i, Inline::Emph(..))))));
        }
    }

    #[test]
    fn equation_env_becomes_display_math_with_source() {
        // Named display-math environments keep their inner LaTeX as an unparsed source string.
        let src = r"\begin{document}\begin{equation}E = mc^2\end{equation}\end{document}";
        let doc = parse_document(src).expect("parse");
        let dm = doc.body.iter().find(|b| matches!(b, Block::DisplayMath { .. })).expect("display math");
        if let Block::DisplayMath { source, .. } = dm {
            assert!(source.contains("E = mc^2"), "kept the equation source: {source:?}");
        }
        // `align` (with `\\` and `&`) is also a display-math env.
        let al = parse_document(r"\begin{document}\begin{align}a &= b \\ c &= d\end{align}\end{document}")
            .expect("parse");
        assert!(al.body.iter().any(|b| matches!(b, Block::DisplayMath { .. })));
    }

    #[test]
    fn unknown_env_still_becomes_environment() {
        let src = r"\begin{document}\begin{center}centered\end{center}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert!(doc.body.iter().any(|b| matches!(b, Block::Environment { name, .. } if name == "center")));
    }

    #[test]
    fn d5_round_trip_fixed_point() {
        // A doc with a figure (caption+label), a table float, a verbatim, an equation, and a quote
        // must be a to_latex fixed point (modulo spans).
        for src in [
            concat!(
                r"\begin{document}",
                r"\begin{figure}\includegraphics{p.png}\caption{Cap}\label{fig:p}\end{figure}",
                r"\begin{table}\begin{tabular}{lc}a & b \\ c & d\end{tabular}\caption{T}\label{tab:t}\end{table}",
                r"\begin{quote}Quoted text.\end{quote}",
                r"\begin{equation}E = mc^2\end{equation}",
                r"\end{document}",
            ),
            "\\begin{document}\\begin{verbatim}raw {code} $here\nline two\\end{verbatim}\\end{document}",
            r"\begin{document}\begin{figure}\caption{No graphic}\end{figure}\end{document}",
        ] {
            let doc = parse_document(src).expect("parse");
            let rendered = doc.to_latex();
            let redoc = parse_document(&rendered).expect("re-parse");
            assert_eq!(
                doc.strip_spans(),
                redoc.strip_spans(),
                "D5 round-trip fixed point failed:\n src = {src:?}\n rendered = {rendered:?}"
            );
        }
    }

    // -- D6: provenance API (walk / node_at) --------------------------------------------------

    /// A realistic corpus document exercising every major Block/Inline kind: a titled `article`
    /// with an `abstract`, a `\section`, a `tabular` in a `table` float, an `itemize`, inline `$…$`
    /// and display `equation` math, a `figure` with `\caption`+`\label`, and a `\cite`.
    const CAPSTONE_SRC: &str = concat!(
        r"\documentclass{article}",
        r"\title{On Widgets}\author{Ada \and Bob}\date{2026}",
        r"\begin{document}",
        r"\maketitle",
        r"\begin{abstract}A short abstract about widgets.\end{abstract}",
        r"\section{Introduction}",
        r"We study widgets \cite{smith}. Energy is $E = mc^2$.",
        r"\begin{itemize}\item First point.\item Second point.\end{itemize}",
        r"\begin{table}\begin{tabular}{lc}a & b \\ c & d\end{tabular}\caption{A table}\label{tab:t}\end{table}",
        r"\begin{equation}\int_0^1 x\,dx = \frac{1}{2}\end{equation}",
        r"\begin{figure}\includegraphics{w.png}\caption{A widget}\label{fig:w}\end{figure}",
        r"\end{document}",
    );

    #[test]
    fn walk_visits_section_before_its_child_paragraph() {
        // Pre-order: a Section is yielded before the Paragraph nested in its body.
        let src = r"\begin{document}\section{Intro}Body text here.\end{document}";
        let doc = parse_document(src).expect("parse");
        let kinds: Vec<&str> = doc.walk().map(|n| n.kind()).collect();
        let sec = kinds.iter().position(|k| *k == "Section").expect("a Section");
        let para = kinds.iter().position(|k| *k == "Paragraph").expect("a Paragraph");
        assert!(sec < para, "Section must precede its child Paragraph: {kinds:?}");
    }

    #[test]
    fn walk_visits_figure_before_its_caption_inlines() {
        // A Figure is yielded before the Text of its caption (pre-order, caption before content).
        let src = r"\begin{document}\begin{figure}\includegraphics{w.png}\caption{Widget}\label{fig:w}\end{figure}\end{document}";
        let doc = parse_document(src).expect("parse");
        let nodes: Vec<NodeRef> = doc.walk().collect();
        let fig = nodes.iter().position(|n| n.kind() == "Figure").expect("a Figure");
        // The caption's "Widget" Text appears after the Figure in pre-order.
        let cap_text = nodes.iter().position(|n| {
            matches!(n, NodeRef::Inline(Inline::Text(t, _)) if t.contains("Widget"))
        });
        assert!(cap_text.is_some(), "caption text is walked: {:?}", nodes.iter().map(|n| n.kind()).collect::<Vec<_>>());
        assert!(fig < cap_text.unwrap(), "Figure precedes its caption inline");
    }

    #[test]
    fn node_at_inside_body_returns_innermost_containing_node() {
        let src = r"\begin{document}\section{Intro}Body text here.\end{document}";
        let doc = parse_document(src).expect("parse");
        // Pick a byte inside the body region (after `\begin{document}`), e.g. inside "Body".
        let byte = src.find("Body text").expect("body text present") + 1;
        let prov = doc.node_at(byte).expect("a node owns this byte");
        // The returned span actually contains the byte.
        assert!(
            prov.span.start <= byte && byte < prov.span.end,
            "node_at span {:?} must contain byte {byte}",
            prov.span
        );
        // And it is the *innermost* (narrowest) containing node: no walked node with a strictly
        // narrower span also contains this byte.
        let best_width = prov.span.end.saturating_sub(prov.span.start);
        for n in doc.walk() {
            let s = n.span();
            if s.start <= byte && byte < s.end {
                assert!(
                    s.end.saturating_sub(s.start) >= best_width,
                    "found a narrower containing node {s:?} than node_at returned {:?}",
                    prov.span
                );
            }
        }
    }

    #[test]
    fn node_at_out_of_range_is_none() {
        let src = r"\begin{document}Hi.\end{document}";
        let doc = parse_document(src).expect("parse");
        // A byte past end of source is owned by no node.
        assert!(doc.node_at(src.len() + 100).is_none());
        // usize::MAX never panics (saturating width, no unchecked indexing).
        assert!(doc.node_at(usize::MAX).is_none());
    }

    // -- S4: precise `node_at` — resolution to the true per-token leaf -------------------------
    //
    // With S3's tight body spans, `node_at(byte)` no longer stops at an enclosing region: it
    // resolves to the *narrowest* node whose precise span contains the byte — the genuine
    // per-token leaf. These tests prove that a byte inside a word/title/inner-run resolves to the
    // leaf that owns it (and that the resolved span slices back to exactly that leaf's source), not
    // to the enclosing `Paragraph`/`Section`/composite. This is the region-coarse caveat, retired.

    #[test]
    fn node_at_resolves_to_text_leaf_not_paragraph() {
        // A byte inside the word "widgets" must resolve to the `Text` leaf owning "widgets" — and
        // its span must slice back to EXACTLY "widgets", not to the enclosing paragraph.
        let src = r"\begin{document}We study widgets everywhere\end{document}";
        let doc = parse_document(src).expect("parse");
        let byte = src.find("widgets").expect("widgets present") + 2; // inside the word
        let prov = doc.node_at(byte).expect("a node owns this byte");
        // The resolved node is the `Text` leaf whose text is exactly "widgets".
        assert!(
            matches!(prov.node, NodeRef::Inline(Inline::Text(t, _)) if t == "widgets"),
            "node_at should resolve to the `widgets` Text leaf, got kind {:?}",
            prov.node.kind()
        );
        // And its span slices back to exactly that word — NOT the enclosing paragraph.
        assert_eq!(&src[prov.span.start..prov.span.end], "widgets");
    }

    #[test]
    fn node_at_in_section_title_resolves_to_heading_inline() {
        // A byte inside a `\section` heading's title must resolve to the title *inline* (the Text
        // leaf), NOT to the whole `Section` block that unions the heading + its owned body.
        let src = r"\begin{document}\section{Introduction}Body text here.\end{document}";
        let doc = parse_document(src).expect("parse");
        let byte = src.find("Introduction").expect("title present") + 3; // inside the title word
        let prov = doc.node_at(byte).expect("a node owns this byte");
        assert!(
            matches!(prov.node, NodeRef::Inline(Inline::Text(t, _)) if t == "Introduction"),
            "node_at should resolve to the heading title inline, got kind {:?} span {:?}",
            prov.node.kind(),
            prov.span
        );
        // The resolved span is the tight title word, strictly narrower than the Section span.
        assert_eq!(&src[prov.span.start..prov.span.end], "Introduction");
        let Block::Section { span: sec_span, .. } = &doc.body[0] else { panic!("expected Section") };
        let sec_width = sec_span.end.saturating_sub(sec_span.start);
        let title_width = prov.span.end.saturating_sub(prov.span.start);
        assert!(
            title_width < sec_width,
            "title span {:?} must be strictly narrower than the Section span {sec_span:?}",
            prov.span
        );
    }

    #[test]
    fn node_at_in_textbf_resolves_to_inner_leaf() {
        // A byte inside `\textbf{bold}`'s inner text resolves to the inner `Text` leaf ("bold"),
        // NOT to the whole `\textbf{…}` composite inline.
        let src = r"\begin{document}see \textbf{bold} now\end{document}";
        let doc = parse_document(src).expect("parse");
        let byte = src.find("bold").expect("bold present") + 1; // inside the inner word
        let prov = doc.node_at(byte).expect("a node owns this byte");
        assert!(
            matches!(prov.node, NodeRef::Inline(Inline::Text(t, _)) if t == "bold"),
            "node_at should resolve to the inner `bold` Text leaf, got kind {:?}",
            prov.node.kind()
        );
        assert_eq!(&src[prov.span.start..prov.span.end], "bold");
    }

    #[test]
    fn body_bytes_resolve_to_containing_node() {
        // HONEST BODY BYTE-COVERAGE (S4 scope, NOT the S5 whole-corpus capstone):
        //
        // For a representative multi-node document, every NON-WHITESPACE body byte both (a) resolves
        // (`node_at(byte).is_some()`) AND (b) resolves to a node whose PRECISE span actually
        // contains that byte (`span.start <= byte < span.end`). We assert this on a representative
        // input only — S4 proves leaf-resolution here; the tightest-covering-leaf guarantee over the
        // full LTXDOC01 corpus (no strictly-narrower node exists for any byte) is the S5 capstone.
        let src = concat!(
            r"\begin{document}",
            r"\section{Widgets}",
            r"We study \textbf{widgets} and $E = mc^2$ here.",
            r"\begin{itemize}\item First.\item Second.\end{itemize}",
            r"\end{document}",
        );
        let doc = parse_document(src).expect("parse");

        let begin = r"\begin{document}";
        let end = r"\end{document}";
        let body_start = src.find(begin).expect("begin marker") + begin.len();
        let body_end = src[body_start..].find(end).expect("end marker") + body_start;

        let bytes = src.as_bytes();
        let mut checked = 0usize;
        for (byte, &b) in bytes.iter().enumerate().take(body_end).skip(body_start) {
            if b.is_ascii_whitespace() {
                continue; // whitespace is not required to be owned (honest scope)
            }
            checked += 1;
            let prov = doc
                .node_at(byte)
                .unwrap_or_else(|| panic!("unresolved non-whitespace body byte {byte} = {:?}", b as char));
            // (b): the resolved node's PRECISE span genuinely contains the byte.
            assert!(
                prov.span.start <= byte && byte < prov.span.end,
                "resolved span {:?} does not contain byte {byte} = {:?}",
                prov.span,
                b as char
            );
        }
        assert!(checked > 40, "sanity: the body should have many non-whitespace bytes, got {checked}");
    }

    #[test]
    fn capstone_round_trip_fixed_point() {
        // The full corpus round-trips modulo spans (parse → to_latex → re-parse == original).
        let doc = parse_document(CAPSTONE_SRC).expect("parse capstone");
        let rendered = doc.to_latex();
        let redoc = parse_document(&rendered).expect("re-parse capstone");
        assert_eq!(
            doc.strip_spans(),
            redoc.strip_spans(),
            "capstone round-trip fixed point failed:\n rendered = {rendered:?}"
        );
    }

    #[test]
    fn capstone_walk_is_total_and_nonpanicking() {
        // walk() visits the whole corpus without panicking and yields a non-trivial forest that
        // includes the headline kinds we planted.
        let doc = parse_document(CAPSTONE_SRC).expect("parse capstone");
        let kinds: Vec<&str> = doc.walk().map(|n| n.kind()).collect();
        for expected in ["Section", "Paragraph", "List", "Table", "Figure", "DisplayMath", "Math", "CrossRef"] {
            assert!(
                kinds.contains(&expected),
                "walk() should surface a {expected} node; got {kinds:?}"
            );
        }
        // Every walked span is within the whole-document span (span integrity at the API surface).
        for n in doc.walk() {
            let s = n.span();
            assert!(s.start >= doc.span.start && s.end <= doc.span.end, "walked span {s:?} escapes doc {:?}", doc.span);
        }
    }

    #[test]
    fn capstone_byte_coverage_body_region() {
        // THE CAPSTONE COVERAGE GUARANTEE (S3-precise spans still tile the body):
        //
        //   For every NON-WHITESPACE byte inside the document *body region*
        //   (`\begin{document}` … `\end{document}`), `node_at(byte).is_some()` — i.e. some walked
        //   node owns it.
        //
        // We scope the assertion to the body region (not the preamble) because `walk()`
        // deliberately covers body Blocks + Inlines (the provenance surface the ADJ pipeline
        // consumes); preamble directives (`\documentclass`, `\title`, …) are indexed into
        // `Preamble`/`Metadata`, which are not per-node walked. With S3 the body spans are now the
        // precise per-node ranges (not region-coarse), and they *still* tile every meaningful byte
        // — coverage is preserved under tightening. The S5 rung will strengthen this to
        // leaf-tightness (the covering node is the tightest one, no strictly-narrower node exists).
        let doc = parse_document(CAPSTONE_SRC).expect("parse capstone");

        let begin = r"\begin{document}";
        let end = r"\end{document}";
        let body_start = CAPSTONE_SRC.find(begin).expect("begin marker") + begin.len();
        let body_end = CAPSTONE_SRC[body_start..].find(end).expect("end marker") + body_start;

        let bytes = CAPSTONE_SRC.as_bytes();
        let mut checked = 0usize;
        for byte in body_start..body_end {
            if bytes[byte].is_ascii_whitespace() {
                continue; // whitespace is not required to be owned (honest scope)
            }
            checked += 1;
            assert!(
                doc.node_at(byte).is_some(),
                "uncovered non-whitespace body byte {byte} = {:?} in {:?}",
                bytes[byte] as char,
                &CAPSTONE_SRC[byte..(byte + 8).min(body_end)]
            );
        }
        assert!(checked > 50, "sanity: the body region should have many non-whitespace bytes, got {checked}");
    }

    // -- S3: precise Document-fold spans ------------------------------------------------------
    //
    // Every BODY Block/Inline that has a real underlying node span now slices back to *exactly*
    // that node's source substring, and composites carry the union of their children's real spans.
    // These tests take a known source, build the Document, and assert `&src[node.span]` == the
    // exact expected substring (the S3 tightness the D6 caveat used to disclaim).

    /// The source substring a span points at.
    fn slice(src: &str, span: Span) -> &str {
        &src[span.start..span.end]
    }

    #[test]
    fn s3_section_title_inline_slices_to_exact_source() {
        // The heading title's `Text` inline slices back to exactly "Introduction", not the region.
        let src = r"\begin{document}\section{Introduction}Body text here.\end{document}";
        let doc = parse_document(src).expect("parse");
        let Block::Section { title, span: sec_span, .. } = &doc.body[0] else {
            panic!("expected a Section");
        };
        // The title's Text inline is tight.
        let t = title
            .iter()
            .find(|i| matches!(i, Inline::Text(txt, _) if txt == "Introduction"))
            .expect("title Text");
        assert_eq!(slice(src, inline_span(t)), "Introduction");
        // The Section's own span is the union of the heading node and its owned children — it
        // covers from the `\section` command to the end of the last owned block ("here.").
        let sec_src = slice(src, *sec_span);
        assert!(sec_src.starts_with(r"\section{Introduction}"), "section span starts at heading: {sec_src:?}");
        assert!(sec_src.ends_with("Body text here."), "section span ends at last owned content: {sec_src:?}");
    }

    #[test]
    fn s3_paragraph_text_run_slices_to_exact_source() {
        // A `Text` run inside a paragraph slices to exactly its source word(s), and the Paragraph
        // span is the union of its inlines' spans (min start .. max end).
        let src = r"\begin{document}widgets everywhere\end{document}";
        let doc = parse_document(src).expect("parse");
        let Block::Paragraph(inls, para_span) = &doc.body[0] else {
            panic!("expected a Paragraph");
        };
        // The paragraph span is the union of its inline spans.
        let union_span = inls.iter().map(inline_span).reduce(union).expect("non-empty");
        assert_eq!(*para_span, union_span, "paragraph span == union of inline spans");
        assert_eq!(slice(src, *para_span), "widgets everywhere", "paragraph slices to its content");
        // A specific Text run slices to exactly its word.
        let w = inls
            .iter()
            .find(|i| matches!(i, Inline::Text(t, _) if t == "widgets"))
            .expect("widgets Text");
        assert_eq!(slice(src, inline_span(w)), "widgets");
    }

    #[test]
    fn s3_strong_inline_slices_to_whole_construct() {
        // A composite inline (`\textbf{bold}`) slices to the whole `\textbf{…}` construct, and its
        // inner `Text` slices to just "bold" — proving the composite ⊇ its precise child.
        let src = r"\begin{document}see \textbf{bold} now\end{document}";
        let doc = parse_document(src).expect("parse");
        let Block::Paragraph(inls, _) = &doc.body[0] else { panic!("expected Paragraph") };
        let strong = inls.iter().find(|i| matches!(i, Inline::Strong(..))).expect("Strong");
        assert_eq!(slice(src, inline_span(strong)), r"\textbf{bold}");
        if let Inline::Strong(children, _) = strong {
            let inner = children.iter().find(|i| matches!(i, Inline::Text(t, _) if t == "bold")).unwrap();
            assert_eq!(slice(src, inline_span(inner)), "bold");
        }
    }

    #[test]
    fn s3_figure_caption_slices_to_exact_source() {
        // A figure's caption content slices to exactly the caption text.
        let src = r"\begin{document}\begin{figure}\includegraphics{w.png}\caption{A widget}\label{fig:w}\end{figure}\end{document}";
        let doc = parse_document(src).expect("parse");
        let fig = doc.body.iter().find(|b| matches!(b, Block::Figure { .. })).expect("figure");
        let Block::Figure { caption, span: fig_span, .. } = fig else { unreachable!() };
        let cap = caption.as_ref().expect("caption");
        assert_eq!(slice(src, cap.span), "A widget", "caption span slices to its content");
        // The figure's own span covers the whole `\begin{figure}…\end{figure}` float.
        let fig_src = slice(src, *fig_span);
        assert!(fig_src.starts_with(r"\begin{figure}"), "figure span starts at \\begin: {fig_src:?}");
        assert!(fig_src.ends_with(r"\end{figure}"), "figure span ends at \\end: {fig_src:?}");
    }

    #[test]
    fn s3_table_cell_slices_to_exact_source() {
        // A tabular cell's `Text` slices to exactly that cell's source token.
        let src = r"\begin{document}\begin{tabular}{lc}alpha & beta \\ gamma & delta\end{tabular}\end{document}";
        let doc = parse_document(src).expect("parse");
        let tbl = doc.body.iter().find(|b| matches!(b, Block::Table { .. })).expect("table");
        let Block::Table { rows, span: tbl_span, .. } = tbl else { unreachable!() };
        // Find the "gamma" cell's Text run.
        let mut found = false;
        for row in rows {
            for cell in row {
                for block in cell {
                    if let Block::Paragraph(inls, _) = block {
                        if let Some(i) = inls.iter().find(|i| matches!(i, Inline::Text(t, _) if t == "gamma")) {
                            assert_eq!(slice(src, inline_span(i)), "gamma");
                            found = true;
                        }
                    }
                }
            }
        }
        assert!(found, "the 'gamma' cell Text run was located");
        // The table's own span covers the `\begin{tabular}…\end{tabular}` extent.
        let tbl_src = slice(src, *tbl_span);
        assert!(tbl_src.starts_with(r"\begin{tabular}"), "table span starts at \\begin{{tabular}}: {tbl_src:?}");
        assert!(tbl_src.ends_with(r"\end{tabular}"), "table span ends at \\end{{tabular}}: {tbl_src:?}");
    }

    #[test]
    fn s3_list_item_span_is_union_of_its_content() {
        // A list item's span is the union (min start .. max end) of its content blocks' real spans,
        // and slices back to the item's source extent.
        let src = r"\begin{document}\begin{itemize}\item first point\item second point\end{itemize}\end{document}";
        let doc = parse_document(src).expect("parse");
        let list = doc.body.iter().find(|b| matches!(b, Block::List { .. })).expect("list");
        let Block::List { items, span: list_span, .. } = list else { unreachable!() };
        for it in items {
            let body_union = it.body.iter().map(block_span).reduce(union).expect("item has body");
            assert_eq!(it.span, body_union, "item span == union of its body block spans");
            // The item slices back to a substring that contains its body text ("first"/"second").
            let s = slice(src, it.span);
            assert!(s.contains("point"), "item slices to its content: {s:?}");
        }
        // The list's own span covers the `\begin{itemize}…\end{itemize}` extent.
        let list_src = slice(src, *list_span);
        assert!(list_src.starts_with(r"\begin{itemize}"), "list span starts at \\begin: {list_src:?}");
        assert!(list_src.ends_with(r"\end{itemize}"), "list span ends at \\end: {list_src:?}");
    }

    #[test]
    fn s3_body_leaf_spans_are_node_source_range_not_region() {
        // The DEFINITIVE S3 tightness assertion: over a rich body, every leaf Text/Math/CrossRef
        // inline slices back to EXACTLY its own source substring (== node source range), NOT a
        // shared enclosing region. Before S3 many of these shared one coarse region span and this
        // slice would have returned the whole region.
        let src = concat!(
            r"\begin{document}",
            r"\section{Intro}",
            r"We study \textbf{widgets} and $E=mc^2$ \cite{smith} here.",
            r"\end{document}",
        );
        let doc = parse_document(src).expect("parse");
        // Collect leaf inlines and check each slices to its own source.
        let mut checked_text = false;
        let mut checked_math = false;
        let mut checked_ref = false;
        for n in doc.walk() {
            if let NodeRef::Inline(i) = n {
                match i {
                    Inline::Text(t, sp) if t == "widgets" => {
                        // "widgets" appears once, inside \textbf{…}; its span is exactly the word.
                        assert_eq!(slice(src, *sp), "widgets");
                        checked_text = true;
                    }
                    Inline::Math { source, span } if source.contains("E=mc^2") => {
                        assert_eq!(slice(src, *span), "$E=mc^2$", "inline math slices to the whole island");
                        checked_math = true;
                    }
                    Inline::CrossRef { command, span, .. } if command == "cite" => {
                        assert_eq!(slice(src, *span), r"\cite{smith}", "cite slices to the whole command");
                        checked_ref = true;
                    }
                    _ => {}
                }
            }
        }
        assert!(checked_text && checked_math && checked_ref, "all three leaf kinds were located and checked");
    }

    #[test]
    fn s3_captioned_table_float_span_covers_caption_bytes() {
        // A `table` float attaches its caption/label to the inner tabular; the resulting Table's
        // span is the union of the tabular extent and the float extent, so it covers the caption.
        let src = concat!(
            r"\begin{document}",
            r"\begin{table}\begin{tabular}{lc}a & b \\ c & d\end{tabular}\caption{Grid}\label{tab:g}\end{table}",
            r"\end{document}",
        );
        let doc = parse_document(src).expect("parse");
        let tbl = doc.body.iter().find(|b| matches!(b, Block::Table { caption: Some(_), .. })).expect("captioned table");
        let Block::Table { span, caption, .. } = tbl else { unreachable!() };
        let tbl_src = slice(src, *span);
        // The span covers the whole `\begin{table}…\end{table}` float (so it owns the caption bytes).
        assert!(tbl_src.starts_with(r"\begin{table}"), "captioned table span starts at the float: {tbl_src:?}");
        assert!(tbl_src.ends_with(r"\end{table}"), "captioned table span ends at the float: {tbl_src:?}");
        // The caption's own span is tight — exactly "Grid".
        assert_eq!(slice(src, caption.as_ref().unwrap().span), "Grid");
    }

    #[test]
    fn s3_containment_tightens_to_exact_for_leaf_text() {
        // The D2-D5 tests asserted child ⊆ region. S3 tightens: a leaf Text's span is EXACTLY its
        // own source characters (`&src[span] == t`), and strictly tighter than the enclosing body
        // region — the defining S3 property that the D6 caveat used to disclaim.
        let src = r"\begin{document}\section{Head}alpha beta gamma\end{document}";
        let doc = parse_document(src).expect("parse");
        // The whole body region string, for contrast.
        let body_region = {
            let b = src.find(r"\begin{document}").unwrap() + r"\begin{document}".len();
            let e = src.find(r"\end{document}").unwrap();
            &src[b..e]
        };
        // Every Text leaf slices to EXACTLY its own string content, and is tighter than the region.
        let mut saw_leaf = false;
        for n in doc.walk() {
            if let NodeRef::Inline(Inline::Text(t, sp)) = n {
                assert_eq!(slice(src, *sp), t, "Text leaf slices to exactly its own content");
                assert!(
                    (sp.end - sp.start) < body_region.len(),
                    "Text leaf span {sp:?} must be strictly tighter than the region"
                );
                saw_leaf = true;
            }
        }
        assert!(saw_leaf, "the document has at least one Text leaf");
    }
}
