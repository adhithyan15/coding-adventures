//! Cross-reference **resolution** (LTXDOC03 S1) — binding each `\ref` to the `\label` that
//! defines it, with precise byte spans on **both** sides.
//!
//! ## Where this sits, and what it mimics
//!
//! LaTeX resolves cross-references in **two passes** through an auxiliary `.aux` file. On the first
//! `latex` run, every `\label{key}` writes a `\newlabel{key}{…}` line into `document.aux` (recording
//! the number/page the label points at); every `\ref{key}` / `\pageref{key}` is left as a
//! placeholder. On the *second* run, LaTeX reads `document.aux` back **before** typesetting, so by
//! the time it meets a `\ref{key}` the label table is already populated and the reference can be
//! filled in. A key that never got a `\newlabel` prints the tell-tale `??` and warns
//! *"Reference `key' on page … undefined"*; a key that got two `\newlabel`s warns *"Label `key'
//! multiply defined"*.
//!
//! This module is the **static, single-pass analogue** of that machinery over an already-parsed
//! [`Document`]. We do not run LaTeX and we do not compute numbers or pages — we bind *structure*:
//! for every reference we answer "**which defining node**, at **which source bytes**, does this
//! `\ref` point at?". That is exactly the correlation the ADJ byte-provenance north star needs: a
//! resolved `\ref` now carries the exact source-byte range of the node it names.
//!
//! ## The model in one glance
//!
//! ```text
//!   \section{Intro}\label{sec:intro}   ── defines ──▶  sec:intro  (Section,  span = the heading)
//!   … see Section~\ref{sec:intro} …    ── uses  ───▶  resolves to sec:intro's Section span
//!   … and \ref{nope} …                 ── uses  ───▶  UNRESOLVED (dangling: no such label)
//! ```
//!
//! Two definition **sources** feed the label table:
//!
//! 1. **Hoisted float/section labels.** LTXDOC01's D3/D5 folds *hoist* a trailing `\label{key}` off
//!    a `\section`/`table`/`figure` into that [`Block`]'s `label: Option<String>` field. When
//!    present, the defining node is that block and its span is the block's span.
//! 2. **Inline `\label{key}`.** Any `\label` that was *not* hoisted survives as an
//!    [`Inline::CrossRef`] with `command == "label"` (e.g. a `\label` in the middle of a paragraph,
//!    or after an `equation`). Its key is the `target`, its span the cross-ref's own span.
//!
//! ## The rules S1 implements
//!
//! - **First definition wins.** If a key is defined more than once, the **first** definition (in
//!   pre-order [`Document::walk`] order) is the one references resolve against — mirroring how a
//!   consumer must pick *some* binding, and matching that LaTeX's `\ref` prints *a* number even when
//!   a label is multiply defined. Every later definition of that key is recorded as a
//!   [`Duplicate`] so the caller can surface the "multiply defined" warning — nothing is dropped and
//!   nothing panics.
//! - **Only the *reference* family resolves.** The commands that ask "what number/page is this?" —
//!   [`REF_COMMANDS`] = `{"ref", "eqref", "pageref"}` — are the ones we resolve. A found key →
//!   [`ResolvedRef`] (ref-span **and** the def-span + kind); a missing key → [`UnresolvedRef`] (the
//!   dangling key + the ref-span).
//! - **`\cite` is a *separate* table (S2, below).** A citation resolves against a **bibliography**
//!   (`.bib` / `thebibliography`), an entirely separate table from the `\label` one. The *label*
//!   pass ([`Document::resolve_references`]) therefore treats `\cite` as **neither** a resolvable
//!   reference **nor** a dangling one — it is simply not a `\label`-table reference. (`\label` is
//!   likewise not a *reference*: it *defines*, it does not *use*.) Binding `\cite` to its
//!   bibliography entry is the job of the parallel **S2** pass, [`Document::resolve_citations`],
//!   documented in its own section further down.
//!
//! ## Totality, borrowing, and bounds
//!
//! [`Document::resolve_references`] is **total**: it never errors and never panics (no
//! `unwrap`/`expect`, no unchecked indexing; all data is plain owned/`Copy` values). It borrows the
//! document immutably and reuses the existing bounded [`Document::walk`] traversal (whose depth is
//! capped upstream by the parser's `MAX_DEPTH`), so it introduces **no** new recursion. The returned
//! [`ReferenceResolution`] holds `Clone`-able plain data: [`Span`]s are `Copy`, and every key is a
//! `String` copied out of the document (owned, so the resolution outlives any borrow of the source).
//!
//! This is pure *analysis* over the tree — it changes nothing about the parser, the fold,
//! [`Document::walk`], [`Document::node_at`], any span, or the `to_latex` round-trip fixed point.

use crate::ast::{Node, NodeKind};
use crate::document::{Block, Document, Inline, NodeRef};
use crate::document_to_latex;
use crate::token::Span;

// -------------------------------------------------------------------------------------------------
// The reference command family (what S1 resolves).
// -------------------------------------------------------------------------------------------------

/// The **reference-family** control words S1 resolves against the label table: `\ref`, `\eqref`
/// (amsmath's parenthesised equation reference), and `\pageref`. Each *uses* a label — it asks "what
/// number / page does `key` have?" — so it resolves against the `\label` table.
///
/// Deliberately **excluded**:
/// - `"label"` — a `\label` *defines* a key, it does not *use* one, so it is a definition source
///   (see [`collect_definitions`]), never a reference.
/// - `"cite"` — a `\cite` resolves against a **bibliography**, a separate table entirely (see the
///   module docs); binding it is a later rung, so S1 treats it as out of scope (neither resolved nor
///   reported unresolved by this pass).
pub const REF_COMMANDS: [&str; 3] = ["ref", "eqref", "pageref"];

/// Is `command` one of the [`REF_COMMANDS`] the ref pass resolves? A tiny, allocation-free predicate
/// used at both the reference-collection and the exclusion-test sites.
fn is_ref_command(command: &str) -> bool {
    REF_COMMANDS.contains(&command)
}

// -------------------------------------------------------------------------------------------------
// What a label defines: the owner "kind" tag.
// -------------------------------------------------------------------------------------------------

/// What kind of document node a label **defines** — the small, inspectable owner tag recorded on
/// every [`LabelDef`]. It names *what* the resolved `\ref` points at (a section? a figure? a bare
/// inline `\label`?) so a consumer can render "Section 3" vs "Figure 2" appropriately, and so a test
/// can assert the resolved target is the right *kind* of node.
///
/// | variant | source | example |
/// |---------|--------|---------|
/// | [`Section`](LabelKind::Section) | a `\section`…`\label` hoisted onto [`Block::Section`] | `\ref{sec:intro}` → a heading |
/// | [`Table`](LabelKind::Table)     | a `table` float's `\label` hoisted onto [`Block::Table`] | `\ref{tab:data}` → a table |
/// | [`Figure`](LabelKind::Figure)   | a `figure` float's `\label` hoisted onto [`Block::Figure`] | `\ref{fig:plot}` → a figure |
/// | [`Inline`](LabelKind::Inline)   | a non-hoisted inline `\label{…}` ([`Inline::CrossRef`]) | `\eqref{eq:x}` → an equation's label |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelKind {
    /// A `\label` hoisted onto a [`Block::Section`] heading.
    Section,
    /// A `\label` hoisted onto a [`Block::Table`] float.
    Table,
    /// A `\label` hoisted onto a [`Block::Figure`] float.
    Figure,
    /// A bare inline `\label{…}` that was not hoisted onto any float/section — an
    /// [`Inline::CrossRef`] with `command == "label"` (e.g. a label after an `equation`).
    Inline,
}

impl LabelKind {
    /// A stable, human-readable name for this owner kind (`"section"`, `"table"`, `"figure"`,
    /// `"inline"`), for structure queries and test assertions that want to name what a label defines
    /// without matching the enum.
    pub fn as_str(&self) -> &'static str {
        match self {
            LabelKind::Section => "section",
            LabelKind::Table => "table",
            LabelKind::Figure => "figure",
            LabelKind::Inline => "inline",
        }
    }
}

// -------------------------------------------------------------------------------------------------
// The record types (plain, Clone-able data).
// -------------------------------------------------------------------------------------------------

/// One **label definition**: a `key` bound to the [`Span`] of the node that defines it, tagged with
/// the [`LabelKind`] of that node. This is one row of the label table — the static analogue of a
/// `.aux` `\newlabel{key}{…}` line, but recording the defining node's **source bytes** rather than
/// its number/page.
///
/// `&src[def.span]` slices back to the defining node's source: the `\section`…owned-body extent for a
/// section label, the `\begin{figure}…\end{figure}` extent for a figure label, or the `\label{key}`
/// construct itself for a bare inline label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelDef {
    /// The label key, verbatim, without braces (`"sec:intro"`).
    pub key: String,
    /// What kind of node this label defines.
    pub kind: LabelKind,
    /// The defining node's precise source [`Span`] (LTXDOC02): the block's span for a hoisted label,
    /// the cross-ref's span for an inline `\label`.
    pub span: Span,
}

/// A **duplicate** label definition: a `key` that was *already* defined earlier, recorded so the
/// caller can surface LaTeX's *"Label `key' multiply defined"* warning. The `span` is **this**
/// (later, losing) definition's span; the winning first definition is the one in
/// [`ReferenceResolution::definitions`] with the same key (and is what references resolve against).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Duplicate {
    /// The multiply-defined key.
    pub key: String,
    /// This later definition's kind.
    pub kind: LabelKind,
    /// This later (losing) definition's span. The first definition wins for resolution.
    pub span: Span,
}

/// A **resolved** reference: a `\ref`/`\eqref`/`\pageref` whose `key` was found in the label table.
/// Carries the byte spans of **both** ends of the binding — the reference's own span *and* the
/// defining node's span (plus that node's kind) — which is precisely the source→source correlation
/// the ADJ byte-provenance pipeline audits against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRef {
    /// The referenced key (`"sec:intro"`).
    pub key: String,
    /// The reference command that was used (`"ref"`, `"eqref"`, or `"pageref"`).
    pub command: String,
    /// The reference construct's own span: `&src[ref_span]` slices back to exactly the
    /// `\ref{key}` / `\eqref{key}` / `\pageref{key}` source.
    pub ref_span: Span,
    /// The **winning** (first) definition's span: `&src[target_span]` slices back to the defining
    /// node's source.
    pub target_span: Span,
    /// The kind of node the reference resolved to.
    pub target_kind: LabelKind,
}

/// An **unresolved** (dangling) reference: a `\ref`/`\eqref`/`\pageref` whose `key` matched **no**
/// label — LaTeX's *"Reference `key' undefined"*, the `??` in the output. We record the dangling key
/// and the reference's own span so a caller can point at the exact offending source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedRef {
    /// The dangling key that no `\label` defines (`"nope"`).
    pub key: String,
    /// The reference command that was used (`"ref"`, `"eqref"`, or `"pageref"`).
    pub command: String,
    /// The reference construct's own span: `&src[ref_span]` slices back to the dangling
    /// `\ref{key}` source.
    pub ref_span: Span,
}

/// The full result of [`Document::resolve_references`]: the four tables a consumer enumerates — the
/// winning label definitions, the duplicate (losing) definitions, the resolved references, and the
/// unresolved (dangling) references. All plain, `Clone`-able data (spans are `Copy`; keys are owned
/// `String`s), so the resolution outlives any borrow of the source and can be stored/serialized.
///
/// **Ordering.** `definitions` and `duplicates` are in [`Document::walk`] pre-order (so "first
/// definition wins" is a stable, source-order rule); `resolved` and `unresolved` are in the pre-order
/// the references appear in the body.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReferenceResolution {
    /// The **winning** label definitions — one row per distinct key (the first definition seen), in
    /// pre-order. This is the label table references resolve against.
    pub definitions: Vec<LabelDef>,
    /// The **duplicate** (later, losing) definitions of already-defined keys, in pre-order — the
    /// *"multiply defined"* warnings.
    pub duplicates: Vec<Duplicate>,
    /// The references that **resolved** to a definition, in pre-order.
    pub resolved: Vec<ResolvedRef>,
    /// The references that did **not** resolve (dangling `\ref`s), in pre-order — the
    /// *"undefined"* warnings.
    pub unresolved: Vec<UnresolvedRef>,
}

impl ReferenceResolution {
    /// Look up the **winning** definition of `key` (the first one seen), if any. Linear over
    /// `definitions`, which is small (one row per distinct label); no allocation.
    pub fn definition(&self, key: &str) -> Option<&LabelDef> {
        self.definitions.iter().find(|d| d.key == key)
    }
}

// -------------------------------------------------------------------------------------------------
// The resolution pass.
// -------------------------------------------------------------------------------------------------

impl Document {
    /// Resolve every cross-reference in this document against its label table (LTXDOC03 S1).
    ///
    /// Two linear passes over the existing [`Document::walk`] traversal:
    ///
    /// 1. **Collect definitions** — every hoisted section/table/figure `\label` and every inline
    ///    `\label{…}`, in pre-order. The first definition of each key wins; later ones become
    ///    [`Duplicate`]s. (See [`collect_definitions`].)
    /// 2. **Resolve references** — every [`REF_COMMANDS`] cross-ref (`\ref`/`\eqref`/`\pageref`) is
    ///    looked up in the table: found → [`ResolvedRef`] (ref-span **and** def-span + kind), missing
    ///    → [`UnresolvedRef`]. `\cite` and `\label` are excluded (see the module docs).
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing; reuses the bounded
    /// `walk()` (no new recursion). Borrows `self` immutably; the returned [`ReferenceResolution`] is
    /// owned plain data (keys copied out, spans `Copy`), so it outlives any borrow of the source.
    pub fn resolve_references(&self) -> ReferenceResolution {
        let (definitions, duplicates) = collect_definitions(self);
        let (resolved, unresolved) = resolve_refs(self, &definitions);
        ReferenceResolution { definitions, duplicates, resolved, unresolved }
    }
}

/// Pass 1 — collect the label table (winning definitions) and the duplicate list.
///
/// Walks the body in pre-order. For each node we ask "does this *define* a label?":
///
/// - a [`Block::Section`]/[`Block::Table`]/[`Block::Figure`] whose `label` is `Some(key)` → a hoisted
///   definition, tagged [`LabelKind::Section`]/`Table`/`Figure`, span = the block's span;
/// - an [`Inline::CrossRef`] with `command == "label"` → an inline definition, tagged
///   [`LabelKind::Inline`], span = the cross-ref's span.
///
/// The **first** definition of a key is pushed to `definitions` (the winner); any subsequent
/// definition of an already-seen key is pushed to `duplicates` instead. We track "seen" by scanning
/// `definitions` (small: one row per distinct key) — no map/allocation beyond the two output vecs.
fn collect_definitions(doc: &Document) -> (Vec<LabelDef>, Vec<Duplicate>) {
    let mut definitions: Vec<LabelDef> = Vec::new();
    let mut duplicates: Vec<Duplicate> = Vec::new();

    // Push one candidate definition, routing it to `definitions` (first-wins) or `duplicates`.
    let mut record = |key: String, kind: LabelKind, span: Span| {
        if definitions.iter().any(|d| d.key == key) {
            duplicates.push(Duplicate { key, kind, span });
        } else {
            definitions.push(LabelDef { key, kind, span });
        }
    };

    for node in doc.walk() {
        match node {
            NodeRef::Block(block) => {
                if let Some((key, kind, span)) = block_label(block) {
                    record(key, kind, span);
                }
            }
            NodeRef::Inline(Inline::CrossRef { command, target, span, .. })
                if command == "label" =>
            {
                record(target.clone(), LabelKind::Inline, *span);
            }
            // Any other inline (including a `\ref`/`\cite`) defines nothing.
            NodeRef::Inline(_) => {}
        }
    }

    (definitions, duplicates)
}

/// If `block` carries a hoisted `\label`, return `(key, kind, block-span)`; otherwise `None`.
///
/// Only the three float/section blocks LTXDOC01 hoists labels onto are definition sources; every
/// other block defines nothing (its inline `\label`s, if any, are collected on the inline side).
fn block_label(block: &Block) -> Option<(String, LabelKind, Span)> {
    match block {
        Block::Section { label: Some(key), span, .. } => {
            Some((key.clone(), LabelKind::Section, *span))
        }
        Block::Table { label: Some(key), span, .. } => {
            Some((key.clone(), LabelKind::Table, *span))
        }
        Block::Figure { label: Some(key), span, .. } => {
            Some((key.clone(), LabelKind::Figure, *span))
        }
        _ => None,
    }
}

/// Pass 2 — resolve every reference-family cross-ref against the collected `definitions`.
///
/// Walks the body again; for each [`Inline::CrossRef`] whose `command` is a [`REF_COMMANDS`] member
/// (`\ref`/`\eqref`/`\pageref`), we look its `target` up in `definitions`:
///
/// - found → a [`ResolvedRef`] recording the reference's own span **and** the winning definition's
///   span + kind;
/// - missing → an [`UnresolvedRef`] recording the dangling key and the reference's span.
///
/// `\label` and `\cite` cross-refs are skipped entirely (a `\label` defines rather than uses; a
/// `\cite` resolves against a bibliography, deferred to a later rung — see the module docs).
fn resolve_refs(
    doc: &Document,
    definitions: &[LabelDef],
) -> (Vec<ResolvedRef>, Vec<UnresolvedRef>) {
    let mut resolved: Vec<ResolvedRef> = Vec::new();
    let mut unresolved: Vec<UnresolvedRef> = Vec::new();

    for node in doc.walk() {
        let NodeRef::Inline(Inline::CrossRef { command, target, span, .. }) = node else {
            continue;
        };
        if !is_ref_command(command) {
            continue; // `\label` (defines) and `\cite` (bibliography — deferred) are not ref-family.
        }
        // First-def-wins lookup: the earliest definition with this key.
        match definitions.iter().find(|d| &d.key == target) {
            Some(def) => resolved.push(ResolvedRef {
                key: target.clone(),
                command: command.clone(),
                ref_span: *span,
                target_span: def.span,
                target_kind: def.kind,
            }),
            None => unresolved.push(UnresolvedRef {
                key: target.clone(),
                command: command.clone(),
                ref_span: *span,
            }),
        }
    }

    (resolved, unresolved)
}

// =================================================================================================
// LTXDOC03 S2 — `\cite` → bibliography binding.
//
// S1 (above) bound `\ref`/`\eqref`/`\pageref` to `\label`. S2 is the *parallel* pass for the
// *other* cross-reference family: `\cite`, which resolves against a **bibliography** rather than the
// label table. The two passes never interfere — they read disjoint command families and produce
// disjoint result types — so a document can carry both `\ref`/`\label` and `\cite`/`\bibitem` and
// each pass sees only its own.
// =================================================================================================

// -------------------------------------------------------------------------------------------------
// The bibliography model S2 mimics.
// -------------------------------------------------------------------------------------------------

// ## What a bibliography *is*, and how S2 binds a `\cite` to it
//
// LaTeX has **two** cross-reference tables, populated and consulted independently:
//
// | family | *defines* | *uses* | table |
// |--------|-----------|--------|-------|
// | labels (S1) | `\label{k}` | `\ref{k}`, `\eqref{k}`, `\pageref{k}` | the `.aux` `\newlabel` table |
// | citations (S2) | `\bibitem{k}` | `\cite{k}` (also `\cite[note]{k}`, `\cite{a,b,c}`) | the bibliography |
//
// A bibliography lives in a `thebibliography` environment — either hand-written or, more usually,
// generated by BibTeX into a `.bbl` file that `\bibliography{db}` `\input`s. Either way the shape is
// the same: a `\begin{thebibliography}{widest-label}` … `\end{thebibliography}` block whose entries
// are `\bibitem{key} Author. Title. Year.` lines. Each `\bibitem{key}` *defines* one citation key;
// each `\cite{key}` in the body *uses* one (or several). Like labels, resolution is a two-pass
// `.aux` dance in real LaTeX: `\bibcite{key}{number}` lines record the assigned number, and a
// `\cite{key}` with no matching `\bibitem` prints the tell-tale `[?]` and warns *"Citation `key'
// undefined"*.
//
// What S2 does (and does not). Mirroring S1, S2 is the *static, single-pass, in-document* analogue:
// it binds each `\cite` **key** to the `\bibitem` that defines it, recording **both** source spans —
// never computing a citation *number* or *sort order* (that is BibTeX/`.bbl` territory). Three
// surface facts about `\cite` drive the design, each confirmed against the parsed AST:
//
// 1. Multi-key. `\cite{a,b,c}` is **one** `Inline::CrossRef` with `target == "a,b,c"` — a single
//    comma-joined string. S2 splits `target` on `,`, trims each key, and resolves every key
//    *independently*, so `\cite{a,b,c}` yields up to three citation bindings that all point back to
//    the same `\cite` construct (they share one `cite_span`). Empty keys (from `\cite{a,,b}` or a
//    trailing comma) are skipped — never a panic, never a phantom binding.
// 2. Optional note. `\cite[p. 3]{key}` keeps the `[p. 3]` in the cross-ref's separate `note` field;
//    `target` is just `"key"`. S2 reads only `target`, so the note never contaminates the key.
// 3. First entry wins. A key defined by two `\bibitem`s (a real authoring mistake — LaTeX warns
//    *"Citation `key' multiply defined"*) resolves against the **first** `\bibitem`, exactly like
//    S1's first-`\label`-wins rule; the later one is recorded as a `DuplicateBib`.
//
// Out of scope for S2 (honest boundary). Only an **in-document** `thebibliography` / `\bibitem`
// bibliography is bound. An *external* BibTeX database (`\bibliography{refs}` reading a `.bib` file,
// or an un-`\input`-ed `.bbl`) is **not** read — S2 does no file I/O and parses no BibTeX. A `\cite`
// whose key lives only in an external `.bib` is therefore reported *unresolved* here (its `\bibitem`
// is not in this document's tree). Citation **numbering**/sorting is likewise out of scope. Both are
// the same honest boundary S1 drew around `.aux` numbers.
//
// The working types below are `BibEntry`, `DuplicateBib`, `ResolvedCite`, `UnresolvedCite`, and the
// `CitationResolution` aggregate.

/// The control word S2 resolves against the bibliography: `\cite`. Kept as a named constant (mirroring
/// S1's [`REF_COMMANDS`]) so the one string that means "this cross-ref is a citation" lives in one
/// place. `\citep`/`\citet`/`\citeauthor` (natbib) are **not** folded to `Inline::CrossRef` by the
/// current `recognize_structure` pass, so S2 binds plain `\cite` only; widening the family is a
/// follow-on rung.
pub const CITE_COMMAND: &str = "cite";

// -------------------------------------------------------------------------------------------------
// The record types (plain, Clone-able data — parallel to S1's LabelDef/Duplicate/ResolvedRef/…).
// -------------------------------------------------------------------------------------------------

/// One **bibliography entry**: a `\bibitem{key}` binding its `key` to the [`Span`] of the
/// `\bibitem{key}` construct. This is one row of the bibliography table — the static analogue of a
/// `.aux` `\bibcite{key}{number}` line, but recording the defining `\bibitem`'s **source bytes**
/// rather than its assigned number.
///
/// **Span choice.** The `span` is the `\bibitem{key}` command's *own* tight span — `&src[entry.span]`
/// slices back to exactly `\bibitem{key}`, **not** the trailing descriptive prose (author/title/year).
/// That prose parses as ordinary sibling `Text`/`Space` inlines with no delimiter marking where one
/// entry's text ends and the next begins, so the `\bibitem{key}` command is the tightest source
/// range we can *honestly* attribute to the entry — the same "tight, defensible span" discipline S1
/// uses for an inline `\label`. (Attributing the prose would require guessing entry boundaries; we
/// do not guess.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BibEntry {
    /// The citation key, verbatim, without braces (`"smith2020"`).
    pub key: String,
    /// The `\bibitem{key}` construct's precise source [`Span`]: `&src[span]` slices back to exactly
    /// `\bibitem{key}`.
    pub span: Span,
}

/// A **duplicate** bibliography entry: a `key` that was *already* defined by an earlier `\bibitem`,
/// recorded so the caller can surface LaTeX's *"Citation `key' multiply defined"* warning. The `span`
/// is **this** (later, losing) `\bibitem`'s span; the winning first entry is the one in
/// [`CitationResolution::entries`] with the same key (and is what citations resolve against).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateBib {
    /// The multiply-defined citation key.
    pub key: String,
    /// This later (losing) `\bibitem`'s span. The first entry wins for resolution.
    pub span: Span,
}

/// A **resolved** citation: one key of a `\cite` whose key was found in the bibliography table.
/// Carries the byte spans of **both** ends of the binding — the `\cite` construct's own span *and*
/// the defining `\bibitem`'s span — the source→source correlation the ADJ byte-provenance pipeline
/// audits against.
///
/// **Per-key, shared cite-span.** Because one `\cite{a,b,c}` yields *several* keys, S2 emits one
/// `ResolvedCite` **per key**, and every binding from the same `\cite` carries the *same*
/// [`cite_span`](ResolvedCite::cite_span). A caller can thus see both per-key resolution *and* which
/// source `\cite` each key came from (group by `cite_span`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCite {
    /// The single citation key this binding resolved (`"smith2020"`) — one key of the (possibly
    /// multi-key) `\cite`.
    pub key: String,
    /// The **citing** `\cite` construct's own span: `&src[cite_span]` slices back to exactly the
    /// `\cite{…}` / `\cite[note]{…}` source. Shared by every key of a multi-key `\cite`.
    pub cite_span: Span,
    /// The **winning** (first) `\bibitem`'s span: `&src[entry_span]` slices back to exactly
    /// `\bibitem{key}`.
    pub entry_span: Span,
}

/// An **unresolved** (dangling) citation: one key of a `\cite` that matched **no** `\bibitem` —
/// LaTeX's *"Citation `key' undefined"*, the `[?]` in the output. We record the dangling key and the
/// citing `\cite`'s own span so a caller can point at the exact offending source. As with
/// [`ResolvedCite`], every unresolved key from a multi-key `\cite` carries the same `cite_span`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnresolvedCite {
    /// The dangling key that no `\bibitem` defines (`"ghost"`).
    pub key: String,
    /// The citing `\cite` construct's own span: `&src[cite_span]` slices back to the dangling
    /// `\cite{…}` source. Shared by every key of a multi-key `\cite`.
    pub cite_span: Span,
}

/// The full result of [`Document::resolve_citations`]: the four tables a consumer enumerates — the
/// winning bibliography entries, the duplicate (losing) entries, the resolved citations, and the
/// unresolved (dangling) citations. All plain, `Clone`-able data (spans are `Copy`; keys are owned
/// `String`s), so the resolution outlives any borrow of the source and can be stored/serialized.
///
/// This is the exact structural parallel of S1's [`ReferenceResolution`]; keeping the two as
/// *separate* aggregates (rather than fusing them) reflects that labels and bibliographies are two
/// independent tables — a consumer that only cares about citations need not carry the label result,
/// and vice versa.
///
/// **Ordering.** `entries` and `duplicate_entries` are in [`Document::walk`] pre-order (so "first
/// entry wins" is a stable, source-order rule); `resolved` and `unresolved` are in the pre-order the
/// `\cite`s appear in the body, and *within* one multi-key `\cite` in left-to-right key order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CitationResolution {
    /// The **winning** bibliography entries — one row per distinct key (the first `\bibitem` seen),
    /// in pre-order. This is the table citations resolve against.
    pub entries: Vec<BibEntry>,
    /// The **duplicate** (later, losing) `\bibitem`s of already-defined keys, in pre-order — the
    /// *"multiply defined"* warnings.
    pub duplicate_entries: Vec<DuplicateBib>,
    /// The citation keys that **resolved** to a bibliography entry, in pre-order (one per key of each
    /// `\cite`).
    pub resolved: Vec<ResolvedCite>,
    /// The citation keys that did **not** resolve (dangling `\cite`s), in pre-order — the
    /// *"undefined"* warnings.
    pub unresolved: Vec<UnresolvedCite>,
}

impl CitationResolution {
    /// Look up the **winning** entry for `key` (the first `\bibitem` seen), if any. Linear over
    /// `entries`, which is small (one row per distinct citation key); no allocation.
    pub fn entry(&self, key: &str) -> Option<&BibEntry> {
        self.entries.iter().find(|e| e.key == key)
    }
}

// -------------------------------------------------------------------------------------------------
// The citation-resolution pass.
// -------------------------------------------------------------------------------------------------

impl Document {
    /// Resolve every `\cite` in this document against its in-document bibliography (LTXDOC03 S2).
    ///
    /// The parallel of [`Document::resolve_references`], for the *citation* family:
    ///
    /// 1. **Collect entries** — every `\bibitem{key}` inside a `thebibliography` environment, in
    ///    pre-order. The first `\bibitem` of each key wins; later ones become [`DuplicateBib`]s. (See
    ///    [`collect_bib_entries`].)
    /// 2. **Resolve citations** — every `\cite`'s `target` is split on commas into individual keys;
    ///    each key is looked up in the table: found → [`ResolvedCite`] (cite-span **and** the
    ///    entry's span), missing → [`UnresolvedCite`] (the dangling key + the cite-span). A multi-key
    ///    `\cite{a,b}` yields one record per key, all sharing that `\cite`'s span. (See
    ///    [`resolve_cites`].)
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing; reuses the bounded
    /// [`Document::walk`] and a `MAX_DEPTH`-bounded environment descent (no new *unbounded*
    /// recursion). Borrows `self` immutably; the returned [`CitationResolution`] is owned plain data
    /// (keys copied out, spans `Copy`), so it outlives any borrow of the source. Independent of, and
    /// non-interfering with, [`Document::resolve_references`] (disjoint command families).
    pub fn resolve_citations(&self) -> CitationResolution {
        let (entries, duplicate_entries) = collect_bib_entries(self);
        let (resolved, unresolved) = resolve_cites(self, &entries);
        CitationResolution { entries, duplicate_entries, resolved, unresolved }
    }
}

/// Pass 1 — collect the bibliography table (winning entries) and the duplicate list.
///
/// Only `\bibitem`s **inside a `thebibliography` environment** count as entries (a stray `\bibitem`
/// outside one is not a real bibliography entry and would be an authoring error). We therefore walk
/// the block forest looking for [`Block::Environment`]s named `"thebibliography"`, and within each we
/// scan its body (recursively, so a `\bibitem` nested in a paragraph/group still surfaces) for the
/// generic `\bibitem` commands — which, as the exploratory parse confirmed, survive D2 lowering as
/// [`Inline::Raw`]-wrapped [`NodeKind::Command`]s (`\bibitem` is not one of the constructs
/// `recognize_structure` folds), NOT as [`Inline::CrossRef`]s.
///
/// The **first** `\bibitem` of a key is pushed to `entries` (the winner); a subsequent `\bibitem` of
/// an already-seen key is pushed to `duplicate_entries`. "Seen" is a scan of `entries` (small: one
/// row per distinct key) — no map/allocation beyond the two output vecs.
fn collect_bib_entries(doc: &Document) -> (Vec<BibEntry>, Vec<DuplicateBib>) {
    let mut entries: Vec<BibEntry> = Vec::new();
    let mut duplicates: Vec<DuplicateBib> = Vec::new();

    // Push one candidate entry, routing it to `entries` (first-wins) or `duplicate_entries`.
    let mut record = |key: String, span: Span| {
        if entries.iter().any(|e| e.key == key) {
            duplicates.push(DuplicateBib { key, span });
        } else {
            entries.push(BibEntry { key, span });
        }
    };

    // Find every `thebibliography` environment, then collect the bibitems in each (in source order).
    for block in &doc.body {
        collect_bibitems_in_bibliographies(block, &mut record);
    }

    (entries, duplicates)
}

/// Recurse the block tree; for each `thebibliography` [`Block::Environment`], scan its body for
/// `\bibitem` entries. We descend into *all* blocks (not just the top level) so a `thebibliography`
/// nested inside another environment is still found — the same "nesting works everywhere" property
/// the D2 fold guarantees. Bounded by the tree depth (parser `MAX_DEPTH`), so no new unbounded
/// recursion.
fn collect_bibitems_in_bibliographies(block: &Block, record: &mut impl FnMut(String, Span)) {
    if let Block::Environment { name, body, .. } = block {
        if name == "thebibliography" {
            // Inside the bibliography: pull out every `\bibitem{key}` in source order.
            for inner in body {
                collect_bibitems_in_block(inner, record);
            }
            // A `thebibliography` nested inside another `thebibliography` is not a real construct;
            // we've handled this one's body above, so no need to recurse further into it here.
            return;
        }
    }
    // Otherwise keep looking deeper for a `thebibliography` among this block's children.
    for child in child_blocks(block) {
        collect_bibitems_in_bibliographies(child, record);
    }
}

/// Within a `thebibliography` body, pull `\bibitem{key}` entries out of one block. Bibitems land in
/// [`Block::Paragraph`] inline runs (as [`Inline::Raw`] commands); we also recurse into any nested
/// blocks defensively so a bibitem inside a group/environment is not missed. Bounded by tree depth.
fn collect_bibitems_in_block(block: &Block, record: &mut impl FnMut(String, Span)) {
    if let Block::Paragraph(inlines, _) = block {
        for inline in inlines {
            if let Some((key, span)) = bibitem_key_span(inline) {
                record(key, span);
            }
        }
    }
    for child in child_blocks(block) {
        collect_bibitems_in_block(child, record);
    }
}

/// If `inline` is a `\bibitem{key}` command (an [`Inline::Raw`] wrapping a [`NodeKind::Command`] named
/// `"bibitem"` with a first mandatory argument), return `(key, span)` where `span` is the
/// `\bibitem{key}` construct's own tight span and `key` is the argument rendered back to source (the
/// verbatim key, no braces). Any other inline → `None`.
///
/// The key is recovered by rendering the first argument's node list with [`document_to_latex`] — the
/// same faithful round-trip S1's cross-ref `target` came from — so a key with awkward characters is
/// preserved verbatim rather than assumed to be a single `Text` node.
fn bibitem_key_span(inline: &Inline) -> Option<(String, Span)> {
    let Inline::Raw(node, span) = inline else {
        return None;
    };
    let NodeKind::Command { name, arguments, .. } = &node.kind else {
        return None;
    };
    if name != "bibitem" {
        return None;
    }
    let key_arg: &Vec<Node> = arguments.first()?;
    Some((document_to_latex(key_arg), *span))
}

/// The child *blocks* of a block, for the bounded bibliography descent. Mirrors the block children
/// [`Document::walk`] visits (environments, quotes, figures, lists, tables, sections), so the descent
/// reaches a `thebibliography`/`\bibitem` wherever the walk would. Inline-only leaves have none.
fn child_blocks(block: &Block) -> Vec<&Block> {
    match block {
        Block::Section { body, .. }
        | Block::Environment { body, .. }
        | Block::Quote(body, _)
        | Block::Figure { content: body, .. } => body.iter().collect(),
        Block::List { items, .. } => items.iter().flat_map(|it| it.body.iter()).collect(),
        Block::Table { rows, .. } => {
            rows.iter().flat_map(|row| row.iter().flat_map(|cell| cell.iter())).collect()
        }
        // Leaves with no child blocks.
        Block::Paragraph(..)
        | Block::CodeBlock { .. }
        | Block::DisplayMath { .. }
        | Block::Raw(..) => Vec::new(),
    }
}

/// Split a `\cite` `target` on commas into individual, trimmed, non-empty keys.
///
/// `\cite{a, b ,c}` → `["a", "b", "c"]`. Whitespace around each key is trimmed (LaTeX ignores it),
/// and empty keys (from `\cite{a,,b}`, a leading/trailing comma, or an empty `\cite{}`) are dropped —
/// they name no entry, so binding them would be meaningless. A single-key `\cite{k}` yields `["k"]`.
fn split_cite_keys(target: &str) -> Vec<&str> {
    target.split(',').map(str::trim).filter(|k| !k.is_empty()).collect()
}

/// Pass 2 — resolve every `\cite`'s keys against the collected bibliography `entries`.
///
/// Walks the body; for each [`Inline::CrossRef`] whose `command` is [`CITE_COMMAND`] (`\cite`), we
/// split its `target` into keys and resolve each independently:
///
/// - found → a [`ResolvedCite`] recording the `\cite`'s span **and** the winning entry's span;
/// - missing → an [`UnresolvedCite`] recording the dangling key and the `\cite`'s span.
///
/// Every key of a multi-key `\cite` carries that one `\cite`'s span, so a caller can group per-`\cite`.
/// Non-`\cite` cross-refs (`\ref`/`\eqref`/`\pageref`/`\label`) are skipped — those are S1's job.
fn resolve_cites(
    doc: &Document,
    entries: &[BibEntry],
) -> (Vec<ResolvedCite>, Vec<UnresolvedCite>) {
    let mut resolved: Vec<ResolvedCite> = Vec::new();
    let mut unresolved: Vec<UnresolvedCite> = Vec::new();

    for node in doc.walk() {
        let NodeRef::Inline(Inline::CrossRef { command, target, span, .. }) = node else {
            continue;
        };
        if command != CITE_COMMAND {
            continue; // `\ref`/`\eqref`/`\pageref`/`\label` are the label family (S1), not citations.
        }
        for key in split_cite_keys(target) {
            // First-entry-wins lookup: the earliest `\bibitem` with this key.
            match entries.iter().find(|e| e.key == key) {
                Some(entry) => resolved.push(ResolvedCite {
                    key: key.to_string(),
                    cite_span: *span,
                    entry_span: entry.span,
                }),
                None => unresolved.push(UnresolvedCite {
                    key: key.to_string(),
                    cite_span: *span,
                }),
            }
        }
    }

    (resolved, unresolved)
}

// -------------------------------------------------------------------------------------------------
// Tests.
// -------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::parse_document;

    /// Parse `src` and resolve its references — the shared harness for every test below.
    fn resolve(src: &str) -> (String, ReferenceResolution) {
        let doc = parse_document(src).expect("parse");
        let res = doc.resolve_references();
        (src.to_string(), res)
    }

    #[test]
    fn ref_to_section_resolves_and_spans_slice_back() {
        // A `\ref{sec:intro}` to a `\section{…}\label{sec:intro}` RESOLVES; the resolved target span
        // slices out the defining section's source, and the ref-span slices out the `\ref` itself.
        // The `\n\n` after the `\label` keeps it a *lone* paragraph so LTXDOC01 hoists it onto the
        // section (per `hoist_label`).
        let src = r"\begin{document}\section{Intro}\label{sec:intro}

Body text. See Section~\ref{sec:intro}.\end{document}";
        let (src, res) = resolve(src);

        // Exactly one definition, one resolved ref, no duplicates, no unresolved.
        assert_eq!(res.definitions.len(), 1, "one label defined");
        assert_eq!(res.duplicates.len(), 0);
        assert_eq!(res.unresolved.len(), 0, "the ref must resolve");
        assert_eq!(res.resolved.len(), 1, "one ref resolved");

        let def = &res.definitions[0];
        assert_eq!(def.key, "sec:intro");
        assert_eq!(def.kind, LabelKind::Section);

        let r = &res.resolved[0];
        assert_eq!(r.key, "sec:intro");
        assert_eq!(r.command, "ref");
        assert_eq!(r.target_kind, LabelKind::Section);

        // LOAD-BEARING: the resolved target span, sliced from source, covers the defining
        // `\section`/heading bytes; the ref span slices back to exactly the `\ref{sec:intro}`.
        let target_src = &src[r.target_span.start..r.target_span.end];
        assert!(
            target_src.starts_with(r"\section{Intro}"),
            "target span must cover the defining \\section, got {target_src:?}"
        );
        let ref_src = &src[r.ref_span.start..r.ref_span.end];
        assert_eq!(ref_src, r"\ref{sec:intro}", "ref span must slice back to the \\ref construct");

        // The definition's own span matches the resolved target span (same node).
        assert_eq!(def.span, r.target_span);
    }

    #[test]
    fn ref_to_figure_float_resolves_with_figure_kind() {
        // A `\ref{fig:plot}` to a labeled `figure` float resolves, kind == Figure, and the target
        // span slices back to the whole `\begin{figure}…\end{figure}`.
        let src = r"\begin{document}\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:plot}\end{figure}

As shown in Figure~\ref{fig:plot}.\end{document}";
        let (src, res) = resolve(src);

        assert_eq!(res.definitions.len(), 1);
        let def = &res.definitions[0];
        assert_eq!(def.key, "fig:plot");
        assert_eq!(def.kind, LabelKind::Figure);

        assert_eq!(res.resolved.len(), 1);
        let r = &res.resolved[0];
        assert_eq!(r.target_kind, LabelKind::Figure);
        assert!(res.unresolved.is_empty());

        let target_src = &src[r.target_span.start..r.target_span.end];
        assert!(
            target_src.starts_with(r"\begin{figure}") && target_src.ends_with(r"\end{figure}"),
            "figure target span must cover the whole float, got {target_src:?}"
        );
        assert_eq!(&src[r.ref_span.start..r.ref_span.end], r"\ref{fig:plot}");
    }

    #[test]
    fn inline_label_collected_and_eqref_resolves_to_it() {
        // An inline `\label{eq:x}` (not hoisted — it sits mid-paragraph, not lone after a heading)
        // is collected as a definition, and a later `\eqref{eq:x}` resolves to it.
        let src = r"\begin{document}The identity \label{eq:x} holds.

By \eqref{eq:x} we conclude.\end{document}";
        let (src, res) = resolve(src);

        assert_eq!(res.definitions.len(), 1, "the inline \\label is a definition");
        let def = &res.definitions[0];
        assert_eq!(def.key, "eq:x");
        assert_eq!(def.kind, LabelKind::Inline, "a bare inline \\label is kind Inline");
        // The inline definition's span slices back to exactly the `\label{eq:x}` construct.
        assert_eq!(&src[def.span.start..def.span.end], r"\label{eq:x}");

        assert_eq!(res.resolved.len(), 1);
        let r = &res.resolved[0];
        assert_eq!(r.command, "eqref");
        assert_eq!(r.key, "eq:x");
        assert_eq!(r.target_kind, LabelKind::Inline);
        assert_eq!(r.target_span, def.span, "eqref resolves to the inline \\label's span");
        assert_eq!(&src[r.ref_span.start..r.ref_span.end], r"\eqref{eq:x}");
    }

    #[test]
    fn dangling_ref_is_unresolved() {
        // A `\ref{nope}` with no matching label is UNRESOLVED: the dangling key is recorded and its
        // ref-span slices back to the offending `\ref`.
        let src = r"\begin{document}See \ref{nope} for details.\end{document}";
        let (src, res) = resolve(src);

        assert!(res.definitions.is_empty(), "no labels defined");
        assert!(res.resolved.is_empty(), "nothing resolves");
        assert_eq!(res.unresolved.len(), 1, "the dangling ref is recorded");

        let u = &res.unresolved[0];
        assert_eq!(u.key, "nope");
        assert_eq!(u.command, "ref");
        assert_eq!(&src[u.ref_span.start..u.ref_span.end], r"\ref{nope}");
    }

    #[test]
    fn duplicate_label_first_def_wins() {
        // A key defined twice is reported as a DUPLICATE; the FIRST definition wins for resolution.
        // Two lone inline `\label{dup}`s in separate paragraphs, then a `\ref{dup}`.
        let src = r"\begin{document}First \label{dup} here.

Second \label{dup} there.

Now \ref{dup}.\end{document}";
        let (_src, res) = resolve(src);

        // One winning definition, one duplicate.
        assert_eq!(res.definitions.len(), 1, "only the first \\label{{dup}} wins");
        assert_eq!(res.duplicates.len(), 1, "the second \\label{{dup}} is a duplicate");

        let winner = &res.definitions[0];
        let dup = &res.duplicates[0];
        assert_eq!(winner.key, "dup");
        assert_eq!(dup.key, "dup");
        // First-def-wins: the winner's span is EARLIER in the source than the duplicate's.
        assert!(
            winner.span.start < dup.span.start,
            "first definition (span {:?}) must precede the duplicate (span {:?})",
            winner.span,
            dup.span
        );

        // The ref resolves to the WINNER (first def), not the duplicate.
        assert_eq!(res.resolved.len(), 1);
        let r = &res.resolved[0];
        assert_eq!(r.target_span, winner.span, "ref resolves against the first definition");
        assert_ne!(r.target_span, dup.span, "ref must NOT resolve to the duplicate");
    }

    #[test]
    fn cite_is_not_a_resolvable_reference() {
        // `\cite{foo}` is bibliography, not a label reference: S1 neither resolves it nor reports it
        // unresolved. Pair it with a real dangling `\ref{bar}` to prove *only* the ref-family ref is
        // reported.
        let src = r"\begin{document}As in \cite{foo} and \ref{bar}.\end{document}";
        let (_src, res) = resolve(src);

        // The `\cite` appears in neither resolved nor unresolved.
        assert!(
            res.resolved.iter().all(|r| r.command != "cite"),
            "\\cite must never be a resolved reference"
        );
        assert!(
            res.unresolved.iter().all(|u| u.command != "cite"),
            "\\cite must never be an unresolved reference"
        );
        // Only the `\ref{bar}` is reported (as dangling — no label `bar`).
        assert_eq!(res.unresolved.len(), 1, "only the \\ref is a ref-family reference");
        assert_eq!(res.unresolved[0].key, "bar");
        assert_eq!(res.unresolved[0].command, "ref");
        assert!(res.resolved.is_empty());
        assert!(res.definitions.is_empty(), "\\cite defines no label; no definitions");
    }

    #[test]
    fn pageref_resolves_like_ref() {
        // `\pageref` is in the reference family: it resolves against the label table too.
        let src = r"\begin{document}\section{Body}\label{sec:body}

See page~\pageref{sec:body}.\end{document}";
        let (_src, res) = resolve(src);
        assert_eq!(res.resolved.len(), 1);
        assert_eq!(res.resolved[0].command, "pageref");
        assert_eq!(res.resolved[0].target_kind, LabelKind::Section);
        assert!(res.unresolved.is_empty());
    }

    #[test]
    fn empty_document_yields_empty_results_no_panic() {
        // An empty document, and a document with text but no labels/refs, both yield empty results
        // and do not panic.
        let (_src, res) = resolve(r"\begin{document}\end{document}");
        assert_eq!(res, ReferenceResolution::default(), "empty doc → empty resolution");

        let (_src, res2) = resolve(r"\begin{document}Just prose, no cross-references at all.\end{document}");
        assert!(res2.definitions.is_empty());
        assert!(res2.duplicates.is_empty());
        assert!(res2.resolved.is_empty());
        assert!(res2.unresolved.is_empty());
    }

    #[test]
    fn definition_lookup_helper_finds_winner() {
        // The `definition(key)` convenience returns the winning def, or None for an unknown key.
        let (_src, res) = resolve(
            r"\begin{document}\section{S}\label{sec:s}

Text.\end{document}",
        );
        assert_eq!(res.definition("sec:s").map(|d| d.kind), Some(LabelKind::Section));
        assert!(res.definition("missing").is_none());
    }

    #[test]
    fn table_float_label_resolves_with_table_kind() {
        // A `table` float's hoisted `\label` defines a Table-kind label a `\ref` resolves to.
        let src = r"\begin{document}\begin{table}\begin{tabular}{lc}a & b \\ c & d\end{tabular}\caption{Data}\label{tab:data}\end{table}

See Table~\ref{tab:data}.\end{document}";
        let (src, res) = resolve(src);
        assert_eq!(res.definitions.len(), 1);
        assert_eq!(res.definitions[0].kind, LabelKind::Table);
        assert_eq!(res.resolved.len(), 1);
        assert_eq!(res.resolved[0].target_kind, LabelKind::Table);
        // The table target span covers the tabular/float source.
        let target_src = &src[res.resolved[0].target_span.start..res.resolved[0].target_span.end];
        assert!(
            target_src.contains(r"\begin{tabular}"),
            "table target span must cover the tabular, got {target_src:?}"
        );
    }

    // ---------------------------------------------------------------------------------------------
    // LTXDOC03 S2 — `\cite` → bibliography binding.
    // ---------------------------------------------------------------------------------------------

    /// Parse `src` and resolve its citations — the shared S2 harness.
    fn resolve_cites_of(src: &str) -> (String, CitationResolution) {
        let doc = parse_document(src).expect("parse");
        let res = doc.resolve_citations();
        (src.to_string(), res)
    }

    #[test]
    fn cite_to_bibitem_resolves_and_spans_slice_back() {
        // A `\cite{smith2020}` to a `\bibitem{smith2020}` in a `thebibliography` RESOLVES; both the
        // entry span and the cite span slice back to exactly their source constructs.
        let src = r"\begin{document}As in \cite{smith2020}.

\begin{thebibliography}{9}
\bibitem{smith2020} Smith, J. A Title. 2020.
\end{thebibliography}\end{document}";
        let (src, res) = resolve_cites_of(src);

        assert_eq!(res.entries.len(), 1, "one bib entry");
        assert_eq!(res.duplicate_entries.len(), 0);
        assert_eq!(res.unresolved.len(), 0, "the cite must resolve");
        assert_eq!(res.resolved.len(), 1, "one cite resolved");

        let entry = &res.entries[0];
        assert_eq!(entry.key, "smith2020");
        // LOAD-BEARING: the entry span slices back to exactly `\bibitem{smith2020}`.
        assert_eq!(&src[entry.span.start..entry.span.end], r"\bibitem{smith2020}");

        let c = &res.resolved[0];
        assert_eq!(c.key, "smith2020");
        assert_eq!(c.entry_span, entry.span, "cite resolves to the winning entry's span");
        // LOAD-BEARING: the cite span slices back to exactly `\cite{smith2020}`.
        assert_eq!(&src[c.cite_span.start..c.cite_span.end], r"\cite{smith2020}");
    }

    #[test]
    fn multi_key_cite_yields_one_binding_per_key_sharing_the_cite_span() {
        // A multi-key `\cite{a,b}` where both are defined → TWO resolved citations, both carrying
        // the SAME `\cite` span, and each pointing at its own entry span.
        let src = r"\begin{document}See \cite{a,b}.

\begin{thebibliography}{9}
\bibitem{a} Author A. First. 2001.
\bibitem{b} Author B. Second. 2002.
\end{thebibliography}\end{document}";
        let (src, res) = resolve_cites_of(src);

        assert_eq!(res.entries.len(), 2, "two entries");
        assert_eq!(res.resolved.len(), 2, "one binding per key");
        assert!(res.unresolved.is_empty());

        let ca = &res.resolved[0];
        let cb = &res.resolved[1];
        assert_eq!(ca.key, "a");
        assert_eq!(cb.key, "b");
        // Both bindings share the ONE `\cite{a,b}` span.
        assert_eq!(ca.cite_span, cb.cite_span, "both keys come from the same \\cite");
        assert_eq!(&src[ca.cite_span.start..ca.cite_span.end], r"\cite{a,b}");
        // Each resolves to its OWN entry span.
        assert_eq!(&src[ca.entry_span.start..ca.entry_span.end], r"\bibitem{a}");
        assert_eq!(&src[cb.entry_span.start..cb.entry_span.end], r"\bibitem{b}");
        assert_ne!(ca.entry_span, cb.entry_span, "distinct keys → distinct entries");
    }

    #[test]
    fn mixed_cite_yields_one_resolved_and_one_unresolved_from_same_cite() {
        // A mixed `\cite{known,unknown}` → one resolved + one unresolved, both from the same span.
        let src = r"\begin{document}Cf. \cite{known,unknown}.

\begin{thebibliography}{9}
\bibitem{known} Known, A. Paper. 2010.
\end{thebibliography}\end{document}";
        let (src, res) = resolve_cites_of(src);

        assert_eq!(res.resolved.len(), 1, "only `known` resolves");
        assert_eq!(res.unresolved.len(), 1, "`unknown` is dangling");

        let r = &res.resolved[0];
        let u = &res.unresolved[0];
        assert_eq!(r.key, "known");
        assert_eq!(u.key, "unknown");
        // Both keys come from the SAME `\cite{known,unknown}` construct.
        assert_eq!(r.cite_span, u.cite_span, "resolved & unresolved share the one \\cite span");
        assert_eq!(&src[r.cite_span.start..r.cite_span.end], r"\cite{known,unknown}");
    }

    #[test]
    fn cite_with_optional_note_resolves_and_note_not_conflated_into_key() {
        // `\cite[p. 3]{key}` resolves; the key is exactly "key" (the `[p. 3]` note stays separate).
        let src = r"\begin{document}As \cite[p. 3]{key} shows.

\begin{thebibliography}{9}
\bibitem{key} Key, A. Ref. 2015.
\end{thebibliography}\end{document}";
        let (src, res) = resolve_cites_of(src);

        assert_eq!(res.resolved.len(), 1);
        assert!(res.unresolved.is_empty(), "the note must not turn the key dangling");
        let c = &res.resolved[0];
        assert_eq!(c.key, "key", "the [p. 3] note is NOT conflated into the key");
        // The cite span covers the whole `\cite[p. 3]{key}` construct (note included).
        assert_eq!(&src[c.cite_span.start..c.cite_span.end], r"\cite[p. 3]{key}");
    }

    #[test]
    fn dangling_cite_is_unresolved() {
        // A `\cite{ghost}` with a bibliography that lacks that key → unresolved.
        let src = r"\begin{document}See \cite{ghost}.

\begin{thebibliography}{9}
\bibitem{real} Real, A. Actual. 2000.
\end{thebibliography}\end{document}";
        let (src, res) = resolve_cites_of(src);

        assert_eq!(res.entries.len(), 1, "the one real entry is collected");
        assert!(res.resolved.is_empty(), "nothing resolves");
        assert_eq!(res.unresolved.len(), 1, "the dangling cite is recorded");
        let u = &res.unresolved[0];
        assert_eq!(u.key, "ghost");
        assert_eq!(&src[u.cite_span.start..u.cite_span.end], r"\cite{ghost}");
    }

    #[test]
    fn duplicate_bibitem_first_entry_wins() {
        // A key defined by two `\bibitem`s → the second is a DUPLICATE; the FIRST wins for resolution.
        let src = r"\begin{document}\cite{dup}.

\begin{thebibliography}{9}
\bibitem{dup} First def. 1990.
\bibitem{dup} Second def. 1991.
\end{thebibliography}\end{document}";
        let (_src, res) = resolve_cites_of(src);

        assert_eq!(res.entries.len(), 1, "only the first \\bibitem{{dup}} wins");
        assert_eq!(res.duplicate_entries.len(), 1, "the second is a duplicate");

        let winner = &res.entries[0];
        let dup = &res.duplicate_entries[0];
        assert_eq!(winner.key, "dup");
        assert_eq!(dup.key, "dup");
        // First-entry-wins: the winner precedes the duplicate in the source.
        assert!(
            winner.span.start < dup.span.start,
            "first entry (span {:?}) must precede the duplicate (span {:?})",
            winner.span,
            dup.span
        );

        // The cite resolves to the WINNER, not the duplicate.
        assert_eq!(res.resolved.len(), 1);
        assert_eq!(res.resolved[0].entry_span, winner.span, "cite resolves against the first entry");
        assert_ne!(res.resolved[0].entry_span, dup.span, "cite must NOT resolve to the duplicate");
    }

    #[test]
    fn cite_with_no_bibliography_is_unresolved_no_panic() {
        // A document with NO thebibliography and a `\cite{x}` → the cite is unresolved (empty table).
        let src = r"\begin{document}As in \cite{x}, obviously.\end{document}";
        let (_src, res) = resolve_cites_of(src);

        assert!(res.entries.is_empty(), "no bibliography → no entries");
        assert!(res.duplicate_entries.is_empty());
        assert!(res.resolved.is_empty(), "nothing to resolve against");
        assert_eq!(res.unresolved.len(), 1, "the cite is dangling");
        assert_eq!(res.unresolved[0].key, "x");
    }

    #[test]
    fn empty_document_yields_empty_citation_results_no_panic() {
        // An empty document → empty citation results, no panic.
        let (_src, res) = resolve_cites_of(r"\begin{document}\end{document}");
        assert_eq!(res, CitationResolution::default(), "empty doc → empty citation resolution");
    }

    #[test]
    fn entry_lookup_helper_finds_winner() {
        // The `entry(key)` convenience returns the winning entry, or None for an unknown key.
        let src = r"\begin{document}\cite{k}.

\begin{thebibliography}{9}
\bibitem{k} K, A. Work. 2020.
\end{thebibliography}\end{document}";
        let (_src, res) = resolve_cites_of(src);
        assert_eq!(res.entry("k").map(|e| e.key.as_str()), Some("k"));
        assert!(res.entry("missing").is_none());
    }

    #[test]
    fn labels_and_citations_coexist_without_interference() {
        // REGRESSION: on a doc with BOTH `\ref`/`\label` AND `\cite`/`\bibitem`, the S1 ref pass
        // still ignores `\cite`, and the S2 cite pass still ignores `\ref`/`\label` — the two tables
        // are disjoint and neither disturbs the other.
        let src = r"\begin{document}\section{Intro}\label{sec:intro}

See Section~\ref{sec:intro} and \cite{smith2020}.

\begin{thebibliography}{9}
\bibitem{smith2020} Smith, J. Title. 2020.
\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");

        // S1: exactly the label/ref binding, no citation leakage.
        let refs = doc.resolve_references();
        assert_eq!(refs.definitions.len(), 1, "one label defined");
        assert_eq!(refs.resolved.len(), 1, "the \\ref resolves");
        assert_eq!(refs.resolved[0].key, "sec:intro");
        assert!(refs.unresolved.is_empty(), "\\cite is NOT a dangling ref");
        assert!(
            refs.resolved.iter().all(|r| r.command != "cite"),
            "\\cite never appears in the ref resolution"
        );

        // S2: exactly the cite/bibitem binding, no label leakage.
        let cites = doc.resolve_citations();
        assert_eq!(cites.entries.len(), 1, "one bib entry");
        assert_eq!(cites.resolved.len(), 1, "the \\cite resolves");
        assert_eq!(cites.resolved[0].key, "smith2020");
        assert!(cites.unresolved.is_empty(), "\\ref is NOT a dangling cite");
        // The `sec:intro` label key never leaks into the citation table.
        assert!(cites.entry("sec:intro").is_none(), "a \\label key is not a bib entry");
    }
}
