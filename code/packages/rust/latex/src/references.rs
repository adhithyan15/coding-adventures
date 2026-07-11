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

use crate::ast::{Node, NodeKind, SectionLevel};
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

/// The placeholder number an equation label carries in the S6 report while equation **numbering** is
/// deferred to LTXDOC03 S8. It is `"?"` — echoing LaTeX's `??` for an as-yet-unresolved number — so a
/// resolved `\eqref` renders a stable, greppable line (`\ref{eq:e} -> Equation ?`) instead of being
/// omitted. S8 will replace this with the real `\theequation` counter value.
pub const EQUATION_NUMBER_PLACEHOLDER: &str = "?";

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
/// | [`Equation`](LabelKind::Equation) | a **non-starred** display-math env's `\label` lifted onto [`Block::DisplayMath`] | `\eqref{eq:e}` → an equation |
/// | [`Inline`](LabelKind::Inline)   | a non-hoisted inline `\label{…}` ([`Inline::CrossRef`]) | `\eqref{eq:x}` → an equation's label |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelKind {
    /// A `\label` hoisted onto a [`Block::Section`] heading.
    Section,
    /// A `\label` hoisted onto a [`Block::Table`] float.
    Table,
    /// A `\label` hoisted onto a [`Block::Figure`] float.
    Figure,
    /// A `\label` lifted out of a **non-starred** display-math environment (`equation`, `align`,
    /// `gather`, `multline`, `eqnarray`) onto its [`Block::DisplayMath`] (LTXDOC03 S7). Starred forms
    /// (`equation*`, …) are unnumbered and never carry this kind. The equation **number** is deferred
    /// to S8, so an `Equation` label carries a placeholder number in the S6 report (see
    /// [`Document::number_labels`]); it is still a real definition, so an `\eqref` to it **resolves**
    /// and is no longer omitted from the cross-reference report.
    Equation,
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
            LabelKind::Equation => "equation",
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
        // LTXDOC03 S7: a `\label` lifted out of a **non-starred** display-math environment onto its
        // `Block::DisplayMath`. Only the non-starred forms carry a `Some(label)` (the lowering sets
        // `label: None` for `equation*`/`\[…\]`/`$$…$$`), so a `Some` here is always numbered.
        Block::DisplayMath { label: Some(key), span, .. } => {
            Some((key.clone(), LabelKind::Equation, *span))
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

// =================================================================================================
// LTXDOC03 S3 — target → NodeRef exposure.
//
// S1 bound each `\ref` to the *bytes* of its `\label`'s node (a [`Span`]); S2 bound each `\cite` to
// the *bytes* of its `\bibitem`'s node. Both hand back a [`Span`] — a source-byte range you can
// slice — but **not** the node itself. So a consumer that resolves `\ref{sec:intro}` learns *where*
// the section is in the source, yet cannot ask "what *kind* of node is this?" or "walk into the
// section's paragraphs". S3 closes that gap: given a resolved target's [`Span`], it hands back the
// actual walked [`NodeRef`], so the caller can read its [`kind`](NodeRef::kind) and — for a
// [`NodeRef::Block`] — descend into its children (a `\ref` to a section can now enumerate the
// section's paragraphs; a `\ref` to a figure can reach its caption text).
//
// ## The lookup: span-equality against the body walk
//
// The primitive is [`Document::node_for_span`]: it walks the body ([`Document::walk`], pre-order) and
// returns the node whose [`span`](NodeRef::span) **exactly equals** the requested one — half-open
// equality of *both* `start` and `end`. This is deliberately *equality*, not *containment*: a
// resolved target's span is, by construction, some walked node's own span (S1 recorded a block's or
// cross-ref's span; S2 recorded a `\bibitem`'s `Inline::Raw` span), so the node we want is the one
// that *is* that span, not merely one that *encloses* it. (Containment is [`Document::node_at`]'s job,
// S4 — a different question: "which leaf owns this arbitrary byte?".)
//
// ## Reachability — which targets yield a node, which yield `None`
//
// The exploratory parse (recorded in the S3 spec) confirmed that **every** S1/S2 target span
// corresponds to **exactly one** walked node — there were *zero* pairs of distinct walked nodes
// sharing an identical span — so the lookup is unambiguous in practice:
//
// | target | walked node it resolves to | reachable? |
// |--------|----------------------------|------------|
// | `\ref`→`\section` (hoisted label) | the [`Block::Section`] | **yes** ([`NodeRef::Block`], kind `"Section"`) |
// | `\ref`→`figure` float | the [`Block::Figure`] | **yes** (kind `"Figure"`) |
// | `\ref`→`table` float | the [`Block::Table`] | **yes** (kind `"Table"`) |
// | `\eqref`→ inline `\label` | the [`Inline::CrossRef`] | **yes** ([`NodeRef::Inline`], kind `"CrossRef"`) |
// | `\cite`→`\bibitem` | the [`Inline::Raw`] `\bibitem` command | **yes** (kind `"Raw"`) — see below |
//
// **The `\bibitem` target *is* walked.** A `\bibitem{key}` sits inside a `thebibliography`
// [`Block::Environment`], whose body [`Document::walk`] descends into; the `\bibitem` survives D2 as an
// [`Inline::Raw`] command inside a [`Block::Paragraph`], which `walk` visits. So its span **does**
// match a walked node and [`Document::cite_target_node`] returns `Some(NodeRef::Inline(..))` (kind
// `"Raw"`), *not* `None`. (This was the one genuinely uncertain case — bibitems could have lived in a
// non-walked region — so it was verified rather than assumed.)
//
// **When `None` is returned.** `node_for_span` returns `None` for any span that is *not* some walked
// body node's own span: a span from an empty document, a preamble/metadata span (those regions are
// classified out of directives and deliberately **not** walked — see [`Document::walk`]), or any
// caller-fabricated span that lands between nodes. This is a *documented, total* outcome — never a
// panic — so a caller can treat "no node" as an ordinary result. Because every S1/S2 *resolved*
// target span is a walked node's span, the accessors below never return `None` for a genuinely
// resolved reference/citation in a well-formed document; `None` is reserved for the honest edge
// (fabricated spans, empty docs).
//
// ## Tie-break (defensive, does not fire in practice)
//
// If two walked nodes ever shared an identical span, `node_for_span` returns the **first in pre-order**
// — the outermost/earliest, since `walk` yields a parent before its children and an earlier sibling
// before a later one. The exploratory parse found no such collision for real targets, so this rule is
// a defensive tie-break for totality, not a behaviour real callers hit; it is documented so the choice
// is deterministic rather than incidental.
//
// ## Additivity, borrowing, and bounds
//
// S3 is **purely additive**: the S1/S2 result types ([`ResolvedRef`], [`ResolvedCite`], [`LabelDef`],
// [`BibEntry`], …) are **unchanged** — they still carry only owned [`Span`]s (no lifetimes), so a
// resolution still outlives any borrow of the source. The [`NodeRef`] is fetched *on demand* through
// these [`Document`] methods (a `NodeRef` borrows the doc, so it cannot live on the owned result
// types). Each lookup is O(nodes) — one reuse of the bounded [`Document::walk`] — introducing no new
// recursion, no allocation beyond `walk`'s own vector, and no panic (no `unwrap`/`expect`, no unchecked
// indexing).
// =================================================================================================

impl Document {
    /// The walked body node whose [`span`](NodeRef::span) **exactly equals** `span` (half-open
    /// equality of *both* `start` and `end`), or `None` if no walked node matches (LTXDOC03 S3).
    ///
    /// This is the load-bearing primitive under S3's ergonomic accessors ([`ref_target_node`],
    /// [`cite_target_node`], [`label_def_node`]). It answers "**which node** *is* these bytes?" —
    /// distinct from [`node_at`](Document::node_at)'s "which leaf *contains* this byte?" (containment,
    /// S4). A resolved S1/S2 target span is, by construction, some walked node's own span, so equality
    /// is the right predicate here.
    ///
    /// **Reachability & `None`.** Every S1/S2 *resolved* target — a section/table/figure block, an
    /// inline `\label`, or a `\bibitem` [`Inline::Raw`] — is a walked body node, so this returns
    /// `Some` for them. It returns `None` for a span that is not any walked node's own span: an empty
    /// document, a preamble/metadata region (not walked — see [`walk`](Document::walk)), or a
    /// fabricated span between nodes. `None` is an ordinary, documented result, never a panic.
    ///
    /// **Tie-break.** If two walked nodes ever shared an identical span (none do for real targets —
    /// the exploratory parse found zero collisions), the **first in pre-order** wins (the outermost /
    /// earliest, since `walk` yields parents before children and earlier siblings first).
    ///
    /// **Total, O(nodes), panic-free.** One reuse of the bounded [`walk`](Document::walk) — no new
    /// recursion, no `unwrap`/`expect`, no unchecked indexing.
    pub fn node_for_span(&self, span: Span) -> Option<NodeRef<'_>> {
        // `find` yields the first (pre-order) node whose span equals `span` — the documented
        // tie-break — and short-circuits once found.
        self.walk().find(|node| node.span() == span)
    }

    /// The actual target **node** a resolved reference points at (LTXDOC03 S3) — the section, figure,
    /// table, or inline `\label` [`NodeRef`] whose span is `r.target_span`, or `None` if that span is
    /// not a walked node (it always is for a genuinely [`ResolvedRef`], so `None` is the honest edge —
    /// see [`node_for_span`](Document::node_for_span)).
    ///
    /// This lifts S1's byte-level binding to a node-level one: from `r.target_span` (the target's
    /// *bytes*) to the target *node*, so the caller can read its [`kind`](NodeRef::kind) and, for a
    /// [`NodeRef::Block`], descend into its children — e.g. `ref_target_node` for a `\ref{sec:intro}`
    /// yields the [`Block::Section`], from which one can enumerate the section's paragraphs.
    pub fn ref_target_node(&self, r: &ResolvedRef) -> Option<NodeRef<'_>> {
        self.node_for_span(r.target_span)
    }

    /// The actual target **node** a resolved citation points at (LTXDOC03 S3) — the `\bibitem`
    /// [`Inline::Raw`] [`NodeRef`] (kind `"Raw"`) whose span is `c.entry_span`, or `None` if that span
    /// is not a walked node (it always is for a genuinely [`ResolvedCite`] — the `\bibitem` inside a
    /// `thebibliography` *is* walked, as the exploratory parse confirmed — so `None` is the honest
    /// edge).
    ///
    /// This lifts S2's byte-level binding to a node-level one: from `c.entry_span` (the entry's
    /// *bytes*) to the `\bibitem` node itself.
    pub fn cite_target_node(&self, c: &ResolvedCite) -> Option<NodeRef<'_>> {
        self.node_for_span(c.entry_span)
    }

    /// The defining **node** of a label definition (LTXDOC03 S3) — the section/table/figure
    /// [`NodeRef::Block`] or inline `\label` [`NodeRef::Inline`] whose span is `d.span`, or `None` if
    /// that span is not a walked node (it always is for a genuine [`LabelDef`], so `None` is the honest
    /// edge). The definition-side companion to [`ref_target_node`](Document::ref_target_node): given a
    /// row of the label table, hand back the node that *defines* it.
    pub fn label_def_node(&self, d: &LabelDef) -> Option<NodeRef<'_>> {
        self.node_for_span(d.span)
    }
}

// =================================================================================================
// LTXDOC03 S4 — document numbering (hierarchical section numbers + flat float counters).
//
// S1 bound each `\ref` to the *bytes* of its `\label`'s node; S3 lifted that to the target *node*.
// Neither gives the **rendered number** a `\ref` prints — the "1.2" in "see Section~1.2", the "3" in
// "Figure~3". S4 assigns those numbers. It is the static, single-pass analogue of LaTeX's **second
// `.aux` pass**: on the first `latex` run each `\refstepcounter` (fired by every numbered `\section`,
// `figure`, `table`, …) writes the counter's value into `document.aux` next to the label; on the
// second run `\ref{key}` reads that value back. We do the same binding **in one walk** over the
// already-parsed [`Document`] — no `.aux` file, no second parse — computing each numbered target's
// value directly.
//
// ## The two counter models LaTeX uses, and which we implement
//
// LaTeX has two shapes of counter, and S4 models both:
//
// 1. **Hierarchical section counters (with deeper-reset).** `\part` … `\subparagraph` share a
//    *nested* counter family: incrementing a coarser counter **resets every finer one to zero**. So
//    after `\section` (→ `1`), a `\subsection` is `1.1`, another is `1.2`, and the *next* `\section`
//    bumps to `2` **and** resets the subsection counter, so its first child is `2.1` again. The
//    printed number is the **dotted join** of the counters from the top level down to this heading's
//    depth: `1`, `1.1`, `1.2`, `1.2.1`, `2`. (Real LaTeX starts the dotted join at the *class's*
//    top-numbered level — `\section` for `article`, `\chapter` for `report`/`book` — and omits
//    levels above it. We do **not** know the class here, so we join from the coarsest level that has
//    actually been *seen* down to this heading's depth; see the missing-parent rule below.)
//
// 2. **Flat float counters (no reset, no hierarchy).** `figure` and `table` each own an *independent*
//    running counter that only ever **increments**: figures are `1, 2, 3, …` in document order, and
//    tables are their **own** `1, 2, 3, …` — a `table` after two figures is `1`, not `3`. (In
//    `article` these are document-global; in `report`/`book` they reset per chapter, which — being a
//    class-dependent, chapter-scoped rule we cannot see — S4 does *not* model. The honest, class-free
//    choice is a single global run per float type.)
//
// ## Every float consumes a counter — labeled or not
//
// LaTeX advances a float's counter **every time** the float appears; a `\label` merely *captures*
// whatever the counter reads at that moment. So an **unlabeled** figure between two labeled ones
// still consumes a number: the labeled figures come out `1` and `3`, with the unlabeled one having
// silently taken `2`. S4 mirrors this exactly — we walk **every** [`Block::Figure`]/[`Block::Table`]
// and advance its counter, then *expose* the value only for the ones that carry a `label`. (Starred
// unnumbered sections are the opposite case: `\section*` sets `numbered == false`, fires no counter,
// and is **skipped** entirely — the following numbered `\section` keeps the number it would have had.)
//
// ## The missing-parent rule (a document that starts deep)
//
// A well-formed document opens with a top-level heading, but nothing *stops* an author writing a
// `\subsection` before any `\section` — the exploratory parse confirmed such a document parses fine
// (a lone `Block::Section { level: Subsection, numbered: true, .. }`). Its parent `\section` counter
// was never incremented, so it sits at its initial **0**. S4's rule: **treat a missing parent as 0**
// (we never clamp it away, and we never skip the heading). We render from the `\section` depth (the
// article default top-numbered level) down to the heading, so the un-opened parent slots surface
// their honest `0`: a lone leading `\subsection` numbers **`0.1`**, a lone `\subsubsection` **`0.0.1`**.
// A plain top-level `\section` itself is just **`1`** (it *is* the reference depth, so there is no
// parent to zero-fill). This is total (no panic), deterministic, and honestly reflects "no parent
// section has been opened yet" — surfacing the `0` rather than silently inventing a `1` the source
// never wrote, the faithful, auditable choice for a byte-provenance model. (The exact leading-zero
// depth is a documented convention, not a LaTeX fact — LaTeX would warn and print `0.1` too, but the
// point is that S4 *picks a rule and sticks to it* rather than panicking on the degenerate input.)
//
// ## What S4 numbers, and what is DEFERRED
//
// S4 numbers **sections** (hierarchical) and **figure/table floats** (flat) — the counters whose
// values a `\ref` most commonly prints. It deliberately does **not** yet assign:
//
// - **Equation numbers** — as of S7 a **non-starred** display-math env's `\label` is *lifted* onto
//   its [`Block::DisplayMath`] and recorded here as a [`LabelKind::Equation`] row, so an `\eqref` to
//   it **resolves** and is no longer omitted from the S6 report. But the equation **counter**
//   (`\theequation`) is still deferred: the row carries the [`EQUATION_NUMBER_PLACEHOLDER`] (`"?"`)
//   rather than a real number. Assigning the true equation number is S8.
// - **Citation `[1]` numbers** — the order-of-first-appearance number `\cite` prints in a numeric
//   bibliography style. That is a *citation-order* traversal over S2's resolution, a separate rung.
// - **Other `\label`-able counters** — `enumerate` item numbers, theorem/footnote counters, etc.
//   These need per-environment counter contexts; also S5+.
//
// This honest boundary mirrors S1/S2/S3: we assign only the numbers we can compute *faithfully* from
// the parsed structure, and name the rest as future work rather than guessing.
//
// ## The result type
//
// [`Document::number_labels`] returns a [`Numbering`]: one owned row per **defined label key**,
// carrying the label's [`LabelKind`] and its rendered number `String`. It mirrors S1/S2's dedicated,
// owned-`String` result types (so it outlives any borrow of the source) and provides a
// [`number_for`](Numbering::number_for) lookup. [`Document::ref_number`] is the payoff convenience:
// given a [`ResolvedRef`] (S1), it returns that reference's target's rendered number — closing the
// loop `\ref{sec:intro}` → `"1.2"`.
// =================================================================================================

/// The number of hierarchical section-counter slots: one per [`SectionLevel`] rank, `\part` (0)
/// through `\subparagraph` (6). A fixed-size array of exactly this many `u32`s carries the live
/// section counters through the numbering walk — no growth, no allocation, no unchecked indexing.
const SECTION_DEPTHS: usize = 7;

/// The **rank** (0-based depth) of a sectioning level: `0` for the coarsest (`\part`) up to `6` for
/// the finest (`\subparagraph`). This is the index into the section-counter array a heading of that
/// level increments, and the depth down to which its dotted number is joined.
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
/// This mirrors [`document::rank`](crate::document)'s private sectioning-fold rank (they must agree —
/// both are the [`SectionLevel`] declaration order), kept as a local copy so S4 does not need that
/// private helper exported. A pure lookup: it cannot panic and allocates nothing.
fn section_rank(level: SectionLevel) -> usize {
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

// -------------------------------------------------------------------------------------------------
// The record types (plain, Clone-able, owned data — parallel to S1/S2's result rows).
// -------------------------------------------------------------------------------------------------

/// One **numbered label**: a defined label `key`, the [`LabelKind`] of the node it defines, and the
/// rendered **number** LaTeX would print for a `\ref` to it. This is one row of S4's numbering table
/// — the static analogue of an `.aux` `\newlabel{key}{{number}{page}…}` line, capturing the number
/// (not the page).
///
/// The `number` is already rendered to its display string: a dotted section number (`"1"`, `"1.2"`,
/// `"1.2.1"`, or `"0.1"` for a lone-deep heading — see the missing-parent rule) for a
/// [`LabelKind::Section`], or a flat float count (`"1"`, `"2"`, …) for a [`LabelKind::Figure`] /
/// [`LabelKind::Table`]. A [`LabelKind::Inline`] label carries **no** counter S4 assigns (its target
/// is typically an equation — deferred to S5), so inline labels are **omitted** from the numbering
/// table entirely (see [`Document::number_labels`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberedLabel {
    /// The label key, verbatim, without braces (`"sec:intro"`).
    pub key: String,
    /// What kind of node this label defines (section / figure / table).
    pub kind: LabelKind,
    /// The rendered number LaTeX would print for a `\ref` to this label (`"1.2"`, `"3"`, …).
    pub number: String,
}

/// The full result of [`Document::number_labels`]: the numbering table — one [`NumberedLabel`] row
/// per **defined, numberable** label key (sections + figure/table floats; inline/equation labels are
/// omitted, deferred to S5). All plain, owned data (keys + numbers are `String`s), so the numbering
/// outlives any borrow of the source and can be stored/serialized — mirroring S1's
/// [`ReferenceResolution`] and S2's [`CitationResolution`].
///
/// **Ordering.** `labels` is in [`Document::walk`] pre-order — the source order the labels' defining
/// nodes appear in — so the table reads top-to-bottom like the document. As with S1's "first
/// definition wins", only the **first** definition of a duplicated key is numbered (the value the
/// winning [`LabelDef`] would carry); a later duplicate does not add a second row.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Numbering {
    /// One row per defined, numberable label key, in pre-order.
    pub labels: Vec<NumberedLabel>,
}

impl Numbering {
    /// The rendered number of `key`, if it is a defined numberable label, else `None`. Linear over
    /// `labels` (small: one row per distinct label); no allocation. `&str` borrow into the owned row.
    pub fn number_for(&self, key: &str) -> Option<&str> {
        self.labels.iter().find(|l| l.key == key).map(|l| l.number.as_str())
    }
}

// -------------------------------------------------------------------------------------------------
// The numbering pass.
// -------------------------------------------------------------------------------------------------

/// A small carrier for the live counters during the numbering walk: the hierarchical section
/// counters (one slot per rank) plus the two independent flat float counters. Kept as a struct so the
/// single walk threads exactly one mutable state, and each `step_*` method reads like the LaTeX
/// operation it mimics.
struct Counters {
    /// The hierarchical section counters, `section[rank]` = the current value at that depth. All
    /// start at `0`; a coarser step resets every finer slot (deeper-reset).
    section: [u32; SECTION_DEPTHS],
    /// The flat `figure` counter — only ever increments (one global run).
    figure: u32,
    /// The flat `table` counter — only ever increments (one global run), independent of `figure`.
    table: u32,
    /// The flat `equation` counter (`\theequation`) — only ever increments (one global run),
    /// independent of `section`/`figure`/`table`. LaTeX numbers display equations in pure document
    /// order, so — exactly like `figure`/`table` — this is a single monotonic run rather than a
    /// section-scoped or hierarchical value. (Real LaTeX *can* reset `\theequation` per section under
    /// classes/packages that redefine it, but the `article` default is a flat run, which is what S8
    /// models.)
    equation: u32,
}

impl Counters {
    /// Fresh counters — every section depth, the figure counter, the table counter, and the equation
    /// counter at `0` (no heading/float/equation seen yet).
    fn new() -> Self {
        Counters { section: [0; SECTION_DEPTHS], figure: 0, table: 0, equation: 0 }
    }

    /// Advance the section counters for a numbered heading at `rank`, and render its dotted number.
    ///
    /// The LaTeX `\refstepcounter{<level>}` semantics, in three steps:
    /// 1. **Increment** `section[rank]` by one.
    /// 2. **Reset** every *finer* slot (`rank+1 ..`) to `0` — the deeper-reset that makes the next
    ///    `\section` restart its subsections at `1`.
    /// 3. **Render** the dotted join from the coarsest *opened* ancestor down to this depth (see
    ///    [`render_section`]) — `1`, `1.1`, `1.2.1` for opened parents, and a single leading `0` for
    ///    a lone deep heading whose parent was never opened (the missing-parent rule → `0.1`).
    fn step_section(&mut self, rank: usize) -> String {
        // `rank` is always a valid index (`section_rank` yields 0..=6, and the array has 7 slots),
        // but guard defensively so no unchecked indexing can ever panic.
        if rank >= SECTION_DEPTHS {
            return String::new();
        }
        self.section[rank] = self.section[rank].saturating_add(1);
        for slot in self.section.iter_mut().skip(rank + 1) {
            *slot = 0;
        }
        render_section(&self.section, rank)
    }

    /// Advance and return the flat `figure` counter (`1` on the first figure, `2` on the next, …).
    fn step_figure(&mut self) -> u32 {
        self.figure = self.figure.saturating_add(1);
        self.figure
    }

    /// Advance and return the flat `table` counter, independent of the figure counter.
    fn step_table(&mut self) -> u32 {
        self.table = self.table.saturating_add(1);
        self.table
    }

    /// Advance and return the flat `equation` counter (`\theequation`), independent of the
    /// section/figure/table counters — the S8 payoff. Mirrors [`step_figure`](Counters::step_figure)
    /// exactly: pre-increment (saturating, so a pathological run can never wrap/panic) and return the
    /// new value, so the first equation numbers `1`, the next `2`, in pure document order.
    fn step_equation(&mut self) -> u32 {
        self.equation = self.equation.saturating_add(1);
        self.equation
    }
}

/// The rank of `\section` — the `article` class's top *numbered* sectioning level, and S4's
/// class-free reference depth for the missing-parent rule (see [`render_section`]).
const SECTION_LEVEL_RANK: usize = 2;

/// Render a section's dotted number from the counter array, for a heading at `rank`.
///
/// The dotted join runs from a **start depth** down to `rank` inclusive. The start depth is chosen so
/// that opened parents appear with their real values and a document that starts deep shows honest
/// leading `0`s for the un-opened parents (the missing-parent rule):
///
/// - **Opened ancestor present.** If any ancestor slot (`0..rank`) is non-zero, start at the
///   **coarsest** such slot. Every in-between slot is then included with its current value. So under
///   `\section` (slot 2 = `1`), a first `\subsection` (slot 3) joins `section[2..=3]` = `1.1`, and a
///   nested `\subsubsection` joins `section[2..=4]` = `1.2.1`.
/// - **No ancestor opened, and this heading is at or above `\section`** (`rank <= `[`SECTION_LEVEL_RANK`]).
///   Start at `rank` itself — a plain top-level `\section`/`\chapter`/`\part` is just its own number,
///   `1`, with no spurious leading `0`.
/// - **No ancestor opened, but this heading is *deeper* than `\section`** (a document that starts
///   deep). The missing-parent rule: start at [`SECTION_LEVEL_RANK`] (the `\section` depth), so the
///   un-opened section counter renders as a leading `0`. A lone leading `\subsection` (rank 3) reads
///   `section[2..=3]` = `0.1`; a lone `\subsubsection` (rank 4) reads `0.0.1`. Each un-opened parent
///   contributes an honest `0` rather than inventing a `1` the source never wrote.
///
/// Total: every index used is within `0..SECTION_DEPTHS` (range-checked), so no unchecked indexing.
fn render_section(section: &[u32; SECTION_DEPTHS], rank: usize) -> String {
    // Guard (mirrors `step_section`): an out-of-range rank renders empty rather than indexing wild.
    if rank >= SECTION_DEPTHS {
        return String::new();
    }
    // The coarsest opened (non-zero) ancestor slot strictly above this heading, if any.
    let first_nonzero = (0..rank).find(|&i| section[i] != 0);
    let start = match first_nonzero {
        // An opened ancestor: the number naturally begins there.
        Some(i) => i,
        // No opened ancestor: a heading at/above `\section` is just its own number; a deeper one
        // (document starts deep) begins at the `\section` depth so un-opened parents read `0`.
        None if rank <= SECTION_LEVEL_RANK => rank,
        None => SECTION_LEVEL_RANK,
    };
    // Join `section[start..=rank]` with dots. `start <= rank < SECTION_DEPTHS`, so the slice is in
    // bounds; iterating the sub-slice (rather than indexing by a range variable) is the panic-free,
    // allocation-minimal way to render the dotted number.
    section[start..=rank].iter().map(u32::to_string).collect::<Vec<_>>().join(".")
}

impl Document {
    /// Assign every defined, numberable label its rendered **number** (LTXDOC03 S4).
    ///
    /// Walks the body **once** in [`Document::walk`] pre-order, threading a single [`Counters`]
    /// state. For each block:
    ///
    /// - a **numbered** [`Block::Section`] (`numbered == true`) increments its depth's section
    ///   counter, resets deeper depths, and — if it carries a hoisted `label` — records that label's
    ///   dotted number. A **starred** `\section*` (`numbered == false`) is skipped: it fires no
    ///   counter and is never numbered.
    /// - **every** [`Block::Figure`] advances the flat figure counter; **every** [`Block::Table`]
    ///   advances the flat table counter (labeled or not — a `\label` only *captures* the value). A
    ///   figure/table that carries a `label` records that label's flat number.
    ///
    /// Inline (`\label` in prose / after an equation) labels carry **no** S4 counter — their target
    /// is typically an equation, deferred to S5 — so they are **omitted** from the returned
    /// [`Numbering`]. Only the first definition of a duplicated key is numbered (matching S1's
    /// first-definition-wins).
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (the counter array is
    /// fixed-size and every index is range-checked); reuses the bounded [`Document::walk`] (no new
    /// recursion). Borrows `self` immutably; the returned [`Numbering`] is owned plain data (keys +
    /// numbers copied out), so it outlives any borrow of the source. The tree is **not** mutated —
    /// numbering is pure analysis, leaving S1/S2/S3 outputs byte-for-byte unchanged.
    pub fn number_labels(&self) -> Numbering {
        let mut counters = Counters::new();
        let mut labels: Vec<NumberedLabel> = Vec::new();

        // Record the first definition of each key only (first-wins, matching S1).
        let mut record = |key: &str, kind: LabelKind, number: String| {
            if !labels.iter().any(|l| l.key == key) {
                labels.push(NumberedLabel { key: key.to_string(), kind, number });
            }
        };

        for node in self.walk() {
            let NodeRef::Block(block) = node else {
                continue; // Only blocks carry the section/float counters S4 assigns.
            };
            match block {
                // A numbered section: step the counter (reset deeper), number its label if any.
                Block::Section { level, numbered: true, label, .. } => {
                    let number = counters.step_section(section_rank(*level));
                    if let Some(key) = label {
                        record(key, LabelKind::Section, number);
                    }
                }
                // A starred `\section*`: fires no counter, is never numbered — skipped.
                Block::Section { numbered: false, .. } => {}
                // Every figure advances the flat figure counter (labeled or not).
                Block::Figure { label, .. } => {
                    let n = counters.step_figure();
                    if let Some(key) = label {
                        record(key, LabelKind::Figure, n.to_string());
                    }
                }
                // Every table advances the independent flat table counter (labeled or not).
                Block::Table { label, .. } => {
                    let n = counters.step_table();
                    if let Some(key) = label {
                        record(key, LabelKind::Table, n.to_string());
                    }
                }
                // LTXDOC03 S8: a **non-starred** display-math env's lifted `\label` is a real,
                // resolvable equation label — and now a **numbered** one. We step the flat equation
                // counter (`\theequation`) and record the row with the real sequential number, so the
                // S6 report prints `Equation 3` rather than the S7 placeholder `Equation ?`. Recording
                // the row — rather than skipping it as we do for `LabelKind::Inline` — is what makes
                // `number_for(key)` return `Some`, so the S6 report *includes* an `\eqref` to this
                // equation. Starred forms set `label: None` (see the D5 lowering), so they never reach
                // here.
                //
                // LaTeX-fidelity limitation: in real LaTeX *every* non-starred display equation
                // consumes the equation counter whether or not it carries a `\label` (like figures and
                // tables — the `\label` only *captures* an already-stepped value). Our AST, however,
                // only marks the **labelled** non-starred case: [`Block::DisplayMath`] carries no
                // `numbered` flag, and the D5 lowering sets `label: None` for *both* starred envs and
                // unlabelled islands (`\[…\]`, `$$…$$`), so an unlabelled-but-numbered `equation` env
                // is indistinguishable from an unnumbered island here. We therefore step the counter
                // **only** for labelled equations. Consequence: if an unlabelled numbered equation
                // sits between two labelled ones, the second labelled equation's number will be one
                // lower than a full LaTeX run would assign. Closing this gap needs a `numbered: bool`
                // on `Block::DisplayMath` (an AST change) and is left to a later slice.
                Block::DisplayMath { label: Some(key), .. } => {
                    record(key, LabelKind::Equation, counters.step_equation().to_string());
                }
                // Any other block carries no counter S4 assigns.
                _ => {}
            }
        }

        Numbering { labels }
    }

    /// The rendered **number** a resolved reference prints (LTXDOC03 S4) — the payoff that ties S1
    /// resolution to S4 numbering: given a [`ResolvedRef`] (`\ref{sec:intro}`), return its target's
    /// number (`"1.2"`), or `None` if the target is not a numberable label (an inline/equation label,
    /// deferred to S5) — a **documented, total** outcome, never a panic.
    ///
    /// Convenience over [`number_labels`](Document::number_labels): it numbers the document, then
    /// looks the reference's `key` up. (A caller numbering *many* references should call
    /// `number_labels` once and reuse the [`Numbering`]; this method re-numbers per call, which is
    /// O(nodes) each — fine for a one-off lookup.)
    pub fn ref_number(&self, r: &ResolvedRef) -> Option<String> {
        self.number_labels().number_for(&r.key).map(str::to_string)
    }

    /// The document's **List of Figures / List of Tables** index (LTXDOC03 S12) — the analogue of
    /// LaTeX's `\listoffigures` / `\listoftables`, rendered as plain text.
    ///
    /// ## What it produces
    ///
    /// Two optional blocks, each a heading followed by one numbered line per float, in **document
    /// order**:
    ///
    /// ```text
    /// List of Figures
    /// 1. <figure 1 caption text>
    /// 2. <figure 2 caption text>
    /// List of Tables
    /// 1. <table 1 caption text>
    /// 2. <table 2 caption text>
    /// ```
    ///
    /// - **Every** [`Block::Figure`] gets a line (labeled or not) numbered `1, 2, 3, …`; then the
    ///   same, independently, for **every** [`Block::Table`]. The numbering mirrors S4 exactly: a
    ///   float's line number is the flat float counter's value at that float, so a `\ref` to a
    ///   labeled float and its List-of index agree. We reuse the same [`Counters`] float walk as
    ///   [`number_labels`](Document::number_labels) so the two can never drift.
    /// - Each line is `<n>. <caption text>`. The **caption text** is the plain rendering of the
    ///   float's `\caption{…}` inlines (text, spaces, and the text inside `\textbf`/`\emph`/other
    ///   font wrappers — the same descent [`ref_target_node`] proves reaches a figure's caption). A
    ///   float that carries **no** `\caption` renders the fixed placeholder `(no caption)`, so the
    ///   numbering stays aligned with the real float count — every float still gets a numbered line.
    /// - The `List of Figures` heading is emitted **only** when there is ≥1 figure; `List of Tables`
    ///   **only** when there is ≥1 table. If the document has **no** floats at all, the fixed marker
    ///   `"(no floats)"` is returned.
    /// - Lines are joined by `\n` with **no** trailing newline and no trailing whitespace.
    ///
    /// ## Additive by construction
    ///
    /// S12 is a brand-new method that reads the existing document blocks; it mutates nothing and
    /// changes no S1–S11 output. Real LaTeX gates the lists on a `\listoffigures` / `\listoftables`
    /// command, but those are not parser-recognized commands here, so — like S11's grouped report —
    /// S12 is exposed as a method the caller invokes directly rather than a gated render.
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing; reuses the bounded
    /// [`Document::walk`] and the fixed-size [`Counters`]. Borrows `self` immutably and returns owned
    /// `String` data, so the result outlives any borrow of the source.
    pub fn list_of_floats(&self) -> String {
        // Two independent, ordered lists — figures and tables — built in a single document-order
        // walk that threads the same `Counters` `number_labels` uses, so the line numbers match S4.
        let mut figures: Vec<String> = Vec::new();
        let mut tables: Vec<String> = Vec::new();
        let mut counters = Counters::new();

        for node in self.walk() {
            let NodeRef::Block(block) = node else {
                continue; // Only float blocks carry the caption + counter S12 renders.
            };
            match block {
                Block::Figure { caption, .. } => {
                    let n = counters.step_figure();
                    figures.push(format!("{n}. {}", caption_text(caption)));
                }
                Block::Table { caption, .. } => {
                    let n = counters.step_table();
                    tables.push(format!("{n}. {}", caption_text(caption)));
                }
                _ => {}
            }
        }

        // Assemble: each list is emitted only when non-empty, heading first. Both empty → marker.
        if figures.is_empty() && tables.is_empty() {
            return "(no floats)".to_string();
        }
        let mut lines: Vec<String> = Vec::new();
        if !figures.is_empty() {
            lines.push("List of Figures".to_string());
            lines.extend(figures);
        }
        if !tables.is_empty() {
            lines.push("List of Tables".to_string());
            lines.extend(tables);
        }
        lines.join("\n")
    }

    /// Resolve every `\nameref{key}` in the body to the **name** (title/caption text) of its target
    /// (LTXDOC03 S13).
    ///
    /// ## What `\nameref` is, and why it needs its own pass
    ///
    /// The `nameref` package's `\nameref{key}` prints the *textual name* of a label's owner rather
    /// than its number: a `\nameref{sec:intro}` typesets **"Introduction"** (the section's title), not
    /// "Section 1"; a `\nameref{fig:p}` typesets the figure's **caption text**. It is the name-valued
    /// sibling of `\ref` (number-valued) and `\pageref` (page-valued).
    ///
    /// Crucially, `"nameref"` is **not** in [`REF_COMMANDS`] — the S1 resolver deliberately binds only
    /// `\ref`/`\eqref`/`\pageref`, so a `\nameref` appears in *neither*
    /// [`ReferenceResolution::resolved`] nor [`ReferenceResolution::unresolved`]. (The scratch parse
    /// confirmed this: `\nameref{sec:intro}` lowers to `Inline::CrossRef { command: "nameref", target:
    /// "sec:intro", .. }`, and `resolve_references()` returns it in no table.) That is *why* S13 is a
    /// brand-new method rather than a tweak to the resolver — it reads the same `\label` table S1
    /// builds, but answers a different question (*what is it called?* not *what number is it?*), and
    /// touches no S1–S12 output.
    ///
    /// ## The walk (mirrors S12's `list_of_floats` shape)
    ///
    /// One document-order [`Document::walk`], collecting every `Inline::CrossRef` whose `command` is
    /// `"nameref"`. Each key is resolved against the winning label table
    /// ([`ReferenceResolution::definition`], the same first-wins table `\ref` uses), then the target's
    /// **name** is read from its defining node:
    ///
    /// - [`LabelKind::Section`] → the section's `title` inlines, flattened via
    ///   [`flatten_inlines_to_text`] (the exact descent S12 uses for captions): `\nameref{sec:intro}`
    ///   → `Introduction`.
    /// - [`LabelKind::Figure`] / [`LabelKind::Table`] → the float's `\caption` text via the shared
    ///   [`caption_text`] (so a `\nameref` and the List-of-Floats entry read the *same* caption). A
    ///   float with no caption yields the same `(no caption)` marker `caption_text` returns.
    /// - [`LabelKind::Equation`] / [`LabelKind::Inline`] → these carry **no** textual name (an
    ///   equation/bare `\label` has a number, not a title), so they render the fixed marker
    ///   `(no name)`. This is the honest boundary: `\nameref` to a nameless target is well-defined but
    ///   has nothing to print.
    /// - a key that **no** `\label` defines → the fixed placeholder `(undefined nameref: <key>)` (the
    ///   name-valued analogue of LaTeX's `??`, echoing S1's honest "undefined" boundary).
    ///
    /// ## The exact rendering contract
    ///
    /// One line per `\nameref`, in body pre-order, formatted `\nameref{<key>} -> <name>` (mirroring the
    /// `\ref{k} -> …` arrow style of the S6 report). Lines are joined by `\n` with **no** trailing
    /// newline. A document with **no** `\nameref` at all returns the fixed marker `(no namerefs)`.
    ///
    /// Concretely, for
    /// `\section{Introduction}\label{sec:intro} … \nameref{sec:intro} … \nameref{fig:p} … \nameref{nope}`:
    ///
    /// ```text
    /// \nameref{sec:intro} -> Introduction
    /// \nameref{fig:p} -> A plot
    /// \nameref{nope} -> (undefined nameref: nope)
    /// ```
    ///
    /// ## Additive by construction
    ///
    /// S13 is a brand-new method that reads existing blocks and the S1 label table; it mutates nothing
    /// and changes no S1–S12 output (`\nameref` was already parsed into `Inline::CrossRef` and already
    /// ignored by the resolver). Like S11/S12 it is a method the caller invokes directly (real LaTeX
    /// gates `\nameref` on loading the `nameref` package, which is not modelled here).
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing; reuses the bounded
    /// [`Document::walk`], the S1 [`Document::resolve_references`] table, and the S3
    /// [`Document::label_def_node`] accessor. Borrows `self` immutably and returns owned `String` data.
    pub fn resolve_namerefs(&self) -> String {
        // The winning label table `\ref` resolves against — first definition of each key wins. We
        // build it once and share it across every `\nameref` lookup below.
        let refs = self.resolve_references();

        let mut lines: Vec<String> = Vec::new();
        for node in self.walk() {
            // Only an inline `\nameref{key}` cross-ref carries a nameref key.
            let NodeRef::Inline(Inline::CrossRef { command, target, .. }) = node else {
                continue;
            };
            if command != "nameref" {
                continue; // A `\ref`/`\eqref`/`\pageref`/`\cite`/`\label` is not a nameref.
            }

            // Resolve the key to its winning definition, then read that node's textual name.
            let name = match refs.definition(target) {
                None => format!("(undefined nameref: {target})"),
                Some(def) => self.nameref_name(def),
            };
            lines.push(format!("\\nameref{{{target}}} -> {name}"));
        }

        if lines.is_empty() {
            return "(no namerefs)".to_string();
        }
        lines.join("\n")
    }

    /// The **name text** a `\nameref` prints for a resolved label definition (LTXDOC03 S13 helper).
    ///
    /// Given the winning [`LabelDef`], reach its defining node via [`Document::label_def_node`] (the S3
    /// span→node accessor) and read the target's name:
    ///
    /// - a [`Block::Section`] → its `title` inlines flattened by [`flatten_inlines_to_text`];
    /// - a [`Block::Figure`]/[`Block::Table`] → its `\caption` via [`caption_text`];
    /// - anything else (an equation/inline label, or the honest edge where the span is not a walked
    ///   node) → the fixed `(no name)` marker.
    ///
    /// Kept separate so the walk in [`Document::resolve_namerefs`] stays a flat collect + render.
    /// Pure and total: no panic, one owned `String` out.
    fn nameref_name(&self, def: &LabelDef) -> String {
        match self.label_def_node(def) {
            Some(NodeRef::Block(Block::Section { title, .. })) => flatten_inlines_to_text(title),
            Some(NodeRef::Block(Block::Figure { caption, .. }))
            | Some(NodeRef::Block(Block::Table { caption, .. })) => caption_text(caption),
            // An equation/inline label (or a non-walked span) has a number, not a name.
            _ => "(no name)".to_string(),
        }
    }

    /// A **per-kind census** of the numbered-label table (LTXDOC03 S14) — how many labels of each
    /// kind this document defines, as a compact plain-text summary.
    ///
    /// ## What it counts, and where the numbers come from
    ///
    /// This is a pure *tally* of the rows [`number_labels`](Document::number_labels) returns, grouped
    /// by [`LabelKind`]. That table is the S4 numbering table — one row per **defined, numberable**
    /// label key — so S14 counts exactly the labels a `\ref` could print a number for. It reuses the
    /// same table (never re-deriving the counts), so the census can never drift from the numbering it
    /// summarises.
    ///
    /// Only four kinds ever reach that table: a numbered `\section`, a `figure`, a `table`, and a
    /// non-starred display `equation` label. A bare inline `\label{…}` ([`LabelKind::Inline`]) is
    /// **not** numbered, so it never appears in `number_labels` and is therefore **not** counted here
    /// (this is confirmed by reading the numbering pass — it records `Inline` rows nowhere).
    ///
    /// ## The exact rendering contract
    ///
    /// One line per kind whose count is `>= 1`, in this **fixed order** (chosen once, so the output is
    /// deterministic and greppable — it never follows document order): **Sections, Figures, Tables,
    /// Equations**. Each line is exactly `"<Kind>: <count>"` with a **fixed plural** label regardless
    /// of the count (so a single section still prints `Sections: 1`, never `Section: 1`):
    ///
    /// ```text
    /// Sections: 2
    /// Figures: 1
    /// Tables: 1
    /// Equations: 1
    /// ```
    ///
    /// A kind whose count is **0 is omitted** entirely (mirroring S11's "kinds with 0 refs are
    /// omitted" convention), so a document with only labeled sections renders just the one
    /// `Sections: N` line. Lines are joined by `\n` with **no** trailing newline (matching S11's
    /// `to_plain_text_by_kind`, S12's `list_of_floats`, and S13's `resolve_namerefs`).
    ///
    /// If **all** counts are 0 — the document defines no numbered label at all — the fixed marker
    /// `"(no labels)"` is returned, so the output is never the empty string (the same stable-marker
    /// discipline S12/S13 use).
    ///
    /// | document | `list_summary()` |
    /// |----------|------------------|
    /// | 2 `\section`+`\label`, 1 fig, 1 table, 1 eq (all labeled) | `Sections: 2`⏎`Figures: 1`⏎`Tables: 1`⏎`Equations: 1` |
    /// | 3 labeled sections, nothing else | `Sections: 3` |
    /// | only a bare inline `\label{marker}` (not numbered) | `(no labels)` |
    ///
    /// ## Additive by construction
    ///
    /// S14 is a brand-new, read-only method that reuses [`number_labels`](Document::number_labels)
    /// and mutates nothing; it changes no S1-S13 output (they are byte-for-byte unchanged) and leaves
    /// the `to_latex` round-trip fixed point intact.
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing; a single pass over the
    /// already-bounded numbering table. Borrows `self` immutably and returns owned `String` data, so
    /// the result outlives any borrow of the source.
    pub fn list_summary(&self) -> String {
        // Tally the numbering table by kind. Inline labels never reach this table (they are not
        // numbered — see `number_labels`), so only the four numbered kinds can be non-zero.
        let numbering = self.number_labels();
        let mut sections = 0u32;
        let mut figures = 0u32;
        let mut tables = 0u32;
        let mut equations = 0u32;
        for label in &numbering.labels {
            match label.kind {
                LabelKind::Section => sections += 1,
                LabelKind::Figure => figures += 1,
                LabelKind::Table => tables += 1,
                LabelKind::Equation => equations += 1,
                // Not numbered, so never present here; counted nowhere.
                LabelKind::Inline => {}
            }
        }

        // Emit one line per non-zero kind in the FIXED order Sections, Figures, Tables, Equations
        // (deterministic, not document order). The plural label is fixed regardless of count.
        let mut lines: Vec<String> = Vec::new();
        if sections >= 1 {
            lines.push(format!("Sections: {sections}"));
        }
        if figures >= 1 {
            lines.push(format!("Figures: {figures}"));
        }
        if tables >= 1 {
            lines.push(format!("Tables: {tables}"));
        }
        if equations >= 1 {
            lines.push(format!("Equations: {equations}"));
        }

        if lines.is_empty() {
            // No numbered label at all → the fixed marker, never the empty string.
            return "(no labels)".to_string();
        }
        lines.join("\n")
    }

    /// The resolved citations **grouped by the source `\cite` they came from** (LTXDOC03 S15) — the
    /// citation-family parallel of S11's `to_plain_text_by_kind` (which groups resolved *references*)
    /// and S13's `resolve_namerefs` (one rendered line per target).
    ///
    /// ## What it does
    ///
    /// S2's [`Document::resolve_citations`] flattens a multi-key `\cite{a,b}` into *several*
    /// [`ResolvedCite`] rows — one per key — every one carrying that single `\cite`'s
    /// [`cite_span`](ResolvedCite::cite_span). This method reads only that `resolved` list and
    /// re-assembles it: it groups the rows back by `cite_span` (so all keys of one `\cite` reunite)
    /// and emits **one line per source `\cite`** that resolved at least one key.
    ///
    /// ## The exact rendering contract
    ///
    /// Each line is the citing command **reconstructed from its resolved keys**:
    /// `\cite{` + the group's keys joined by `", "` + `}`. The keys are **only the resolved ones**,
    /// in their original left-to-right order — a *dangling* key (one no `\bibitem` defines) is
    /// **excluded**, so a `\cite{a,ghost}` where only `a` resolves renders `\cite{a}`, not
    /// `\cite{a,ghost}`. (We reconstruct rather than slice `&src[cite_span]` precisely because the
    /// source text would still contain the dangling `ghost`; reconstruction shows exactly what
    /// *bound*.)
    ///
    /// The groups appear in **first-appearance order of their `cite_span`** — i.e. the source order of
    /// the `\cite`s (S2's `resolved` is already in body pre-order, so the first time each distinct
    /// `cite_span` is seen fixes that group's position). Lines are joined by `\n` with **no** trailing
    /// newline (matching S11's `to_plain_text_by_kind`, S12's `list_of_floats`, S13's
    /// `resolve_namerefs`, and S14's `list_summary`).
    ///
    /// A document with **no** resolved citations — none present, or every cited key dangling — returns
    /// the fixed marker `(no resolved citations)`, never the empty string (the same stable-marker
    /// discipline S12/S13/S14 use).
    ///
    /// Concretely, for a body `\cite{smith2020, jones2019}` (both defined) then `\cite{a, ghost}`
    /// (only `a` defined), against a bibliography defining `smith2020`, `jones2019`, `a`:
    ///
    /// ```text
    /// \cite{smith2020, jones2019}
    /// \cite{a}
    /// ```
    ///
    /// ## Additive by construction
    ///
    /// S15 is a brand-new, read-only method that reuses [`resolve_citations`](Document::resolve_citations)
    /// and mutates nothing; it changes no S1-S14 output (they are byte-for-byte unchanged) and leaves
    /// the `to_latex` round-trip fixed point intact.
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all —
    /// keys are already owned `String`s); a single pass over the already-bounded `resolved` list.
    /// Borrows `self` immutably and returns owned `String` data, so the result outlives any borrow of
    /// the source.
    pub fn citations_by_source(&self) -> String {
        // S2 already flattened every `\cite` into per-key `ResolvedCite` rows in body pre-order, each
        // tagged with its source `\cite`'s span. We only read that list.
        let resolution = self.resolve_citations();

        // Group the resolved keys by their shared `cite_span`, preserving the FIRST-APPEARANCE order
        // of the cite_spans (source order of the `\cite`s). A `Vec` of `(cite_span, keys)` — not a
        // hash map — keeps that order deterministic and the code allocation-light (the number of
        // distinct `\cite`s is small). Keys within a group stay in their existing left-to-right order.
        let mut groups: Vec<(Span, Vec<&str>)> = Vec::new();
        for cite in &resolution.resolved {
            match groups.iter_mut().find(|(span, _)| *span == cite.cite_span) {
                Some((_, keys)) => keys.push(&cite.key),
                None => groups.push((cite.cite_span, vec![&cite.key])),
            }
        }

        if groups.is_empty() {
            // No `\cite` resolved a single key → the fixed marker, never the empty string.
            return "(no resolved citations)".to_string();
        }

        // One line per source `\cite`: `\cite{` + resolved keys joined by `", "` + `}`. Dangling keys
        // never entered `resolved`, so they are excluded here by construction.
        groups
            .iter()
            .map(|(_, keys)| format!("\\cite{{{}}}", keys.join(", ")))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The **duplicate (multiply-defined) bibliography entries** (LTXDOC03 S16) — the citation-family
    /// parallel of S6's *"Dangling citations"* footer, but for the *other* bibliography warning LaTeX
    /// emits: *"Citation `key' multiply defined"*.
    ///
    /// ## What it does
    ///
    /// S2's [`Document::resolve_citations`] collects every `\bibitem{key}` inside a `thebibliography`
    /// in [`Document::walk`] pre-order. The **first** `\bibitem` of each key wins (it becomes the row
    /// citations resolve against); every **later** `\bibitem` of an already-defined key is a *losing*
    /// duplicate, recorded in [`CitationResolution::duplicate_entries`]. No method has surfaced that
    /// list until now — this method renders it, one warning per losing `\bibitem`.
    ///
    /// ## The exact rendering contract
    ///
    /// One line per losing duplicate, in the existing pre-order of `duplicate_entries` (the source
    /// order of the offending `\bibitem`s — **not** re-sorted). Each line is the offending command
    /// **reconstructed from its key**: `\bibitem{` + the duplicate's key + `}`. We reconstruct from the
    /// owned key rather than slice `&src[span]` — matching how S13's `resolve_namerefs` and S15's
    /// `citations_by_source` rebuild commands from keys — so the render needs no source borrow and can
    /// never index out of bounds.
    ///
    /// **Every** losing `\bibitem` yields its own line; we do **not** de-duplicate. If a key is defined
    /// *three* times, the second and third both lose, so two `\bibitem{key}` lines are emitted (one per
    /// *"multiply defined"* warning LaTeX would raise) — surfacing every duplicate, not the fact that a
    /// key is duplicated. The winning first `\bibitem` is never listed here (it is the entry in
    /// [`CitationResolution::entries`], not a duplicate).
    ///
    /// A document with **no** duplicate entries — no bibliography, or every key defined exactly once —
    /// returns the fixed marker `(no duplicate bibliography entries)`, never the empty string (the same
    /// stable-marker discipline S12/S13/S14/S15 use). Lines are joined by `\n` with **no** trailing
    /// newline (matching S11's `to_plain_text_by_kind`, S12's `list_of_floats`, S13's
    /// `resolve_namerefs`, S14's `list_summary`, and S15's `citations_by_source`).
    ///
    /// Concretely, for a `thebibliography` that defines `smith` twice and `jones` once:
    ///
    /// ```text
    /// \begin{thebibliography}{9}
    /// \bibitem{smith} First Smith. 1990.
    /// \bibitem{jones} Jones. 1991.
    /// \bibitem{smith} Second Smith. 1992.
    /// \end{thebibliography}
    /// ```
    ///
    /// only the *second* `\bibitem{smith}` loses, so the report is the single line:
    ///
    /// ```text
    /// \bibitem{smith}
    /// ```
    ///
    /// ## Additive by construction
    ///
    /// S16 is a brand-new, read-only method that reuses [`resolve_citations`](Document::resolve_citations)
    /// and mutates nothing; it changes no S1-S15 output (they are byte-for-byte unchanged) and leaves
    /// the `to_latex` round-trip fixed point intact.
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all —
    /// keys are already owned `String`s); a single pass over the already-bounded `duplicate_entries`
    /// list. Borrows `self` immutably and returns owned `String` data, so the result outlives any borrow
    /// of the source.
    pub fn duplicate_bibliography_entries(&self) -> String {
        // S2 already routed every later `\bibitem` of an already-defined key into `duplicate_entries`,
        // in body pre-order (first-entry-wins). We only read that list.
        let resolution = self.resolve_citations();

        if resolution.duplicate_entries.is_empty() {
            // No multiply-defined `\bibitem` → the fixed marker, never the empty string.
            return "(no duplicate bibliography entries)".to_string();
        }

        // One line per losing duplicate, in the existing pre-order (NOT re-sorted; NOT de-duplicated —
        // every "multiply defined" warning gets its own line). Reconstruct `\bibitem{key}` from the
        // owned key, so there is no source slicing.
        resolution
            .duplicate_entries
            .iter()
            .map(|dup| format!("\\bibitem{{{}}}", dup.key))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The **unresolved (dangling) citations grouped by their source `\cite`** (LTXDOC03 S17) — the
    /// citation-family parallel of S6's flat *"Dangling citations"* footer, but rendered **per source
    /// `\cite`**: a distinct new view, and the DANGLING-key mirror of S15's
    /// [`citations_by_source`](Document::citations_by_source).
    ///
    /// ## What it does
    ///
    /// S2's [`Document::resolve_citations`] flattens every `\cite` into per-key rows, splitting them
    /// into the **resolved** keys (those a `\bibitem` defines) and the **unresolved** keys (LaTeX's
    /// *"Citation `key' undefined"*, the `[?]` in the output), each row tagged with the citing `\cite`'s
    /// own `cite_span`. S6 already reports the dangling keys as one *flat* footer line; S15 groups the
    /// *resolved* keys per source `\cite`. S17 fills the remaining cell of that 2×2: it groups the
    /// *dangling* keys per source `\cite`, emitting one reconstructed `\cite{…}` line per source `\cite`
    /// that has **at least one** dangling key.
    ///
    /// ## The exact rendering contract
    ///
    /// Read [`CitationResolution::unresolved`] and group its keys by their shared `cite_span`,
    /// preserving the **first-appearance order** of the cite_spans (source order of the `\cite`s) — a
    /// `Vec` of `(cite_span, keys)`, **not** a hash map, so the order is deterministic. Keys within a
    /// group stay in their existing left-to-right order.
    ///
    /// One line per source `\cite` with ≥1 dangling key: `\cite{` + that group's dangling keys joined
    /// by `", "` + `}`, reconstructed from the owned `key` `String`s (no source slicing, matching S13's
    /// `resolve_namerefs`, S15's `citations_by_source`, and S16's `duplicate_bibliography_entries`).
    /// Because `unresolved` holds **only** the dangling keys, a `\cite{a, ghost}` where `a` resolves and
    /// `ghost` dangles renders `\cite{ghost}` (only the dangling key) — the exact analogue of how S15
    /// shows only the *resolved* keys of a mixed `\cite`.
    ///
    /// A document with **no** unresolved citations — every cited key resolves, or there are no
    /// citations at all — returns the fixed marker `(no unresolved citations)`, never the empty string
    /// (the same stable-marker discipline S12/S13/S14/S15/S16 use). Lines are joined by `\n` with **no**
    /// trailing newline (matching S11's `to_plain_text_by_kind`, S12's `list_of_floats`, S13's
    /// `resolve_namerefs`, S14's `list_summary`, S15's `citations_by_source`, and S16's
    /// `duplicate_bibliography_entries`).
    ///
    /// Concretely, for a body citing `\cite{a, ghost}` (where `a` resolves) and `\cite{x, y}` (neither
    /// defined):
    ///
    /// ```text
    /// \cite{ghost}
    /// \cite{x, y}
    /// ```
    ///
    /// The first line drops the resolved `a` and keeps only the dangling `ghost`; the second reunites
    /// both dangling keys of the fully-dangling `\cite` on one comma-space-joined line, in source order.
    ///
    /// ## Additive by construction
    ///
    /// S17 is a brand-new, read-only method that reuses [`resolve_citations`](Document::resolve_citations)
    /// and mutates nothing; it changes no S1-S16 output (they are byte-for-byte unchanged) and leaves
    /// the `to_latex` round-trip fixed point intact.
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all —
    /// keys are already owned `String`s); a single pass over the already-bounded `unresolved` list.
    /// Borrows `self` immutably and returns owned `String` data, so the result outlives any borrow of
    /// the source.
    pub fn unresolved_citations_by_source(&self) -> String {
        // S2 already flattened every `\cite` into per-key rows and routed the dangling keys into
        // `unresolved` (in body pre-order, and within a multi-key `\cite` in left-to-right order), each
        // tagged with its source `\cite`'s span. We only read that list.
        let resolution = self.resolve_citations();

        // Group the dangling keys by their shared `cite_span`, preserving the FIRST-APPEARANCE order of
        // the cite_spans (source order of the `\cite`s). A `Vec` of `(cite_span, keys)` — not a hash
        // map — keeps that order deterministic and the code allocation-light (the number of distinct
        // `\cite`s is small). Keys within a group stay in their existing left-to-right order.
        let mut groups: Vec<(Span, Vec<&str>)> = Vec::new();
        for cite in &resolution.unresolved {
            match groups.iter_mut().find(|(span, _)| *span == cite.cite_span) {
                Some((_, keys)) => keys.push(&cite.key),
                None => groups.push((cite.cite_span, vec![&cite.key])),
            }
        }

        if groups.is_empty() {
            // Every cited key resolved (or there were no citations) → the fixed marker, never the empty
            // string.
            return "(no unresolved citations)".to_string();
        }

        // One line per source `\cite` with ≥1 dangling key: `\cite{` + its dangling keys joined by
        // `", "` + `}`. Resolved keys never entered `unresolved`, so they are excluded by construction.
        groups
            .iter()
            .map(|(_, keys)| format!("\\cite{{{}}}", keys.join(", ")))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The **unresolved (dangling) references grouped by their source `\ref`** (LTXDOC03 S18) — the
    /// `\ref`-family parallel of S17's dangling-CITATION report, and the DANGLING mirror of the resolved
    /// `\ref` family: one reconstructed `\<command>{key}` line per dangling reference, **command-aware**
    /// so `\eqref` and `\pageref` render as themselves.
    ///
    /// ## What it does
    ///
    /// S3's [`Document::resolve_references`] walks every `\ref`/`\eqref`/`\pageref` in body pre-order and
    /// splits them into the **resolved** references (those a `\label` defines) and the **unresolved**
    /// references (LaTeX's *"Reference `key' undefined"*, the `??` in the output), each recorded in
    /// [`ReferenceResolution::unresolved`] as an [`UnresolvedRef`] carrying its dangling `key`, the
    /// `command` that was used (`"ref"` / `"eqref"` / `"pageref"`), and its own `ref_span`. S6 already
    /// reports the dangling keys as one *flat* comma-joined footer (`"Dangling references: k1, k2"`); S18
    /// is a **distinct** view — it reconstructs each dangling reference on **its own line**, preserving
    /// the command it was written with, so a caller can point at the exact offending `\eqref{…}` rather
    /// than a flattened `\ref`-shaped list.
    ///
    /// ## The exact rendering contract
    ///
    /// Read [`ReferenceResolution::unresolved`] and group its entries by their shared `ref_span`,
    /// preserving the **first-appearance order** of the ref_spans (source order of the references) — a
    /// `Vec` of `(ref_span, keys)`, **not** a hash map, so the order is deterministic and the code reads
    /// identically to S17's grouping. Unlike a `\cite{a, b}` (multi-key), a `\ref`/`\eqref`/`\pageref`
    /// takes exactly **one** key, so every group holds a single entry — the structural mirror is kept for
    /// readability, but each group emits exactly one line.
    ///
    /// One line per dangling reference: `\` + that reference's own `command` + `{` + its `key` + `}`,
    /// reconstructed from the owned `command`/`key` `String`s (no source slicing, matching S13's
    /// `resolve_namerefs`, S15's `citations_by_source`, and S17's `unresolved_citations_by_source`).
    /// Because the line is rebuilt from the ref's **own** `command`, a dangling `\eqref{eq:x}` renders
    /// `\eqref{eq:x}` and a dangling `\pageref{p}` renders `\pageref{p}` — the command is **never**
    /// hard-coded to `\ref`.
    ///
    /// A document with **no** unresolved references — every reference resolves, or there are none at all
    /// — returns the fixed marker `(no unresolved references)`, never the empty string (the same
    /// stable-marker discipline S12/S13/S14/S15/S16/S17 use). Lines are joined by `\n` with **no**
    /// trailing newline (matching S15's `citations_by_source` and S17's
    /// `unresolved_citations_by_source`).
    ///
    /// Concretely, for a body with `\eqref{eq:ghost}` and `\pageref{p:ghost}` (neither defined by a
    /// `\label`):
    ///
    /// ```text
    /// \eqref{eq:ghost}
    /// \pageref{p:ghost}
    /// ```
    ///
    /// Each line preserves the command it was written with, one dangling reference per line, in source
    /// order.
    ///
    /// ## Additive by construction
    ///
    /// S18 is a brand-new, read-only method that reuses [`resolve_references`](Document::resolve_references)
    /// and mutates nothing; it changes no S1-S17 output (they are byte-for-byte unchanged) and leaves the
    /// `to_latex` round-trip fixed point intact.
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all —
    /// `command` and `key` are already owned `String`s); a single pass over the already-bounded
    /// `unresolved` list. Borrows `self` immutably and returns owned `String` data, so the result
    /// outlives any borrow of the source.
    pub fn unresolved_references_by_source(&self) -> String {
        // S3 already split every `\ref`/`\eqref`/`\pageref` into resolved and dangling entries, routing
        // the dangling ones into `unresolved` (in body pre-order), each carrying its own `command`,
        // `key`, and `ref_span`. We only read that list.
        let resolution = self.resolve_references();

        // Group the dangling references by their shared `ref_span`, preserving the FIRST-APPEARANCE
        // order of the ref_spans (source order of the references). A `Vec` of `(ref_span, refs)` — not a
        // hash map — keeps that order deterministic and mirrors S17's grouping idiom exactly. Unlike a
        // multi-key `\cite`, each reference takes exactly one key, so every group holds a single entry;
        // the structural mirror is kept only so this code reads identically to S17.
        let mut groups: Vec<(Span, Vec<&UnresolvedRef>)> = Vec::new();
        for u in &resolution.unresolved {
            match groups.iter_mut().find(|(span, _)| *span == u.ref_span) {
                Some((_, refs)) => refs.push(u),
                None => groups.push((u.ref_span, vec![u])),
            }
        }

        if groups.is_empty() {
            // Every reference resolved (or there were none) → the fixed marker, never the empty string.
            return "(no unresolved references)".to_string();
        }

        // One line per dangling reference, reconstructed from its OWN command and key: `\<command>{key}`.
        // We iterate each group's single entry so `\eqref` / `\pageref` render as themselves rather than
        // all being flattened to `\ref`.
        groups
            .iter()
            .flat_map(|(_, refs)| {
                refs.iter()
                    .map(|u| format!("\\{}{{{}}}", u.command, u.key))
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The **winning bibliography entries as a numbered list** (LTXDOC03 S19) — the rendered
    /// bibliography a reader actually sees: one numbered line per *winning* `\bibitem`, the table
    /// citations resolve against.
    ///
    /// ## What it does
    ///
    /// S2's [`Document::resolve_citations`] collects every `\bibitem{key}` inside a `thebibliography`
    /// environment, keeping the **first** `\bibitem` of each distinct key as the *winning* entry (in
    /// [`CitationResolution::entries`], body pre-order) and routing every later re-definition into the
    /// *losing* [`CitationResolution::duplicate_entries`]. The other renderers view neighboring cells
    /// of that resolution: S16's [`duplicate_bibliography_entries`](Document::duplicate_bibliography_entries)
    /// renders the **losing** `duplicate_entries` (as `\bibitem{key}` warning lines); S15's
    /// [`citations_by_source`](Document::citations_by_source) renders the per-source *resolved cite
    /// keys*. S19 fills the remaining view: the **winning** `entries` themselves, rendered as a
    /// bibliography looks — a distinct, brand-new view over the same resolution.
    ///
    /// ## The exact rendering contract
    ///
    /// Read [`CitationResolution::entries`] in its existing pre-order and number it **1-based**, one
    /// line per winning entry: `format!("[{}] {}", n, entry.key)` → `[1] smith2020`, `[2] jones2019`,
    /// … The `[n] key` shape is chosen **deliberately** so this reads as a *rendered bibliography* and
    /// is visually distinct from S16's `\bibitem{key}` losing-duplicate lines — the two never look
    /// alike even when they list overlapping keys. Each line is reconstructed from the entry's **owned
    /// `key` `String`** (no source slicing at all, matching S13's `resolve_namerefs`, S15's
    /// `citations_by_source`, S16's `duplicate_bibliography_entries`, and S17/S18's dangling reports).
    ///
    /// Because `entries` already holds only the **first** `\bibitem` of each key (later re-definitions
    /// live in `duplicate_entries`, never here), a `\bibitem{dup}` written twice appears **once** in
    /// this list — the winning entry — exactly as a real bibliography renders one line per key.
    ///
    /// A document with **no** bibliography entries — no `thebibliography`, or an empty one — returns
    /// the fixed marker `(no bibliography entries)`, never the empty string (the same stable-marker
    /// discipline S12/S13/S14/S15/S16/S17/S18 use). Lines are joined by `\n` with **no** trailing
    /// newline (matching S11's `to_plain_text_by_kind` and every S12-S18 renderer).
    ///
    /// Concretely, for a `thebibliography` defining `smith` (twice) and `jones`:
    ///
    /// ```text
    /// \begin{thebibliography}{9}
    /// \bibitem{smith} First Smith. 1990.
    /// \bibitem{jones} Jones. 1991.
    /// \bibitem{smith} Second Smith. 1992.
    /// \end{thebibliography}
    /// ```
    ///
    /// the winning list is the two numbered lines (the second `\bibitem{smith}` is a duplicate, not a
    /// second entry):
    ///
    /// ```text
    /// [1] smith
    /// [2] jones
    /// ```
    ///
    /// ## Additive by construction
    ///
    /// S19 is a brand-new, read-only method that reuses [`resolve_citations`](Document::resolve_citations)
    /// and mutates nothing; it changes no S1-S18 output (they are byte-for-byte unchanged) and leaves
    /// the `to_latex` round-trip fixed point intact.
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all —
    /// keys are already owned `String`s); a single pass over the already-bounded `entries` list.
    /// Borrows `self` immutably and returns owned `String` data, so the result outlives any borrow of
    /// the source.
    pub fn bibliography_entries(&self) -> String {
        // S2 already collected the winning entries — the first `\bibitem` of each distinct key, in
        // body pre-order (later re-definitions went to `duplicate_entries`, not here). We only read
        // that list.
        let resolution = self.resolve_citations();

        if resolution.entries.is_empty() {
            // No `thebibliography` (or an empty one) → the fixed marker, never the empty string.
            return "(no bibliography entries)".to_string();
        }

        // One numbered line per winning entry, 1-based, in the existing pre-order. `enumerate()` is
        // 0-based, so `n + 1` gives the 1-based label. Reconstruct `[n] key` from the owned key, so
        // there is no source slicing; the `[n] key` shape is deliberately distinct from S16's
        // `\bibitem{key}` losing-duplicate lines.
        resolution
            .entries
            .iter()
            .enumerate()
            .map(|(n, entry)| format!("[{}] {}", n + 1, entry.key))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The **losing duplicate label definitions** (LTXDOC03 S20) — the label-family mirror of S16's
    /// [`duplicate_bibliography_entries`](Document::duplicate_bibliography_entries).
    ///
    /// ## What it does
    ///
    /// S1's [`Document::resolve_references`] splits every `\label` into the **winning** first
    /// definition of each key ([`ReferenceResolution::definitions`]) and the **losing** later
    /// re-definitions of an already-defined key ([`ReferenceResolution::duplicates`] — LaTeX's *"Label
    /// `key' multiply defined"* warning). S16 renders the losing `\bibitem` duplicates; S20 is the
    /// exact `\label` parallel: it renders the losing `\label` duplicates, one reconstructed
    /// `\label{key}` line per multiply-defined definition. The winning first definition of each key
    /// lives in `definitions` (and is what `\ref`/`\eqref`/`\pageref` resolve against), never here.
    ///
    /// ## The exact rendering contract
    ///
    /// One line per losing duplicate, in the existing pre-order (**NOT** re-sorted, **NOT**
    /// de-duplicated — every *"multiply defined"* warning gets its own line, exactly like S16). Each
    /// line is reconstructed as `\label{` + `dup.key` + `}` from the owned `key` `String` — there is
    /// **no** source slicing at all (matching S13's `resolve_namerefs`, S15's `citations_by_source`,
    /// S16's `duplicate_bibliography_entries`, and S17-S19's reports). Labels are always *defined* by
    /// `\label{…}`, so `\label{key}` is the correct reconstruction regardless of the duplicate's
    /// [`LabelKind`] (a re-`\label`ed section, figure, equation, or bare inline label all render the
    /// same `\label{key}` form).
    ///
    /// A document with **no** duplicate labels — every key defined exactly once, or no labels at all —
    /// returns the fixed marker `(no duplicate label definitions)`, never the empty string (the same
    /// stable-marker discipline S12-S19 use). Lines are joined by `\n` with **no** trailing newline
    /// (matching S11's `to_plain_text_by_kind` and every S12-S19 renderer).
    ///
    /// Concretely, for a body that writes `\label{dup}` twice (and `\label{once}` once):
    ///
    /// ```text
    /// First \label{dup} here.  Second \label{dup} there.  And \label{once}.
    /// ```
    ///
    /// only the *second* `\label{dup}` loses, so the report is the single line:
    ///
    /// ```text
    /// \label{dup}
    /// ```
    ///
    /// ## Additive by construction
    ///
    /// S20 is a brand-new, read-only method that reuses [`resolve_references`](Document::resolve_references)
    /// and mutates nothing; it changes no S1-S19 output (they are byte-for-byte unchanged) and leaves
    /// the `to_latex` round-trip fixed point intact.
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all —
    /// keys are already owned `String`s); a single pass over the already-bounded `duplicates` list.
    /// Borrows `self` immutably and returns owned `String` data, so the result outlives any borrow of
    /// the source.
    pub fn duplicate_label_definitions(&self) -> String {
        // S1 already routed every later `\label` of an already-defined key into `duplicates`, in body
        // pre-order (first-definition-wins). We only read that list.
        let resolution = self.resolve_references();

        if resolution.duplicates.is_empty() {
            // No multiply-defined `\label` → the fixed marker, never the empty string.
            return "(no duplicate label definitions)".to_string();
        }

        // One line per losing duplicate, in the existing pre-order (NOT re-sorted; NOT de-duplicated —
        // every "multiply defined" warning gets its own line). Reconstruct `\label{key}` from the
        // owned key, so there is no source slicing; `\label{…}` is the right form for any LabelKind.
        resolution
            .duplicates
            .iter()
            .map(|dup| format!("\\label{{{}}}", dup.key))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The **winning label definitions** (LTXDOC03 S22) — the `\label{key}` definitions that
    /// references resolve against, the label-family analogue of S19's
    /// [`bibliography_entries`](Document::bibliography_entries) and the *winning-side* counterpart of
    /// S20's [`duplicate_label_definitions`](Document::duplicate_label_definitions).
    ///
    /// ## What it does
    ///
    /// S1's [`Document::resolve_references`] splits every `\label` into the **winning** first
    /// definition of each key ([`ReferenceResolution::definitions`] — one row per distinct key, in
    /// [`Document::walk`] pre-order, and precisely what `\ref`/`\eqref`/`\pageref` resolve against) and
    /// the **losing** later re-definitions ([`ReferenceResolution::duplicates`] — LaTeX's *"Label
    /// `key' multiply defined"* warning). The neighboring label renderers view the other cells of that
    /// resolution: S20's [`duplicate_label_definitions`](Document::duplicate_label_definitions) renders
    /// the **losing** `duplicates` (as `\label{key}` warning lines). S22 fills the remaining view: the
    /// **winning** `definitions` themselves — one reconstructed `\label{key}` line per distinct key, in
    /// the order the definitions were first seen. It is the exact `\label` parallel of S19, which
    /// renders the winning `\bibitem` entries.
    ///
    /// ## The exact rendering contract
    ///
    /// Read [`ReferenceResolution::definitions`] in its existing pre-order and render one line per
    /// winning definition: `\label{` + `def.key` + `}`. The list is **NOT** re-sorted and **NOT**
    /// de-duplicated — but no de-duplication is needed, because `definitions` already holds exactly one
    /// row per distinct key (every later re-definition of an already-defined key went to `duplicates`,
    /// S20's domain, never here). Each line is reconstructed from the definition's **owned `key`
    /// `String`** — there is **no** source slicing at all (matching S13's `resolve_namerefs`, S15's
    /// `citations_by_source`, S16's `duplicate_bibliography_entries`, S19's `bibliography_entries`, and
    /// S20's `duplicate_label_definitions`). Labels are always *defined* by `\label{…}`, so `\label{key}`
    /// is the correct reconstruction regardless of the definition's [`LabelKind`] (a section, figure,
    /// equation, or bare inline label all render the same `\label{key}` form).
    ///
    /// A document with **no** label definitions returns the fixed marker `(no label definitions)`,
    /// never the empty string (the same stable-marker discipline S12-S21 use). Lines are joined by `\n`
    /// with **no** trailing newline (matching S11's `to_plain_text_by_kind` and every S12-S21 renderer).
    ///
    /// Concretely, for a body that defines `\label{sec:intro}` (a section), `\label{eq:main}` (an
    /// equation), and then re-uses `\label{sec:intro}` on a later subsection:
    ///
    /// ```text
    /// \section{Intro}\label{sec:intro}
    /// \begin{equation}\label{eq:main} x=1 \end{equation}
    /// \subsection{Dup}\label{sec:intro}
    /// ```
    ///
    /// only the *first* `\label{sec:intro}` wins, so the winning key `sec:intro` appears **once**
    /// (its later re-definition is a duplicate, surfaced by S20, not a second `definitions` row):
    ///
    /// ```text
    /// \label{sec:intro}
    /// \label{eq:main}
    /// ```
    ///
    /// ## Additive by construction
    ///
    /// S22 is a brand-new, read-only method that reuses [`resolve_references`](Document::resolve_references)
    /// and mutates nothing; it changes no S1-S21 output (they are byte-for-byte unchanged) and leaves
    /// the `to_latex` round-trip fixed point intact.
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all —
    /// keys are already owned `String`s); a single pass over the already-bounded `definitions` list.
    /// Borrows `self` immutably and returns owned `String` data, so the result outlives any borrow of
    /// the source.
    pub fn label_definitions(&self) -> String {
        // S1 already collected the winning definitions — the first `\label` of each distinct key, in
        // body pre-order (later re-definitions went to `duplicates`, not here). We only read that list.
        let resolution = self.resolve_references();

        if resolution.definitions.is_empty() {
            // No `\label` at all → the fixed marker, never the empty string.
            return "(no label definitions)".to_string();
        }

        // One line per winning definition, in the existing pre-order (NOT re-sorted; NOT de-duplicated —
        // `definitions` already holds exactly one row per distinct key). Reconstruct `\label{key}` from
        // the owned key, so there is no source slicing; `\label{…}` is the right form for any LabelKind.
        resolution
            .definitions
            .iter()
            .map(|def| format!("\\label{{{}}}", def.key))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The **winning label definitions grouped by their [`LabelKind`]** (LTXDOC03 S23) — a per-kind
    /// census of the `\label` definitions, the by-kind grouping companion of S22's
    /// [`label_definitions`](Document::label_definitions) (the same discipline S18's
    /// [`unresolved_references_by_source`](Document::unresolved_references_by_source) and S17's
    /// [`unresolved_citations_by_source`](Document::unresolved_citations_by_source) bring to the
    /// reference/citation families).
    ///
    /// ## What it does
    ///
    /// S1's [`Document::resolve_references`] collects the **winning** first definition of each `\label`
    /// key into [`ReferenceResolution::definitions`] — one row per distinct key, in
    /// [`Document::walk`] pre-order, each tagged with the [`LabelKind`] of the node it defines (a
    /// re-`\label`ed section, table, figure, equation, or bare inline label). S22's
    /// [`label_definitions`](Document::label_definitions) renders that list **flat**, one
    /// `\label{key}` per line in pure pre-order. S23 renders the *same* winning `definitions` list
    /// **grouped by kind**: it walks the [`LabelKind`] variants in a fixed order and, for each kind
    /// that has at least one definition, emits every definition of that kind — so a reader can see, at
    /// a glance, which keys are sections, which are equations, which are bare inline labels. S22 and
    /// S23 are two *views* of one underlying list (S22 by source order, S23 by kind); neither mutates
    /// anything.
    ///
    /// ## The exact rendering contract
    ///
    /// Iterate the [`LabelKind`] variants in their **enum declaration order** —
    /// [`Section`](LabelKind::Section), [`Table`](LabelKind::Table), [`Figure`](LabelKind::Figure),
    /// [`Equation`](LabelKind::Equation), [`Inline`](LabelKind::Inline) — a fixed, deterministic
    /// order that does **not** depend on the document. For each kind, filter
    /// [`ReferenceResolution::definitions`] to the definitions of that kind, **preserving their
    /// existing pre-order** within the kind, and emit one line each. This single stable-ordered pass
    /// (fixed kind order on the outside, pre-order on the inside) keeps the output deterministic
    /// **without** a hash map — mirroring the `Vec`-of-groups discipline S17/S18 use to avoid
    /// hash-order nondeterminism.
    ///
    /// Each line has the shape `[<kind>] \label{<key>}`, where `<kind>` is the stable lowercase tag
    /// from [`LabelKind::as_str`] (`"section"`, `"table"`, `"figure"`, `"equation"`, `"inline"`) and
    /// `<key>` is reconstructed from the definition's **owned `key` `String`** — there is **no** source
    /// slicing at all (matching S13's `resolve_namerefs`, S19's `bibliography_entries`, S20's
    /// `duplicate_label_definitions`, and S22's `label_definitions`). Prefixing every line with its
    /// `[kind]` makes the per-kind census visible on each line while staying one-line-per-definition
    /// and never re-parseable back as raw LaTeX (the `[kind]` prefix is a report annotation, not source
    /// — S23 is a *report*, not a round-trippable rendering). A kind with **no** definitions produces
    /// **no** lines and **no** empty header (there is never a bare `[table]` group for a doc with no
    /// tables).
    ///
    /// A document with **no** label definitions at all returns the fixed marker
    /// `(no label definitions)` — the **same** marker S22 uses (S23 groups the identical list, so the
    /// empty case is identical), never the empty string (the stable-marker discipline S12-S22 share).
    /// Lines are joined by `\n` with **no** trailing newline (matching S11's `to_plain_text_by_kind`
    /// and every S12-S22 renderer).
    ///
    /// Concretely, for a body defining a section label `sec:intro`, an equation label `eq:main`, and a
    /// bare inline label `note`:
    ///
    /// ```text
    /// \section{Intro}\label{sec:intro}
    /// \begin{equation}\label{eq:main} x=1 \end{equation}
    /// \label{note}
    /// ```
    ///
    /// the winning definitions render grouped by kind (`section` sorts before `equation` and `inline`
    /// in the fixed order, so `sec:intro` leads even though `note` is also a definition):
    ///
    /// ```text
    /// [section] \label{sec:intro}
    /// [equation] \label{eq:main}
    /// [inline] \label{note}
    /// ```
    ///
    /// ## Additive by construction
    ///
    /// S23 is a brand-new, read-only method that reuses [`resolve_references`](Document::resolve_references)
    /// and mutates nothing; it changes no S1-S22 output (they are byte-for-byte unchanged) and leaves
    /// the `to_latex` round-trip fixed point intact. It is a *second view* of the same winning
    /// `definitions` list S22 renders flat — grouping never adds, drops, or reorders definitions
    /// relative to what `resolve_references` produced.
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all —
    /// keys are already owned `String`s); a single stable-ordered pass (fixed kind order × pre-order
    /// filter) over the already-bounded `definitions` list. Borrows `self` immutably and returns owned
    /// `String` data, so the result outlives any borrow of the source.
    pub fn label_definitions_by_kind(&self) -> String {
        // S1 already collected the winning definitions — the first `\label` of each distinct key, in
        // body pre-order, each tagged with its `LabelKind`. We only read that list.
        let resolution = self.resolve_references();

        if resolution.definitions.is_empty() {
            // No `\label` at all → the SAME fixed marker S22 uses (S23 groups the identical list).
            return "(no label definitions)".to_string();
        }

        // The FIXED kind order = the enum declaration order. Iterating this explicit slice (rather than
        // a hash map keyed by kind) makes the group order deterministic and document-independent, the
        // same `Vec`-of-groups discipline S17/S18 use to avoid hash-order nondeterminism.
        const KIND_ORDER: [LabelKind; 5] = [
            LabelKind::Section,
            LabelKind::Table,
            LabelKind::Figure,
            LabelKind::Equation,
            LabelKind::Inline,
        ];

        // One stable-ordered pass: for each kind in the fixed order, take the definitions of that kind
        // in their existing pre-order and emit `[<kind>] \label{<key>}`. A kind with no definitions
        // contributes no lines (no empty header). Keys are owned, so there is no source slicing.
        KIND_ORDER
            .iter()
            .flat_map(|kind| {
                resolution
                    .definitions
                    .iter()
                    .filter(move |def| def.kind == *kind)
                    .map(|def| format!("[{}] \\label{{{}}}", def.kind.as_str(), def.key))
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The **resolved (successfully-matched) references grouped by their source `\ref`** (LTXDOC03
    /// S21) — the exact structural mirror of S18's
    /// [`unresolved_references_by_source`](Document::unresolved_references_by_source), but for the
    /// references that **did** resolve: one reconstructed `\<command>{key}` line per resolved
    /// reference, **command-aware** so `\eqref` and `\pageref` render as themselves.
    ///
    /// ## What it does
    ///
    /// S3's [`Document::resolve_references`] walks every `\ref`/`\eqref`/`\pageref` in body pre-order
    /// and splits them into the **resolved** references (those a `\label` defines — recorded in
    /// [`ReferenceResolution::resolved`] as a [`ResolvedRef`]) and the **unresolved** references
    /// (LaTeX's *"Reference `key' undefined"*, the `??` in the output). S18 reports the *dangling*
    /// half of that split; S21 reports the **winning** half — the references that bound to a real
    /// `\label`. Each [`ResolvedRef`] carries the `key`, the `command` that was used (`"ref"` /
    /// `"eqref"` / `"pageref"`), and its own `ref_span` (plus the `target_span`/`target_kind` of the
    /// definition it bound to, which S21 does not need to render). S21 reconstructs each resolved
    /// reference on **its own line**, preserving the command it was written with, so a caller can
    /// point at the exact matched `\eqref{…}` rather than a flattened `\ref`-shaped list.
    ///
    /// ## The exact rendering contract
    ///
    /// Read [`ReferenceResolution::resolved`] and group its entries by their shared `ref_span`,
    /// preserving the **first-appearance order** of the ref_spans (source order of the references) —
    /// a `Vec` of `(ref_span, refs)`, **not** a hash map, so the order is deterministic and the code
    /// reads identically to S18's grouping. As in S18, a `\ref`/`\eqref`/`\pageref` takes exactly
    /// **one** key, so every group holds a single entry — the structural mirror is kept for
    /// readability, but each group emits exactly one line.
    ///
    /// One line per resolved reference: `\` + that reference's own `command` + `{` + its `key` + `}`,
    /// reconstructed from the owned `command`/`key` `String`s (no source slicing, matching S13's
    /// `resolve_namerefs`, S15's `citations_by_source`, S17's `unresolved_citations_by_source`, and
    /// S18's `unresolved_references_by_source`). Because the line is rebuilt from the ref's **own**
    /// `command`, a resolved `\eqref{eq:main}` renders `\eqref{eq:main}` and a resolved
    /// `\pageref{sec:intro}` renders `\pageref{sec:intro}` — the command is **never** hard-coded to
    /// `\ref`. Dangling references never entered `resolved`, so they are excluded by construction (a
    /// `\ref{nope}` with no `\label` appears in S18, not here).
    ///
    /// A document with **no** resolved references — every reference dangles, or there are none at all
    /// — returns the fixed marker `(no resolved references)`, never the empty string (the same
    /// stable-marker discipline S12/S13/S14/S15/S16/S17/S18/S19/S20 use). Lines are joined by `\n`
    /// with **no** trailing newline (matching S15's `citations_by_source` and S18's
    /// `unresolved_references_by_source`).
    ///
    /// Concretely, for a body defining `\label{sec:intro}` and `\label{eq:main}` and then writing
    /// `\ref{sec:intro}`, `\eqref{eq:main}`, and `\pageref{sec:intro}` (all of which resolve):
    ///
    /// ```text
    /// \ref{sec:intro}
    /// \eqref{eq:main}
    /// \pageref{sec:intro}
    /// ```
    ///
    /// Each line preserves the command it was written with, one resolved reference per line, in
    /// source order.
    ///
    /// ## Additive by construction
    ///
    /// S21 is a brand-new, read-only method that reuses [`resolve_references`](Document::resolve_references)
    /// and mutates nothing; it changes no S1-S20 output (they are byte-for-byte unchanged) and leaves
    /// the `to_latex` round-trip fixed point intact.
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all —
    /// `command` and `key` are already owned `String`s); a single pass over the already-bounded
    /// `resolved` list. Borrows `self` immutably and returns owned `String` data, so the result
    /// outlives any borrow of the source.
    pub fn resolved_references_by_source(&self) -> String {
        // S3 already split every `\ref`/`\eqref`/`\pageref` into resolved and dangling entries, routing
        // the resolved ones into `resolved` (in body pre-order), each carrying its own `command`,
        // `key`, and `ref_span`. We only read that list.
        let resolution = self.resolve_references();

        // Group the resolved references by their shared `ref_span`, preserving the FIRST-APPEARANCE
        // order of the ref_spans (source order of the references). A `Vec` of `(ref_span, refs)` — not a
        // hash map — keeps that order deterministic and mirrors S18's grouping idiom exactly. Unlike a
        // multi-key `\cite`, each reference takes exactly one key, so every group holds a single entry;
        // the structural mirror is kept only so this code reads identically to S18.
        let mut groups: Vec<(Span, Vec<&ResolvedRef>)> = Vec::new();
        for r in &resolution.resolved {
            match groups.iter_mut().find(|(span, _)| *span == r.ref_span) {
                Some((_, refs)) => refs.push(r),
                None => groups.push((r.ref_span, vec![r])),
            }
        }

        if groups.is_empty() {
            // Every reference dangled (or there were none) → the fixed marker, never the empty string.
            return "(no resolved references)".to_string();
        }

        // One line per resolved reference, reconstructed from its OWN command and key: `\<command>{key}`.
        // We iterate each group's single entry so `\eqref` / `\pageref` render as themselves rather than
        // all being flattened to `\ref`.
        groups
            .iter()
            .flat_map(|(_, refs)| refs.iter().map(|r| format!("\\{}{{{}}}", r.command, r.key)))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The **resolved (successfully-matched) references grouped by the [`LabelKind`] they resolved
    /// TO** (LTXDOC03 S24) — the exact by-kind grouping companion of S21's
    /// [`resolved_references_by_source`](Document::resolved_references_by_source), the same discipline
    /// S23's [`label_definitions_by_kind`](Document::label_definitions_by_kind) brings to the *label
    /// definitions* family. Where S21 lists the resolved references flat in source order, S24 buckets
    /// the **same** resolved references by the *kind of node each one pointed at*, so a reader can see,
    /// at a glance, which references land on sections, which on equations, which on bare inline labels.
    ///
    /// ## What it does
    ///
    /// S1's [`Document::resolve_references`] walks every `\ref`/`\eqref`/`\pageref` in body pre-order
    /// and splits them into the **resolved** references (those a `\label` defines — recorded in
    /// [`ReferenceResolution::resolved`] as a [`ResolvedRef`]) and the **unresolved** references
    /// (LaTeX's *"Reference `key' undefined"*, the `??` in the output). Each [`ResolvedRef`] carries
    /// its `key`, the `command` that was used (`"ref"` / `"eqref"` / `"pageref"`), and the
    /// [`LabelKind`] (`target_kind`) of the definition it bound to. S21 renders that `resolved` list
    /// **flat**, one `\<command>{key}` per line in source pre-order. S24 renders the *same* `resolved`
    /// list **grouped by `target_kind`**: it walks the [`LabelKind`] variants in a fixed order and, for
    /// each kind that has at least one resolved ref, emits every ref that resolved to that kind. S21
    /// and S24 are two *views* of one underlying list (S21 by source order, S24 by target kind);
    /// neither mutates anything.
    ///
    /// ## The exact rendering contract
    ///
    /// Iterate the [`LabelKind`] variants in their **enum declaration order** —
    /// [`Section`](LabelKind::Section), [`Table`](LabelKind::Table), [`Figure`](LabelKind::Figure),
    /// [`Equation`](LabelKind::Equation), [`Inline`](LabelKind::Inline) — a fixed, deterministic order
    /// that does **not** depend on the document (the SAME `const KIND_ORDER` slice S23 uses). For each
    /// kind, filter [`ReferenceResolution::resolved`] to the refs whose `target_kind` is that kind,
    /// **preserving their existing pre-order** within the kind, and emit one line each. This single
    /// stable-ordered pass (fixed kind order on the outside, pre-order on the inside) keeps the output
    /// deterministic **without** a hash map — the same `Vec`-scan discipline S17/S18/S23 use to avoid
    /// hash-order nondeterminism.
    ///
    /// Each line has the shape `[<kind>] \<command>{<key>}`, where `<kind>` is the stable lowercase tag
    /// from [`LabelKind::as_str`] (`"section"`, `"table"`, `"figure"`, `"equation"`, `"inline"`),
    /// `<command>` is the ref's **own** command (so a resolved `\eqref{eq:m}` renders
    /// `[equation] \eqref{eq:m}` and a resolved `\pageref{sec:i}` renders `[section] \pageref{sec:i}` —
    /// the command is **never** hard-coded to `\ref`, matching S21's command-awareness), and `<key>` is
    /// reconstructed from the ref's **owned `key` `String`** — there is **no** source slicing at all
    /// (matching S21's `resolved_references_by_source` and S23's `label_definitions_by_kind`). Prefixing
    /// every line with its `[kind]` makes the per-kind census visible on each line while staying
    /// one-line-per-ref; the `[kind]` prefix is a report annotation, not source — S24 is a *report*, not
    /// a round-trippable rendering. A kind with **no** resolved refs produces **no** lines and **no**
    /// empty header (there is never a bare `[table]` group for a doc that references no tables).
    /// Dangling references never entered `resolved`, so they are excluded by construction (a
    /// `\ref{nope}` with no `\label` appears in S18's `unresolved_references_by_source`, not here).
    ///
    /// A document with **no** resolved references — every reference dangles, or there are none at all —
    /// returns the fixed marker `(no resolved references)`, the **same** marker S21 uses (S24 groups
    /// the identical list, so the empty case is identical), never the empty string (the stable-marker
    /// discipline S12-S23 share). Lines are joined by `\n` with **no** trailing newline (matching S21's
    /// `resolved_references_by_source` and every S11-S23 renderer).
    ///
    /// Concretely, for a body defining `\label{sec:intro}` (a section) and `\label{eq:main}` (an
    /// equation) and then writing `\ref{sec:intro}`, `\eqref{eq:main}`, and `\pageref{sec:intro}` (all
    /// of which resolve), the resolved refs render grouped by target kind (`section` sorts before
    /// `equation` in the fixed order, so both refs to `sec:intro` lead even though the `\eqref` appears
    /// between them in source):
    ///
    /// ```text
    /// [section] \ref{sec:intro}
    /// [section] \pageref{sec:intro}
    /// [equation] \eqref{eq:main}
    /// ```
    ///
    /// ## Additive by construction
    ///
    /// S24 is a brand-new, read-only method that reuses [`resolve_references`](Document::resolve_references)
    /// and mutates nothing; it changes no S1-S23 output (they are byte-for-byte unchanged) and leaves
    /// the `to_latex` round-trip fixed point intact. It is a *second view* of the same `resolved` list
    /// S21 renders flat — grouping never adds, drops, or reorders resolved references relative to what
    /// `resolve_references` produced.
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all —
    /// `command` and `key` are already owned `String`s; `ref_span`/`target_span` are not used); a
    /// single stable-ordered pass (fixed kind order × pre-order filter) over the already-bounded
    /// `resolved` list. Borrows `self` immutably and returns owned `String` data, so the result
    /// outlives any borrow of the source.
    pub fn resolved_references_by_kind(&self) -> String {
        // S1 already split every `\ref`/`\eqref`/`\pageref` into resolved and dangling entries, routing
        // the resolved ones into `resolved` (in body pre-order), each carrying its own `command`, `key`,
        // and `target_kind` (the kind of the label it bound to). We only read that list.
        let resolution = self.resolve_references();

        if resolution.resolved.is_empty() {
            // Every reference dangled (or there were none) → the SAME fixed marker S21 uses.
            return "(no resolved references)".to_string();
        }

        // The FIXED kind order = the enum declaration order (the SAME slice S23 uses). Iterating this
        // explicit slice (rather than a hash map keyed by kind) makes the group order deterministic and
        // document-independent, the same `Vec`-scan discipline S17/S18/S23 use to avoid hash-order
        // nondeterminism.
        const KIND_ORDER: [LabelKind; 5] = [
            LabelKind::Section,
            LabelKind::Table,
            LabelKind::Figure,
            LabelKind::Equation,
            LabelKind::Inline,
        ];

        // One stable-ordered pass: for each kind in the fixed order, take the resolved refs whose
        // `target_kind` is that kind, in their existing pre-order, and emit `[<kind>] \<command>{<key>}`.
        // A kind with no resolved refs contributes no lines (no empty header). Command is the ref's own,
        // so `\eqref`/`\pageref` render as themselves; keys are owned, so there is no source slicing.
        KIND_ORDER
            .iter()
            .flat_map(|kind| {
                resolution
                    .resolved
                    .iter()
                    .filter(move |r| r.target_kind == *kind)
                    .map(|r| format!("[{}] \\{}{{{}}}", r.target_kind.as_str(), r.command, r.key))
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The **per-kind CENSUS of the winning label definitions** (LTXDOC03 S25) — the *count*
    /// companion of S23's [`label_definitions_by_kind`](Document::label_definitions_by_kind). Where
    /// S23 renders one **line per definition** (a `[kind] \label{key}` list, grouped by kind), S25
    /// renders one **line per kind** carrying just the integer **count** of winning definitions of
    /// that kind — a `<kind>: <n>` tally, not a list. It is to S23 what S14's
    /// [`list_summary`](Document::list_summary) (`"Sections: 1"`) is to a full enumeration: a numeric
    /// summary over the *same* winning `definitions` list, so a reader sees "how many of each kind"
    /// without scanning the individual keys.
    ///
    /// ## What it does
    ///
    /// S1's [`Document::resolve_references`] collects the **winning** first definition of each `\label`
    /// key into [`ReferenceResolution::definitions`] — one row per distinct key, in
    /// [`Document::walk`] pre-order, each tagged with the [`LabelKind`] of the node it defines (a
    /// re-`\label`ed section, table, figure, equation, or bare inline label). S22's
    /// [`label_definitions`](Document::label_definitions) renders that list flat; S23's
    /// [`label_definitions_by_kind`](Document::label_definitions_by_kind) renders it grouped by kind,
    /// still one line per definition. S25 collapses each kind's group to a **single count line**: it
    /// walks the [`LabelKind`] variants in a fixed order and, for each kind that has **at least one**
    /// winning definition, emits `<kind>: <n>` where `<n>` is how many winning definitions carry that
    /// kind. Only the winning `definitions` are counted — a `\label{dup}` written twice contributes
    /// **one** to its kind's count, because its later re-definition is a [`Duplicate`] (S20's domain),
    /// never a second row in `definitions`. S23 and S25 are two *views* of one underlying list (S23 the
    /// per-definition list, S25 the per-kind count); neither mutates anything.
    ///
    /// ## The exact rendering contract
    ///
    /// Iterate the [`LabelKind`] variants in their **enum declaration order** —
    /// [`Section`](LabelKind::Section), [`Table`](LabelKind::Table), [`Figure`](LabelKind::Figure),
    /// [`Equation`](LabelKind::Equation), [`Inline`](LabelKind::Inline) — a fixed, deterministic order
    /// that does **not** depend on the document (the SAME `const KIND_ORDER` slice S23/S24 use). For
    /// each kind, count the [`ReferenceResolution::definitions`] whose `kind` is that kind, and — only
    /// if the count is **at least one** — emit one line `<kind>: <n>`, where `<kind>` is the stable
    /// lowercase tag from [`LabelKind::as_str`] (`"section"`, `"table"`, `"figure"`, `"equation"`,
    /// `"inline"`) — the **same** kind string S23 renders — and `<n>` is the decimal count. This single
    /// stable-ordered pass keeps the output deterministic **without** a hash map — the same `Vec`-scan
    /// discipline S17/S18/S23/S24 use to avoid hash-order nondeterminism. A kind with a **zero** count
    /// produces **no** line (there is never a bare `table: 0` for a doc with no table labels).
    ///
    /// A document with **no** winning label definitions at all returns the fixed marker
    /// `(no label definitions)` — the **same** marker S22/S23 use (S25 counts the identical list, so
    /// the empty case is identical), never the empty string (the stable-marker discipline S12–S24
    /// share). Lines are joined by `\n` with **no** trailing newline (matching every S11–S24 renderer).
    ///
    /// Concretely, for a body defining a section label `sec:intro`, two equation labels `eq:a`/`eq:b`,
    /// and a bare inline label `note`:
    ///
    /// ```text
    /// section: 1
    /// equation: 2
    /// inline: 1
    /// ```
    ///
    /// (the `section` count leads, then `equation`, then `inline`, in the fixed kind order; the `table`
    /// and `figure` kinds have zero definitions and so contribute no lines).
    ///
    /// ## Additive by construction
    ///
    /// S25 is a brand-new, read-only method that reuses [`resolve_references`](Document::resolve_references)
    /// and mutates nothing; it changes no S1–S24 output (they are byte-for-byte unchanged) and leaves
    /// the `to_latex` round-trip fixed point intact. It is a *third view* of the same winning
    /// `definitions` list S22 renders flat and S23 groups by kind — counting never adds, drops, or
    /// reorders definitions relative to what `resolve_references` produced.
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all —
    /// only the `kind` field is read); a single stable-ordered pass (fixed kind order × pre-order
    /// filter/count) over the already-bounded `definitions` list. Borrows `self` immutably and returns
    /// owned `String` data, so the result outlives any borrow of the source.
    pub fn label_kind_counts(&self) -> String {
        // S1 already collected the winning definitions — the first `\label` of each distinct key, in
        // body pre-order, each tagged with its `LabelKind`. We only read (and count) that list.
        let resolution = self.resolve_references();

        if resolution.definitions.is_empty() {
            // No `\label` at all → the SAME fixed marker S22/S23 use (S25 counts the identical list).
            return "(no label definitions)".to_string();
        }

        // The FIXED kind order = the enum declaration order (the SAME slice S23/S24 use). Iterating this
        // explicit slice (rather than a hash map keyed by kind) makes the line order deterministic and
        // document-independent, the same `Vec`-scan discipline S17/S18/S23/S24 use to avoid hash-order
        // nondeterminism.
        const KIND_ORDER: [LabelKind; 5] = [
            LabelKind::Section,
            LabelKind::Table,
            LabelKind::Figure,
            LabelKind::Equation,
            LabelKind::Inline,
        ];

        // One stable-ordered pass: for each kind in the fixed order, count the definitions of that kind
        // and — only when the count is >= 1 — emit `<kind>: <n>`. A kind with zero definitions is
        // filtered out (`filter` on `count > 0`), so it contributes no line. The kind string is the
        // SAME `LabelKind::as_str` tag S23 renders; there is no source slicing (only `kind` is read).
        KIND_ORDER
            .iter()
            .filter_map(|kind| {
                let count = resolution
                    .definitions
                    .iter()
                    .filter(|def| def.kind == *kind)
                    .count();
                (count > 0).then(|| format!("{}: {}", kind.as_str(), count))
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The **per-kind CENSUS of the resolved references** (LTXDOC03 S26) — the *count* companion of
    /// S24's [`resolved_references_by_kind`](Document::resolved_references_by_kind). Where S24 renders
    /// one **line per resolved reference** (an `[kind] \<command>{key}` list, grouped by the kind of
    /// node each ref bound to), S26 renders one **line per kind** carrying just the integer **count**
    /// of resolved references that landed on that kind — a `<kind>: <n>` tally, not a list. It is to
    /// S24 what S25's [`label_kind_counts`](Document::label_kind_counts) is to S23: a numeric summary
    /// over the *same* list (here the **resolved** references), so a reader sees "how many refs land on
    /// each kind" without scanning the individual keys.
    ///
    /// ## What it does
    ///
    /// S1's [`Document::resolve_references`] walks every `\ref`/`\eqref`/`\pageref` in body pre-order
    /// and splits them into the **resolved** references (those a `\label` defines — recorded in
    /// [`ReferenceResolution::resolved`] as a [`ResolvedRef`]) and the **unresolved** (dangling) ones
    /// (LaTeX's *"Reference `key' undefined"*, the `??` in the output — S18's domain). Each
    /// [`ResolvedRef`] carries the [`LabelKind`] (`target_kind`) of the definition it bound to. S21
    /// renders that `resolved` list **flat**, S24 renders it **grouped by `target_kind`** (one line per
    /// ref). S26 collapses each kind's group to a **single count line**: it walks the [`LabelKind`]
    /// variants in a fixed order and, for each kind that has **at least one** resolved ref, emits
    /// `<kind>: <n>` where `<n>` is how many resolved refs carry that `target_kind`. Only the
    /// **resolved** refs are counted — a dangling `\ref{nope}` lives in
    /// [`ReferenceResolution::unresolved`] (S18), never in `resolved`, so it is excluded by
    /// construction. S24 and S26 are two *views* of one underlying list (S24 the per-ref list, S26 the
    /// per-kind count); neither mutates anything.
    ///
    /// ## The exact rendering contract
    ///
    /// Iterate the [`LabelKind`] variants in their **enum declaration order** —
    /// [`Section`](LabelKind::Section), [`Table`](LabelKind::Table), [`Figure`](LabelKind::Figure),
    /// [`Equation`](LabelKind::Equation), [`Inline`](LabelKind::Inline) — a fixed, deterministic order
    /// that does **not** depend on the document (the SAME `const KIND_ORDER` slice S23/S24/S25 use).
    /// For each kind, count the [`ReferenceResolution::resolved`] refs whose `target_kind` is that kind,
    /// and — only if the count is **at least one** — emit one line `<kind>: <n>`, where `<kind>` is the
    /// stable lowercase tag from [`LabelKind::as_str`] (`"section"`, `"table"`, `"figure"`,
    /// `"equation"`, `"inline"`) — the **same** kind string S24 renders — and `<n>` is the decimal
    /// count. This single stable-ordered pass keeps the output deterministic **without** a hash map —
    /// the same `Vec`-scan discipline S17/S18/S23/S24/S25 use to avoid hash-order nondeterminism. A kind
    /// with a **zero** count produces **no** line (there is never a bare `table: 0` for a doc with no
    /// refs to tables).
    ///
    /// A document with **no** resolved references — every reference dangles, or there are none at all —
    /// returns the fixed marker `(no resolved references)`, the **same** marker S21/S24 use (S26 counts
    /// the identical list, so the empty case is identical), never the empty string (the stable-marker
    /// discipline S12–S25 share). Lines are joined by `\n` with **no** trailing newline (matching every
    /// S11–S25 renderer).
    ///
    /// Concretely, for a body defining two section labels `sec:a`/`sec:b` and one equation label
    /// `eq:e`, then writing `\ref{sec:a}`, `\ref{sec:b}`, and `\eqref{eq:e}` (all of which resolve):
    ///
    /// ```text
    /// section: 2
    /// equation: 1
    /// ```
    ///
    /// (the `section` count leads, then `equation`, in the fixed kind order; the `table`, `figure`, and
    /// `inline` kinds have zero resolved refs and so contribute no lines).
    ///
    /// ## Additive by construction
    ///
    /// S26 is a brand-new, read-only method that reuses [`resolve_references`](Document::resolve_references)
    /// and mutates nothing; it changes no S1–S25 output (they are byte-for-byte unchanged) and leaves
    /// the `to_latex` round-trip fixed point intact. It is a *third view* of the same `resolved` list
    /// S21 renders flat and S24 groups by kind — counting never adds, drops, or reorders resolved
    /// references relative to what `resolve_references` produced.
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all —
    /// only the `target_kind` field is read); a single stable-ordered pass (fixed kind order × pre-order
    /// filter/count) over the already-bounded `resolved` list. Borrows `self` immutably and returns
    /// owned `String` data, so the result outlives any borrow of the source.
    pub fn resolved_reference_kind_counts(&self) -> String {
        // S1 already split every `\ref`/`\eqref`/`\pageref` into resolved and dangling entries, routing
        // the resolved ones into `resolved` (in body pre-order), each carrying the `target_kind` of the
        // label it bound to. We only read (and count) that list; dangling refs live in `unresolved`.
        let resolution = self.resolve_references();

        if resolution.resolved.is_empty() {
            // Every reference dangled (or there were none) → the SAME fixed marker S21/S24 use.
            return "(no resolved references)".to_string();
        }

        // The FIXED kind order = the enum declaration order (the SAME slice S23/S24/S25 use). Iterating
        // this explicit slice (rather than a hash map keyed by kind) makes the line order deterministic
        // and document-independent, the same `Vec`-scan discipline S17/S18/S23/S24/S25 use to avoid
        // hash-order nondeterminism.
        const KIND_ORDER: [LabelKind; 5] = [
            LabelKind::Section,
            LabelKind::Table,
            LabelKind::Figure,
            LabelKind::Equation,
            LabelKind::Inline,
        ];

        // One stable-ordered pass: for each kind in the fixed order, count the resolved refs whose
        // `target_kind` is that kind and — only when the count is >= 1 — emit `<kind>: <n>`. A kind with
        // zero resolved refs is filtered out (`filter` on `count > 0`), so it contributes no line. The
        // kind string is the SAME `LabelKind::as_str` tag S24 renders; there is no source slicing (only
        // `target_kind` is read).
        KIND_ORDER
            .iter()
            .filter_map(|kind| {
                let count = resolution
                    .resolved
                    .iter()
                    .filter(|r| r.target_kind == *kind)
                    .count();
                (count > 0).then(|| format!("{}: {}", kind.as_str(), count))
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The **single-integer TOTAL of the unresolved (dangling) references** (LTXDOC03 S27) — the
    /// *count-total* companion of S18's
    /// [`unresolved_references_by_source`](Document::unresolved_references_by_source). Where S18
    /// renders one **line per dangling reference** (a `\<command>{key}` list, in body pre-order),
    /// S27 renders one **line** carrying just the decimal **count** of those dangling refs — the
    /// number of `\ref`/`\eqref`/`\pageref` that no `\label` defines. It is to S18 what S25's
    /// [`label_kind_counts`](Document::label_kind_counts) and S26's
    /// [`resolved_reference_kind_counts`](Document::resolved_reference_kind_counts) are to their
    /// list views: a numeric summary over the *same* list — here the **unresolved** references — so
    /// a reader sees "how many refs dangle" without scanning the individual keys.
    ///
    /// ## What it does
    ///
    /// S1's [`Document::resolve_references`] walks every `\ref`/`\eqref`/`\pageref` in body pre-order
    /// and splits them into the **resolved** references (those a `\label` defines — S21/S24/S26's
    /// domain) and the **unresolved** (dangling) ones — LaTeX's *"Reference `key' undefined"*, the
    /// `??` in the output — recorded in [`ReferenceResolution::unresolved`] as [`UnresolvedRef`]s.
    /// S18 renders that `unresolved` list as one line per dangling ref; S27 collapses the whole list
    /// to a **single count line** — the decimal `.len()` of `unresolved`. Unlike S25/S26, no per-kind
    /// census is possible here: a dangling ref bound to **no** definition, so it carries **no**
    /// `target_kind` — there is nothing to group by, so a single total is the clean move. Only the
    /// **unresolved** refs are counted; a resolved `\ref{sec:i}` lives in
    /// [`ReferenceResolution::resolved`] (S21), never in `unresolved`, so it is excluded by
    /// construction.
    ///
    /// ## The exact rendering contract
    ///
    /// Read the length of [`ReferenceResolution::unresolved`] and render it as its decimal `String`,
    /// **always** on a single line with **no** trailing newline. This is a **count** renderer, so its
    /// honest value when there are no dangling references is the number `0` — the string `"0"`, **not**
    /// a `(no …)` marker (a total count of zero *is* a number; the `(no …)` marker discipline belongs
    /// to the *list* renderers S18/S21/S24, whose empty case has no lines to show). There is no source
    /// slicing and no `target_kind` read at all — only `.len()` is taken.
    ///
    /// Concretely, for a body defining `\label{sec:i}` and then writing `\ref{sec:i}` (resolves),
    /// `\ref{nope}` (dangles), and `\ref{gone}` (dangles):
    ///
    /// ```text
    /// 2
    /// ```
    ///
    /// (two references dangle; the one resolved `\ref{sec:i}` is excluded). A document with no dangling
    /// references — every ref resolves, or there are none at all — returns `"0"`.
    ///
    /// ## Additive by construction
    ///
    /// S27 is a brand-new, read-only method that reuses [`resolve_references`](Document::resolve_references)
    /// and mutates nothing; it changes no S1–S26 output (they are byte-for-byte unchanged) and leaves
    /// the `to_latex` round-trip fixed point intact. It is a *second view* of the same `unresolved`
    /// list S18 renders per-source — counting never adds, drops, or reorders the dangling references
    /// relative to what `resolve_references` produced.
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all —
    /// only `.len()` is read, never a `target_kind`, which a dangling ref never carries); a single
    /// read of the already-bounded `unresolved` list's length. Borrows `self` immutably and returns
    /// owned `String` data, so the result outlives any borrow of the source.
    pub fn unresolved_reference_count(&self) -> String {
        // S1 already split every `\ref`/`\eqref`/`\pageref` into resolved and dangling entries, routing
        // the dangling ones into `unresolved` (in body pre-order). We only read that list's length; the
        // resolved refs live in `resolved` (S21). A dangling ref bound to no definition, so it carries
        // no `target_kind` — a per-kind census is not viable here, and a single total is the clean move.
        self.resolve_references().unresolved.len().to_string()
    }

    /// The **single-integer TOTAL of the resolved references** (LTXDOC03 S28) — the *count-total*
    /// companion of S21's [`resolved_references_by_source`](Document::resolved_references_by_source)
    /// and S24's [`resolved_references_by_kind`](Document::resolved_references_by_kind). Where S21/S24
    /// render one **line per resolved reference** (a `\<command>{key}` list, flat in body pre-order or
    /// grouped by target kind), S28 renders one **line** carrying just the decimal **count** of those
    /// resolved refs — the number of `\ref`/`\eqref`/`\pageref` that some `\label` defines. It is the
    /// exact resolved-side twin of S27's
    /// [`unresolved_reference_count`](Document::unresolved_reference_count): together the two totals
    /// split every reference into the pair (resolved, dangling), so S28 + S27 = the total reference
    /// count. It is to S21/S24 what S25's [`label_kind_counts`](Document::label_kind_counts) and S26's
    /// [`resolved_reference_kind_counts`](Document::resolved_reference_kind_counts) are to their list
    /// views: a numeric summary over the *same* list — here the **resolved** references — so a reader
    /// sees "how many refs resolve" without scanning the individual keys.
    ///
    /// ## What it does
    ///
    /// S1's [`Document::resolve_references`] walks every `\ref`/`\eqref`/`\pageref` in body pre-order
    /// and splits them into the **resolved** references (those a `\label` defines — S21/S24/S26's
    /// domain, recorded in [`ReferenceResolution::resolved`] as [`ResolvedRef`]s) and the
    /// **unresolved** (dangling) ones (S18/S27's domain). S21/S24 render that `resolved` list as one
    /// line per resolved ref; S28 collapses the whole list to a **single count line** — the decimal
    /// `.len()` of `resolved`. Unlike S26 (which *can* census the resolved refs by the `target_kind`
    /// each bound to), S28 takes no per-kind view at all: it reads only the length, never a
    /// `target_kind`, so section, table, and equation references all fold into one total. Only the
    /// **resolved** refs are counted; a dangling `\ref{nope}` lives in
    /// [`ReferenceResolution::unresolved`] (S18/S27), never in `resolved`, so it is excluded by
    /// construction.
    ///
    /// ## The exact rendering contract
    ///
    /// Read the length of [`ReferenceResolution::resolved`] and render it as its decimal `String`,
    /// **always** on a single line with **no** trailing newline. This is a **count** renderer, so its
    /// honest value when there are no resolved references is the number `0` — the string `"0"`, **not**
    /// a `(no resolved references)` marker (a total count of zero *is* a number; the `(no …)` marker
    /// discipline belongs to the *list* renderers S21/S24, whose empty case has no lines to show). This
    /// mirrors S27 exactly, which emits `"0"` for an empty `unresolved` list. There is no source
    /// slicing and no `target_kind` read at all — only `.len()` is taken.
    ///
    /// Concretely, for a body defining `\label{sec:i}` and then writing `\ref{sec:i}` (resolves),
    /// `\ref{nope}` (dangles), and `\ref{sec:i}` again (resolves):
    ///
    /// ```text
    /// 2
    /// ```
    ///
    /// (two references resolve; the one dangling `\ref{nope}` is excluded). A document with no resolved
    /// references — every ref dangles, or there are none at all — returns `"0"`.
    ///
    /// ## Additive by construction
    ///
    /// S28 is a brand-new, read-only method that reuses [`resolve_references`](Document::resolve_references)
    /// and mutates nothing; it changes no S1–S27 output (they are byte-for-byte unchanged) and leaves
    /// the `to_latex` round-trip fixed point intact. It is a *second view* of the same `resolved` list
    /// S21/S24 render per-source and per-kind — counting never adds, drops, or reorders the resolved
    /// references relative to what `resolve_references` produced.
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all —
    /// only `.len()` is read, never a `target_kind`); a single read of the already-bounded `resolved`
    /// list's length. Borrows `self` immutably and returns owned `String` data, so the result outlives
    /// any borrow of the source.
    pub fn resolved_reference_count(&self) -> String {
        // S1 already split every `\ref`/`\eqref`/`\pageref` into resolved and dangling entries, routing
        // the resolved ones into `resolved` (in body pre-order). We only read that list's length; the
        // dangling refs live in `unresolved` (S18/S27). No `target_kind` is read — section, table, and
        // equation references all fold into a single total, the resolved-side twin of S27's count.
        self.resolve_references().resolved.len().to_string()
    }

    /// The **single-integer TOTAL of the label definitions** (LTXDOC03 S29) — the *count-total*
    /// companion of S22's [`label_definitions`](Document::label_definitions) and S23's
    /// [`label_definitions_by_kind`](Document::label_definitions_by_kind). Where S22/S23 render one
    /// **line per winning label definition** (a `\label{key}` list, flat in pre-order or grouped by
    /// kind), S29 renders one **line** carrying just the decimal **count** of those definitions — the
    /// number of distinct `\label` keys the document defines. It is the exact label-definition-side
    /// analogue of the reference-side totals S27's
    /// [`unresolved_reference_count`](Document::unresolved_reference_count) and S28's
    /// [`resolved_reference_count`](Document::resolved_reference_count) provide for the two reference
    /// tables. It is to S22/S23 what S25's [`label_kind_counts`](Document::label_kind_counts) is: a
    /// numeric summary over the *same* list — here the **winning** label definitions — so a reader
    /// sees "how many labels are defined" without scanning the individual keys.
    ///
    /// ## What it does
    ///
    /// S1's [`Document::resolve_references`] splits every `\label` into the **winning** first
    /// definition of each key ([`ReferenceResolution::definitions`] — one row per distinct key, in
    /// [`Document::walk`] pre-order) and the **losing** later re-definitions
    /// ([`ReferenceResolution::duplicates`] — LaTeX's *"Label `key' multiply defined"* warning, S20's
    /// domain). S22/S23 render that `definitions` list as one line per winning definition; S29
    /// collapses the whole list to a **single count line** — the decimal `.len()` of `definitions`.
    /// Unlike S25 (which *can* census the definitions by [`LabelKind`]), S29 takes no per-kind view at
    /// all: it reads only the length, never a `kind`, so section, figure, equation, and inline labels
    /// all fold into one total. Only the **winning** definitions are counted; a later duplicate
    /// `\label{dup}` lives in [`ReferenceResolution::duplicates`] (S20), never in `definitions`, so it
    /// is excluded by construction — the count is exactly the number of lines S22 lists.
    ///
    /// ## The exact rendering contract
    ///
    /// Read the length of [`ReferenceResolution::definitions`] and render it as its decimal `String`,
    /// **always** on a single line with **no** trailing newline. This is a **count** renderer, so its
    /// honest value when there are no label definitions is the number `0` — the string `"0"`, **not**
    /// a `(no label definitions)` marker (a total count of zero *is* a number; the `(no …)` marker
    /// discipline belongs to the *list* renderers S22/S23, whose empty case has no lines to show). This
    /// mirrors S27/S28 exactly, which emit `"0"` for their empty lists. There is no source slicing and
    /// no `kind` read at all — only `.len()` is taken.
    ///
    /// Concretely, for a body defining `\label{sec:intro}` (a section), `\label{eq:main}` (an
    /// equation), and then re-using `\label{sec:intro}` on a later subsection (a duplicate):
    ///
    /// ```text
    /// 2
    /// ```
    ///
    /// (two distinct keys are defined; the later duplicate `\label{sec:intro}` is excluded). A document
    /// with no label definitions at all returns `"0"`.
    ///
    /// ## Additive by construction
    ///
    /// S29 is a brand-new, read-only method that reuses [`resolve_references`](Document::resolve_references)
    /// and mutates nothing; it changes no S1–S28 output (they are byte-for-byte unchanged) and leaves
    /// the `to_latex` round-trip fixed point intact. It is a *second view* of the same `definitions`
    /// list S22/S23 render flat and per-kind — counting never adds, drops, or reorders the definitions
    /// relative to what `resolve_references` produced.
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all —
    /// only `.len()` is read, never a `kind`); a single read of the already-bounded `definitions`
    /// list's length. Borrows `self` immutably and returns owned `String` data, so the result outlives
    /// any borrow of the source.
    pub fn label_definition_count(&self) -> String {
        // S1 already collected the winning definitions — the first `\label` of each distinct key, in
        // body pre-order (later re-definitions went to `duplicates`, S20, not here). We only read that
        // list's length. No `kind` is read — section, figure, equation, and inline labels all fold into
        // a single total, the label-definition-side analogue of S27/S28's reference-side counts.
        self.resolve_references().definitions.len().to_string()
    }

    /// The single-integer TOTAL of the winning bibliography entries (LTXDOC03 S30) — the
    /// **citation-side** twin of S29's [`label_definition_count`](Document::label_definition_count).
    ///
    /// This is the last member of the *totals family*: S27's
    /// [`unresolved_reference_count`](Document::unresolved_reference_count) and S28's
    /// [`resolved_reference_count`](Document::resolved_reference_count) count the two reference tables,
    /// S29's [`label_definition_count`](Document::label_definition_count) counts the winning label
    /// definitions, and S30 counts the winning bibliography entries. It is to S19's
    /// [`bibliography_entries`](Document::bibliography_entries) exactly what S29 is to S22's
    /// [`label_definitions`](Document::label_definitions): a numeric summary over the *same* list — here
    /// the **winning** `\bibitem` entries — so a reader sees "how many bibliography entries there are"
    /// without scanning the individual keys.
    ///
    /// ## What it does
    ///
    /// S2's [`Document::resolve_citations`] splits every `\bibitem` inside a `thebibliography`
    /// environment into the **winning** first entry of each key ([`CitationResolution::entries`] — one
    /// row per distinct key, in [`Document::walk`] pre-order) and the **losing** later re-definitions
    /// ([`CitationResolution::duplicate_entries`] — LaTeX's *"Citation `key' multiply defined"*
    /// warning, S16's domain). S19 renders that `entries` list as one numbered line per winning entry;
    /// S30 collapses the whole list to a **single count line** — the decimal `.len()` of `entries`.
    /// Only the **winning** entries are counted; a later duplicate `\bibitem{dup}` lives in
    /// [`CitationResolution::duplicate_entries`] (S16), never in `entries`, so it is excluded by
    /// construction — the count is exactly the number of lines S19 lists.
    ///
    /// ## The exact rendering contract
    ///
    /// Read the length of [`CitationResolution::entries`] and render it as its decimal `String`,
    /// **always** on a single line with **no** trailing newline. This is a **count** renderer, so its
    /// honest value when there are no bibliography entries is the number `0` — the string `"0"`,
    /// **not** a `(no bibliography entries)` marker (a total count of zero *is* a number; the `(no …)`
    /// marker discipline belongs to the *list* renderer S19, whose empty case has no lines to show).
    /// This mirrors S27/S28/S29 exactly, which emit `"0"` for their empty lists. There is no source
    /// slicing — only `.len()` is taken.
    ///
    /// Concretely, for a `thebibliography` with `\bibitem{a}`, `\bibitem{b}`, `\bibitem{c}`, and then a
    /// re-used `\bibitem{a}` (a duplicate):
    ///
    /// ```text
    /// 3
    /// ```
    ///
    /// (three distinct keys are defined; the later duplicate `\bibitem{a}` is excluded). A document
    /// with no `\bibitem` at all returns `"0"`.
    ///
    /// ## Additive by construction
    ///
    /// S30 is a brand-new, read-only method that reuses [`resolve_citations`](Document::resolve_citations)
    /// and mutates nothing; it changes no S1–S29 output (they are byte-for-byte unchanged) and leaves
    /// the `to_latex` round-trip fixed point intact. It is a *second view* of the same `entries` list
    /// S19 renders flat — counting never adds, drops, or reorders the entries relative to what
    /// `resolve_citations` produced.
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all —
    /// only `.len()` is read); a single read of the already-bounded `entries` list's length. Borrows
    /// `self` immutably and returns owned `String` data, so the result outlives any borrow of the
    /// source.
    pub fn bibliography_entry_count(&self) -> String {
        // S2 already collected the winning entries — the first `\bibitem` of each distinct key, in
        // body pre-order (later re-definitions went to `duplicate_entries`, S16, not here). We only
        // read that list's length. This is the citation-side analogue of S29's label-definition count
        // and completes the totals family (S27/S28 references, S29 labels, S30 bibliography).
        self.resolve_citations().entries.len().to_string()
    }

    /// The **single-integer TOTAL of the resolved citations** (LTXDOC03 S31) — the exact
    /// **citation-side** twin of S28's [`resolved_reference_count`](Document::resolved_reference_count).
    ///
    /// This extends the *totals family* onto the resolved-citation table: S27's
    /// [`unresolved_reference_count`](Document::unresolved_reference_count) and S28's
    /// [`resolved_reference_count`](Document::resolved_reference_count) count the two reference tables,
    /// S29's [`label_definition_count`](Document::label_definition_count) counts the winning label
    /// definitions, S30's [`bibliography_entry_count`](Document::bibliography_entry_count) counts the
    /// winning bibliography entries, and S31 counts the **resolved** citations. It is to S15's
    /// [`citations_by_source`](Document::citations_by_source) exactly what S28 is to S21/S24's resolved-
    /// reference lists: a numeric summary over the *same* list — here the **resolved** `\cite` keys — so
    /// a reader sees "how many citations resolve" without scanning the individual keys.
    ///
    /// ## What it does
    ///
    /// S2's [`Document::resolve_citations`] walks every `\cite` in body pre-order and splits its keys
    /// (a multi-key `\cite{a,b}` yields one record per key) into the **resolved** citations — those a
    /// `\bibitem` defines ([`CitationResolution::resolved`] as [`ResolvedCite`]s, S15's domain) — and the
    /// **unresolved** (dangling) ones ([`CitationResolution::unresolved`], S17's domain). S15 renders
    /// that `resolved` list grouped by source `\cite`; S31 collapses the whole list to a **single count
    /// line** — the decimal `.len()` of `resolved`. It reads only the length, never a `cite_span` or
    /// `entry_span`, so every resolved key — regardless of which `\cite` it came from — folds into one
    /// total. Only the **resolved** keys are counted; a dangling `\cite{ghost}` lives in
    /// [`CitationResolution::unresolved`] (S17), never in `resolved`, so it is excluded by construction.
    ///
    /// ## The exact rendering contract
    ///
    /// Read the length of [`CitationResolution::resolved`] and render it as its decimal `String`,
    /// **always** on a single line with **no** trailing newline. This is a **count** renderer, so its
    /// honest value when there are no resolved citations is the number `0` — the string `"0"`, **not**
    /// a `(no resolved citations)` marker (a total count of zero *is* a number; the `(no …)` marker
    /// discipline belongs to the *list* renderer S15, whose empty case has no lines to show). This
    /// mirrors S27/S28/S29/S30 exactly, which emit `"0"` for their empty lists. There is no source
    /// slicing — only `.len()` is taken.
    ///
    /// Concretely, for a body `\cite{a,b}` (both defined) then `\cite{c,ghost}` (only `c` defined),
    /// against a bibliography defining `a`, `b`, `c`:
    ///
    /// ```text
    /// 3
    /// ```
    ///
    /// (three keys resolve — `a`, `b`, `c`; the one dangling `ghost` is excluded). A document with no
    /// resolved citations — every cited key dangling, or none at all — returns `"0"`.
    ///
    /// ## Additive by construction
    ///
    /// S31 is a brand-new, read-only method that reuses [`resolve_citations`](Document::resolve_citations)
    /// and mutates nothing; it changes no S1–S30 output (they are byte-for-byte unchanged) and leaves the
    /// `to_latex` round-trip fixed point intact. It is a *second view* of the same `resolved` list S15
    /// renders per-source — counting never adds, drops, or reorders the resolved citations relative to
    /// what `resolve_citations` produced.
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all —
    /// only `.len()` is read); a single read of the already-bounded `resolved` list's length. Borrows
    /// `self` immutably and returns owned `String` data, so the result outlives any borrow of the
    /// source.
    pub fn citation_count(&self) -> String {
        // S2 already flattened every `\cite` into per-key records, routing the ones a `\bibitem`
        // defines into `resolved` (in body pre-order; the dangling keys went to `unresolved`, S17). We
        // only read that list's length. No `cite_span`/`entry_span` is read — every resolved key folds
        // into a single total, the resolved-citation-side twin of S28's resolved-reference count.
        self.resolve_citations().resolved.len().to_string()
    }

    /// The **single-integer TOTAL of the unresolved (dangling) citations** (LTXDOC03 S32) — the exact
    /// **citation-side** twin of S27's [`unresolved_reference_count`](Document::unresolved_reference_count),
    /// and the **dangling sibling** of S31's resolved-citation total
    /// [`citation_count`](Document::citation_count).
    ///
    /// This closes the totals family over the citation family the way S27/S28 close it over the
    /// reference family. S27's [`unresolved_reference_count`](Document::unresolved_reference_count) counts
    /// the *dangling* `\ref`s and S28's [`resolved_reference_count`](Document::resolved_reference_count)
    /// the *resolved* ones; on the citation side S31's [`citation_count`](Document::citation_count) counts
    /// the *resolved* `\cite` keys and S32 counts the *dangling* ones. Together S31 and S32 **partition**
    /// every per-key `\cite` record S2 produced: `citation_count + unresolved_citation_count` is exactly
    /// the number of citation keys in the body, because [`Document::resolve_citations`] routes each key
    /// into exactly one of `resolved` or `unresolved`. S32 is to S17's
    /// [`unresolved_citations_by_source`](Document::unresolved_citations_by_source) exactly what S27 is to
    /// S18's per-source dangling-reference list: a numeric summary over the *same* list — here the
    /// **dangling** `\cite` keys — so a reader sees "how many citations dangle" without scanning the keys.
    ///
    /// ## What it does
    ///
    /// S2's [`Document::resolve_citations`] walks every `\cite` in body pre-order and splits its keys (a
    /// multi-key `\cite{a,b}` yields one record per key) into the **resolved** citations — those a
    /// `\bibitem` defines ([`CitationResolution::resolved`], S15's domain) — and the **unresolved**
    /// (dangling) ones ([`CitationResolution::unresolved`] as [`UnresolvedCite`]s, S17's domain). S17
    /// renders that `unresolved` list grouped by source `\cite`; S32 collapses the whole list to a
    /// **single count line** — the decimal `.len()` of `unresolved`. It reads only the length, never a
    /// `cite_span` or a dangling `key`, so every unresolved key — regardless of which `\cite` it came
    /// from — folds into one total. Only the **dangling** keys are counted; a resolved `\cite{a}` lives
    /// in [`CitationResolution::resolved`] (S15/S31), never in `unresolved`, so it is excluded by
    /// construction.
    ///
    /// ## The exact rendering contract
    ///
    /// Read the length of [`CitationResolution::unresolved`] and render it as its decimal `String`,
    /// **always** on a single line with **no** trailing newline. This is a **count** renderer, so its
    /// honest value when there are no dangling citations is the number `0` — the string `"0"`, **not** a
    /// `(no unresolved citations)` marker (a total count of zero *is* a number; the `(no …)` marker
    /// discipline belongs to the *list* renderer S17, whose empty case has no lines to show). This
    /// mirrors S27/S28/S29/S30/S31 exactly, which emit `"0"` for their empty lists. There is no source
    /// slicing — only `.len()` is taken.
    ///
    /// Concretely, for a body `\cite{a,b}` (both defined) then `\cite{c,ghost}` (only `c` defined),
    /// against a bibliography defining `a`, `b`, `c`:
    ///
    /// ```text
    /// 1
    /// ```
    ///
    /// (one key dangles — `ghost`; the three resolved keys `a`, `b`, `c` are excluded, and are the "3"
    /// S31 reports). A document with no dangling citations — every cited key resolves, or none at all —
    /// returns `"0"`.
    ///
    /// ## Additive by construction
    ///
    /// S32 is a brand-new, read-only method that reuses [`resolve_citations`](Document::resolve_citations)
    /// and mutates nothing; it changes no S1–S31 output (they are byte-for-byte unchanged) and leaves the
    /// `to_latex` round-trip fixed point intact. It is a *second view* of the same `unresolved` list S17
    /// renders per-source — counting never adds, drops, or reorders the dangling citations relative to
    /// what `resolve_citations` produced.
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all —
    /// only `.len()` is read); a single read of the already-bounded `unresolved` list's length. Borrows
    /// `self` immutably and returns owned `String` data, so the result outlives any borrow of the
    /// source.
    pub fn unresolved_citation_count(&self) -> String {
        // S2 already flattened every `\cite` into per-key records, routing the ones NO `\bibitem`
        // defines into `unresolved` (in body pre-order; the resolved keys went to `resolved`, S15/S31).
        // We only read that list's length. No `cite_span`/`key` is read — every dangling key folds into
        // a single total, the unresolved-citation-side twin of S27's unresolved-reference count.
        self.resolve_citations().unresolved.len().to_string()
    }

    /// The **single-integer TOTAL of the duplicate ("multiply defined") `\bibitem`s** (LTXDOC03 S33) —
    /// the count of the *later, losing* `\bibitem`s of already-defined keys, the **warning-side**
    /// companion of the resolved (S30/S31) and unresolved (S32) citation totals.
    ///
    /// This extends the *totals family* onto the last citation-family table it had not yet summarized.
    /// S30's [`bibliography_entry_count`](Document::bibliography_entry_count) counts the *winning*
    /// bibliography entries, S31's [`citation_count`](Document::citation_count) the *resolved* `\cite`
    /// keys, and S32's [`unresolved_citation_count`](Document::unresolved_citation_count) the *dangling*
    /// ones; S33 counts the *duplicate* `\bibitem`s — the `\bibitem`s LaTeX would flag with
    /// *"Citation `key' multiply defined"*. It is to S16's
    /// [`duplicate_bibliography_entries`](Document::duplicate_bibliography_entries) exactly what S30 is to
    /// S19's `bibliography_entries`: a numeric summary over the *same* list — here the **losing**
    /// duplicate `\bibitem`s — so a reader sees "how many `\bibitem`s are multiply defined" without
    /// scanning the individual warning lines.
    ///
    /// ## What it does
    ///
    /// S2's [`Document::resolve_citations`] collects every `\bibitem{key}` inside a `thebibliography` in
    /// [`Document::walk`] pre-order. The **first** `\bibitem` of each key wins (it becomes a row in
    /// [`CitationResolution::entries`], S19/S30's domain); every **later** `\bibitem` of an
    /// already-defined key is a *losing* duplicate, routed to
    /// [`CitationResolution::duplicate_entries`] (S16's domain). S16 renders that list as one
    /// `\bibitem{key}` warning line per losing duplicate; S33 collapses the whole list to a **single
    /// count line** — the decimal `.len()` of `duplicate_entries`. It reads only the length, never a
    /// `key` or a `span`, so every losing `\bibitem` — regardless of key — folds into one total. Only the
    /// **losing** `\bibitem`s are counted; the winning first `\bibitem` of a key lives in `entries`
    /// (S19/S30), never in `duplicate_entries`, so it is excluded by construction. We do **not**
    /// de-duplicate: a key defined *three* times contributes *two* losing duplicates (one per
    /// *"multiply defined"* warning LaTeX would raise), exactly the two `\bibitem{key}` lines S16 emits.
    ///
    /// ## The exact rendering contract
    ///
    /// Read the length of [`CitationResolution::duplicate_entries`] and render it as its decimal
    /// `String`, **always** on a single line with **no** trailing newline. This is a **count** renderer,
    /// so its honest value when there are no duplicate `\bibitem`s is the number `0` — the string `"0"`,
    /// **not** a `(no duplicate bibliography entries)` marker (a total count of zero *is* a number; the
    /// `(no …)` marker discipline belongs to the *list* renderer S16, whose empty case has no lines to
    /// show). This mirrors S27/S28/S29/S30/S31/S32 exactly, which emit `"0"` for their empty lists. There
    /// is no source slicing — only `.len()` is taken.
    ///
    /// Concretely, for a `thebibliography` that defines `smith` twice and `jones` once:
    ///
    /// ```text
    /// \begin{thebibliography}{9}
    /// \bibitem{smith} First Smith. 1990.
    /// \bibitem{jones} Jones. 1991.
    /// \bibitem{smith} Second Smith. 1992.
    /// \end{thebibliography}
    /// ```
    ///
    /// only the *second* `\bibitem{smith}` loses, so S33 reports:
    ///
    /// ```text
    /// 1
    /// ```
    ///
    /// (one losing duplicate — the second `\bibitem{smith}`; the winning first `\bibitem{smith}` and the
    /// lone `\bibitem{jones}` are in `entries`, the "2" S30 reports). A document with **no** duplicate
    /// `\bibitem`s — no bibliography, or every key defined exactly once — returns `"0"`.
    ///
    /// **Partition note.** S30 counts `entries` (one per distinct key, the winners) and S33 counts
    /// `duplicate_entries` (the losers); together they **partition** every `\bibitem` inside a
    /// `thebibliography` — `bibliography_entry_count + duplicate_bibliography_count` is exactly the number
    /// of `\bibitem`s, because [`Document::resolve_citations`] routes each `\bibitem` into exactly one of
    /// `entries` or `duplicate_entries`.
    ///
    /// ## Additive by construction
    ///
    /// S33 is a brand-new, read-only method that reuses [`resolve_citations`](Document::resolve_citations)
    /// and mutates nothing; it changes no S1–S32 output (they are byte-for-byte unchanged) and leaves the
    /// `to_latex` round-trip fixed point intact. It is a *second view* of the same `duplicate_entries`
    /// list S16 renders per-source — counting never adds, drops, or reorders the duplicate `\bibitem`s
    /// relative to what `resolve_citations` produced.
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all —
    /// only `.len()` is read); a single read of the already-bounded `duplicate_entries` list's length.
    /// Borrows `self` immutably and returns owned `String` data, so the result outlives any borrow of the
    /// source.
    pub fn duplicate_bibliography_count(&self) -> String {
        // S2 already routed every later `\bibitem` of an already-defined key into `duplicate_entries`,
        // in body pre-order (first-entry-wins; the winning first `\bibitem`s went to `entries`, S19/S30).
        // We only read that list's length. No `key`/`span` is read — every losing `\bibitem` folds into a
        // single total, the "multiply defined" warning-side companion of S30/S31/S32 and the numeric
        // summary of S16's per-source duplicate list.
        self.resolve_citations().duplicate_entries.len().to_string()
    }

    /// The **single-integer TOTAL of the duplicate ("multiply defined") `\label`s** (LTXDOC03 S34) —
    /// the count of the *later, losing* `\label` definitions of already-defined keys, the
    /// **label-side twin** of S33's
    /// [`duplicate_bibliography_count`](Document::duplicate_bibliography_count).
    ///
    /// This is the warning-side member of the *label totals family*. S29's
    /// [`label_definition_count`](Document::label_definition_count) counts the *winning* label
    /// definitions (the first `\label` of each distinct key); S34 counts the *losing* ones — the
    /// `\label`s LaTeX would flag with *"Label `key' multiply defined"*. It is to S20's
    /// [`duplicate_label_definitions`](Document::duplicate_label_definitions) exactly what S33 is to
    /// S16's `duplicate_bibliography_entries`, and what S29 is to S22's `label_definitions`: a numeric
    /// summary over the *same* list — here the **losing** duplicate `\label`s — so a reader sees "how
    /// many labels are multiply defined" without scanning the individual warning lines. It mirrors S33
    /// on the label side just as S29 (`label_definition_count`) mirrors S30
    /// (`bibliography_entry_count`).
    ///
    /// ## What it does
    ///
    /// S1's [`Document::resolve_references`] splits every `\label` into the **winning** first
    /// definition of each key ([`ReferenceResolution::definitions`] — S22/S29's domain) and the
    /// **losing** later re-definitions ([`ReferenceResolution::duplicates`] — S20's domain), both in
    /// [`Document::walk`] pre-order. S20 renders the losing list as one `\label{key}` warning line per
    /// duplicate; S34 collapses the whole list to a **single count line** — the decimal `.len()` of
    /// `duplicates`. It reads only the length, never a `key`, `kind`, or `span`, so every losing
    /// `\label` — section, figure, equation, or inline, regardless of key — folds into one total. Only
    /// the **losing** definitions are counted; the winning first `\label` of a key lives in
    /// `definitions` (S22/S29), never in `duplicates`, so it is excluded by construction. We do **not**
    /// de-duplicate: a key defined *three* times contributes *two* losing duplicates (one per
    /// *"multiply defined"* warning LaTeX would raise), exactly the two `\label{key}` lines S20 emits.
    ///
    /// ## The exact rendering contract
    ///
    /// Read the length of [`ReferenceResolution::duplicates`] and render it as its decimal `String`,
    /// **always** on a single line with **no** trailing newline. This is a **count** renderer, so its
    /// honest value when there are no duplicate `\label`s is the number `0` — the string `"0"`, **not**
    /// a `(no duplicate label definitions)` marker (a total count of zero *is* a number; the `(no …)`
    /// marker discipline belongs to the *list* renderer S20, whose empty case has no lines to show).
    /// This mirrors S27/S28/S29/S30/S31/S32/S33 exactly, which emit `"0"` for their empty lists. There
    /// is no source slicing — only `.len()` is taken.
    ///
    /// Concretely, for a body that defines `\label{x}` twice and `\label{y}` once:
    ///
    /// ```text
    /// 1
    /// ```
    ///
    /// (one losing duplicate — the second `\label{x}`; the winning first `\label{x}` and the lone
    /// `\label{y}` are in `definitions`, the "2" S29 reports). A document with **no** duplicate
    /// `\label`s — no labels at all, or every key defined exactly once — returns `"0"`.
    ///
    /// **Partition note.** S29 counts `definitions` (one per distinct key, the winners) and S34 counts
    /// `duplicates` (the losers); together they **partition** every `\label` in the document —
    /// `label_definition_count + duplicate_label_count` is exactly the number of `\label`s, because
    /// [`Document::resolve_references`] routes each `\label` into exactly one of `definitions` or
    /// `duplicates`.
    ///
    /// ## Additive by construction
    ///
    /// S34 is a brand-new, read-only method that reuses [`resolve_references`](Document::resolve_references)
    /// and mutates nothing; it changes no S1–S33 output (they are byte-for-byte unchanged) and leaves the
    /// `to_latex` round-trip fixed point intact. It is a *second view* of the same `duplicates` list S20
    /// renders per-source — counting never adds, drops, or reorders the duplicate `\label`s relative to
    /// what `resolve_references` produced.
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing (no source slicing at all —
    /// only `.len()` is read); a single read of the already-bounded `duplicates` list's length. Borrows
    /// `self` immutably and returns owned `String` data, so the result outlives any borrow of the
    /// source.
    pub fn duplicate_label_count(&self) -> String {
        // S1 already routed every later `\label` of an already-defined key into `duplicates`, in body
        // pre-order (first-definition-wins; the winning first `\label`s went to `definitions`, S22/S29).
        // We only read that list's length. No `key`/`kind`/`span` is read — every losing `\label` folds
        // into a single total, the "multiply defined" warning-side companion of S29 and the label-side
        // twin of S33, the numeric summary of S20's per-source duplicate list.
        self.resolve_references().duplicates.len().to_string()
    }
}

/// The plain-text rendering of a float's optional `\caption{…}` (LTXDOC03 S12).
///
/// `Some(caption)` → the caption's inlines flattened to their visible text: [`Inline::Text`] runs
/// verbatim, [`Inline::Space`] as a single space, and the text *inside* font wrappers
/// (`\textbf`/`\emph`/`\texttt`/other `Styled`) recursively — the same descent the caption test
/// exercises. Leading/trailing whitespace is trimmed so a `\caption{ x }` reads as `x`. `None` (no
/// `\caption`) → the fixed placeholder `(no caption)`, which keeps every float on its own numbered
/// line so the List-of numbering stays aligned with the real float count.
fn caption_text(caption: &Option<crate::document::Caption>) -> String {
    let Some(cap) = caption else {
        return "(no caption)".to_string();
    };
    flatten_inlines_to_text(&cap.content)
}

/// Flatten a run of inlines to their **visible plain text**, trimmed (LTXDOC03 S12/S13).
///
/// The single descent shared by [`caption_text`] (S12, float captions) and
/// [`Document::resolve_namerefs`] (S13, section titles) so both name-rendering paths agree on
/// exactly which inlines contribute text and how:
///
/// - [`Inline::Text`] / [`Inline::Code`] → their string verbatim;
/// - [`Inline::Space`] → a single ASCII space;
/// - [`Inline::Strong`] / [`Inline::Emph`] / [`Inline::Styled`] → **recurse** into the wrapped
///   content (a `\textbf{Intro}` heading reads as `Intro`, dropping only the font wrapper);
/// - every other inline (math, cross-ref, accent, raw) contributes **no** plain text.
///
/// Leading/trailing whitespace is trimmed, so `\caption{ x }` and `\section{ Intro }` both read as
/// their tight text. Returns owned `String` data (one allocation for the accumulator), so the
/// result outlives any borrow of the source.
fn flatten_inlines_to_text(inlines: &[Inline]) -> String {
    fn flatten(inlines: &[Inline], out: &mut String) {
        for i in inlines {
            match i {
                Inline::Text(t, _) => out.push_str(t),
                Inline::Space(_) => out.push(' '),
                Inline::Strong(c, _) | Inline::Emph(c, _) => flatten(c, out),
                Inline::Styled { content, .. } => flatten(content, out),
                Inline::Code(t, _) => out.push_str(t),
                _ => {}
            }
        }
    }
    let mut s = String::new();
    flatten(inlines, &mut s);
    s.trim().to_string()
}

// =================================================================================================
// LTXDOC03 S5 — citation numbering (bracketed bibliography numbers over S2's resolution).
//
// S1 numbered nothing; S2 bound each `\cite` to its `\bibitem`; S4 numbered *sections and floats*
// but explicitly left **citations** unnumbered. S5 fills that gap: it assigns the bracketed number
// LaTeX prints for a `\cite` — the `[2]` in "as shown in [2]" — over the bibliography S2 already
// resolved. It is the citation-family analogue of S4's [`Document::ref_number`].
//
// ## LaTeX's citation-numbering model (the `.aux` dance, numbered style)
//
// In the default *numeric, unsorted* bibliography style (`plain`-family, hand-written
// `thebibliography`, or `unsrt`), every `\bibitem` is numbered by its **position in the list**: the
// first `\bibitem` is `[1]`, the second `[2]`, the third `[3]`, …. On the first `latex` run each
// `\bibitem{key}` fires the `enumiv`/bibliography counter and writes a `\bibcite{key}{n}` line into
// `document.aux`; on the second run every `\cite{key}` reads `n` back and prints it in **brackets**.
// A `\cite` to two keys, `\cite{a,c}`, prints both numbers, `[1, 3]`. A `\cite` whose key has no
// `\bibitem` prints the tell-tale `[?]` and warns *"Citation `key' undefined"*.
//
// S5 is the static, single-pass, in-document analogue: a flat counter over S2's **already-ordered**
// winning entry list. No `.aux`, no second parse — we number [`CitationResolution::entries`] by their
// index (the exploratory parse confirmed `entries` is in `\bibitem` *listing order*), rendering entry
// `entries[0]` as `[1]`, `entries[1]` as `[2]`, and so on.
//
// ## The three rules S5 implements (each confirmed against S2's data)
//
// 1. **Listing-order numbering.** `entries[i]` → `i + 1`, rendered bracketed. Because S2 collects
//    `\bibitem`s in pre-order (source order), `entries[0]` *is* the first `\bibitem` in the source, so
//    the index-based number matches LaTeX's list position exactly.
// 2. **First-`\bibitem`-wins duplicates consume no number.** A key defined by two `\bibitem`s puts the
//    *first* in `entries` and the *second* in [`CitationResolution::duplicate_entries`] — the losing
//    duplicate is **not** in `entries`, so it never advances the counter. Confirmed: with entries
//    `a, b, c` and a later duplicate `\bibitem{a}`, `entries == [a, b, c]` and `c` is still `[3]` (the
//    duplicate did not push it to `[4]`). This mirrors LaTeX: a re-declared `\bibitem` warns and is
//    numbered *the same* as the first — it does not consume a fresh slot.
// 3. **Dangling `\cite`s are unnumbered.** A `\cite{missing}` whose key has no `\bibitem` is in
//    [`CitationResolution::unresolved`], so it carries no [`ResolvedCite`] and there is no entry to
//    number — [`CitationNumbering::number_for`] returns `None` for it (never a panic). This is the
//    `[?]` case, the honest boundary S1-S4 all draw around undefined keys.
//
// ## What S5 numbers, and what is DEFERRED (the honest boundary)
//
// S5 numbers **in-document `thebibliography` citations only**, in the default numeric/unsorted style.
// It deliberately does **not** yet assign:
//
// - **Equation numbers** — as of S7 the AST *does* model an equation label: a non-starred
//   display-math env's `\label` is lifted onto [`Block::DisplayMath::label`], and it resolves and is
//   reported (with the [`EQUATION_NUMBER_PLACEHOLDER`]). What remains deferred is the equation
//   **counter** (`\theequation`) that would replace that placeholder with a real number — an S8 rung.
// - **Author-year / natbib sorted styles.** `plainnat`/`abbrvnat`/`alpha` renumber or re-*label*
//   entries (`[Smith2020]`, `[Smi20]`) and often **sort** the list, changing the number a key prints.
//   S5 models only the *listing-order numeric* style; sorted/author-year styles are a later rung.
// - **External `.bib` databases.** As with S2, only an in-document `thebibliography` is numbered; a
//   `\bibliography{refs}` reading an external `.bib`/`.bbl` is not (S5 does no file I/O, parses no
//   BibTeX). A `\cite` whose key lives only in an external database is *unresolved* by S2, hence
//   unnumbered here.
//
// ## The result type and payoff
//
// [`Document::number_citations`] returns a [`CitationNumbering`]: one owned row per numbered
// bibliography key, carrying its raw ordinal (`1`, `2`, …) and its rendered bracketed number (`"[1]"`,
// `"[2]"`, …). It mirrors S4's [`Numbering`] shape (owned `String`s + `Copy` ordinal) and provides a
// [`number_for`](CitationNumbering::number_for) lookup. [`Document::cite_number`] is the S2→S5 payoff
// convenience: given a resolved `\cite` (S2's [`ResolvedCite`]), return that citation's bracketed
// number — closing the loop `\cite{foo}` → `"[2]"`.
//
// **Additive & pure.** The S1-S4 result types are unchanged; S5 reads S2's [`CitationResolution`] and
// produces a new owned aggregate, mutating nothing about the tree or any prior pass.
// =================================================================================================

/// Render an entry's raw ordinal as the bracketed LaTeX citation number: `1` → `"[1]"`, `2` → `"[2]"`.
///
/// The single source of the bracket style — `\cite` prints its entry's number **in square brackets**
/// in the default numeric style, and every S5 rendering funnels through here so the bracket convention
/// lives in exactly one place (if a future rung needs a different delimiter, it changes only this
/// helper). Pure and total: a `usize` format, no allocation beyond the returned `String`, no panic.
fn render_cite_number(ordinal: usize) -> String {
    format!("[{ordinal}]")
}

// -------------------------------------------------------------------------------------------------
// The record types (plain, Clone-able, owned data — parallel to S4's NumberedLabel/Numbering).
// -------------------------------------------------------------------------------------------------

/// One **numbered citation**: a bibliography `key`, its raw `ordinal` (1-based list position), and the
/// rendered bracketed `number` LaTeX would print for a `\cite` to it. This is one row of S5's citation
/// numbering table — the static analogue of an `.aux` `\bibcite{key}{n}` line, capturing the number in
/// its printed bracketed form.
///
/// The `ordinal` is the entry's 1-based position in the (listing-order) winning entry list —
/// `entries[0]` → `1`, `entries[1]` → `2`, …; the `number` is that ordinal rendered through
/// [`render_cite_number`] (`"[1]"`, `"[2]"`, …). Both are exposed: the `ordinal` for callers that want
/// the bare count (to re-render in a different style, or sort), the `number` for the ready-to-print
/// bracketed string a `\cite` emits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberedCitation {
    /// The citation key, verbatim, without braces (`"smith2020"`).
    pub key: String,
    /// The entry's 1-based ordinal — its position in the `\bibitem` listing (`1`, `2`, …).
    pub ordinal: usize,
    /// The rendered bracketed number LaTeX would print for a `\cite` to this key (`"[1]"`, `"[2]"`, …).
    pub number: String,
}

/// The full result of [`Document::number_citations`]: the citation-numbering table — one
/// [`NumberedCitation`] row per **numbered bibliography key** (the winning `\bibitem`s, in listing
/// order; dangling `\cite`s and losing duplicates are omitted). All plain, owned data (keys + numbers
/// are `String`s, ordinals are `Copy`), so the numbering outlives any borrow of the source and can be
/// stored/serialized — mirroring S4's [`Numbering`].
///
/// **Ordering.** `entries` is in `\bibitem` **listing order** (the pre-order S2 collected the winning
/// entries in), so the table reads top-to-bottom like the bibliography and each row's `ordinal` is
/// just its index + 1. Only winning entries appear: a duplicate `\bibitem` (in S2's
/// [`CitationResolution::duplicate_entries`]) does **not** add a row and does **not** advance the
/// count — exactly as LaTeX numbers a re-declared entry the same as its first declaration.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CitationNumbering {
    /// One row per numbered bibliography key, in `\bibitem` listing order.
    pub entries: Vec<NumberedCitation>,
}

impl CitationNumbering {
    /// The rendered bracketed number of `key`, if it is a numbered bibliography entry, else `None`.
    /// Linear over `entries` (small: one row per distinct entry); no allocation. `&str` borrow into
    /// the owned row.
    pub fn number_for(&self, key: &str) -> Option<&str> {
        self.entries.iter().find(|e| e.key == key).map(|e| e.number.as_str())
    }
}

// -------------------------------------------------------------------------------------------------
// The citation-numbering pass.
// -------------------------------------------------------------------------------------------------

impl Document {
    /// Number every bibliography entry with its bracketed `\cite` number (LTXDOC03 S5).
    ///
    /// Built directly on S2: it runs [`resolve_citations`](Document::resolve_citations), then numbers
    /// the winning [`CitationResolution::entries`] by their **listing-order index** — `entries[0]` →
    /// `[1]`, `entries[1]` → `[2]`, …. Because S2 collects `\bibitem`s in source pre-order, the index
    /// matches LaTeX's list position exactly (the default numeric/unsorted style).
    ///
    /// A losing duplicate `\bibitem` (in [`CitationResolution::duplicate_entries`]) is **not** in
    /// `entries`, so it neither adds a row nor advances the counter — a re-declared entry is numbered
    /// the same as its first declaration, consuming no fresh slot. A dangling `\cite` has no entry, so
    /// it is simply absent from the table (unnumbered).
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing; the ordinal comes from
    /// `enumerate` over `entries` (bounded by the number of distinct entries), reusing S2's bounded
    /// collection (no new recursion). Borrows `self` immutably; the returned [`CitationNumbering`] is
    /// owned plain data (keys + numbers copied out, ordinals `Copy`), so it outlives any borrow of the
    /// source. The tree is **not** mutated — numbering is pure analysis, leaving S1-S4 outputs
    /// byte-for-byte unchanged.
    pub fn number_citations(&self) -> CitationNumbering {
        let resolution = self.resolve_citations();
        let entries = resolution
            .entries
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                // 1-based ordinal: `entries[0]` is `[1]`. `index + 1` cannot overflow in any real
                // document (the entry count is bounded by the parsed tree), and `saturating_add`
                // keeps it total even at the theoretical `usize::MAX` edge.
                let ordinal = index.saturating_add(1);
                NumberedCitation {
                    key: entry.key.clone(),
                    ordinal,
                    number: render_cite_number(ordinal),
                }
            })
            .collect();
        CitationNumbering { entries }
    }

    /// The rendered bracketed **number** a resolved citation prints (LTXDOC03 S5) — the payoff that
    /// ties S2 resolution to S5 numbering: given a [`ResolvedCite`] (`\cite{foo}`'s binding to its
    /// `\bibitem`), return that entry's number (`"[2]"`), or `None` if the key is not a numbered entry
    /// — a **documented, total** outcome, never a panic. (In practice a genuine [`ResolvedCite`]
    /// *always* names a winning entry, so `None` is the honest edge — e.g. a hand-built `ResolvedCite`
    /// with a key no `\bibitem` defines.)
    ///
    /// Convenience over [`number_citations`](Document::number_citations): it numbers the document, then
    /// looks the citation's `key` up. (A caller numbering *many* citations should call
    /// `number_citations` **once** and reuse the [`CitationNumbering`]; this method re-numbers per
    /// call, which is O(entries) each — fine for a one-off lookup.)
    pub fn cite_number(&self, c: &ResolvedCite) -> Option<String> {
        self.number_citations().number_for(&c.key).map(str::to_string)
    }
}

// =================================================================================================
// LTXDOC03 S6 — the cross-reference report (the consumer that composes S1/S2/S4/S5).
//
// S1 bound each `\ref` to its `\label`; S2 bound each `\cite` to its `\bibitem`; S4 numbered the
// labels; S5 numbered the citations. Each pass produced its own owned result type, but **nothing yet
// assembles them into a single consumer-facing artifact**. S6 is that assembly: one method that walks
// S1's resolved `\ref`s and S2's resolved `\cite`s and produces an **owned, plain-data report** where
// each entry carries its rendered *number* (from S4/S5) alongside its key/command/kind. It is the
// payoff rung — the proof that the five analysis passes *compose* into an auditable whole, exactly the
// shape a byte-provenance consumer wants: "here is every cross-reference in this document, what it
// points at, and the number it prints."
//
// ## What S6 is, and (just as importantly) what it is *not*
//
// S6 is a **pure consumer**. It adds **no new AST walk** of its own: it calls S1
// ([`resolve_references`](Document::resolve_references)) and S2
// ([`resolve_citations`](Document::resolve_citations)) — each of which already reuses the bounded
// [`Document::walk`] — and S4 ([`number_labels`](Document::number_labels)) / S5
// ([`number_citations`](Document::number_citations)) to number each family **once**, then *looks each
// key up* in the resulting number table. So the whole report costs a constant number of the existing
// bounded passes (never a per-entry re-numbering — the anti-pattern the S4/S5 convenience methods
// [`ref_number`](Document::ref_number) / [`cite_number`](Document::cite_number) warn about when called
// in a loop). There is no new parsing, no AST change, no new recursion.
//
// ## The two families, side by side
//
// | family | source pass (binding) | number pass | report entry |
// |--------|-----------------------|-------------|--------------|
// | `\ref`  | S1 `ResolvedRef { key, command, kind }` | S4 `Numbering::number_for(key)` | [`RefEntry`]  |
// | `\cite` | S2 `ResolvedCite { key }`               | S5 `CitationNumbering::number_for(key)` | [`CiteEntry`] |
//
// ## Dangling references and citations — surfaced *separately* (the "??" / "[?]" markers)
//
// A `\ref{missing}` (no such `\label`) lands in S1's `unresolved`; a `\cite{ghost}` (no such
// `\bibitem`) lands in S2's `unresolved`. These are LaTeX's tell-tale `??` (undefined reference) and
// `[?]` (undefined citation) markers. S6 does **not** silently drop them and does **not** fold them in
// among the resolved entries with some fake number — it surfaces them in their **own** vectors
// ([`dangling_refs`](CrossReferenceReport::dangling_refs) /
// [`dangling_cites`](CrossReferenceReport::dangling_cites)), each holding just the offending *key*. A
// consumer building a "problems" list reads exactly those two vectors; a consumer building the
// resolved cross-reference index reads `refs`/`cites`. This separation is the deliberate, documented
// choice (the alternative — one list with `number: Option<String>` — buries the distinction inside a
// field the caller must remember to check; two vectors make "resolved vs dangling" a *type-level*
// fact).
//
// ## The one subtlety: a *resolved* `\ref` whose target is not *numbered*
//
// Every entry in S1's `resolved` has a matching `\label` — but not every `\label` is *numbered*. S4
// numbers sections, figure/table floats, and (as of S7) **non-starred display-math equation labels**.
// A **bare inline `\label`** (a `\label` not lifted onto any block — [`LabelKind::Inline`]) is still
// deliberately left unnumbered. So a `\ref{eq:x}` to a *bare inline* `\label{eq:x}` *resolves* (it is
// in S1's `resolved`) yet has **no** S4 number, and S6's rule for such an entry is: **omit it from
// `refs`**. An equation label lifted out of a `\begin{equation}` (S7), by contrast, *is* numbered — it
// carries the [`EQUATION_NUMBER_PLACEHOLDER`] (`"?"`) until S8 assigns the real counter — so an
// `\eqref` to it is **included** in `refs` (rendering `\ref{eq:e} -> Equation ?`) rather than omitted.
// (A bare-inline unnumbered-but-resolved `\ref` is neither *dangling* — its label exists — nor
// *renderable*, so it belongs in neither `refs` nor `dangling_refs`; it reappears once inline-label
// numbering ships.) Citations have no analogous gap — every *resolved* `\cite` names a winning
// `\bibitem`, which S5 always numbers — so every S2-resolved cite becomes a [`CiteEntry`].
//
// ## What stays OUT of S6 (inherited honest boundaries)
//
// - **Equation counter values** — S7 lifts and *resolves* equation labels (they appear in `refs` with
//   the [`EQUATION_NUMBER_PLACEHOLDER`]), but the true `\theequation` number is deferred to S8. A
//   *bare inline* `\label` (not in a numbered display-math env) is still omitted from `refs`.
// - **Author-year / natbib sorted citation styles** and **external `.bib`/`.bbl` databases** — out of
//   scope at S2/S5, so out of scope here too (S6 reports only what those passes resolved).
//
// ## The result types and the rendered report
//
// [`Document::cross_reference_report`] returns a [`CrossReferenceReport`]: `refs` (one [`RefEntry`]
// per numbered resolved `\ref`, in S1 pre-order), `cites` (one [`CiteEntry`] per resolved `\cite`, in
// S2 pre-order), and the two dangling-key vectors. All owned plain data (`String`s + `Copy`
// [`LabelKind`]), mirroring S4/S5, so the report outlives any borrow of the source and can be
// stored/serialized. [`CrossReferenceReport::to_plain_text`] renders it to a stable, deterministic,
// human-readable string (the exact format is pinned in that method's docs and in the tests).
// =================================================================================================

/// A **human-readable name** for a label kind, capitalised for the plain-text report: `"Section"`,
/// `"Table"`, `"Figure"`, `"Inline"`. This is the display form S6 prints (`\ref{s:i} -> Section 1`),
/// distinct from [`LabelKind::as_str`]'s lowercase structural name (`"section"`). Kept as a single
/// helper so the capitalisation convention lives in exactly one place. Pure and total — a fixed match,
/// no allocation beyond the returned `&'static str`, no panic.
fn kind_display_name(kind: LabelKind) -> &'static str {
    match kind {
        LabelKind::Section => "Section",
        LabelKind::Table => "Table",
        LabelKind::Figure => "Figure",
        LabelKind::Equation => "Equation",
        LabelKind::Inline => "Inline",
    }
}

/// Render **one** resolved-reference [`RefEntry`] to its single plain-text line — the shared
/// per-command rendering used by **both** [`CrossReferenceReport::to_plain_text`] (S6, flat pre-order)
/// and [`CrossReferenceReport::to_plain_text_by_kind`] (S11, grouped-by-kind). Factoring the three
/// precedence-ordered renderings into this one place is what guarantees the flat and grouped reports
/// can **never** drift — they call the identical code, so a `\ref`/`\eqref`/`\pageref` line is
/// byte-for-byte the same wherever it appears.
///
/// Three renderings, tried in this precedence order, keyed off the *surface command* + *target kind*
/// (LTXDOC03 S9/S10):
///
///   1. An `\eqref` whose target is an **equation** renders amsmath-style — the `\eqref` spelling is
///      kept and the number is **parenthesised**: `\eqref{eq:e} -> Equation (1)`. (`\eqref` on
///      `article`'s equation counter typesets "(1)", so the report mirrors that surface form.) (S9.)
///   2. Otherwise, a `\pageref` (to **any** kind) renders with the `\pageref` spelling and the literal
///      placeholder `page ?`: `\pageref{sec:i} -> page ?`. A page reference asks *what page* the target
///      is on; the crate has NO page model, so the target's kind and number are irrelevant — the
///      honest fixed placeholder is `page ?` (the `?` mirrors LaTeX's own `??` for an unresolved page
///      ref). (S10.)
///   3. Every other resolved reference — all `\ref` and any `\eqref` to a **non-equation** kind —
///      renders with the canonical `\ref` prefix and a **bare** number: `\ref{sec:intro} -> Section
///      1.2`.
///
/// Pure and total: string building only (no indexing, no `unwrap`), allocating just the returned line.
fn render_resolved_ref(r: &RefEntry) -> String {
    if r.command == "eqref" && r.kind == LabelKind::Equation {
        format!(r"\eqref{{{}}} -> {} ({})", r.key, kind_display_name(r.kind), r.number)
    } else if r.command == "pageref" {
        // No page model: a fixed, honest placeholder, independent of kind/number.
        format!(r"\pageref{{{}}} -> page ?", r.key)
    } else {
        format!(r"\ref{{{}}} -> {} {}", r.key, kind_display_name(r.kind), r.number)
    }
}

/// The **pluralised** capitalised subheading a kind gets in the S11 grouped report: `"Sections"`,
/// `"Figures"`, `"Tables"`, `"Equations"`, `"Inline"`. Distinct from [`kind_display_name`]'s singular
/// per-line form (`"Section"`) because the S11 subheading names a *group* of references. `Inline` has
/// no natural plural here (it is not a numbered display kind), so it is left as-is. Pure, total, one
/// fixed match.
fn kind_group_heading(kind: LabelKind) -> &'static str {
    match kind {
        LabelKind::Section => "Sections",
        LabelKind::Table => "Tables",
        LabelKind::Figure => "Figures",
        LabelKind::Equation => "Equations",
        LabelKind::Inline => "Inline",
    }
}

/// The **fixed kind order** the S11 grouped report emits its subheadings in — Sections, then Figures,
/// then Tables, then Equations, then Inline — *regardless* of the source order references appear in.
/// A group is only emitted if it has ≥1 resolved ref (see
/// [`to_plain_text_by_kind`](CrossReferenceReport::to_plain_text_by_kind)); within a group the refs
/// keep their pre-order.
const S11_KIND_ORDER: [LabelKind; 5] = [
    LabelKind::Section,
    LabelKind::Figure,
    LabelKind::Table,
    LabelKind::Equation,
    LabelKind::Inline,
];

// -------------------------------------------------------------------------------------------------
// The report record types (plain, Clone-able, owned data — mirroring S4/S5's owned-String rows).
// -------------------------------------------------------------------------------------------------

/// One **resolved-and-numbered reference** row of the cross-reference report: a `\ref`/`\eqref`/
/// `\pageref` that both resolved (S1) *and* carries a rendered S4 number. It fuses S1's binding
/// (`key`, `command`, `kind`) with S4's `number` — everything a consumer needs to render "see
/// Section 1.2" without re-consulting the passes.
///
/// - `key` — the referenced label key, verbatim (`"sec:intro"`), from S1's [`ResolvedRef::key`].
/// - `command` — the reference command used (`"ref"`, `"eqref"`, or `"pageref"`), from
///   [`ResolvedRef::command`].
/// - `kind` — the [`LabelKind`] of the node the reference points at (section / table / figure /
///   equation), from [`ResolvedRef::target_kind`]. (A bare [`LabelKind::Inline`] target is *not*
///   numbered by S4, so it never becomes a `RefEntry` — see the S6 module docs — hence in practice
///   `kind` is one of Section/Table/Figure/Equation.)
/// - `number` — the rendered number S4 assigns the target (`"1.2"`, `"3"`, …), from
///   [`Numbering::number_for`]. For a [`LabelKind::Equation`] this is the
///   [`EQUATION_NUMBER_PLACEHOLDER`] (`"?"`) until S8 wires the equation counter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefEntry {
    /// The referenced label key, verbatim, without braces (`"sec:intro"`).
    pub key: String,
    /// The reference command used (`"ref"`, `"eqref"`, or `"pageref"`).
    pub command: String,
    /// The kind of node the reference resolved to (section / table / figure).
    pub kind: LabelKind,
    /// The rendered number the target prints (`"1.2"`, `"3"`, …), from S4.
    pub number: String,
}

/// One **resolved-and-numbered citation** row of the cross-reference report: a `\cite` key that
/// resolved (S2) and carries its rendered S5 bracketed number. Every resolved `\cite` yields a
/// `CiteEntry` (unlike references, there is no "resolved-but-unnumbered" gap — a winning `\bibitem` is
/// always numbered by S5).
///
/// - `key` — the citation key, verbatim (`"smith2020"`), from S2's [`ResolvedCite::key`]. A multi-key
///   `\cite{a,b}` produces one `CiteEntry` **per key** (S2 already split them), so `a` and `b` are two
///   separate rows.
/// - `number` — the rendered bracketed number S5 assigns the entry (`"[1]"`, `"[2]"`, …), from
///   [`CitationNumbering::number_for`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CiteEntry {
    /// The citation key, verbatim, without braces (`"smith2020"`).
    pub key: String,
    /// The rendered bracketed number the citation prints (`"[1]"`, `"[2]"`, …), from S5.
    pub number: String,
}

/// The full result of [`Document::cross_reference_report`]: the assembled cross-reference report — the
/// consumer artifact that composes S1 (resolve refs) + S2 (resolve cites) + S4 (label numbers) + S5
/// (citation numbers) into one auditable table. All plain, owned data (`String`s + `Copy`
/// [`LabelKind`]), so the report outlives any borrow of the source and can be stored/serialized,
/// mirroring S1/S2/S4/S5's owned aggregates.
///
/// **Ordering.** `refs` is in S1 pre-order (the order the `\ref`s appear in the body, filtered to the
/// numbered ones); `cites` is in S2 pre-order (body order, one row per key of each `\cite`);
/// `dangling_refs` is in S1's `unresolved` order and `dangling_cites` in S2's `unresolved` order.
/// Every ordering is deterministic and source-derived, so the report (and its
/// [`to_plain_text`](CrossReferenceReport::to_plain_text) rendering) is stable across runs.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CrossReferenceReport {
    /// One row per **resolved and numbered** `\ref`, in S1 pre-order. A resolved `\ref` whose target
    /// is an (unnumbered) inline/equation label is *omitted* (it has no S4 number — see the S6 docs).
    pub refs: Vec<RefEntry>,
    /// One row per **resolved** `\cite` key, in S2 pre-order (one per key of a multi-key `\cite`).
    pub cites: Vec<CiteEntry>,
    /// The keys of **dangling** `\ref`s (S1's `unresolved`) — LaTeX's `??` undefined references.
    pub dangling_refs: Vec<String>,
    /// The keys of **dangling** `\cite`s (S2's `unresolved`) — LaTeX's `[?]` undefined citations.
    pub dangling_cites: Vec<String>,
}

impl CrossReferenceReport {
    /// Render this report to a **stable, deterministic, human-readable** plain-text string (LTXDOC03
    /// S6). The exact format — pinned so tests and consumers can rely on it byte-for-byte:
    ///
    /// - One line per resolved reference, in `refs` order. There are **three** renderings, tried in
    ///   this precedence order (LTXDOC03 S10):
    ///   - An `\eqref` whose target is an **equation** renders amsmath-style, keeping the `\eqref`
    ///     spelling and **parenthesising** the number:
    ///     `` \eqref{<key>} -> <Kind> (<number>) `` — e.g. `` \eqref{eq:e} -> Equation (1) ``. This
    ///     mirrors how amsmath's `\eqref` typesets the equation number as `(1)`. (S9, unchanged.)
    ///   - Otherwise, a `\pageref` (to **any** kind — Section/Table/Figure/Equation/Inline) renders
    ///     with the `\pageref` spelling and the literal placeholder `page ?`:
    ///     `` \pageref{<key>} -> page ? `` — e.g. `` \pageref{sec:i} -> page ? ``. A `\pageref` asks
    ///     *what page* its target is on, but the crate has **no page model**, so neither the target's
    ///     kind nor its number is relevant; the honest, fixed placeholder is `page ?` (the `?` mirrors
    ///     LaTeX's own `??` for an unresolved page reference, and the S7 number-placeholder pattern).
    ///     (S10, NEW — previously a `\pageref` rendered identically to a `\ref`.)
    ///   - Every **other** resolved reference — all `\ref` and any `\eqref` to a **non-equation**
    ///     kind — renders with the canonical `\ref` prefix and a **bare** number:
    ///     `` \ref{<key>} -> <Kind> <number> `` — e.g. `` \ref{sec:intro} -> Section 1.2 ``. The
    ///     `\ref` prefix here names the *binding*, not the surface command, unchanged from S8.
    ///
    ///   In the first and third renderings `<Kind>` is the capitalised kind name
    ///   ([`kind_display_name`]); the `\pageref` rendering carries no kind or number at all.
    /// - One line per resolved citation, in `cites` order:
    ///   `` \cite{<key>} -> <number> `` — e.g. `` \cite{smith} -> [2] ``.
    /// - **Only if** `dangling_refs` is non-empty, a footer line:
    ///   `Dangling references: <k1>, <k2>, …` (keys joined by `", "`, in order).
    /// - **Only if** `dangling_cites` is non-empty, a footer line:
    ///   `Dangling citations: <k1>, <k2>, …`.
    ///
    /// Lines are joined by a single `\n` with **no trailing newline** and no trailing whitespace on
    /// any line. An **empty report** (no refs, cites, or dangling keys) renders the fixed string
    /// `"(no cross-references)"` so the output is never the empty string (a stable, greppable
    /// "nothing here" marker). Sections appear in a fixed order — resolved refs, resolved cites,
    /// dangling refs, dangling cites — so the whole rendering is a pure function of the report.
    ///
    /// Total & panic-free: string building only (no indexing, no `unwrap`), allocating just the output
    /// `String`.
    pub fn to_plain_text(&self) -> String {
        let mut lines: Vec<String> = Vec::new();

        // Resolved references — each rendered by the shared per-command helper
        // [`render_resolved_ref`], so this S6 rendering and the S11 grouped rendering
        // ([`to_plain_text_by_kind`](CrossReferenceReport::to_plain_text_by_kind)) can never drift.
        for r in &self.refs {
            lines.push(render_resolved_ref(r));
        }
        // Resolved citations: `\cite{key} -> number`.
        for c in &self.cites {
            lines.push(format!(r"\cite{{{}}} -> {}", c.key, c.number));
        }
        // Dangling footers, each only when non-empty (keys joined by ", ").
        if !self.dangling_refs.is_empty() {
            lines.push(format!("Dangling references: {}", self.dangling_refs.join(", ")));
        }
        if !self.dangling_cites.is_empty() {
            lines.push(format!("Dangling citations: {}", self.dangling_cites.join(", ")));
        }

        if lines.is_empty() {
            // A stable, greppable marker so the rendering is never the empty string.
            "(no cross-references)".to_string()
        } else {
            lines.join("\n")
        }
    }

    /// Render the report's **resolved references only**, grouped under fixed-order kind subheadings
    /// (LTXDOC03 S11). A *sibling* of [`to_plain_text`](CrossReferenceReport::to_plain_text) — it
    /// leaves that flat rendering untouched — that re-organises the **same** resolved-ref lines from
    /// source order into per-kind groups, so a reader can see "which sections / figures / equations
    /// does this document cross-reference?" at a glance.
    ///
    /// The exact format, pinned so tests and consumers can rely on it byte-for-byte:
    ///
    /// - Kinds are emitted in a **fixed order** — **Sections, Figures, Tables, Equations, Inline** —
    ///   *regardless* of the order references appear in the source (see [`S11_KIND_ORDER`]).
    /// - For each kind with **≥1** resolved ref, a subheading line — the pluralised capitalised kind
    ///   name plus a colon (`Sections:`, `Figures:`, `Tables:`, `Equations:`, `Inline:`, from
    ///   [`kind_group_heading`]) — followed by **one line per ref**, each **indented by two spaces**
    ///   then rendered by the **shared** [`render_resolved_ref`] helper — the *identical* per-command
    ///   rule [`to_plain_text`](CrossReferenceReport::to_plain_text) uses, so an `\eqref` to an
    ///   equation still reads `\eqref{eq:e} -> Equation (1)` and a `\pageref` still reads
    ///   `\pageref{key} -> page ?`. Within a kind group the refs keep their existing pre-order (this
    ///   filters [`refs`](CrossReferenceReport::refs) for that kind, preserving order).
    ///
    ///   ```text
    ///   Sections:
    ///     \ref{sec:intro} -> Section 1
    ///     \ref{sec:methods} -> Section 2
    ///   Figures:
    ///     \ref{fig:plot} -> Figure 1
    ///   ```
    /// - A kind with **zero** resolved refs is **omitted entirely** — no empty subheading.
    /// - Only resolved **references** are grouped; citations and the dangling footers are **not**
    ///   included (this method stays focused on the kind-grouped resolved refs — the flat
    ///   [`to_plain_text`](CrossReferenceReport::to_plain_text) remains the place for the full report).
    /// - If there are **zero** resolved refs at all, the fixed string `"(no resolved references)"` is
    ///   returned (a stable, greppable marker — the S11 analogue of `to_plain_text`'s
    ///   `"(no cross-references)"`), so the output is never the empty string.
    ///
    /// Lines are joined by a single `\n` with **no trailing newline** and no trailing whitespace on any
    /// line. The kind order is fixed and the within-group order is source-derived, so the whole
    /// rendering is a pure function of the report.
    ///
    /// Total & panic-free: string building only (no indexing, no `unwrap`), allocating just the output
    /// `String`.
    pub fn to_plain_text_by_kind(&self) -> String {
        let mut lines: Vec<String> = Vec::new();

        // Walk the fixed kind order; for each kind, gather its refs in pre-order and, if any, emit a
        // subheading followed by the two-space-indented per-ref lines (shared render helper).
        for &kind in &S11_KIND_ORDER {
            let group: Vec<&RefEntry> = self.refs.iter().filter(|r| r.kind == kind).collect();
            if group.is_empty() {
                continue; // omit empty kinds entirely — no bare subheading.
            }
            lines.push(format!("{}:", kind_group_heading(kind)));
            for r in group {
                // Two-space indent, then the SAME per-command line the flat report emits.
                lines.push(format!("  {}", render_resolved_ref(r)));
            }
        }

        if lines.is_empty() {
            // A stable, greppable marker so the rendering is never the empty string.
            "(no resolved references)".to_string()
        } else {
            lines.join("\n")
        }
    }
}

// -------------------------------------------------------------------------------------------------
// The report assembly.
// -------------------------------------------------------------------------------------------------

impl Document {
    /// Assemble the **cross-reference report** for this document (LTXDOC03 S6) — the consumer artifact
    /// that composes S1/S2/S4/S5 into one owned, auditable table.
    ///
    /// The assembly, with **no new AST walk** (each family is numbered **once**, then looked up):
    ///
    /// 1. Run S1 [`resolve_references`](Document::resolve_references) and number the labels **once**
    ///    with S4 [`number_labels`](Document::number_labels). For each [`ResolvedRef`], look its `key`
    ///    up in the [`Numbering`] via [`number_for`](Numbering::number_for): a `Some(number)` becomes a
    ///    [`RefEntry`] (key + command + kind + number); a `None` (a resolved `\ref` to an *unnumbered*
    ///    inline/equation label — see the S6 module docs) is **omitted**. S1's `unresolved` keys become
    ///    [`dangling_refs`](CrossReferenceReport::dangling_refs).
    /// 2. Run S2 [`resolve_citations`](Document::resolve_citations) and number the entries **once** with
    ///    S5 [`number_citations`](Document::number_citations). For each [`ResolvedCite`], look its `key`
    ///    up in the [`CitationNumbering`] → a [`CiteEntry`] (key + bracketed number). (A resolved cite
    ///    always names a winning entry, which S5 always numbers, so this lookup is `Some` in practice;
    ///    a defensive `None` is simply skipped, never a panic.) S2's `unresolved` keys become
    ///    [`dangling_cites`](CrossReferenceReport::dangling_cites).
    ///
    /// **Numbered once, not per-item.** Numbering the two families a single time (then O(1)-ish
    /// `number_for` lookups) is deliberate: it avoids the per-entry re-numbering the
    /// [`ref_number`](Document::ref_number) / [`cite_number`](Document::cite_number) convenience
    /// methods do (fine for a one-off, wasteful in a loop). So the report costs a *constant* number of
    /// the existing bounded passes — S1, S2, S4, S5 — no more.
    ///
    /// **Total & panic-free.** No `unwrap`/`expect`, no unchecked indexing; it only *calls* the S1/S2/
    /// S4/S5 passes (each of which reuses the bounded [`Document::walk`]) and copies owned data out of
    /// their results — introducing no new recursion. Borrows `self` immutably; the returned
    /// [`CrossReferenceReport`] is owned plain data, so it outlives any borrow of the source. The tree
    /// is **not** mutated — the report is pure composition, leaving S1–S5 outputs byte-for-byte
    /// unchanged.
    pub fn cross_reference_report(&self) -> CrossReferenceReport {
        // ---- References (S1 binding × S4 numbering) ----
        let references = self.resolve_references();
        let label_numbers = self.number_labels(); // numbered ONCE, reused for every lookup below.

        let mut refs: Vec<RefEntry> = Vec::new();
        for r in &references.resolved {
            // A resolved `\ref` with an S4 number becomes a row; a resolved-but-unnumbered one
            // (inline/equation label — deferred) is omitted, so every row carries a real number.
            if let Some(number) = label_numbers.number_for(&r.key) {
                refs.push(RefEntry {
                    key: r.key.clone(),
                    command: r.command.clone(),
                    kind: r.target_kind,
                    number: number.to_string(),
                });
            }
        }
        let dangling_refs: Vec<String> =
            references.unresolved.iter().map(|u| u.key.clone()).collect();

        // ---- Citations (S2 binding × S5 numbering) ----
        let citations = self.resolve_citations();
        let cite_numbers = self.number_citations(); // numbered ONCE, reused for every lookup below.

        let mut cites: Vec<CiteEntry> = Vec::new();
        for c in &citations.resolved {
            // A resolved cite always names a winning, numbered entry; a defensive `None` is skipped.
            if let Some(number) = cite_numbers.number_for(&c.key) {
                cites.push(CiteEntry { key: c.key.clone(), number: number.to_string() });
            }
        }
        let dangling_cites: Vec<String> =
            citations.unresolved.iter().map(|u| u.key.clone()).collect();

        CrossReferenceReport { refs, cites, dangling_refs, dangling_cites }
    }
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
    fn s7_equation_label_is_registered_as_equation_kind() {
        // LTXDOC03 S7: a `\label` in a non-starred `equation` env is a real label def tagged Equation.
        let src = r"\begin{document}\begin{equation} E = mc^2 \label{eq:e} \end{equation}\end{document}";
        let (_src, res) = resolve(src);
        assert_eq!(res.definitions.len(), 1, "one equation label defined");
        let def = &res.definitions[0];
        assert_eq!(def.key, "eq:e");
        assert_eq!(def.kind, LabelKind::Equation);
        assert_eq!(def.kind.as_str(), "equation");
    }

    #[test]
    fn s7_starred_equation_registers_no_label() {
        // A starred `equation*` lifts no label, so there is NO definition (and the ref dangles).
        let src = r"\begin{document}\begin{equation*} E = mc^2 \label{eq:e} \end{equation*} \ref{eq:e}\end{document}";
        let (_src, res) = resolve(src);
        assert_eq!(res.definitions.len(), 0, "starred env defines no equation label");
        assert_eq!(res.unresolved.len(), 1, "the ref has nothing to resolve against");
        assert_eq!(res.unresolved[0].key, "eq:e");
    }

    #[test]
    fn s7_eqref_to_equation_is_included_in_cross_reference_report() {
        // The whole point of S7: a resolved `\eqref`/`\ref` to a lifted equation label is INCLUDED in
        // the S6 report, no longer omitted. S8 upgrade: the number it renders is now the real
        // sequential equation number (`Equation 1`), not the S7 placeholder (`Equation ?`).
        let src = r"\begin{document}\begin{equation} E = mc^2 \label{eq:e} \end{equation} See \eqref{eq:e} and \ref{eq:e}.\end{document}";
        let doc = parse_document(src).expect("parse");
        let report = doc.cross_reference_report();
        assert_eq!(report.refs.len(), 2, "both \\eqref and \\ref are included");
        for entry in &report.refs {
            assert_eq!(entry.key, "eq:e");
            assert_eq!(entry.kind, LabelKind::Equation);
            assert_eq!(entry.number, "1", "S8: the equation carries the real counter value");
        }
        assert!(report.dangling_refs.is_empty(), "nothing dangles");
        // The rendered report lines name the binding. Under S9 the `\eqref` (first, in source order)
        // parenthesises its number and keeps the `\eqref` spelling; the plain `\ref` stays bare.
        let text = report.to_plain_text();
        assert_eq!(text, "\\eqref{eq:e} -> Equation (1)\n\\ref{eq:e} -> Equation 1");
    }

    #[test]
    fn s9_eqref_to_equation_parenthesises_its_number() {
        // LTXDOC03 S9: an `\eqref` to an equation renders amsmath-style — `\eqref` spelling kept and
        // the number parenthesised — while a sibling `\ref` to the same equation stays a bare number.
        // The source lists `\eqref` BEFORE `\ref`, so that is the report's `refs` (S1 pre-order) order.
        let src = r"\begin{document}\begin{equation} E = mc^2 \label{eq:e} \end{equation} See \eqref{eq:e} and \ref{eq:e}.\end{document}";
        let doc = parse_document(src).expect("parse");
        let report = doc.cross_reference_report();
        let text = report.to_plain_text();
        assert_eq!(text, "\\eqref{eq:e} -> Equation (1)\n\\ref{eq:e} -> Equation 1");
    }

    #[test]
    fn s9_ref_to_equation_is_a_bare_number() {
        // A plain `\ref` (not `\eqref`) to an equation is UNCHANGED from S8: `\ref` prefix, bare
        // number, no parentheses. S9 only touches the `\eqref`-to-equation case.
        let src = r"\begin{document}\begin{equation} E = mc^2 \label{eq:e} \end{equation} See \ref{eq:e}.\end{document}";
        let doc = parse_document(src).expect("parse");
        let report = doc.cross_reference_report();
        let text = report.to_plain_text();
        assert_eq!(text, "\\ref{eq:e} -> Equation 1");
    }

    #[test]
    fn s9_two_eqrefs_number_sequentially_and_each_parenthesises() {
        // Two labelled equations number 1 then 2 (S8 sequential counter); each `\eqref` to them
        // parenthesises its own number, so the report reads `(1)` then `(2)`.
        let src = r"\begin{document}\begin{equation} a = 1 \label{eq:a} \end{equation}\begin{equation} b = 2 \label{eq:b} \end{equation} See \eqref{eq:a} and \eqref{eq:b}.\end{document}";
        let doc = parse_document(src).expect("parse");
        let report = doc.cross_reference_report();
        let text = report.to_plain_text();
        assert_eq!(text, "\\eqref{eq:a} -> Equation (1)\n\\eqref{eq:b} -> Equation (2)");
    }

    #[test]
    fn s10_pageref_renders_page_placeholder() {
        // LTXDOC03 S10: a resolved `\pageref` to a labelled section renders with the `\pageref`
        // spelling and the fixed `page ?` placeholder — NOT the S8 `\ref{...} -> Section 1` line.
        // (The crate has no page model, so a page reference cannot report a real page number.)
        let src = r"\begin{document}\section{Intro}\label{sec:i} \pageref{sec:i}\end{document}";
        let doc = parse_document(src).expect("parse");
        let report = doc.cross_reference_report();
        let text = report.to_plain_text();
        assert_eq!(text, "\\pageref{sec:i} -> page ?");
    }

    #[test]
    fn s10_ref_still_bare_and_pageref_distinct() {
        // A doc with BOTH a `\ref` and a `\pageref` to the same section: the `\ref` is UNCHANGED from
        // S8 (`\ref{sec:i} -> Section 1`) while the `\pageref` now diverges (`\pageref{sec:i} -> page
        // ?`). `\ref` first in source order, so it is the report's first line (S1 pre-order).
        let src = r"\begin{document}\section{Intro}\label{sec:i} \ref{sec:i} \pageref{sec:i}\end{document}";
        let doc = parse_document(src).expect("parse");
        let report = doc.cross_reference_report();
        let text = report.to_plain_text();
        assert_eq!(text, "\\ref{sec:i} -> Section 1\n\\pageref{sec:i} -> page ?");
    }

    #[test]
    fn s10_pageref_to_equation_is_still_page() {
        // A `\pageref` to a labelled EQUATION ignores the `\eqref`-to-equation special-case entirely:
        // it is a page reference, so it still renders `\pageref{eq:e} -> page ?` (no parentheses, no
        // Equation kind, no number). This proves the `\pageref` branch takes precedence over the S8
        // else-branch and is orthogonal to the S9 amsmath branch.
        let src = r"\begin{document}\begin{equation} E = mc^2 \label{eq:e} \end{equation} \pageref{eq:e}\end{document}";
        let doc = parse_document(src).expect("parse");
        let report = doc.cross_reference_report();
        let text = report.to_plain_text();
        assert_eq!(text, "\\pageref{eq:e} -> page ?");
    }

    #[test]
    fn s8_single_equation_numbers_as_one() {
        // A single lifted equation label numbers as "1" — the S8 counter's first value.
        let src = r"\begin{document}\begin{equation} E = mc^2 \label{eq:e} \end{equation}\end{document}";
        let doc = parse_document(src).expect("parse");
        let numbering = doc.number_labels();
        assert_eq!(numbering.number_for("eq:e"), Some("1"));
    }

    #[test]
    fn s8_two_equations_number_sequentially_in_document_order() {
        // Two lifted equation labels, in document order, number "1" then "2" — a pure monotonic run.
        let src = r"\begin{document}\begin{equation} a = 1 \label{eq:a} \end{equation}\begin{equation} b = 2 \label{eq:b} \end{equation}\end{document}";
        let doc = parse_document(src).expect("parse");
        let numbering = doc.number_labels();
        assert_eq!(numbering.number_for("eq:a"), Some("1"), "first equation is 1");
        assert_eq!(numbering.number_for("eq:b"), Some("2"), "second equation is 2");
    }

    #[test]
    fn s8_equation_counter_is_independent_of_section_counter() {
        // A `\section` before the equation numbers as "1" (section counter), but the equation still
        // numbers "1" — the equation counter is independent of the section counter.
        let src = r"\begin{document}\section{Intro}\label{sec:i} \begin{equation} E = mc^2 \label{eq:e} \end{equation}\end{document}";
        let doc = parse_document(src).expect("parse");
        let numbering = doc.number_labels();
        assert_eq!(numbering.number_for("sec:i"), Some("1"), "the section is 1");
        assert_eq!(numbering.number_for("eq:e"), Some("1"), "the equation is still 1, not perturbed");
    }

    #[test]
    fn s8_equation_counter_is_independent_of_figure_counter() {
        // A figure BETWEEN two equations must not perturb the equation sequence: eq:a=1, eq:b=2, with
        // the figure numbering "1" on its own independent (flat figure) counter.
        let src = r"\begin{document}\begin{equation} a = 1 \label{eq:a} \end{equation}\begin{figure}\caption{F}\label{fig:f}\end{figure}\begin{equation} b = 2 \label{eq:b} \end{equation}\end{document}";
        let doc = parse_document(src).expect("parse");
        let numbering = doc.number_labels();
        assert_eq!(numbering.number_for("eq:a"), Some("1"), "first equation is 1");
        assert_eq!(numbering.number_for("fig:f"), Some("1"), "the figure is 1 on its own counter");
        assert_eq!(numbering.number_for("eq:b"), Some("2"), "second equation is 2, figure did not perturb it");
    }

    #[test]
    fn s8_labelled_equation_to_latex_round_trip_is_a_fixed_point() {
        // S8 is pure analysis (numbering), leaving the tree unchanged: a labelled equation still
        // round-trips through `to_latex()` byte-for-byte (unchanged from S7).
        let src = r"\begin{document}\begin{equation} E = mc^2 \label{eq:e} \end{equation}\end{document}";
        let doc = parse_document(src).expect("parse");
        let once = doc.to_latex();
        let twice = parse_document(&once).expect("re-parse").to_latex();
        assert_eq!(once, twice, "to_latex() is a fixed point for a labelled equation");
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

    // ---------------------------------------------------------------------------------------------
    // LTXDOC03 S3 — target → NodeRef exposure.
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn ref_target_node_for_section_is_the_real_section_block() {
        // A `\ref{sec:intro}` → `\section` resolves (S1), and `ref_target_node` returns the ACTUAL
        // `Block::Section` node — kind "Section", span slices back to the `\section` source — and we
        // can DESCEND into it (its `body`/`title` are populated), proving it is the real node, not a
        // hollow handle.
        let src = r"\begin{document}\section{Intro}\label{sec:intro}

First paragraph of the intro. See Section~\ref{sec:intro}.\end{document}";
        let doc = parse_document(src).expect("parse");
        let refs = doc.resolve_references();

        let r = &refs.resolved[0];
        let node = doc.ref_target_node(r).expect("resolved ref must yield its target node");

        // It is a Block, of the Section kind, whose span slices back to the defining `\section`.
        assert_eq!(node.kind(), "Section", "target node is the section block");
        let NodeRef::Block(Block::Section { title, body, .. }) = node else {
            panic!("expected a Section block, got {}", node.kind());
        };
        let span = node.span();
        assert!(
            src[span.start..span.end].starts_with(r"\section{Intro}"),
            "the node's span slices back to the defining \\section"
        );
        // DESCEND: the section's title inline is reachable (the real "Intro" heading), and its owned
        // body carries the following paragraph — a hollow handle could not expose these.
        assert!(!title.is_empty(), "the section's title inlines are reachable");
        assert!(
            body.iter().any(|b| matches!(b, Block::Paragraph(..))),
            "the section owns the following paragraph (we can walk into its children)"
        );
    }

    #[test]
    fn ref_target_node_for_figure_reaches_its_caption() {
        // A `\ref{fig:plot}` → figure: `ref_target_node` returns the figure Block; kind == "Figure",
        // and we can reach its caption text — proving descent into the real float.
        let src = r"\begin{document}\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:plot}\end{figure}

As shown in Figure~\ref{fig:plot}.\end{document}";
        let doc = parse_document(src).expect("parse");
        let refs = doc.resolve_references();

        let r = &refs.resolved[0];
        let node = doc.ref_target_node(r).expect("resolved figure ref must yield its node");

        assert_eq!(node.kind(), "Figure");
        let NodeRef::Block(Block::Figure { caption, .. }) = node else {
            panic!("expected a Figure block, got {}", node.kind());
        };
        // DESCEND: the figure's caption is reachable and holds the "A plot" text — we slice the
        // caption's text inline back to source to prove we reached the real caption content.
        let cap = caption.as_ref().expect("the figure carries a \\caption");
        let has_plot_text = cap.content.iter().any(|i| match i {
            Inline::Text(t, _) => t.contains("plot"),
            _ => false,
        });
        assert!(has_plot_text, "the figure's caption text ('A plot') is reachable via the node");
    }

    #[test]
    fn ref_target_node_for_inline_label_is_the_crossref_inline() {
        // An inline `\eqref{eq:x}` → inline `\label` CrossRef: `ref_target_node` returns the
        // `NodeRef::Inline` for that CrossRef (kind "CrossRef"), span-matching the `\label{eq:x}`.
        let src = r"\begin{document}The identity \label{eq:x} holds.

By \eqref{eq:x} we conclude.\end{document}";
        let doc = parse_document(src).expect("parse");
        let refs = doc.resolve_references();

        let r = &refs.resolved[0];
        let node = doc.ref_target_node(r).expect("resolved eqref must yield the inline label node");

        assert_eq!(node.kind(), "CrossRef", "an inline \\label target is a CrossRef inline");
        assert!(matches!(node, NodeRef::Inline(Inline::CrossRef { .. })));
        let span = node.span();
        assert_eq!(
            &src[span.start..span.end],
            r"\label{eq:x}",
            "the node's span slices back to the inline \\label"
        );
    }

    #[test]
    fn cite_target_node_is_the_walked_bibitem_raw_inline() {
        // A `\cite{smith2020}` → `\bibitem`: the exploratory parse confirmed the `\bibitem` inside a
        // `thebibliography` IS walked (as an `Inline::Raw` command). So `cite_target_node` returns
        // `Some` — kind "Raw", span slicing back to exactly `\bibitem{smith2020}`.
        let src = r"\begin{document}As in \cite{smith2020}.

\begin{thebibliography}{9}
\bibitem{smith2020} Smith, J. A Title. 2020.
\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");
        let cites = doc.resolve_citations();

        let c = &cites.resolved[0];
        let node = doc
            .cite_target_node(c)
            .expect("a \\bibitem IS walked, so the cite target node must resolve");

        assert_eq!(node.kind(), "Raw", "a \\bibitem lowers to an Inline::Raw command");
        assert!(matches!(node, NodeRef::Inline(Inline::Raw(..))));
        let span = node.span();
        assert_eq!(
            &src[span.start..span.end],
            r"\bibitem{smith2020}",
            "the bibitem node's span slices back to exactly the \\bibitem construct"
        );
    }

    #[test]
    fn label_def_node_returns_the_defining_node() {
        // `label_def_node` (the definition-side companion) returns the node that DEFINES a label —
        // here the `Block::Section`.
        let src = r"\begin{document}\section{Body}\label{sec:body}

Text.\end{document}";
        let doc = parse_document(src).expect("parse");
        let refs = doc.resolve_references();

        let def = &refs.definitions[0];
        let node = doc.label_def_node(def).expect("a definition's node must resolve");
        assert_eq!(node.kind(), "Section");
        assert_eq!(node.span(), def.span, "the def node's span is the definition's span");
    }

    #[test]
    fn node_for_span_with_no_matching_span_returns_none() {
        // A span that matches NO walked node → `None` (no panic). We fabricate a span well past the
        // end of the source, which no node can own.
        let src = r"\begin{document}\section{Intro}\label{sec:intro}

Body.\end{document}";
        let doc = parse_document(src).expect("parse");
        // A span between two nodes / off the end matches nothing.
        assert!(doc.node_for_span(Span::new(9000, 9001)).is_none(), "no walked node → None");
        // A zero-width off-by-one span (start of one node, but not its end) also matches nothing.
        assert!(doc.node_for_span(Span::new(16, 17)).is_none(), "half-open equality is exact");
    }

    #[test]
    fn node_for_span_agrees_with_ref_target_node() {
        // `node_for_span(r.target_span)` returns the SAME node as `ref_target_node(r)` — the accessor
        // is a thin wrapper over the primitive.
        let src = r"\begin{document}\section{Intro}\label{sec:intro}

Body. See \ref{sec:intro}.\end{document}";
        let doc = parse_document(src).expect("parse");
        let refs = doc.resolve_references();
        let r = &refs.resolved[0];

        let via_primitive = doc.node_for_span(r.target_span);
        let via_accessor = doc.ref_target_node(r);
        assert_eq!(via_primitive, via_accessor, "accessor == node_for_span(target_span)");
        assert!(via_primitive.is_some(), "the resolved target IS a walked node");
    }

    #[test]
    fn empty_document_node_lookups_yield_none_no_panic() {
        // An empty document has no walked nodes; any span lookup yields `None` and never panics.
        let doc = parse_document(r"\begin{document}\end{document}").expect("parse");
        assert!(doc.node_for_span(Span::new(0, 0)).is_none());
        assert!(doc.node_for_span(Span::new(5, 10)).is_none());
    }

    #[test]
    fn s3_is_purely_additive_s1_s2_outputs_unchanged() {
        // REGRESSION: the S3 methods are purely additive — calling them does not perturb the S1/S2
        // resolutions, which remain byte-for-byte what they were before S3.
        let src = r"\begin{document}\section{Intro}\label{sec:intro}

See Section~\ref{sec:intro} and \cite{smith2020}.

\begin{thebibliography}{9}
\bibitem{smith2020} Smith, J. Title. 2020.
\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");

        let refs_before = doc.resolve_references();
        let cites_before = doc.resolve_citations();

        // Exercise every S3 accessor.
        let _ = doc.ref_target_node(&refs_before.resolved[0]);
        let _ = doc.cite_target_node(&cites_before.resolved[0]);
        let _ = doc.label_def_node(&refs_before.definitions[0]);
        let _ = doc.node_for_span(refs_before.resolved[0].target_span);

        // Re-resolving yields the identical result (nothing mutated).
        assert_eq!(doc.resolve_references(), refs_before, "S1 output unchanged by S3");
        assert_eq!(doc.resolve_citations(), cites_before, "S2 output unchanged by S3");
    }

    // ---------------------------------------------------------------------------------------------
    // LTXDOC03 S4 — document numbering (hierarchical sections + flat float counters).
    // ---------------------------------------------------------------------------------------------

    /// Parse `src` and number its labels — the shared S4 harness.
    fn number(src: &str) -> Numbering {
        parse_document(src).expect("parse").number_labels()
    }

    #[test]
    fn nested_sections_number_hierarchically_with_deeper_reset() {
        // \section{A} = 1, \subsection{B} = 1.1, \subsection{C} = 1.2, \section{D} = 2.
        // The two `\n\n` after each `\label` keep it a lone paragraph so LTXDOC01 hoists it.
        let src = r"\begin{document}\section{A}\label{s:a}

\subsection{B}\label{s:b}

\subsection{C}\label{s:c}

\section{D}\label{s:d}

Body.\end{document}";
        let num = number(src);

        assert_eq!(num.number_for("s:a"), Some("1"), "first section is 1");
        assert_eq!(num.number_for("s:b"), Some("1.1"), "first subsection under 1 is 1.1");
        assert_eq!(num.number_for("s:c"), Some("1.2"), "second subsection is 1.2");
        assert_eq!(num.number_for("s:d"), Some("2"), "the next section bumps to 2 (deeper reset)");

        // All four are Section-kind rows, in source (pre-order).
        assert_eq!(num.labels.len(), 4);
        assert!(num.labels.iter().all(|l| l.kind == LabelKind::Section));
        assert_eq!(num.labels[0].key, "s:a");
        assert_eq!(num.labels[3].key, "s:d");
    }

    #[test]
    fn subsubsection_nests_three_deep() {
        // \section = 1, \subsection = 1.1, \subsubsection = 1.1.1.
        let src = r"\begin{document}\section{A}\label{s:a}

\subsection{B}\label{s:b}

\subsubsection{C}\label{s:c}

Text.\end{document}";
        let num = number(src);
        assert_eq!(num.number_for("s:a"), Some("1"));
        assert_eq!(num.number_for("s:b"), Some("1.1"));
        assert_eq!(num.number_for("s:c"), Some("1.1.1"));
    }

    #[test]
    fn starred_section_consumes_no_number() {
        // A `\section*{Unnumbered}` between two numbered sections does NOT advance the counter: the
        // following numbered section is still 2, not 3.
        let src = r"\begin{document}\section{First}\label{s:1}

\section*{Unnumbered}

\section{Third}\label{s:3}

Body.\end{document}";
        let num = number(src);

        assert_eq!(num.number_for("s:1"), Some("1"), "first numbered section is 1");
        assert_eq!(
            num.number_for("s:3"),
            Some("2"),
            "the starred section consumed no number, so this is 2 (not 3)"
        );
        // The starred section carries no label and contributes no row.
        assert_eq!(num.labels.len(), 2, "only the two numbered sections are numbered");
    }

    #[test]
    fn figures_and_tables_are_independent_flat_counters() {
        // Two labeled figures → 1 and 2; a labeled table → 1 (its OWN counter, not continuing figs).
        let src = r"\begin{document}\begin{figure}\includegraphics{a.png}\caption{A}\label{fig:a}\end{figure}

\begin{figure}\includegraphics{b.png}\caption{B}\label{fig:b}\end{figure}

\begin{table}\begin{tabular}{lc}x & y\end{tabular}\caption{T}\label{tab:t}\end{table}

Body.\end{document}";
        let num = number(src);

        assert_eq!(num.number_for("fig:a"), Some("1"), "first figure is 1");
        assert_eq!(num.number_for("fig:b"), Some("2"), "second figure is 2");
        assert_eq!(num.number_for("tab:t"), Some("1"), "the table is 1 on its OWN counter");

        // Kinds are right.
        assert_eq!(num.labels.iter().find(|l| l.key == "fig:a").map(|l| l.kind), Some(LabelKind::Figure));
        assert_eq!(num.labels.iter().find(|l| l.key == "tab:t").map(|l| l.kind), Some(LabelKind::Table));
    }

    #[test]
    fn unlabeled_float_still_consumes_a_number() {
        // An UNlabeled figure between two labeled figures: the labeled ones come out 1 and 3 (the
        // unlabeled figure silently took 2) — proving every float advances the counter.
        let src = r"\begin{document}\begin{figure}\includegraphics{a.png}\caption{A}\label{fig:a}\end{figure}

\begin{figure}\includegraphics{mid.png}\caption{Mid}\end{figure}

\begin{figure}\includegraphics{c.png}\caption{C}\label{fig:c}\end{figure}

Body.\end{document}";
        let num = number(src);

        assert_eq!(num.number_for("fig:a"), Some("1"), "first labeled figure is 1");
        assert_eq!(
            num.number_for("fig:c"),
            Some("3"),
            "the unlabeled figure consumed 2, so the next labeled one is 3"
        );
        // Only the two LABELED figures are numbered rows; the unlabeled one has no key to record.
        assert_eq!(num.labels.len(), 2, "only labeled floats appear in the table");
    }

    #[test]
    fn ref_number_ties_s1_resolution_to_s4_numbering() {
        // LOAD-BEARING payoff: a `\ref{s:b}` to a `\subsection` under a `\section` prints "1.1".
        let src = r"\begin{document}\section{A}\label{s:a}

\subsection{B}\label{s:b}

See Section~\ref{s:b}.\end{document}";
        let doc = parse_document(src).expect("parse");
        let refs = doc.resolve_references();

        // The `\ref{s:b}` resolved (S1) …
        let r = refs.resolved.iter().find(|r| r.key == "s:b").expect("the \\ref{s:b} resolves");
        // … and its target's rendered number (S4) is "1.1".
        assert_eq!(
            doc.ref_number(r),
            Some("1.1".to_string()),
            "\\ref{{s:b}} → its subsection's number 1.1"
        );
    }

    #[test]
    fn ref_number_for_undefined_key_is_none_no_panic() {
        // A `\ref` to a key with no definition → its number is None (no panic).
        let src = r"\begin{document}See \ref{nope} here.\end{document}";
        let doc = parse_document(src).expect("parse");
        let refs = doc.resolve_references();
        // The dangling ref is not in `resolved`, so we synthesize a ResolvedRef-shaped lookup via the
        // Numbering directly: an undefined key has no number.
        assert!(doc.number_labels().number_for("nope").is_none(), "undefined key → no number");
        assert!(refs.resolved.is_empty(), "and it never resolved in the first place");
    }

    #[test]
    fn empty_document_yields_empty_numbering_no_panic() {
        // An empty document → empty numbering, no panic.
        let num = number(r"\begin{document}\end{document}");
        assert_eq!(num, Numbering::default(), "empty doc → empty numbering");
        assert!(num.number_for("anything").is_none());
    }

    #[test]
    fn lone_deep_subsection_uses_documented_missing_parent_rule() {
        // The missing-parent rule: a document that starts with a `\subsection` (no `\section` opened)
        // numbers it "0.1" — one honest leading `0` for the un-opened parent section.
        let src = r"\begin{document}\subsection{Deep}\label{s:deep}

Text.\end{document}";
        let num = number(src);
        assert_eq!(
            num.number_for("s:deep"),
            Some("0.1"),
            "a lone leading \\subsection numbers as 0.1 (missing parent = 0)"
        );
    }

    #[test]
    fn plain_top_level_section_is_just_one() {
        // A plain top-level `\section` (no ancestors) is "1", NOT "0.1" — it is itself the reference
        // depth, so there is no parent to zero-fill (the counterpart of the missing-parent rule).
        let src = r"\begin{document}\section{Top}\label{s:top}

Text.\end{document}";
        let num = number(src);
        assert_eq!(num.number_for("s:top"), Some("1"), "a plain top-level section is 1");
    }

    #[test]
    fn inline_and_equation_labels_are_not_numbered_yet() {
        // An inline `\label` (deferred to S5) is NOT numbered — it does not appear in the table, and
        // its number lookup is None (documented, total). We keep a numbered section alongside to prove
        // the section IS numbered while the inline label is skipped.
        let src = r"\begin{document}\section{S}\label{sec:s}

The identity \label{eq:x} holds here.

Text.\end{document}";
        let num = number(src);
        assert_eq!(num.number_for("sec:s"), Some("1"), "the section is numbered");
        assert!(num.number_for("eq:x").is_none(), "the inline \\label is deferred to S5 (not numbered)");
        assert_eq!(num.labels.len(), 1, "only the section label is a numbered row");
    }

    #[test]
    fn numbering_does_not_mutate_the_tree_s1_s2_s3_unchanged() {
        // REGRESSION: numbering is pure analysis — running it leaves S1/S2/S3 outputs byte-for-byte
        // unchanged (the tree is never mutated).
        let src = r"\begin{document}\section{Intro}\label{sec:intro}

See Section~\ref{sec:intro} and \cite{smith2020}.

\begin{thebibliography}{9}
\bibitem{smith2020} Smith, J. Title. 2020.
\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");

        let refs_before = doc.resolve_references();
        let cites_before = doc.resolve_citations();

        // Number the labels (exercises the S4 walk).
        let num = doc.number_labels();
        assert_eq!(num.number_for("sec:intro"), Some("1"));

        // S1/S2 outputs are untouched, and S3 node lookup still agrees.
        assert_eq!(doc.resolve_references(), refs_before, "S1 output unchanged by S4");
        assert_eq!(doc.resolve_citations(), cites_before, "S2 output unchanged by S4");
        assert!(
            doc.ref_target_node(&refs_before.resolved[0]).is_some(),
            "S3 lookup still resolves after numbering"
        );
    }

    // ---------------------------------------------------------------------------------------------
    // LTXDOC03 S5 — citation numbering (bracketed bibliography numbers).
    // ---------------------------------------------------------------------------------------------

    /// Parse `src` and number its citations — the shared S5 harness.
    fn number_cites(src: &str) -> CitationNumbering {
        parse_document(src).expect("parse").number_citations()
    }

    #[test]
    fn bibitems_number_by_listing_order() {
        // Three `\bibitem`s in a `thebibliography` number `[1]`, `[2]`, `[3]` in listing order —
        // the exact bracketed strings LaTeX prints for a `\cite` to each.
        let src = r"\begin{document}\cite{a} \cite{b} \cite{c}.

\begin{thebibliography}{9}
\bibitem{a} A. First. 2001.
\bibitem{b} B. Second. 2002.
\bibitem{c} C. Third. 2003.
\end{thebibliography}\end{document}";
        let num = number_cites(src);

        assert_eq!(num.entries.len(), 3, "three numbered entries");
        // LOAD-BEARING: the exact bracketed strings, in listing order.
        assert_eq!(num.number_for("a"), Some("[1]"), "first \\bibitem is [1]");
        assert_eq!(num.number_for("b"), Some("[2]"), "second \\bibitem is [2]");
        assert_eq!(num.number_for("c"), Some("[3]"), "third \\bibitem is [3]");

        // The raw ordinals are the 1-based list positions.
        assert_eq!(num.entries[0].ordinal, 1);
        assert_eq!(num.entries[1].ordinal, 2);
        assert_eq!(num.entries[2].ordinal, 3);
        // Rows are in listing order (keys read top-to-bottom like the bibliography).
        let keys: Vec<&str> = num.entries.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, ["a", "b", "c"]);
    }

    #[test]
    fn cite_number_ties_s2_resolution_to_s5_numbering() {
        // THE PAYOFF: a resolved `\cite{b}` (S2) numbers to "[2]" (S5) — the second `\bibitem`.
        let src = r"\begin{document}As shown in \cite{b}.

\begin{thebibliography}{9}
\bibitem{a} A. First. 2001.
\bibitem{b} B. Second. 2002.
\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");

        let cites = doc.resolve_citations();
        assert_eq!(cites.resolved.len(), 1, "the \\cite{{b}} resolves");
        let c = &cites.resolved[0];
        assert_eq!(c.key, "b");
        // LOAD-BEARING: S2's resolved cite → S5's bracketed number.
        assert_eq!(doc.cite_number(c).as_deref(), Some("[2]"), "\\cite{{b}} prints [2]");
    }

    #[test]
    fn multi_key_cite_numbers_each_key_independently() {
        // A multi-key `\cite{a,c}` → its two resolved records number to "[1]" and "[3]" respectively
        // (each key looks up its own entry's list position).
        let src = r"\begin{document}See \cite{a,c}.

\begin{thebibliography}{9}
\bibitem{a} A. First. 2001.
\bibitem{b} B. Second. 2002.
\bibitem{c} C. Third. 2003.
\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");

        let cites = doc.resolve_citations();
        assert_eq!(cites.resolved.len(), 2, "both keys of \\cite{{a,c}} resolve");
        let ca = cites.resolved.iter().find(|c| c.key == "a").expect("a resolves");
        let cc = cites.resolved.iter().find(|c| c.key == "c").expect("c resolves");
        assert_eq!(doc.cite_number(ca).as_deref(), Some("[1]"), "a is [1]");
        assert_eq!(doc.cite_number(cc).as_deref(), Some("[3]"), "c is [3]");
    }

    #[test]
    fn dangling_cite_key_is_not_numbered_no_panic() {
        // A `\cite{missing}` is in S2's `unresolved` (no ResolvedCite), so there is nothing to
        // number. `number_for("missing")` is None — the honest `[?]` edge, no panic.
        let src = r"\begin{document}See \cite{missing}.

\begin{thebibliography}{9}
\bibitem{real} Real. Actual. 2000.
\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");

        let num = doc.number_citations();
        assert_eq!(num.number_for("real"), Some("[1]"), "the real entry is [1]");
        assert!(num.number_for("missing").is_none(), "a dangling key has no number");

        // And the dangling `\cite` really is unresolved (so there is no ResolvedCite to pass to
        // cite_number in the first place).
        let cites = doc.resolve_citations();
        assert!(cites.resolved.is_empty(), "nothing resolves");
        assert_eq!(cites.unresolved.len(), 1);
        assert_eq!(cites.unresolved[0].key, "missing");
    }

    #[test]
    fn duplicate_bibitem_consumes_no_number_others_unshifted() {
        // A re-declared `\bibitem{a}` later in the list is a losing duplicate: it does NOT renumber
        // or shift the others (b stays "[2]", c stays "[3]"), and consumes no number of its own.
        let src = r"\begin{document}\cite{a} \cite{b} \cite{c}.

\begin{thebibliography}{9}
\bibitem{a} A. First. 2001.
\bibitem{b} B. Second. 2002.
\bibitem{c} C. Third. 2003.
\bibitem{a} A. Duplicate later. 2099.
\end{thebibliography}\end{document}";
        let num = number_cites(src);

        // Exactly three numbered entries — the duplicate did NOT add a fourth row.
        assert_eq!(num.entries.len(), 3, "the duplicate \\bibitem{{a}} consumes no number");
        assert_eq!(num.number_for("a"), Some("[1]"), "a keeps its first-declaration number [1]");
        assert_eq!(num.number_for("b"), Some("[2]"), "b is unshifted at [2]");
        assert_eq!(num.number_for("c"), Some("[3]"), "c is unshifted at [3] (not pushed to [4])");
    }

    #[test]
    fn empty_document_yields_empty_citation_numbering_no_panic() {
        // An empty document, and one with no `thebibliography`, both yield empty numbering, no panic.
        assert_eq!(
            number_cites(r"\begin{document}\end{document}"),
            CitationNumbering::default(),
            "empty doc → empty citation numbering"
        );
        assert_eq!(
            number_cites(r"\begin{document}Text with \cite{x} but no bibliography.\end{document}"),
            CitationNumbering::default(),
            "no bibliography → no numbered entries"
        );
    }

    #[test]
    fn citation_numbering_does_not_mutate_the_tree_s1_s2_s3_s4_unchanged() {
        // REGRESSION: citation numbering is pure analysis — running it leaves S1/S2/S3/S4 outputs
        // byte-for-byte unchanged (the tree is never mutated).
        let src = r"\begin{document}\section{Intro}\label{sec:intro}

See Section~\ref{sec:intro} and \cite{smith2020}.

\begin{thebibliography}{9}
\bibitem{smith2020} Smith, J. Title. 2020.
\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");

        let refs_before = doc.resolve_references();
        let cites_before = doc.resolve_citations();
        let num_before = doc.number_labels();

        // Number the citations (exercises the S5 pass).
        let cite_num = doc.number_citations();
        assert_eq!(cite_num.number_for("smith2020"), Some("[1]"));

        // S1/S2/S4 outputs are untouched, and S3 node lookup still agrees.
        assert_eq!(doc.resolve_references(), refs_before, "S1 output unchanged by S5");
        assert_eq!(doc.resolve_citations(), cites_before, "S2 output unchanged by S5");
        assert_eq!(doc.number_labels(), num_before, "S4 output unchanged by S5");
        assert!(
            doc.cite_target_node(&cites_before.resolved[0]).is_some(),
            "S3 lookup still resolves after citation numbering"
        );
    }

    // ---------------------------------------------------------------------------------------------
    // LTXDOC03 S6 — the cross-reference report (consumer composing S1/S2/S4/S5).
    // ---------------------------------------------------------------------------------------------

    /// Parse `src` and build its cross-reference report — the shared S6 harness.
    fn report(src: &str) -> CrossReferenceReport {
        parse_document(src).expect("parse").cross_reference_report()
    }

    #[test]
    fn report_composes_a_ref_and_a_cite_with_their_numbers() {
        // A doc with BOTH a labeled `\section`+`\ref` AND a `thebibliography`+`\cite`: the report
        // fuses S1's binding with S4's number for the ref, and S2's binding with S5's number for the
        // cite — one RefEntry (Section 1) and one CiteEntry ([2]).
        let src = r"\begin{document}\section{Intro}\label{s:i}

See Section~\ref{s:i} and \cite{b}.

\begin{thebibliography}{9}
\bibitem{a} A. First. 2001.
\bibitem{b} B. Second. 2002.
\end{thebibliography}\end{document}";
        let rep = report(src);

        // One resolved-and-numbered reference, EVERY field exact.
        assert_eq!(rep.refs.len(), 1, "one numbered ref");
        assert_eq!(rep.refs[0].key, "s:i");
        assert_eq!(rep.refs[0].command, "ref");
        assert_eq!(rep.refs[0].kind, LabelKind::Section);
        assert_eq!(rep.refs[0].number, "1");

        // One resolved-and-numbered citation, EVERY field exact (the SECOND \bibitem → [2]).
        assert_eq!(rep.cites.len(), 1, "one numbered cite");
        assert_eq!(rep.cites[0].key, "b");
        assert_eq!(rep.cites[0].number, "[2]");

        // No dangling entries.
        assert!(rep.dangling_refs.is_empty());
        assert!(rep.dangling_cites.is_empty());
    }

    #[test]
    fn report_to_plain_text_renders_the_exact_pinned_string() {
        // LOAD-BEARING: the plain-text rendering is a stable, pinned multi-line string — a resolved
        // ref line and a resolved cite line, joined by a single `\n`, no trailing newline.
        let src = r"\begin{document}\section{Intro}\label{s:i}

See Section~\ref{s:i} and \cite{b}.

\begin{thebibliography}{9}
\bibitem{a} A. First. 2001.
\bibitem{b} B. Second. 2002.
\end{thebibliography}\end{document}";
        let rep = report(src);
        assert_eq!(
            rep.to_plain_text(),
            "\\ref{s:i} -> Section 1\n\\cite{b} -> [2]",
            "the pinned resolved-only rendering"
        );
    }

    #[test]
    fn report_surfaces_dangling_refs_and_cites_separately() {
        // A dangling `\ref{nope}` and dangling `\cite{ghost}` appear in the dangling vecs (exact
        // keys), NOT in `refs`/`cites`; the plain-text footer lists them.
        let src = r"\begin{document}See \ref{nope} and \cite{ghost}.\end{document}";
        let rep = report(src);

        assert!(rep.refs.is_empty(), "the dangling ref is NOT a resolved row");
        assert!(rep.cites.is_empty(), "the dangling cite is NOT a resolved row");
        assert_eq!(rep.dangling_refs, vec!["nope".to_string()], "dangling ref key surfaced");
        assert_eq!(rep.dangling_cites, vec!["ghost".to_string()], "dangling cite key surfaced");

        // The plain-text footer lists both dangling families, no resolved lines above them.
        assert_eq!(
            rep.to_plain_text(),
            "Dangling references: nope\nDangling citations: ghost",
            "the pinned dangling-only rendering"
        );
    }

    #[test]
    fn report_numbers_each_key_of_a_multi_key_cite() {
        // A multi-key `\cite{a,b}` → two CiteEntry rows numbered [1] and [2] (each key looks up its
        // own entry's list position).
        let src = r"\begin{document}See \cite{a,b}.

\begin{thebibliography}{9}
\bibitem{a} A. First. 2001.
\bibitem{b} B. Second. 2002.
\end{thebibliography}\end{document}";
        let rep = report(src);

        assert_eq!(rep.cites.len(), 2, "one row per key of the multi-key \\cite");
        assert_eq!(rep.cites[0].key, "a");
        assert_eq!(rep.cites[0].number, "[1]");
        assert_eq!(rep.cites[1].key, "b");
        assert_eq!(rep.cites[1].number, "[2]");
        assert!(rep.refs.is_empty());
        assert!(rep.dangling_cites.is_empty());

        // Both cite lines render in order.
        assert_eq!(
            rep.to_plain_text(),
            "\\cite{a} -> [1]\n\\cite{b} -> [2]",
            "both multi-key cite lines, in order"
        );
    }

    #[test]
    fn report_omits_resolved_but_unnumbered_inline_label_ref() {
        // A `\ref{eq:x}` to an INLINE `\label{eq:x}` resolves (S1) but has NO S4 number (inline labels
        // are deferred): it is OMITTED from `refs` (neither a numbered row nor a dangling one). A
        // numbered section ref alongside proves the numbered one IS included.
        let src = r"\begin{document}\section{Intro}\label{s:i}

The identity \label{eq:x} holds. See \ref{s:i} and \eqref{eq:x}.\end{document}";
        let rep = report(src);

        // Only the section ref is a numbered row; the inline-label ref is omitted.
        assert_eq!(rep.refs.len(), 1, "the unnumbered inline-label ref is omitted");
        assert_eq!(rep.refs[0].key, "s:i");
        assert_eq!(rep.refs[0].number, "1");
        // And it is NOT reported as dangling either (its label exists — it is simply unnumbered).
        assert!(!rep.dangling_refs.contains(&"eq:x".to_string()), "resolved, so not dangling");
        assert!(rep.dangling_refs.is_empty());
    }

    #[test]
    fn report_empty_document_is_empty_and_renders_stable_marker_no_panic() {
        // An empty document → an entirely empty report, and `to_plain_text` renders the pinned
        // "(no cross-references)" marker (never the empty string), with no panic.
        let rep = report(r"\begin{document}\end{document}");
        assert_eq!(rep, CrossReferenceReport::default(), "empty doc → empty report");
        assert_eq!(
            rep.to_plain_text(),
            "(no cross-references)",
            "empty report renders the stable marker"
        );
    }

    #[test]
    fn report_does_not_mutate_the_tree_s1_through_s5_unchanged() {
        // REGRESSION: the report is pure composition — building it leaves S1/S2/S3/S4/S5 outputs
        // byte-for-byte unchanged (the tree is never mutated).
        let src = r"\begin{document}\section{Intro}\label{sec:intro}

See Section~\ref{sec:intro} and \cite{smith2020}.

\begin{thebibliography}{9}
\bibitem{smith2020} Smith, J. Title. 2020.
\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");

        let refs_before = doc.resolve_references();
        let cites_before = doc.resolve_citations();
        let num_before = doc.number_labels();
        let cite_num_before = doc.number_citations();

        // Build the report (exercises the S6 composition).
        let rep = doc.cross_reference_report();
        assert_eq!(rep.refs.len(), 1);
        assert_eq!(rep.cites.len(), 1);

        // S1–S5 outputs are all untouched, and an S3 node lookup still agrees.
        assert_eq!(doc.resolve_references(), refs_before, "S1 output unchanged by S6");
        assert_eq!(doc.resolve_citations(), cites_before, "S2 output unchanged by S6");
        assert_eq!(doc.number_labels(), num_before, "S4 output unchanged by S6");
        assert_eq!(doc.number_citations(), cite_num_before, "S5 output unchanged by S6");
        assert!(
            doc.ref_target_node(&refs_before.resolved[0]).is_some(),
            "S3 lookup still resolves after building the report"
        );
    }

    // ---------------------------------------------------------------------------------------------
    // LTXDOC03 S11 — the grouped-by-kind cross-reference report (`to_plain_text_by_kind`).
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn s11_groups_refs_by_kind_in_fixed_order() {
        // Sections + a figure + an equation, referenced by `\ref`/`\eqref`. The grouped rendering
        // emits the FIXED kind order (Sections, Figures, Equations here — Tables/Inline absent, so
        // omitted), each subheading followed by two-space-indented ref lines in pre-order, with the
        // real S4 numbers (sec:intro→1, sec:methods→2, fig:plot→1, eq:e→(1)).
        let src = r"\begin{document}\section{Intro}\label{sec:intro}

\section{Methods}\label{sec:methods}

\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:plot}\end{figure}

\begin{equation} a = 1 \label{eq:e} \end{equation}

See \ref{sec:intro}, \ref{sec:methods}, \ref{fig:plot}, \eqref{eq:e}.\end{document}";
        let rep = report(src);
        assert_eq!(
            rep.to_plain_text_by_kind(),
            "Sections:\n  \\ref{sec:intro} -> Section 1\n  \\ref{sec:methods} -> Section 2\n\
             Figures:\n  \\ref{fig:plot} -> Figure 1\n\
             Equations:\n  \\eqref{eq:e} -> Equation (1)",
            "the pinned grouped-by-kind rendering (fixed kind order, two-space indent)"
        );
    }

    #[test]
    fn s11_eqref_and_pageref_render_rules_hold_in_groups() {
        // The grouped lines reuse the SAME shared per-command helper as the flat report:
        //   - an `\eqref` to an equation shows `\eqref{eq:e} -> Equation (1)` under `Equations:`;
        //   - a `\pageref` shows `\pageref{...} -> page ?` under its TARGET kind's group — here the
        //     `\pageref{sec:intro}` targets a Section, so it sits in the `Sections:` group.
        let src = r"\begin{document}\section{Intro}\label{sec:intro}

\begin{equation} a = 1 \label{eq:e} \end{equation}

See \ref{sec:intro}, \pageref{sec:intro}, \eqref{eq:e}.\end{document}";
        let rep = report(src);
        assert_eq!(
            rep.to_plain_text_by_kind(),
            "Sections:\n  \\ref{sec:intro} -> Section 1\n  \\pageref{sec:intro} -> page ?\n\
             Equations:\n  \\eqref{eq:e} -> Equation (1)",
            "grouped lines obey the S9 eqref-parenthesises and S10 pageref-placeholder rules"
        );
    }

    #[test]
    fn s11_empty_returns_marker() {
        // A doc with no resolved refs → the fixed S11 empty marker (distinct from the flat report's
        // "(no cross-references)").
        let rep = report(r"\begin{document}\end{document}");
        assert_eq!(
            rep.to_plain_text_by_kind(),
            "(no resolved references)",
            "no resolved refs renders the stable S11 marker"
        );
    }

    #[test]
    fn s11_dangling_and_cites_do_not_appear_in_grouped_report() {
        // A doc with a dangling `\ref{nope}` and a `\cite` but NO resolved refs → still the empty
        // marker: the grouped method is focused on resolved refs only (no citations, no dangling
        // footers).
        let src = r"\begin{document}See \ref{nope} and \cite{ghost}.\end{document}";
        let rep = report(src);
        assert_eq!(
            rep.to_plain_text_by_kind(),
            "(no resolved references)",
            "citations and dangling keys are not part of the grouped resolved-ref report"
        );
    }

    #[test]
    fn s11_to_plain_text_unchanged() {
        // GUARD (additivity/refactor): a representative doc's flat `to_plain_text()` still returns its
        // exact prior S6 string, byte-for-byte, after the shared-helper refactor and the new S11
        // method. If this line ever changes, the refactor broke additivity.
        let src = r"\begin{document}\section{Intro}\label{s:i}

See Section~\ref{s:i} and \cite{b}.

\begin{thebibliography}{9}
\bibitem{a} A. First. 2001.
\bibitem{b} B. Second. 2002.
\end{thebibliography}\end{document}";
        let rep = report(src);
        assert_eq!(
            rep.to_plain_text(),
            "\\ref{s:i} -> Section 1\n\\cite{b} -> [2]",
            "flat to_plain_text() output is byte-for-byte unchanged by S11"
        );
    }

    // -- LTXDOC03 S12 — the List of Figures / List of Tables index (`list_of_floats`). ------------

    #[test]
    fn s12_lists_figures_and_tables_in_order() {
        // Two captioned figures then a captioned table → a `List of Figures` block numbered 1,2 in
        // document order, followed by a `List of Tables` block numbered from 1 (independent counter).
        // Each line is `<n>. <caption text>`, captions rendered as their plain text.
        let src = r"\begin{document}\begin{figure}\includegraphics{p.png}\caption{First plot}\label{fig:a}\end{figure}
\begin{figure}\includegraphics{q.png}\caption{Second plot}\label{fig:b}\end{figure}
\begin{table}\begin{tabular}{lc}a & b \\ c & d\end{tabular}\caption{Data table}\end{table}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.list_of_floats(),
            "List of Figures\n1. First plot\n2. Second plot\nList of Tables\n1. Data table",
        );
    }

    #[test]
    fn s12_uncaptioned_float_uses_placeholder() {
        // A figure with NO `\caption` still gets a numbered line — its caption text renders as the
        // fixed `(no caption)` placeholder, so numbering stays aligned with the real float count.
        let src = r"\begin{document}\begin{figure}\includegraphics{p.png}\label{fig:a}\end{figure}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.list_of_floats(), "List of Figures\n1. (no caption)");
    }

    #[test]
    fn s12_empty_returns_marker() {
        // A document with no floats at all → the fixed `(no floats)` marker (neither heading emitted).
        let src = r"\begin{document}\section{Intro}\label{s:i}

Body text with no floats.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.list_of_floats(), "(no floats)");
    }

    // ---------------------------------------------------------------------------------------------
    // LTXDOC03 S13 — `\nameref` resolution to a target's title/caption text.
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn s13_resolves_section_and_figure_names() {
        // A `\nameref{sec:intro}` → the section's TITLE ("Introduction"); a `\nameref{fig:p}` → the
        // figure's CAPTION text ("A plot"). Both render `\nameref{key} -> <name>`, in body order.
        let src = r"\begin{document}\section{Introduction}\label{sec:intro}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}

See \nameref{sec:intro} and \nameref{fig:p}.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.resolve_namerefs(),
            "\\nameref{sec:intro} -> Introduction\n\\nameref{fig:p} -> A plot"
        );
    }

    #[test]
    fn s13_undefined_key_renders_placeholder() {
        // A `\nameref{nope}` whose key no `\label` defines renders the fixed `(undefined nameref: …)`
        // placeholder — the name-valued analogue of LaTeX's `??`. A resolved section name precedes it,
        // proving the two branches coexist in one deterministic report.
        let src = r"\begin{document}\section{Methods}\label{sec:m}

See \nameref{sec:m} and \nameref{nope}.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.resolve_namerefs(),
            "\\nameref{sec:m} -> Methods\n\\nameref{nope} -> (undefined nameref: nope)"
        );
    }

    #[test]
    fn s13_equation_and_inline_targets_have_no_name() {
        // A `\nameref` to an EQUATION label (a number, not a title) and to a bare inline `\label`
        // (likewise nameless) both render the fixed `(no name)` marker — the honest boundary.
        let src = r"\begin{document}\begin{equation}\label{eq:e}E=mc^2\end{equation}

Text \label{marker}. See \nameref{eq:e} and \nameref{marker}.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.resolve_namerefs(),
            "\\nameref{eq:e} -> (no name)\n\\nameref{marker} -> (no name)"
        );
    }

    #[test]
    fn s13_no_namerefs_returns_marker() {
        // A document with `\ref` but NO `\nameref` returns the fixed `(no namerefs)` marker — a plain
        // `\ref` is not a nameref, so it never appears in this report.
        let src = r"\begin{document}\section{Intro}\label{s:i}

See Section~\ref{s:i}.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.resolve_namerefs(), "(no namerefs)");
    }

    #[test]
    fn s13_is_additive_leaves_s1_s12_outputs_unchanged() {
        // S13 reads the document but changes nothing: `\nameref` is absent from BOTH resolved and
        // unresolved ref tables (it is not a REF_COMMAND), and the S12 list_of_floats is unaffected.
        let src = r"\begin{document}\section{Introduction}\label{sec:intro}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}

See Section~\ref{sec:intro}, \nameref{sec:intro}, and \nameref{fig:p}.\end{document}";
        let doc = parse_document(src).expect("parse");

        let refs = doc.resolve_references();
        // Exactly ONE resolved ref (the `\ref{sec:intro}`); the two `\nameref`s are in neither table.
        assert_eq!(refs.resolved.len(), 1, "only the \\ref resolves; \\namerefs are not REF_COMMANDS");
        assert_eq!(refs.resolved[0].command, "ref");
        assert!(refs.unresolved.is_empty(), "no undefined refs; \\namerefs are not tabled here either");

        // S12 output is unchanged by S13's presence.
        assert_eq!(doc.list_of_floats(), "List of Figures\n1. A plot");

        // And S13 itself renders both namerefs correctly.
        assert_eq!(
            doc.resolve_namerefs(),
            "\\nameref{sec:intro} -> Introduction\n\\nameref{fig:p} -> A plot"
        );
    }

    // ---------------------------------------------------------------------------------------------
    // LTXDOC03 S14 — per-kind census of the numbered-label table (`list_summary`).
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn s14_counts_each_kind() {
        // Two labeled sections, one labeled figure, one labeled table, one labeled equation. The
        // census emits ONE line per kind (count >= 1) in the fixed order Sections, Figures, Tables,
        // Equations, with the fixed plural label regardless of count.
        let src = r"\begin{document}\section{One}\label{sec:a}
\section{Two}\label{sec:b}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}
\begin{table}\begin{tabular}{lc}a & b\end{tabular}\caption{Data}\label{tab:d}\end{table}
\begin{equation}\label{eq:e}E=mc^2\end{equation}

Text \label{marker} inline.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.list_summary(), "Sections: 2\nFigures: 1\nTables: 1\nEquations: 1");
    }

    #[test]
    fn s14_omits_zero_kinds() {
        // Only sections are labeled → ONLY the `Sections:` line appears; the zero-count Figures,
        // Tables, and Equations lines are omitted entirely. Note the fixed plural even though there
        // are two — and that a single labeled section would still print `Sections: 1`.
        let src = r"\begin{document}\section{Alpha}\label{s:a}
\section{Beta}\label{s:b}
\section{Gamma}\label{s:c}

Body text with no floats or equations.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.list_summary(), "Sections: 3");
    }

    #[test]
    fn s14_empty_returns_marker() {
        // A document with NO numbered labels at all (a bare inline `\label` is NOT numbered, so it
        // never appears in `number_labels`) → the fixed `(no labels)` marker, never the empty string.
        let src = r"\begin{document}Text \label{marker} with only a bare inline label.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.list_summary(), "(no labels)");
    }

    #[test]
    fn s14_is_additive_leaves_s1_s13_outputs_unchanged() {
        // On a representative doc carrying a section, a figure, an equation, a `\ref`, and a
        // `\nameref`, S14 changes NONE of the S1-S13 outputs — it only reads `number_labels`.
        let src = r"\begin{document}\section{Introduction}\label{sec:intro}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}

\begin{equation}\label{eq:e}E=mc^2\end{equation}

See Section~\ref{sec:intro}, \nameref{sec:intro}, and \nameref{fig:p}.\end{document}";
        let doc = parse_document(src).expect("parse");

        // S1/S6 flat report — the resolved `\ref` with its S4 number, nothing perturbed.
        assert_eq!(
            doc.cross_reference_report().to_plain_text(),
            "\\ref{sec:intro} -> Section 1"
        );
        // S11 grouped-by-kind report — the same single ref under its `Sections:` group.
        assert_eq!(
            doc.cross_reference_report().to_plain_text_by_kind(),
            "Sections:\n  \\ref{sec:intro} -> Section 1"
        );
        // S12 list of floats — one figure line, unaffected.
        assert_eq!(doc.list_of_floats(), "List of Figures\n1. A plot");
        // S13 nameref resolution — both namerefs render their names, unaffected.
        assert_eq!(
            doc.resolve_namerefs(),
            "\\nameref{sec:intro} -> Introduction\n\\nameref{fig:p} -> A plot"
        );

        // And S14 itself produces the per-kind census (one section, one figure, one equation).
        assert_eq!(doc.list_summary(), "Sections: 1\nFigures: 1\nEquations: 1");
    }

    // ---------------------------------------------------------------------------------------------
    // LTXDOC03 S15 — resolved citations grouped by their source `\cite` (`citations_by_source`).
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn s15_groups_by_cite() {
        // A multi-key `\cite{a,b}` (both resolving) and a separate `\cite{c}`. The report reunites the
        // two keys of the first `\cite` on ONE line and emits the second `\cite` on its own line, in
        // source order.
        let src = r"\begin{document}
See \cite{a,b} and \cite{c}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\bibitem{b} Author B.
\bibitem{c} Author C.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.citations_by_source(), "\\cite{a, b}\n\\cite{c}");
    }

    #[test]
    fn s15_multikey_partial() {
        // A `\cite{a,ghost}` where `a` resolves and `ghost` is dangling → the line shows ONLY the
        // resolved key (`\cite{a}`). We reconstruct from resolved keys, so the dangling `ghost` — which
        // the raw source `\cite{a,ghost}` still contains — is excluded by construction.
        let src = r"\begin{document}
See \cite{a,ghost}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.citations_by_source(), "\\cite{a}");
    }

    #[test]
    fn s15_empty_returns_marker() {
        // A document whose only `\cite` is entirely dangling (no `\bibitem` defines its key) has NO
        // resolved citations → the fixed `(no resolved citations)` marker, never the empty string.
        let src = r"\begin{document}
See \cite{nope}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.citations_by_source(), "(no resolved citations)");
    }

    #[test]
    fn s15_is_additive_leaves_s1_s14_outputs_unchanged() {
        // On a representative doc carrying a section, a figure, an equation, a `\ref`, a `\nameref`,
        // and two `\cite`s (one multi-key, one dangling key), S15 changes NONE of the S1-S14 outputs —
        // it only reads `resolve_citations`.
        let src = r"\begin{document}\section{Introduction}\label{sec:intro}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}

\begin{equation}\label{eq:e}E=mc^2\end{equation}

See Section~\ref{sec:intro}, \nameref{sec:intro}, \nameref{fig:p}, and \cite{a,b} plus \cite{c,ghost}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\bibitem{b} Author B.
\bibitem{c} Author C.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");

        // S1/S6 flat report — the resolved `\ref` and the three resolved `\cite`s with their `[n]`
        // markers, plus the dangling `ghost` footer, all unchanged by S15.
        assert_eq!(
            doc.cross_reference_report().to_plain_text(),
            "\\ref{sec:intro} -> Section 1\n\\cite{a} -> [1]\n\\cite{b} -> [2]\n\\cite{c} -> [3]\nDangling citations: ghost"
        );
        // S11 grouped-by-kind report — the single ref under its `Sections:` group, unchanged.
        assert_eq!(
            doc.cross_reference_report().to_plain_text_by_kind(),
            "Sections:\n  \\ref{sec:intro} -> Section 1"
        );
        // S12 list of floats — one figure line, unchanged.
        assert_eq!(doc.list_of_floats(), "List of Figures\n1. A plot");
        // S13 nameref resolution — both namerefs render their names, unchanged.
        assert_eq!(
            doc.resolve_namerefs(),
            "\\nameref{sec:intro} -> Introduction\n\\nameref{fig:p} -> A plot"
        );
        // S14 per-kind census — one section, one figure, one equation, unchanged.
        assert_eq!(doc.list_summary(), "Sections: 1\nFigures: 1\nEquations: 1");

        // And S15 itself groups the resolved cites: `{a,b}` fully resolves; `{c,ghost}` keeps only `c`.
        assert_eq!(doc.citations_by_source(), "\\cite{a, b}\n\\cite{c}");
    }

    // ---------------------------------------------------------------------------------------------
    // LTXDOC03 S16 — duplicate (multiply-defined) bibliography entries (`duplicate_bibliography_entries`).
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn s16_reports_duplicate_bibitem() {
        // A `thebibliography` defining `smith` TWICE and `jones` once. The first `\bibitem{smith}` wins;
        // the SECOND is the losing duplicate. `jones` (defined once) is not a duplicate. So exactly one
        // line — the offending `\bibitem{smith}` — is surfaced.
        let src = r"\begin{document}\cite{smith}.
\begin{thebibliography}{9}
\bibitem{smith} First Smith. 1990.
\bibitem{jones} Jones. 1991.
\bibitem{smith} Second Smith. 1992.
\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.duplicate_bibliography_entries(), "\\bibitem{smith}");
    }

    #[test]
    fn s16_two_distinct_duplicates() {
        // Two different keys, `a` and `b`, EACH defined twice. Each losing (second) `\bibitem` yields its
        // own line, in pre-order: the duplicate of `a` appears before the duplicate of `b` (source order
        // of the losing entries), so the report is `\bibitem{a}` then `\bibitem{b}`.
        let src = r"\begin{document}
\begin{thebibliography}{9}
\bibitem{a} First A.
\bibitem{b} First B.
\bibitem{a} Second A.
\bibitem{b} Second B.
\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.duplicate_bibliography_entries(), "\\bibitem{a}\n\\bibitem{b}");
    }

    #[test]
    fn s16_empty_returns_marker() {
        // A bibliography whose every key is defined exactly once has NO duplicates → the fixed marker,
        // never the empty string.
        let src = r"\begin{document}
\begin{thebibliography}{9}
\bibitem{a} Author A.
\bibitem{b} Author B.
\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.duplicate_bibliography_entries(), "(no duplicate bibliography entries)");
    }

    #[test]
    fn s16_is_additive_leaves_s1_s15_outputs_unchanged() {
        // On a representative doc carrying a section, a figure, an equation, a `\ref`, a `\nameref`,
        // two `\cite`s (one multi-key, one dangling key), AND a duplicate `\bibitem` (`a` defined twice),
        // S16 changes NONE of the S1-S15 outputs — it only reads `resolve_citations`.
        let src = r"\begin{document}\section{Introduction}\label{sec:intro}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}

\begin{equation}\label{eq:e}E=mc^2\end{equation}

See Section~\ref{sec:intro}, \nameref{sec:intro}, \nameref{fig:p}, and \cite{a,b} plus \cite{c,ghost}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\bibitem{b} Author B.
\bibitem{c} Author C.
\bibitem{a} Author A again.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");

        // S1/S6 flat report — the resolved `\ref` and the three resolved `\cite`s with their `[n]`
        // markers, plus the dangling `ghost` footer, all unchanged by S16.
        assert_eq!(
            doc.cross_reference_report().to_plain_text(),
            "\\ref{sec:intro} -> Section 1\n\\cite{a} -> [1]\n\\cite{b} -> [2]\n\\cite{c} -> [3]\nDangling citations: ghost"
        );
        // S11 grouped-by-kind report — the single ref under its `Sections:` group, unchanged.
        assert_eq!(
            doc.cross_reference_report().to_plain_text_by_kind(),
            "Sections:\n  \\ref{sec:intro} -> Section 1"
        );
        // S12 list of floats — one figure line, unchanged.
        assert_eq!(doc.list_of_floats(), "List of Figures\n1. A plot");
        // S13 nameref resolution — both namerefs render their names, unchanged.
        assert_eq!(
            doc.resolve_namerefs(),
            "\\nameref{sec:intro} -> Introduction\n\\nameref{fig:p} -> A plot"
        );
        // S14 per-kind census — one section, one figure, one equation, unchanged.
        assert_eq!(doc.list_summary(), "Sections: 1\nFigures: 1\nEquations: 1");
        // S15 grouped cites — `{a,b}` fully resolves; `{c,ghost}` keeps only `c`, unchanged.
        assert_eq!(doc.citations_by_source(), "\\cite{a, b}\n\\cite{c}");

        // And S16 itself surfaces the one duplicate: the SECOND `\bibitem{a}`.
        assert_eq!(doc.duplicate_bibliography_entries(), "\\bibitem{a}");
    }

    // ---------------------------------------------------------------------------------------------
    // LTXDOC03 S17 — unresolved (dangling) citations grouped by source `\cite`
    // (`unresolved_citations_by_source`).
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn s17_reports_dangling_cite() {
        // A `\cite{ghost}` whose key no `\bibitem` defines (the bibliography defines a DIFFERENT key).
        // `ghost` dangles → one line, the reconstructed `\cite{ghost}`.
        let src = r"\begin{document}
See \cite{ghost}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.unresolved_citations_by_source(), "\\cite{ghost}");
    }

    #[test]
    fn s17_groups_multi_key_only_dangling_shown() {
        // A `\cite{known, ghost}` where `known` resolves and `ghost` dangles → the line shows ONLY the
        // dangling key (`\cite{ghost}`). `unresolved` holds only the dangling keys, so the resolved
        // `known` is excluded by construction — the DANGLING mirror of S15's resolved-only rendering.
        let src = r"\begin{document}
See \cite{known, ghost}.
\begin{thebibliography}{9}
\bibitem{known} Author K.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.unresolved_citations_by_source(), "\\cite{ghost}");
    }

    #[test]
    fn s17_fully_dangling_multi_key() {
        // A `\cite{x, y}` where NEITHER key is defined → both dangling keys reunite on ONE line, in
        // left-to-right source order, comma-space joined: `\cite{x, y}`.
        let src = r"\begin{document}
See \cite{x, y}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.unresolved_citations_by_source(), "\\cite{x, y}");
    }

    #[test]
    fn s17_two_distinct_cites_source_order() {
        // Two separate dangling `\cite`s → two lines, in the source order the `\cite`s appear, joined
        // by `\n`. The first-appearance grouping keeps `ghost1` ahead of `ghost2`.
        let src = r"\begin{document}
See \cite{ghost1} and later \cite{ghost2}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.unresolved_citations_by_source(),
            "\\cite{ghost1}\n\\cite{ghost2}"
        );
    }

    #[test]
    fn s17_empty_returns_marker() {
        // A document whose every cited key resolves has NO unresolved citations → the fixed
        // `(no unresolved citations)` marker, never the empty string.
        let src = r"\begin{document}
See \cite{a}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.unresolved_citations_by_source(), "(no unresolved citations)");
    }

    #[test]
    fn s17_is_additive_leaves_s1_s16_outputs_unchanged() {
        // On a representative doc carrying a section, a figure, an equation, a `\ref`, a `\nameref`,
        // two `\cite`s (one multi-key, one dangling key), AND a duplicate `\bibitem` (`a` defined
        // twice), S17 changes NONE of the S1-S16 outputs — it only reads `resolve_citations`.
        let src = r"\begin{document}\section{Introduction}\label{sec:intro}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}

\begin{equation}\label{eq:e}E=mc^2\end{equation}

See Section~\ref{sec:intro}, \nameref{sec:intro}, \nameref{fig:p}, and \cite{a,b} plus \cite{c,ghost}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\bibitem{b} Author B.
\bibitem{c} Author C.
\bibitem{a} Author A again.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");

        // S1/S6 flat report — unchanged.
        assert_eq!(
            doc.cross_reference_report().to_plain_text(),
            "\\ref{sec:intro} -> Section 1\n\\cite{a} -> [1]\n\\cite{b} -> [2]\n\\cite{c} -> [3]\nDangling citations: ghost"
        );
        // S11 grouped-by-kind report — unchanged.
        assert_eq!(
            doc.cross_reference_report().to_plain_text_by_kind(),
            "Sections:\n  \\ref{sec:intro} -> Section 1"
        );
        // S12 list of floats — unchanged.
        assert_eq!(doc.list_of_floats(), "List of Figures\n1. A plot");
        // S13 nameref resolution — unchanged.
        assert_eq!(
            doc.resolve_namerefs(),
            "\\nameref{sec:intro} -> Introduction\n\\nameref{fig:p} -> A plot"
        );
        // S14 per-kind census — unchanged.
        assert_eq!(doc.list_summary(), "Sections: 1\nFigures: 1\nEquations: 1");
        // S15 grouped resolved cites — unchanged.
        assert_eq!(doc.citations_by_source(), "\\cite{a, b}\n\\cite{c}");
        // S16 duplicate bibliography entries — unchanged.
        assert_eq!(doc.duplicate_bibliography_entries(), "\\bibitem{a}");

        // And S17 itself groups the dangling cites: `{a,b}` fully resolves (no line); `{c,ghost}` keeps
        // only the dangling `ghost`.
        assert_eq!(doc.unresolved_citations_by_source(), "\\cite{ghost}");
    }

    // ---------------------------------------------------------------------------------------------
    // LTXDOC03 S18 — unresolved (dangling) references grouped by source `\ref`
    // (`unresolved_references_by_source`). The `\ref`-family mirror of S17, command-aware.
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn s18_reports_dangling_ref() {
        // A `\ref{nope}` whose key no `\label` defines → one line, the reconstructed `\ref{nope}`.
        let src = r"\begin{document}
See \ref{nope}.
\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.unresolved_references_by_source(), "\\ref{nope}");
    }

    #[test]
    fn s18_preserves_command_eqref_pageref() {
        // A dangling `\eqref{eq:ghost}` and a dangling `\pageref{p:ghost}` → each line preserves the
        // command it was written with (NOT flattened to `\ref`), one per line, in source order.
        let src = r"\begin{document}
See \eqref{eq:ghost} on \pageref{p:ghost}.
\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.unresolved_references_by_source(),
            "\\eqref{eq:ghost}\n\\pageref{p:ghost}"
        );
    }

    #[test]
    fn s18_two_distinct_dangling_refs_source_order() {
        // Two separate dangling `\ref`s → two lines, in the source order the references appear, joined by
        // `\n`. The first-appearance grouping keeps `nope1` ahead of `nope2`.
        let src = r"\begin{document}
See \ref{nope1} and later \ref{nope2}.
\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.unresolved_references_by_source(),
            "\\ref{nope1}\n\\ref{nope2}"
        );
    }

    #[test]
    fn s18_resolved_ref_excluded() {
        // A `\ref{sec:intro}` that DOES resolve to a `\label{sec:intro}` never enters `unresolved`, so it
        // is excluded by construction. With no dangling references, the fixed marker is returned.
        let src = r"\begin{document}\section{Introduction}\label{sec:intro}
See Section~\ref{sec:intro}.
\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.unresolved_references_by_source(),
            "(no unresolved references)"
        );
    }

    #[test]
    fn s18_empty_returns_marker() {
        // A document with no references at all → the fixed `(no unresolved references)` marker, never the
        // empty string.
        let src = r"\begin{document}
Just some text with no references.
\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.unresolved_references_by_source(),
            "(no unresolved references)"
        );
    }

    #[test]
    fn s18_is_additive_leaves_s1_s17_outputs_unchanged() {
        // On a representative doc carrying a section, a figure, an equation, a resolved `\ref`, a
        // `\nameref`, two `\cite`s (one multi-key, one dangling key), a duplicate `\bibitem`, AND a
        // dangling `\eqref` (`eq:ghost`), S18 changes NONE of the S1-S17 outputs — it only reads
        // `resolve_references`.
        let src = r"\begin{document}\section{Introduction}\label{sec:intro}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}

\begin{equation}\label{eq:e}E=mc^2\end{equation}

See Section~\ref{sec:intro}, \eqref{eq:ghost}, \nameref{sec:intro}, \nameref{fig:p}, and \cite{a,b} plus \cite{c,ghost}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\bibitem{b} Author B.
\bibitem{c} Author C.
\bibitem{a} Author A again.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");

        // S1/S6 flat report — unchanged.
        assert_eq!(
            doc.cross_reference_report().to_plain_text(),
            "\\ref{sec:intro} -> Section 1\n\\cite{a} -> [1]\n\\cite{b} -> [2]\n\\cite{c} -> [3]\nDangling references: eq:ghost\nDangling citations: ghost"
        );
        // S11 grouped-by-kind report — unchanged.
        assert_eq!(
            doc.cross_reference_report().to_plain_text_by_kind(),
            "Sections:\n  \\ref{sec:intro} -> Section 1"
        );
        // S12 list of floats — unchanged.
        assert_eq!(doc.list_of_floats(), "List of Figures\n1. A plot");
        // S13 nameref resolution — unchanged.
        assert_eq!(
            doc.resolve_namerefs(),
            "\\nameref{sec:intro} -> Introduction\n\\nameref{fig:p} -> A plot"
        );
        // S14 per-kind census — unchanged.
        assert_eq!(doc.list_summary(), "Sections: 1\nFigures: 1\nEquations: 1");
        // S15 grouped resolved cites — unchanged.
        assert_eq!(doc.citations_by_source(), "\\cite{a, b}\n\\cite{c}");
        // S16 duplicate bibliography entries — unchanged.
        assert_eq!(doc.duplicate_bibliography_entries(), "\\bibitem{a}");
        // S17 grouped dangling cites — unchanged.
        assert_eq!(doc.unresolved_citations_by_source(), "\\cite{ghost}");

        // And S18 itself surfaces the dangling `\eqref`, command preserved.
        assert_eq!(
            doc.unresolved_references_by_source(),
            "\\eqref{eq:ghost}"
        );
    }

    #[test]
    fn s19_lists_winning_entries() {
        // Two distinct `\bibitem`s → a numbered list, one line each, 1-based, in source order.
        let src = r"\begin{document}
\begin{thebibliography}{9}
\bibitem{a} Author A.
\bibitem{b} Author B.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.bibliography_entries(), "[1] a\n[2] b");
    }

    #[test]
    fn s19_only_first_of_duplicate_key_wins() {
        // A `\bibitem{dup}` written twice yields ONE winning entry (`dup`); the later re-definition
        // lives in `duplicate_entries` (S16), not in the winning list. A peer `\bibitem{other}` shows
        // the numbering/order stays 1-based in source order.
        let src = r"\begin{document}
\begin{thebibliography}{9}
\bibitem{dup} First dup.
\bibitem{other} Other.
\bibitem{dup} Second dup.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.bibliography_entries(), "[1] dup\n[2] other");
        // Cross-check: the losing duplicate is the S16 view, not the S19 winning list.
        assert_eq!(doc.duplicate_bibliography_entries(), "\\bibitem{dup}");
    }

    #[test]
    fn s19_numbered_in_preorder() {
        // Three distinct entries → three numbered lines in source (pre-order) order.
        let src = r"\begin{document}
\begin{thebibliography}{9}
\bibitem{x} X.
\bibitem{y} Y.
\bibitem{z} Z.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.bibliography_entries(), "[1] x\n[2] y\n[3] z");
    }

    #[test]
    fn s19_empty_returns_marker() {
        // A document with no `thebibliography` at all → the fixed `(no bibliography entries)` marker,
        // never the empty string.
        let src = r"\begin{document}
Just some text with no bibliography.
\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.bibliography_entries(), "(no bibliography entries)");
    }

    #[test]
    fn s19_is_additive_leaves_s1_s18_outputs_unchanged() {
        // On a representative doc carrying a section, a figure, an equation, a resolved `\ref`, a
        // `\nameref`, two `\cite`s (one multi-key, one dangling key), a duplicate `\bibitem`, AND a
        // dangling `\eqref` (`eq:ghost`), S19 changes NONE of the S1-S18 outputs — it only reads
        // `resolve_citations`.
        let src = r"\begin{document}\section{Introduction}\label{sec:intro}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}

\begin{equation}\label{eq:e}E=mc^2\end{equation}

See Section~\ref{sec:intro}, \eqref{eq:ghost}, \nameref{sec:intro}, \nameref{fig:p}, and \cite{a,b} plus \cite{c,ghost}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\bibitem{b} Author B.
\bibitem{c} Author C.
\bibitem{a} Author A again.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");

        // S1/S6 flat report — unchanged.
        assert_eq!(
            doc.cross_reference_report().to_plain_text(),
            "\\ref{sec:intro} -> Section 1\n\\cite{a} -> [1]\n\\cite{b} -> [2]\n\\cite{c} -> [3]\nDangling references: eq:ghost\nDangling citations: ghost"
        );
        // S11 grouped-by-kind report — unchanged.
        assert_eq!(
            doc.cross_reference_report().to_plain_text_by_kind(),
            "Sections:\n  \\ref{sec:intro} -> Section 1"
        );
        // S12 list of floats — unchanged.
        assert_eq!(doc.list_of_floats(), "List of Figures\n1. A plot");
        // S13 nameref resolution — unchanged.
        assert_eq!(
            doc.resolve_namerefs(),
            "\\nameref{sec:intro} -> Introduction\n\\nameref{fig:p} -> A plot"
        );
        // S14 per-kind census — unchanged.
        assert_eq!(doc.list_summary(), "Sections: 1\nFigures: 1\nEquations: 1");
        // S15 grouped resolved cites — unchanged.
        assert_eq!(doc.citations_by_source(), "\\cite{a, b}\n\\cite{c}");
        // S16 duplicate bibliography entries — unchanged.
        assert_eq!(doc.duplicate_bibliography_entries(), "\\bibitem{a}");
        // S17 grouped dangling cites — unchanged.
        assert_eq!(doc.unresolved_citations_by_source(), "\\cite{ghost}");
        // S18 grouped dangling refs — unchanged.
        assert_eq!(doc.unresolved_references_by_source(), "\\eqref{eq:ghost}");

        // And S19 itself surfaces the winning entries as a numbered list — the duplicate `a` appears
        // once (the winner), `b` and `c` follow, in source pre-order.
        assert_eq!(doc.bibliography_entries(), "[1] a\n[2] b\n[3] c");
    }

    #[test]
    fn s20_reports_duplicate_label() {
        // A `\label{dup}` written twice: the FIRST wins (→ `definitions`), the SECOND loses (→
        // `duplicates`). S20 renders only the losing later one, reconstructed as `\label{dup}`.
        let src = r"\begin{document}First \label{dup} here.

Second \label{dup} there.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.duplicate_label_definitions(), "\\label{dup}");
    }

    #[test]
    fn s20_two_distinct_duplicates_preorder() {
        // Two different keys, each defined twice. Each losing (second) definition gets its own line,
        // in body pre-order: `alpha`'s duplicate appears before `beta`'s.
        let src = r"\begin{document}A \label{alpha} one.

B \label{beta} two.

A' \label{alpha} again.

B' \label{beta} again.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.duplicate_label_definitions(),
            "\\label{alpha}\n\\label{beta}"
        );
    }

    #[test]
    fn s20_no_duplicates_returns_marker() {
        // Every label defined exactly once → the fixed marker, never the empty string.
        let src = r"\begin{document}One \label{one} here, two \label{two} there.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.duplicate_label_definitions(),
            "(no duplicate label definitions)"
        );
    }

    #[test]
    fn s20_is_additive_leaves_s1_s19_outputs_unchanged() {
        // On a representative doc carrying a section, a figure, an equation, a resolved `\ref`, a
        // `\nameref`, two `\cite`s (one multi-key, one dangling key), a duplicate `\bibitem`, a
        // dangling `\eqref` (`eq:ghost`), AND a duplicate `\label{dup}`, S20 changes NONE of the
        // S1-S19 outputs — it only reads `resolve_references`.
        let src = r"\begin{document}\section{Introduction}\label{sec:intro}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}

\begin{equation}\label{eq:e}E=mc^2\end{equation}

First \label{dup} here.

Second \label{dup} there.

See Section~\ref{sec:intro}, \eqref{eq:ghost}, \nameref{sec:intro}, \nameref{fig:p}, and \cite{a,b} plus \cite{c,ghost}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\bibitem{b} Author B.
\bibitem{c} Author C.
\bibitem{a} Author A again.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");

        // S1/S6 flat report — unchanged.
        assert_eq!(
            doc.cross_reference_report().to_plain_text(),
            "\\ref{sec:intro} -> Section 1\n\\cite{a} -> [1]\n\\cite{b} -> [2]\n\\cite{c} -> [3]\nDangling references: eq:ghost\nDangling citations: ghost"
        );
        // S11 grouped-by-kind report — unchanged.
        assert_eq!(
            doc.cross_reference_report().to_plain_text_by_kind(),
            "Sections:\n  \\ref{sec:intro} -> Section 1"
        );
        // S12 list of floats — unchanged.
        assert_eq!(doc.list_of_floats(), "List of Figures\n1. A plot");
        // S13 nameref resolution — unchanged.
        assert_eq!(
            doc.resolve_namerefs(),
            "\\nameref{sec:intro} -> Introduction\n\\nameref{fig:p} -> A plot"
        );
        // S14 per-kind census — unchanged.
        assert_eq!(doc.list_summary(), "Sections: 1\nFigures: 1\nEquations: 1");
        // S15 grouped resolved cites — unchanged.
        assert_eq!(doc.citations_by_source(), "\\cite{a, b}\n\\cite{c}");
        // S16 duplicate bibliography entries — unchanged.
        assert_eq!(doc.duplicate_bibliography_entries(), "\\bibitem{a}");
        // S17 grouped dangling cites — unchanged.
        assert_eq!(doc.unresolved_citations_by_source(), "\\cite{ghost}");
        // S18 grouped dangling refs — unchanged.
        assert_eq!(doc.unresolved_references_by_source(), "\\eqref{eq:ghost}");
        // S19 numbered winning bibliography — unchanged.
        assert_eq!(doc.bibliography_entries(), "[1] a\n[2] b\n[3] c");

        // And S20 itself surfaces only the losing later `\label{dup}` (the first `dup` wins).
        assert_eq!(doc.duplicate_label_definitions(), "\\label{dup}");
    }

    // ---------------------------------------------------------------------------------------------
    // LTXDOC03 S21 — resolved (successfully-matched) references grouped by source `\ref`
    // (`resolved_references_by_source`). The RESOLVED mirror of S18, command-aware.
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn s21_preserves_command_ref_eqref_pageref() {
        // A resolved `\ref`, `\eqref`, and `\pageref` (each bound to a real `\label`) → each line
        // preserves the command it was written with (NOT flattened to `\ref`), one per line, in source
        // order. This is the command-aware core of S21.
        let src = r"\begin{document}\section{Intro}\label{sec:intro}
See \ref{sec:intro} and \eqref{eq:main} and \pageref{sec:intro}.
\begin{equation}\label{eq:main}x=1\end{equation}
\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.resolved_references_by_source(),
            "\\ref{sec:intro}\n\\eqref{eq:main}\n\\pageref{sec:intro}"
        );
    }

    #[test]
    fn s21_no_references_returns_marker() {
        // A document with no references at all → the fixed `(no resolved references)` marker, never the
        // empty string.
        let src = r"\begin{document}
Just some text with no references.
\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.resolved_references_by_source(),
            "(no resolved references)"
        );
    }

    #[test]
    fn s21_only_dangling_returns_marker() {
        // A document whose ONLY reference dangles (`\ref{nope}` with no `\label`) has zero resolved
        // references → the fixed marker, never the empty string. The dangling ref lives in S18, not here.
        let src = r"\begin{document}
See \ref{nope}.
\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.resolved_references_by_source(),
            "(no resolved references)"
        );
    }

    #[test]
    fn s21_mixed_lists_only_resolved() {
        // A doc mixing a resolved `\ref{sec:intro}` with a dangling `\ref{nope}` → S21 lists ONLY the
        // resolved one; the dangling `\ref{nope}` must NOT appear (it is excluded by construction).
        let src = r"\begin{document}\section{Intro}\label{sec:intro}
See \ref{sec:intro} and also \ref{nope}.
\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.resolved_references_by_source(), "\\ref{sec:intro}");
    }

    #[test]
    fn s21_is_additive_leaves_s1_s20_outputs_unchanged() {
        // On a representative doc carrying a section, a figure, an equation, a resolved `\ref`, a
        // `\nameref`, two `\cite`s (one multi-key, one dangling key), a duplicate `\bibitem`, a
        // dangling `\eqref` (`eq:ghost`), a duplicate `\label{dup}`, AND a resolved `\eqref{eq:e}`,
        // S21 changes NONE of the S1-S20 outputs — it only reads `resolve_references`. This also pins
        // the `\n`-join with no trailing newline on the multi-ref S21 case.
        let src = r"\begin{document}\section{Introduction}\label{sec:intro}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}

\begin{equation}\label{eq:e}E=mc^2\end{equation}

First \label{dup} here.

Second \label{dup} there.

See Section~\ref{sec:intro}, \eqref{eq:e}, \eqref{eq:ghost}, \nameref{sec:intro}, \nameref{fig:p}, and \cite{a,b} plus \cite{c,ghost}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\bibitem{b} Author B.
\bibitem{c} Author C.
\bibitem{a} Author A again.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");

        // S1/S6 flat report — the resolved `\eqref{eq:e}` (Equation (1)) now appears alongside the
        // resolved `\ref`; the dangling `eq:ghost` stays in the footer. S6's content is a function of
        // the same resolution S21 reads, so this string is what S6 already produces for this doc —
        // running S21 does not perturb it.
        assert_eq!(
            doc.cross_reference_report().to_plain_text(),
            "\\ref{sec:intro} -> Section 1\n\\eqref{eq:e} -> Equation (1)\n\\cite{a} -> [1]\n\\cite{b} -> [2]\n\\cite{c} -> [3]\nDangling references: eq:ghost\nDangling citations: ghost"
        );
        // S11 grouped-by-kind report — the resolved `\eqref{eq:e}` shows under Equations.
        assert_eq!(
            doc.cross_reference_report().to_plain_text_by_kind(),
            "Sections:\n  \\ref{sec:intro} -> Section 1\nEquations:\n  \\eqref{eq:e} -> Equation (1)"
        );
        // S12 list of floats — unchanged.
        assert_eq!(doc.list_of_floats(), "List of Figures\n1. A plot");
        // S13 nameref resolution — unchanged.
        assert_eq!(
            doc.resolve_namerefs(),
            "\\nameref{sec:intro} -> Introduction\n\\nameref{fig:p} -> A plot"
        );
        // S14 per-kind census — unchanged.
        assert_eq!(doc.list_summary(), "Sections: 1\nFigures: 1\nEquations: 1");
        // S15 grouped resolved cites — unchanged.
        assert_eq!(doc.citations_by_source(), "\\cite{a, b}\n\\cite{c}");
        // S16 duplicate bibliography entries — unchanged.
        assert_eq!(doc.duplicate_bibliography_entries(), "\\bibitem{a}");
        // S17 grouped dangling cites — unchanged.
        assert_eq!(doc.unresolved_citations_by_source(), "\\cite{ghost}");
        // S18 grouped dangling refs — unchanged (only the dangling `\eqref{eq:ghost}`).
        assert_eq!(doc.unresolved_references_by_source(), "\\eqref{eq:ghost}");
        // S19 numbered winning bibliography — unchanged.
        assert_eq!(doc.bibliography_entries(), "[1] a\n[2] b\n[3] c");
        // S20 losing duplicate labels — unchanged.
        assert_eq!(doc.duplicate_label_definitions(), "\\label{dup}");

        // And S21 itself surfaces only the RESOLVED references: the `\ref{sec:intro}` and the
        // `\eqref{eq:e}` (command preserved), in source order, `\n`-joined with no trailing newline.
        // The dangling `\eqref{eq:ghost}` is excluded (it lives in S18).
        assert_eq!(
            doc.resolved_references_by_source(),
            "\\ref{sec:intro}\n\\eqref{eq:e}"
        );
    }

    // ---------------------------------------------------------------------------------------------
    // LTXDOC03 S22 — winning label definitions (`label_definitions`). The label-family mirror of
    // S19's `bibliography_entries`, and the winning-side counterpart of S20.
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn s22_lists_winning_definitions_in_preorder() {
        // Two distinct labels defined in source order → each renders `\label{key}`, one per line, in
        // body pre-order (`k1` before `k2`).
        let src = r"\begin{document}First \label{k1} here.

Second \label{k2} there.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.label_definitions(), "\\label{k1}\n\\label{k2}");
    }

    #[test]
    fn s22_no_labels_returns_marker() {
        // A document with no `\label` at all → the fixed `(no label definitions)` marker, never the
        // empty string.
        let src = r"\begin{document}Just some text with no labels.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.label_definitions(), "(no label definitions)");
    }

    #[test]
    fn s22_duplicate_key_wins_once() {
        // `\label{dup}` written twice: the FIRST wins (→ `definitions`), the SECOND loses (→
        // `duplicates`). S22 renders the winning key EXACTLY ONCE; the losing later one is S20's domain.
        let src = r"\begin{document}First \label{dup} here.

Second \label{dup} there.\end{document}";
        let doc = parse_document(src).expect("parse");
        // Winning side: the key appears once.
        assert_eq!(doc.label_definitions(), "\\label{dup}");
        // Cross-check the losing side: S20 surfaces the second (losing) `\label{dup}`.
        assert_eq!(doc.duplicate_label_definitions(), "\\label{dup}");
    }

    #[test]
    fn s22_newline_join_no_trailing_newline() {
        // Three distinct labels → exact string equality pins the `\n`-join with NO trailing newline.
        let src = r"\begin{document}\label{a}

\label{b}

\label{c}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.label_definitions(), "\\label{a}\n\\label{b}\n\\label{c}");
    }

    #[test]
    fn s22_is_additive_leaves_s1_s21_outputs_unchanged() {
        // On a representative doc carrying a section, a figure, an equation, a resolved `\ref`, a
        // resolved `\eqref`, a `\nameref`, two `\cite`s (one multi-key, one dangling key), a duplicate
        // `\bibitem`, a dangling `\eqref` (`eq:ghost`), AND a duplicate `\label{dup}`, S22 changes NONE
        // of the S1-S21 outputs — it only reads `resolve_references`. This also pins the `\n`-join with
        // no trailing newline on the multi-label S22 case, and that the winning `dup` appears ONCE.
        let src = r"\begin{document}\section{Introduction}\label{sec:intro}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}

\begin{equation}\label{eq:e}E=mc^2\end{equation}

First \label{dup} here.

Second \label{dup} there.

See Section~\ref{sec:intro}, \eqref{eq:e}, \eqref{eq:ghost}, \nameref{sec:intro}, \nameref{fig:p}, and \cite{a,b} plus \cite{c,ghost}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\bibitem{b} Author B.
\bibitem{c} Author C.
\bibitem{a} Author A again.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");

        // S1/S6 flat report — unchanged.
        assert_eq!(
            doc.cross_reference_report().to_plain_text(),
            "\\ref{sec:intro} -> Section 1\n\\eqref{eq:e} -> Equation (1)\n\\cite{a} -> [1]\n\\cite{b} -> [2]\n\\cite{c} -> [3]\nDangling references: eq:ghost\nDangling citations: ghost"
        );
        // S11 grouped-by-kind report — unchanged.
        assert_eq!(
            doc.cross_reference_report().to_plain_text_by_kind(),
            "Sections:\n  \\ref{sec:intro} -> Section 1\nEquations:\n  \\eqref{eq:e} -> Equation (1)"
        );
        // S12 list of floats — unchanged.
        assert_eq!(doc.list_of_floats(), "List of Figures\n1. A plot");
        // S13 nameref resolution — unchanged.
        assert_eq!(
            doc.resolve_namerefs(),
            "\\nameref{sec:intro} -> Introduction\n\\nameref{fig:p} -> A plot"
        );
        // S14 per-kind census — unchanged.
        assert_eq!(doc.list_summary(), "Sections: 1\nFigures: 1\nEquations: 1");
        // S15 grouped resolved cites — unchanged.
        assert_eq!(doc.citations_by_source(), "\\cite{a, b}\n\\cite{c}");
        // S16 duplicate bibliography entries — unchanged.
        assert_eq!(doc.duplicate_bibliography_entries(), "\\bibitem{a}");
        // S17 grouped dangling cites — unchanged.
        assert_eq!(doc.unresolved_citations_by_source(), "\\cite{ghost}");
        // S18 grouped dangling refs — unchanged.
        assert_eq!(doc.unresolved_references_by_source(), "\\eqref{eq:ghost}");
        // S19 numbered winning bibliography — unchanged.
        assert_eq!(doc.bibliography_entries(), "[1] a\n[2] b\n[3] c");
        // S20 losing duplicate labels — unchanged.
        assert_eq!(doc.duplicate_label_definitions(), "\\label{dup}");
        // S21 resolved references — unchanged.
        assert_eq!(
            doc.resolved_references_by_source(),
            "\\ref{sec:intro}\n\\eqref{eq:e}"
        );

        // And S22 itself surfaces only the WINNING label definitions — one row per distinct key, in
        // pre-order, `\n`-joined with no trailing newline. The duplicate `dup` appears ONCE (its losing
        // second definition lives in S20, not here).
        assert_eq!(
            doc.label_definitions(),
            "\\label{sec:intro}\n\\label{fig:p}\n\\label{eq:e}\n\\label{dup}"
        );
    }

    // ---------------------------------------------------------------------------------------------
    // LTXDOC03 S23 — winning label definitions grouped by kind (`label_definitions_by_kind`). The
    // by-kind grouping companion of S22's flat `label_definitions`; two views of one list.
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn s23_groups_different_kinds_in_fixed_kind_order() {
        // A section label, an equation label, and a bare inline label → grouped by kind in the fixed
        // enum order (Section before Equation before Inline), each `[kind] \label{key}`. Note the
        // source order is section, equation, inline — which already matches the fixed kind order here.
        let src = r"\begin{document}\section{Intro}\label{sec:intro}
\begin{equation}\label{eq:main}x=1\end{equation}
\label{note}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.label_definitions_by_kind(),
            "[section] \\label{sec:intro}\n[equation] \\label{eq:main}\n[inline] \\label{note}"
        );
    }

    #[test]
    fn s23_reorders_source_to_fixed_kind_order() {
        // Source order is inline THEN section, but the fixed kind order pulls the section ahead of the
        // inline — proving S23 groups by kind rather than echoing source order.
        let src = r"\begin{document}\label{note}
\section{Intro}\label{sec:intro}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.label_definitions_by_kind(),
            "[section] \\label{sec:intro}\n[inline] \\label{note}"
        );
    }

    #[test]
    fn s23_same_kind_grouped_in_preorder() {
        // Two inline labels of the SAME kind → listed together under `inline`, in their existing
        // pre-order (`a` before `b`). Only the `inline` group appears; no empty groups for other kinds.
        let src = r"\begin{document}\label{a}

\label{b}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.label_definitions_by_kind(),
            "[inline] \\label{a}\n[inline] \\label{b}"
        );
    }

    #[test]
    fn s23_no_labels_returns_marker() {
        // A document with no `\label` at all → the same fixed `(no label definitions)` marker S22 uses,
        // never the empty string.
        let src = r"\begin{document}Just some text with no labels.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.label_definitions_by_kind(), "(no label definitions)");
    }

    #[test]
    fn s23_newline_join_no_trailing_newline() {
        // A section + figure + inline label → exact string equality pins the `\n`-join with NO trailing
        // newline, and the fixed kind order (Section, then Figure, then Inline).
        let src = r"\begin{document}\section{Intro}\label{sec:i}
\begin{figure}\includegraphics{p.png}\caption{P}\label{fig:p}\end{figure}

\label{note}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.label_definitions_by_kind(),
            "[section] \\label{sec:i}\n[figure] \\label{fig:p}\n[inline] \\label{note}"
        );
    }

    #[test]
    fn s23_is_additive_leaves_s1_s22_outputs_unchanged() {
        // Same representative doc as the S22 additive test: a section, a figure, an equation, a
        // resolved `\ref`, a resolved `\eqref`, `\nameref`s, two `\cite`s (one multi-key, one dangling),
        // a duplicate `\bibitem`, a dangling `\eqref`, AND a duplicate `\label{dup}`. S23 changes NONE
        // of the S1-S22 outputs — it only reads `resolve_references`. S22's flat `label_definitions`
        // and S23's grouped `label_definitions_by_kind` are two views of the SAME winning `definitions`
        // list; both are pinned here to show S23 neither adds, drops, nor reorders relative to S22.
        let src = r"\begin{document}\section{Introduction}\label{sec:intro}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}

\begin{equation}\label{eq:e}E=mc^2\end{equation}

First \label{dup} here.

Second \label{dup} there.

See Section~\ref{sec:intro}, \eqref{eq:e}, \eqref{eq:ghost}, \nameref{sec:intro}, \nameref{fig:p}, and \cite{a,b} plus \cite{c,ghost}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\bibitem{b} Author B.
\bibitem{c} Author C.
\bibitem{a} Author A again.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");

        // S1/S6 flat report — unchanged.
        assert_eq!(
            doc.cross_reference_report().to_plain_text(),
            "\\ref{sec:intro} -> Section 1\n\\eqref{eq:e} -> Equation (1)\n\\cite{a} -> [1]\n\\cite{b} -> [2]\n\\cite{c} -> [3]\nDangling references: eq:ghost\nDangling citations: ghost"
        );
        // S11 grouped-by-kind report — unchanged.
        assert_eq!(
            doc.cross_reference_report().to_plain_text_by_kind(),
            "Sections:\n  \\ref{sec:intro} -> Section 1\nEquations:\n  \\eqref{eq:e} -> Equation (1)"
        );
        // S12 list of floats — unchanged.
        assert_eq!(doc.list_of_floats(), "List of Figures\n1. A plot");
        // S13 nameref resolution — unchanged.
        assert_eq!(
            doc.resolve_namerefs(),
            "\\nameref{sec:intro} -> Introduction\n\\nameref{fig:p} -> A plot"
        );
        // S14 per-kind census — unchanged.
        assert_eq!(doc.list_summary(), "Sections: 1\nFigures: 1\nEquations: 1");
        // S15 grouped resolved cites — unchanged.
        assert_eq!(doc.citations_by_source(), "\\cite{a, b}\n\\cite{c}");
        // S16 duplicate bibliography entries — unchanged.
        assert_eq!(doc.duplicate_bibliography_entries(), "\\bibitem{a}");
        // S17 grouped dangling cites — unchanged.
        assert_eq!(doc.unresolved_citations_by_source(), "\\cite{ghost}");
        // S18 grouped dangling refs — unchanged.
        assert_eq!(doc.unresolved_references_by_source(), "\\eqref{eq:ghost}");
        // S19 numbered winning bibliography — unchanged.
        assert_eq!(doc.bibliography_entries(), "[1] a\n[2] b\n[3] c");
        // S20 losing duplicate labels — unchanged.
        assert_eq!(doc.duplicate_label_definitions(), "\\label{dup}");
        // S21 resolved references — unchanged.
        assert_eq!(
            doc.resolved_references_by_source(),
            "\\ref{sec:intro}\n\\eqref{eq:e}"
        );
        // S22 flat winning label definitions — unchanged.
        assert_eq!(
            doc.label_definitions(),
            "\\label{sec:intro}\n\\label{fig:p}\n\\label{eq:e}\n\\label{dup}"
        );

        // And S23 itself: the SAME winning definitions, grouped by kind in the fixed enum order
        // (Section, Figure, Equation, Inline). `sec:intro` (section) leads, then `fig:p` (figure),
        // then `eq:e` (equation), then `dup` (a bare inline `\label`). `\n`-joined, no trailing newline.
        assert_eq!(
            doc.label_definitions_by_kind(),
            "[section] \\label{sec:intro}\n[figure] \\label{fig:p}\n[equation] \\label{eq:e}\n[inline] \\label{dup}"
        );
    }

    // ---------------------------------------------------------------------------------------------
    // LTXDOC03 S24 — resolved references grouped by the KIND they resolved TO
    // (`resolved_references_by_kind`). The by-kind grouping companion of S21's flat, source-ordered
    // `resolved_references_by_source`; two views of one `resolved` list, command-aware.
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn s24_groups_different_kinds_in_fixed_kind_order() {
        // A `\ref` to a section and an `\eqref` to an equation → grouped by target kind in the fixed
        // enum order (Section before Equation), each `[kind] \<command>{key}` (command-aware). Source
        // order here already matches the fixed kind order.
        let src = r"\begin{document}\section{Intro}\label{sec:i}
\begin{equation}\label{eq:m}x=1\end{equation}
\ref{sec:i} \eqref{eq:m}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.resolved_references_by_kind(),
            "[section] \\ref{sec:i}\n[equation] \\eqref{eq:m}"
        );
    }

    #[test]
    fn s24_reorders_source_to_fixed_kind_order() {
        // Source order is the `\eqref` (equation) THEN the `\ref` (section), but the fixed kind order
        // pulls the section ahead of the equation — proving S24 groups by target kind rather than
        // echoing source order.
        let src = r"\begin{document}\section{Intro}\label{sec:i}
\begin{equation}\label{eq:m}x=1\end{equation}
\eqref{eq:m} \ref{sec:i}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.resolved_references_by_kind(),
            "[section] \\ref{sec:i}\n[equation] \\eqref{eq:m}"
        );
    }

    #[test]
    fn s24_same_kind_grouped_in_preorder_command_aware() {
        // Two refs to the SAME section (a `\ref` then a `\pageref`) → listed together under `section`,
        // in their existing pre-order (`\ref` before `\pageref`), each preserving its own command. Only
        // the `section` group appears; no empty groups for other kinds.
        let src = r"\begin{document}\section{Intro}\label{sec:i}
\ref{sec:i} \pageref{sec:i}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.resolved_references_by_kind(),
            "[section] \\ref{sec:i}\n[section] \\pageref{sec:i}"
        );
    }

    #[test]
    fn s24_only_dangling_or_none_returns_marker() {
        // A document whose only reference dangles (`\ref{nope}` with no `\label`) → the fixed
        // `(no resolved references)` marker S21 uses, never the empty string.
        let dangling = r"\begin{document}\ref{nope}\end{document}";
        let doc = parse_document(dangling).expect("parse");
        assert_eq!(doc.resolved_references_by_kind(), "(no resolved references)");

        // And a document with no references at all → the SAME marker.
        let none = r"\begin{document}Just some text with no references.\end{document}";
        let doc = parse_document(none).expect("parse");
        assert_eq!(doc.resolved_references_by_kind(), "(no resolved references)");
    }

    #[test]
    fn s24_newline_join_no_trailing_newline_excludes_dangling() {
        // A `\ref` to a section, an `\eqref` to an equation, plus a DANGLING `\ref{nope}` →
        // exact string equality pins the `\n`-join with NO trailing newline, the fixed kind order
        // (Section then Equation), AND that the dangling `\ref{nope}` is excluded (it never entered
        // `resolved`).
        let src = r"\begin{document}\section{Intro}\label{sec:i}
\begin{equation}\label{eq:m}x=1\end{equation}
\ref{sec:i} \eqref{eq:m} \ref{nope}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.resolved_references_by_kind(),
            "[section] \\ref{sec:i}\n[equation] \\eqref{eq:m}"
        );
        // The dangling ref shows up in S18, confirming it was routed to `unresolved`, not `resolved`.
        assert_eq!(doc.unresolved_references_by_source(), "\\ref{nope}");
    }

    #[test]
    fn s24_is_additive_leaves_s1_s23_outputs_unchanged() {
        // Same representative doc as the S23 additive test. S24 changes NONE of the S1-S23 outputs — it
        // only reads `resolve_references`. S21's flat `resolved_references_by_source` and S24's grouped
        // `resolved_references_by_kind` are two views of the SAME `resolved` list; both are pinned here
        // to show S24 neither adds, drops, nor reorders relative to S21.
        let src = r"\begin{document}\section{Introduction}\label{sec:intro}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}

\begin{equation}\label{eq:e}E=mc^2\end{equation}

First \label{dup} here.

Second \label{dup} there.

See Section~\ref{sec:intro}, \eqref{eq:e}, \eqref{eq:ghost}, \nameref{sec:intro}, \nameref{fig:p}, and \cite{a,b} plus \cite{c,ghost}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\bibitem{b} Author B.
\bibitem{c} Author C.
\bibitem{a} Author A again.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");

        // S1/S6 flat report — unchanged.
        assert_eq!(
            doc.cross_reference_report().to_plain_text(),
            "\\ref{sec:intro} -> Section 1\n\\eqref{eq:e} -> Equation (1)\n\\cite{a} -> [1]\n\\cite{b} -> [2]\n\\cite{c} -> [3]\nDangling references: eq:ghost\nDangling citations: ghost"
        );
        // S11 grouped-by-kind report — unchanged.
        assert_eq!(
            doc.cross_reference_report().to_plain_text_by_kind(),
            "Sections:\n  \\ref{sec:intro} -> Section 1\nEquations:\n  \\eqref{eq:e} -> Equation (1)"
        );
        // S12 list of floats — unchanged.
        assert_eq!(doc.list_of_floats(), "List of Figures\n1. A plot");
        // S13 nameref resolution — unchanged.
        assert_eq!(
            doc.resolve_namerefs(),
            "\\nameref{sec:intro} -> Introduction\n\\nameref{fig:p} -> A plot"
        );
        // S14 per-kind census — unchanged.
        assert_eq!(doc.list_summary(), "Sections: 1\nFigures: 1\nEquations: 1");
        // S15 grouped resolved cites — unchanged.
        assert_eq!(doc.citations_by_source(), "\\cite{a, b}\n\\cite{c}");
        // S16 duplicate bibliography entries — unchanged.
        assert_eq!(doc.duplicate_bibliography_entries(), "\\bibitem{a}");
        // S17 grouped dangling cites — unchanged.
        assert_eq!(doc.unresolved_citations_by_source(), "\\cite{ghost}");
        // S18 grouped dangling refs — unchanged.
        assert_eq!(doc.unresolved_references_by_source(), "\\eqref{eq:ghost}");
        // S19 numbered winning bibliography — unchanged.
        assert_eq!(doc.bibliography_entries(), "[1] a\n[2] b\n[3] c");
        // S20 losing duplicate labels — unchanged.
        assert_eq!(doc.duplicate_label_definitions(), "\\label{dup}");
        // S21 resolved references (flat, source order) — unchanged.
        assert_eq!(
            doc.resolved_references_by_source(),
            "\\ref{sec:intro}\n\\eqref{eq:e}"
        );
        // S22 flat winning label definitions — unchanged.
        assert_eq!(
            doc.label_definitions(),
            "\\label{sec:intro}\n\\label{fig:p}\n\\label{eq:e}\n\\label{dup}"
        );
        // S23 grouped winning label definitions — unchanged.
        assert_eq!(
            doc.label_definitions_by_kind(),
            "[section] \\label{sec:intro}\n[figure] \\label{fig:p}\n[equation] \\label{eq:e}\n[inline] \\label{dup}"
        );

        // And S24 itself: the SAME resolved refs S21 lists flat, now grouped by target kind in the
        // fixed enum order. The `\ref{sec:intro}` (section) leads, then the `\eqref{eq:e}` (equation);
        // the `\eqref{eq:ghost}` dangled and never entered `resolved`. `\n`-joined, no trailing newline.
        assert_eq!(
            doc.resolved_references_by_kind(),
            "[section] \\ref{sec:intro}\n[equation] \\eqref{eq:e}"
        );
    }

    // ---------------------------------------------------------------------------------------------
    // LTXDOC03 S25 — per-kind CENSUS (counts) of the winning label definitions (`label_kind_counts`).
    // The count companion of S23's `label_definitions_by_kind`: one `<kind>: <n>` line per kind (in
    // the fixed enum order), a numeric summary over the SAME winning `definitions` list.
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn s25_counts_multiple_kinds_in_fixed_kind_order_zero_kinds_omitted() {
        // A section label, two equation labels, and a bare inline label → one `<kind>: <n>` line per
        // kind in the fixed enum order (Section, then Equation, then Inline). The Table and Figure
        // kinds have zero definitions and so contribute NO lines (never `table: 0`).
        let src = r"\begin{document}\section{Intro}\label{sec:intro}
\begin{equation}\label{eq:a}x=1\end{equation}
\begin{equation}\label{eq:b}y=2\end{equation}
\label{note}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.label_kind_counts(),
            "section: 1\nequation: 2\ninline: 1"
        );
    }

    #[test]
    fn s25_exactly_one_kind() {
        // Two inline labels of the SAME (and only) kind → a single `inline: 2` line. No other kinds,
        // no trailing newline.
        let src = r"\begin{document}\label{a}

\label{b}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.label_kind_counts(), "inline: 2");
    }

    #[test]
    fn s25_no_labels_returns_marker() {
        // A document with no `\label` at all → the same fixed `(no label definitions)` marker S22/S23
        // use, never the empty string.
        let src = r"\begin{document}Just some text with no labels.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.label_kind_counts(), "(no label definitions)");
    }

    #[test]
    fn s25_duplicate_definitions_count_only_the_winner() {
        // `dup` is `\label`ed TWICE (inline); only the WINNING first definition is in `definitions`,
        // the loser is a duplicate (S20's domain). So the inline count is 1, NOT 2 — S25 counts winning
        // definitions, never duplicates. A distinct `once` inline label makes the winning inline count 2.
        let src = r"\begin{document}First \label{dup} here.

Second \label{dup} there.

\label{once}\end{document}";
        let doc = parse_document(src).expect("parse");
        // Two DISTINCT winning inline keys (`dup` once as the winner, `once`) → inline: 2.
        assert_eq!(doc.label_kind_counts(), "inline: 2");
        // Confirm the losing re-`\label{dup}` was routed to duplicates (S20), not counted here.
        assert_eq!(doc.duplicate_label_definitions(), "\\label{dup}");
    }

    #[test]
    fn s25_newline_join_no_trailing_newline() {
        // A section + figure + inline label → exact string equality pins the `\n`-join with NO trailing
        // newline, and the fixed kind order (Section, then Figure, then Inline — Table and Equation
        // absent), each line a `<kind>: 1` count.
        let src = r"\begin{document}\section{Intro}\label{sec:i}
\begin{figure}\includegraphics{p.png}\caption{P}\label{fig:p}\end{figure}

\label{note}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.label_kind_counts(),
            "section: 1\nfigure: 1\ninline: 1"
        );
    }

    #[test]
    fn s25_is_additive_leaves_s1_s24_outputs_unchanged() {
        // Same representative doc as the S24 additive test. S25 changes NONE of the S1-S24 outputs — it
        // only reads `resolve_references`. S22's flat `label_definitions`, S23's grouped
        // `label_definitions_by_kind`, and S25's per-kind counts are three views of the SAME winning
        // `definitions` list; all are pinned here to show S25 neither adds, drops, nor reorders, and
        // that its counts agree with S23's grouping.
        let src = r"\begin{document}\section{Introduction}\label{sec:intro}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}

\begin{equation}\label{eq:e}E=mc^2\end{equation}

First \label{dup} here.

Second \label{dup} there.

See Section~\ref{sec:intro}, \eqref{eq:e}, \eqref{eq:ghost}, \nameref{sec:intro}, \nameref{fig:p}, and \cite{a,b} plus \cite{c,ghost}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\bibitem{b} Author B.
\bibitem{c} Author C.
\bibitem{a} Author A again.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");

        // S1/S6 flat report — unchanged.
        assert_eq!(
            doc.cross_reference_report().to_plain_text(),
            "\\ref{sec:intro} -> Section 1\n\\eqref{eq:e} -> Equation (1)\n\\cite{a} -> [1]\n\\cite{b} -> [2]\n\\cite{c} -> [3]\nDangling references: eq:ghost\nDangling citations: ghost"
        );
        // S11 grouped-by-kind report — unchanged.
        assert_eq!(
            doc.cross_reference_report().to_plain_text_by_kind(),
            "Sections:\n  \\ref{sec:intro} -> Section 1\nEquations:\n  \\eqref{eq:e} -> Equation (1)"
        );
        // S12 list of floats — unchanged.
        assert_eq!(doc.list_of_floats(), "List of Figures\n1. A plot");
        // S13 nameref resolution — unchanged.
        assert_eq!(
            doc.resolve_namerefs(),
            "\\nameref{sec:intro} -> Introduction\n\\nameref{fig:p} -> A plot"
        );
        // S14 per-kind census — unchanged.
        assert_eq!(doc.list_summary(), "Sections: 1\nFigures: 1\nEquations: 1");
        // S15 grouped resolved cites — unchanged.
        assert_eq!(doc.citations_by_source(), "\\cite{a, b}\n\\cite{c}");
        // S16 duplicate bibliography entries — unchanged.
        assert_eq!(doc.duplicate_bibliography_entries(), "\\bibitem{a}");
        // S17 grouped dangling cites — unchanged.
        assert_eq!(doc.unresolved_citations_by_source(), "\\cite{ghost}");
        // S18 grouped dangling refs — unchanged.
        assert_eq!(doc.unresolved_references_by_source(), "\\eqref{eq:ghost}");
        // S19 numbered winning bibliography — unchanged.
        assert_eq!(doc.bibliography_entries(), "[1] a\n[2] b\n[3] c");
        // S20 losing duplicate labels — unchanged.
        assert_eq!(doc.duplicate_label_definitions(), "\\label{dup}");
        // S21 resolved references (flat, source order) — unchanged.
        assert_eq!(
            doc.resolved_references_by_source(),
            "\\ref{sec:intro}\n\\eqref{eq:e}"
        );
        // S22 flat winning label definitions — unchanged.
        assert_eq!(
            doc.label_definitions(),
            "\\label{sec:intro}\n\\label{fig:p}\n\\label{eq:e}\n\\label{dup}"
        );
        // S23 grouped winning label definitions — unchanged.
        assert_eq!(
            doc.label_definitions_by_kind(),
            "[section] \\label{sec:intro}\n[figure] \\label{fig:p}\n[equation] \\label{eq:e}\n[inline] \\label{dup}"
        );
        // S24 grouped resolved references — unchanged.
        assert_eq!(
            doc.resolved_references_by_kind(),
            "[section] \\ref{sec:intro}\n[equation] \\eqref{eq:e}"
        );

        // And S25 itself: the per-kind COUNTS of the SAME winning definitions, in the fixed enum order.
        // One section (`sec:intro`), one figure (`fig:p`), one equation (`eq:e`), one winning inline
        // (`dup` — its second `\label` is a duplicate, not a second definition). `\n`-joined, no
        // trailing newline. Table has zero definitions and so is omitted.
        assert_eq!(
            doc.label_kind_counts(),
            "section: 1\nfigure: 1\nequation: 1\ninline: 1"
        );
    }

    // ---------------------------------------------------------------------------------------------
    // LTXDOC03 S26 — per-kind CENSUS (counts) of the RESOLVED references
    // (`resolved_reference_kind_counts`). The count companion of S24's
    // `resolved_references_by_kind`: one `<kind>: <n>` line per kind (in the fixed enum order), a
    // numeric summary over the SAME `resolved` list. Dangling refs live in `unresolved` (S18).
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn s26_counts_multiple_kinds_in_fixed_kind_order_zero_kinds_omitted() {
        // A `\ref` to a section, two `\eqref`s to two equations, and a `\ref` to a bare inline label →
        // one `<kind>: <n>` line per kind in the fixed enum order (Section, then Equation, then Inline).
        // The Table and Figure kinds have zero resolved refs and so contribute NO lines (never
        // `table: 0`).
        let src = r"\begin{document}\section{Intro}\label{sec:i}
\begin{equation}\label{eq:a}x=1\end{equation}
\begin{equation}\label{eq:b}y=2\end{equation}
\label{note}
\ref{sec:i} \eqref{eq:a} \eqref{eq:b} \ref{note}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.resolved_reference_kind_counts(),
            "section: 1\nequation: 2\ninline: 1"
        );
    }

    #[test]
    fn s26_exactly_one_kind() {
        // Two refs to the SAME (and only) section → a single `section: 2` line. No other kinds, no
        // trailing newline.
        let src = r"\begin{document}\section{Intro}\label{sec:i}
\ref{sec:i} \pageref{sec:i}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.resolved_reference_kind_counts(), "section: 2");
    }

    #[test]
    fn s26_two_refs_to_two_section_labels_count_two() {
        // Two `\ref`s to two DIFFERENT section labels → `section: 2` (multiple resolved refs to the
        // same kind aggregate into the kind's count).
        let src = r"\begin{document}\section{One}\label{sec:a}
\section{Two}\label{sec:b}
\ref{sec:a} \ref{sec:b}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.resolved_reference_kind_counts(), "section: 2");
    }

    #[test]
    fn s26_all_dangling_or_none_returns_marker() {
        // A document whose only reference dangles (`\ref{nope}` with no `\label`) → the fixed
        // `(no resolved references)` marker S21/S24 use, never a `<kind>: 0` line — the dangling ref
        // lands in `unresolved` (S18), not `resolved`.
        let dangling = r"\begin{document}\ref{nope}\end{document}";
        let doc = parse_document(dangling).expect("parse");
        assert_eq!(
            doc.resolved_reference_kind_counts(),
            "(no resolved references)"
        );
        // Cross-check: the dangling ref is in `unresolved`, so it never contributed a zero count.
        assert_eq!(doc.unresolved_references_by_source(), "\\ref{nope}");

        // And a document with no references at all → the SAME marker.
        let none = r"\begin{document}Just some text with no references.\end{document}";
        let doc = parse_document(none).expect("parse");
        assert_eq!(
            doc.resolved_reference_kind_counts(),
            "(no resolved references)"
        );
    }

    #[test]
    fn s26_newline_join_no_trailing_newline_excludes_dangling() {
        // A `\ref` to a section, an `\eqref` to an equation, plus a DANGLING `\ref{nope}` → exact
        // string equality pins the `\n`-join with NO trailing newline, the fixed kind order (Section
        // then Equation), AND that the dangling `\ref{nope}` is excluded from the counts (it never
        // entered `resolved`).
        let src = r"\begin{document}\section{Intro}\label{sec:i}
\begin{equation}\label{eq:m}x=1\end{equation}
\ref{sec:i} \eqref{eq:m} \ref{nope}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(
            doc.resolved_reference_kind_counts(),
            "section: 1\nequation: 1"
        );
        // The dangling ref shows up in S18, confirming it was routed to `unresolved`, not counted here.
        assert_eq!(doc.unresolved_references_by_source(), "\\ref{nope}");
    }

    #[test]
    fn s26_is_additive_leaves_s1_s25_outputs_unchanged() {
        // Same representative doc as the S24/S25 additive tests. S26 changes NONE of the S1-S25 outputs
        // — it only reads `resolve_references`. S21's flat `resolved_references_by_source`, S24's
        // grouped `resolved_references_by_kind`, and S26's per-kind counts are three views of the SAME
        // `resolved` list; all are pinned here to show S26 neither adds, drops, nor reorders, and that
        // its counts agree with S24's grouping.
        let src = r"\begin{document}\section{Introduction}\label{sec:intro}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}

\begin{equation}\label{eq:e}E=mc^2\end{equation}

First \label{dup} here.

Second \label{dup} there.

See Section~\ref{sec:intro}, \eqref{eq:e}, \eqref{eq:ghost}, \nameref{sec:intro}, \nameref{fig:p}, and \cite{a,b} plus \cite{c,ghost}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\bibitem{b} Author B.
\bibitem{c} Author C.
\bibitem{a} Author A again.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");

        // S1/S6 flat report — unchanged.
        assert_eq!(
            doc.cross_reference_report().to_plain_text(),
            "\\ref{sec:intro} -> Section 1\n\\eqref{eq:e} -> Equation (1)\n\\cite{a} -> [1]\n\\cite{b} -> [2]\n\\cite{c} -> [3]\nDangling references: eq:ghost\nDangling citations: ghost"
        );
        // S11 grouped-by-kind report — unchanged.
        assert_eq!(
            doc.cross_reference_report().to_plain_text_by_kind(),
            "Sections:\n  \\ref{sec:intro} -> Section 1\nEquations:\n  \\eqref{eq:e} -> Equation (1)"
        );
        // S12 list of floats — unchanged.
        assert_eq!(doc.list_of_floats(), "List of Figures\n1. A plot");
        // S13 nameref resolution — unchanged.
        assert_eq!(
            doc.resolve_namerefs(),
            "\\nameref{sec:intro} -> Introduction\n\\nameref{fig:p} -> A plot"
        );
        // S14 per-kind census — unchanged.
        assert_eq!(doc.list_summary(), "Sections: 1\nFigures: 1\nEquations: 1");
        // S15 grouped resolved cites — unchanged.
        assert_eq!(doc.citations_by_source(), "\\cite{a, b}\n\\cite{c}");
        // S16 duplicate bibliography entries — unchanged.
        assert_eq!(doc.duplicate_bibliography_entries(), "\\bibitem{a}");
        // S17 grouped dangling cites — unchanged.
        assert_eq!(doc.unresolved_citations_by_source(), "\\cite{ghost}");
        // S18 grouped dangling refs — unchanged.
        assert_eq!(doc.unresolved_references_by_source(), "\\eqref{eq:ghost}");
        // S19 numbered winning bibliography — unchanged.
        assert_eq!(doc.bibliography_entries(), "[1] a\n[2] b\n[3] c");
        // S20 losing duplicate labels — unchanged.
        assert_eq!(doc.duplicate_label_definitions(), "\\label{dup}");
        // S21 resolved references (flat, source order) — unchanged.
        assert_eq!(
            doc.resolved_references_by_source(),
            "\\ref{sec:intro}\n\\eqref{eq:e}"
        );
        // S22 flat winning label definitions — unchanged.
        assert_eq!(
            doc.label_definitions(),
            "\\label{sec:intro}\n\\label{fig:p}\n\\label{eq:e}\n\\label{dup}"
        );
        // S23 grouped winning label definitions — unchanged.
        assert_eq!(
            doc.label_definitions_by_kind(),
            "[section] \\label{sec:intro}\n[figure] \\label{fig:p}\n[equation] \\label{eq:e}\n[inline] \\label{dup}"
        );
        // S24 grouped resolved references — unchanged.
        assert_eq!(
            doc.resolved_references_by_kind(),
            "[section] \\ref{sec:intro}\n[equation] \\eqref{eq:e}"
        );
        // S25 per-kind label-definition counts — unchanged.
        assert_eq!(
            doc.label_kind_counts(),
            "section: 1\nfigure: 1\nequation: 1\ninline: 1"
        );

        // And S26 itself: the per-kind COUNTS of the SAME resolved references, in the fixed enum order.
        // One resolved ref lands on a section (`\ref{sec:intro}`) and one on an equation (`\eqref{eq:e}`);
        // the `\eqref{eq:ghost}` dangles (excluded). `\n`-joined, no trailing newline. Table, figure,
        // and inline have zero resolved refs and so are omitted.
        assert_eq!(
            doc.resolved_reference_kind_counts(),
            "section: 1\nequation: 1"
        );
    }

    // ---------------------------------------------------------------------------------------------
    // LTXDOC03 S27 — single-integer TOTAL of the unresolved (dangling) references
    // (`unresolved_reference_count`). The count-total companion of S18's
    // `unresolved_references_by_source`: one decimal line = `.len()` of the SAME `unresolved` list.
    // Dangling refs carry no `target_kind`, so no per-kind census is possible — a total is the move.
    // Being a COUNT renderer, its empty value is the honest number "0", NOT a `(no …)` marker.
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn s27_two_dangling_plus_one_resolved_counts_two() {
        // Two `\ref`s that dangle (`nope`, `gone`) plus one that resolves (`sec:i`) → the count of
        // dangling refs is exactly "2"; the resolved `\ref{sec:i}` is excluded (it lands in `resolved`).
        let src = r"\begin{document}\section{Intro}\label{sec:i}
\ref{sec:i} \ref{nope} \ref{gone}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.unresolved_reference_count(), "2");
        // Cross-check: the two dangling refs are exactly what S18 enumerates.
        assert_eq!(
            doc.unresolved_references_by_source(),
            "\\ref{nope}\n\\ref{gone}"
        );
    }

    #[test]
    fn s27_all_refs_resolve_counts_zero() {
        // Every reference resolves to a real `\label` → zero danglers. A COUNT renderer's honest value
        // for an empty list is the number "0" (never a `(no …)` marker).
        let src = r"\begin{document}\section{One}\label{sec:a}
\section{Two}\label{sec:b}
\ref{sec:a} \ref{sec:b} \pageref{sec:a}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.unresolved_reference_count(), "0");
        // And S18 (the list view) has no lines to show for the same doc.
        assert_eq!(
            doc.unresolved_references_by_source(),
            "(no unresolved references)"
        );
    }

    #[test]
    fn s27_no_references_at_all_counts_zero() {
        // A document with NO references whatsoever → still "0" (the count of an empty `unresolved` list).
        let src = r"\begin{document}Just some text with no references.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.unresolved_reference_count(), "0");
    }

    #[test]
    fn s27_mixed_kinds_of_danglers_count_the_integer() {
        // Several danglers of DIFFERENT intended kinds — a `\ref`, an `\eqref`, and a `\pageref`, none
        // of which any `\label` defines. No kind is tracked for a dangling ref (it bound to nothing), so
        // the answer is simply the integer count of dangling refs: "3".
        let src = r"\begin{document}\section{Intro}\label{sec:i}
\ref{sec:i} \ref{no1} \eqref{eq:no2} \pageref{no3}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.unresolved_reference_count(), "3");
    }

    #[test]
    fn s27_count_equals_number_of_s18_lines() {
        // Cross-check the total against S18's per-source list: the count S27 returns MUST equal the
        // number of lines S18 enumerates (they are two views of the SAME `unresolved` list).
        let src = r"\begin{document}\section{Intro}\label{sec:i}
\ref{sec:i} \ref{a} \eqref{eq:b} \pageref{c} \ref{d}\end{document}";
        let doc = parse_document(src).expect("parse");
        let s18 = doc.unresolved_references_by_source();
        let s18_line_count = s18.lines().count();
        assert_eq!(s18_line_count, 4);
        assert_eq!(doc.unresolved_reference_count(), s18_line_count.to_string());
        assert_eq!(doc.unresolved_reference_count(), "4");
    }

    #[test]
    fn s27_is_additive_leaves_s1_s26_outputs_unchanged() {
        // Same representative doc as the S24/S25/S26 additive tests. S27 changes NONE of the S1-S26
        // outputs — it only reads `resolve_references`. Its count is a second view of the SAME
        // `unresolved` list S18 renders per-source; pinned here to show S27 neither adds, drops, nor
        // reorders, and that its total agrees with the number of lines S18 enumerates.
        let src = r"\begin{document}\section{Introduction}\label{sec:intro}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}

\begin{equation}\label{eq:e}E=mc^2\end{equation}

First \label{dup} here.

Second \label{dup} there.

See Section~\ref{sec:intro}, \eqref{eq:e}, \eqref{eq:ghost}, \nameref{sec:intro}, \nameref{fig:p}, and \cite{a,b} plus \cite{c,ghost}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\bibitem{b} Author B.
\bibitem{c} Author C.
\bibitem{a} Author A again.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");

        // A handful of prior renderers — byte-for-byte unchanged.
        // S14 per-kind census.
        assert_eq!(doc.list_summary(), "Sections: 1\nFigures: 1\nEquations: 1");
        // S19 numbered winning bibliography.
        assert_eq!(doc.bibliography_entries(), "[1] a\n[2] b\n[3] c");
        // S22 flat winning label definitions.
        assert_eq!(
            doc.label_definitions(),
            "\\label{sec:intro}\n\\label{fig:p}\n\\label{eq:e}\n\\label{dup}"
        );
        // S18 grouped dangling refs — the one `\eqref{eq:ghost}` dangles.
        assert_eq!(doc.unresolved_references_by_source(), "\\eqref{eq:ghost}");
        // S25 per-kind label-definition counts.
        assert_eq!(
            doc.label_kind_counts(),
            "section: 1\nfigure: 1\nequation: 1\ninline: 1"
        );
        // S26 per-kind resolved-reference counts.
        assert_eq!(
            doc.resolved_reference_kind_counts(),
            "section: 1\nequation: 1"
        );

        // And S27 itself: exactly ONE reference dangles (`\eqref{eq:ghost}`), so the count is "1" —
        // agreeing with the single line S18 enumerates for the same doc.
        assert_eq!(doc.unresolved_reference_count(), "1");
        assert_eq!(
            doc.unresolved_reference_count(),
            doc.unresolved_references_by_source().lines().count().to_string()
        );
    }

    // ---------------------------------------------------------------------------------------------
    // LTXDOC03 S28 — single-integer TOTAL of the resolved references (`resolved_reference_count`).
    // The count-total companion of S21's `resolved_references_by_source` / S24's
    // `resolved_references_by_kind`: one decimal line = `.len()` of the SAME `resolved` list. It is
    // the exact resolved-side twin of S27's `unresolved_reference_count` — together S28 + S27 split
    // every reference into (resolved, dangling). No `target_kind` is read, so all kinds fold into one
    // total. Being a COUNT renderer, its empty value is the honest number "0", NOT a `(no …)` marker.
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn s28_two_resolved_plus_one_dangling_counts_two() {
        // Two `\ref`s that resolve (`sec:i` twice) plus one that dangles (`nope`) → the count of
        // resolved refs is exactly "2"; the dangling `\ref{nope}` is excluded (it lands in `unresolved`).
        // Cross-check: on the SAME doc S27 returns "1" — the two totals split the references.
        let src = r"\begin{document}\section{Intro}\label{sec:i}
\ref{sec:i} \pageref{sec:i} \ref{nope}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.resolved_reference_count(), "2");
        // The dangling side: exactly one, so S28 + S27 = 3 = the total reference count.
        assert_eq!(doc.unresolved_reference_count(), "1");
        // Cross-check: the two resolved refs are exactly what S21 enumerates.
        assert_eq!(
            doc.resolved_references_by_source(),
            "\\ref{sec:i}\n\\pageref{sec:i}"
        );
    }

    #[test]
    fn s28_no_resolvable_refs_counts_zero() {
        // Every reference dangles (no `\label` defines them) → zero resolved refs. A COUNT renderer's
        // honest value for an empty list is the number "0" (never a `(no …)` marker).
        let src = r"\begin{document}\ref{nope} \eqref{eq:ghost} \pageref{gone}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.resolved_reference_count(), "0");
        // And S21 (the list view) shows its fixed marker for the same doc — the divergence is intended.
        assert_eq!(
            doc.resolved_references_by_source(),
            "(no resolved references)"
        );
    }

    #[test]
    fn s28_no_references_at_all_counts_zero() {
        // A document with NO references whatsoever → still "0" (the count of an empty `resolved` list).
        let src = r"\begin{document}Just some text with no references.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.resolved_reference_count(), "0");
    }

    #[test]
    fn s28_mixed_target_kinds_count_the_integer() {
        // Several resolved refs across DIFFERENT target kinds — a section, a table, and an equation. No
        // kind is tracked by S28 (only `.len()` is read), so the answer is simply the integer count of
        // resolved refs: "3".
        let src = r"\begin{document}\section{Intro}\label{sec:i}
\begin{table}\caption{T}\label{tab:t}\end{table}
\begin{equation}\label{eq:e}E=mc^2\end{equation}
\ref{sec:i} \ref{tab:t} \eqref{eq:e}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.resolved_reference_count(), "3");
    }

    #[test]
    fn s28_count_equals_number_of_s21_lines() {
        // Cross-check the total against S21's per-source list: the count S28 returns MUST equal the
        // number of lines S21 enumerates (they are two views of the SAME `resolved` list).
        let src = r"\begin{document}\section{Intro}\label{sec:i}
\begin{equation}\label{eq:e}E=mc^2\end{equation}
\ref{sec:i} \eqref{eq:e} \pageref{sec:i} \ref{nope}\end{document}";
        let doc = parse_document(src).expect("parse");
        let s21 = doc.resolved_references_by_source();
        let s21_line_count = s21.lines().count();
        assert_eq!(s21_line_count, 3);
        assert_eq!(doc.resolved_reference_count(), s21_line_count.to_string());
        assert_eq!(doc.resolved_reference_count(), "3");
    }

    #[test]
    fn s28_is_additive_leaves_s1_s27_outputs_unchanged() {
        // Same representative doc as the S24/S25/S26/S27 additive tests. S28 changes NONE of the S1-S27
        // outputs — it only reads `resolve_references`. Its count is a second view of the SAME
        // `resolved` list S21/S24 render per-source and per-kind; pinned here to show S28 neither adds,
        // drops, nor reorders, and that its total agrees with the number of lines S21 enumerates.
        let src = r"\begin{document}\section{Introduction}\label{sec:intro}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}

\begin{equation}\label{eq:e}E=mc^2\end{equation}

First \label{dup} here.

Second \label{dup} there.

See Section~\ref{sec:intro}, \eqref{eq:e}, \eqref{eq:ghost}, \nameref{sec:intro}, \nameref{fig:p}, and \cite{a,b} plus \cite{c,ghost}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\bibitem{b} Author B.
\bibitem{c} Author C.
\bibitem{a} Author A again.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");

        // A handful of prior renderers — byte-for-byte unchanged.
        // S14 per-kind census.
        assert_eq!(doc.list_summary(), "Sections: 1\nFigures: 1\nEquations: 1");
        // S22 flat winning label definitions.
        assert_eq!(
            doc.label_definitions(),
            "\\label{sec:intro}\n\\label{fig:p}\n\\label{eq:e}\n\\label{dup}"
        );
        // S21 flat resolved refs — `\ref{sec:intro}` and `\eqref{eq:e}` resolve (`\eqref{eq:ghost}`
        // dangles; `\nameref`/`\cite` are outside the `\ref` family).
        assert_eq!(
            doc.resolved_references_by_source(),
            "\\ref{sec:intro}\n\\eqref{eq:e}"
        );
        // S25 per-kind label-definition counts.
        assert_eq!(
            doc.label_kind_counts(),
            "section: 1\nfigure: 1\nequation: 1\ninline: 1"
        );
        // S26 per-kind resolved-reference counts.
        assert_eq!(
            doc.resolved_reference_kind_counts(),
            "section: 1\nequation: 1"
        );
        // S27 dangling-reference count — exactly one (`\eqref{eq:ghost}`).
        assert_eq!(doc.unresolved_reference_count(), "1");

        // And S28 itself: exactly TWO references resolve, so the count is "2" — agreeing with the two
        // lines S21 enumerates for the same doc.
        assert_eq!(doc.resolved_reference_count(), "2");
        assert_eq!(
            doc.resolved_reference_count(),
            doc.resolved_references_by_source().lines().count().to_string()
        );
    }

    // ---------------------------------------------------------------------------------------------
    // LTXDOC03 S29 — single-integer TOTAL of the label definitions (`label_definition_count`). The
    // count-total companion of S22's `label_definitions` / S23's `label_definitions_by_kind`: one
    // decimal line = `.len()` of the SAME winning `definitions` list. It is the label-definition-side
    // analogue of S27/S28's reference-side totals. No `kind` is read, so all kinds fold into one
    // total. Being a COUNT renderer, its empty value is the honest number "0", NOT a `(no …)` marker.
    // A later duplicate `\label` is in `duplicates` (S20), never `definitions`, so it is excluded —
    // the count is exactly the number of lines S22 lists.
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn s29_multiple_definitions_count_the_integer() {
        // Three distinct label keys defined (a section, an equation, an inline label) → the count is
        // exactly "3".
        let src = r"\begin{document}\section{Intro}\label{sec:i}
\begin{equation}\label{eq:e}E=mc^2\end{equation}
Some \label{inl} text.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.label_definition_count(), "3");
        // Cross-check: the three winning definitions are exactly what S22 enumerates.
        assert_eq!(
            doc.label_definitions(),
            "\\label{sec:i}\n\\label{eq:e}\n\\label{inl}"
        );
    }

    #[test]
    fn s29_duplicate_label_counted_once_matching_s22() {
        // A key `\label{dup}` defined twice: only the FIRST (winning) definition is in `definitions`;
        // the later one is a DUPLICATE (S20's domain), never a second `definitions` row. So the count
        // is the number of DISTINCT keys, exactly matching the number of lines S22 lists.
        let src = r"\begin{document}\section{Intro}\label{sec:i}
First \label{dup} here.
Second \label{dup} there.\end{document}";
        let doc = parse_document(src).expect("parse");
        // Two distinct keys: `sec:i` and `dup`.
        assert_eq!(doc.label_definition_count(), "2");
        // The later `\label{dup}` is a duplicate, surfaced by S20, not counted here.
        assert_eq!(doc.duplicate_label_definitions(), "\\label{dup}");
        // The count MUST equal the number of lines S22 lists (two views of the SAME list).
        assert_eq!(
            doc.label_definition_count(),
            doc.label_definitions().lines().count().to_string()
        );
        assert_eq!(doc.label_definitions(), "\\label{sec:i}\n\\label{dup}");
    }

    #[test]
    fn s29_no_labels_counts_zero() {
        // A document with NO `\label` at all → "0" (the count of an empty `definitions` list). A COUNT
        // renderer's honest value for an empty list is the number "0", never a `(no …)` marker.
        let src = r"\begin{document}Just some text with no labels.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.label_definition_count(), "0");
        // And S22 (the list view) shows its fixed marker for the same doc — the divergence is intended.
        assert_eq!(doc.label_definitions(), "(no label definitions)");
    }

    #[test]
    fn s29_mixed_document_counts_only_label_definitions() {
        // A mixed doc — labels + refs + citations. S29 counts ONLY the label-definition total; the
        // `\ref`s and `\cite`s do not perturb it. Three distinct label keys are defined, so the answer
        // is "3" regardless of how many references or citations appear.
        let src = r"\begin{document}\section{Intro}\label{sec:i}
\begin{equation}\label{eq:e}E=mc^2\end{equation}
\begin{figure}\caption{P}\label{fig:p}\end{figure}
See \ref{sec:i}, \eqref{eq:e}, \ref{ghost} and \cite{a,b}.
\begin{thebibliography}{9}\bibitem{a} A.\bibitem{b} B.\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.label_definition_count(), "3");
        // Unaffected by the refs (2 resolve, 1 dangles) or the citations.
        assert_eq!(doc.resolved_reference_count(), "2");
        assert_eq!(doc.unresolved_reference_count(), "1");
        // Still equals the number of lines S22 lists.
        assert_eq!(
            doc.label_definition_count(),
            doc.label_definitions().lines().count().to_string()
        );
    }

    #[test]
    fn s29_count_equals_number_of_s22_lines() {
        // Cross-check the total against S22's flat list: the count S29 returns MUST equal the number of
        // lines S22 enumerates (they are two views of the SAME winning `definitions` list).
        let src = r"\begin{document}\section{A}\label{a}
\section{B}\label{b}
\begin{equation}\label{c}x=1\end{equation}
\label{a}\end{document}";
        let doc = parse_document(src).expect("parse");
        let s22 = doc.label_definitions();
        let s22_line_count = s22.lines().count();
        assert_eq!(s22_line_count, 3);
        assert_eq!(doc.label_definition_count(), s22_line_count.to_string());
        assert_eq!(doc.label_definition_count(), "3");
    }

    #[test]
    fn s29_is_additive_leaves_s1_s28_outputs_unchanged() {
        // Same representative doc as the S24/S25/S26/S27/S28 additive tests. S29 changes NONE of the
        // S1-S28 outputs — it only reads `resolve_references`. Its count is a second view of the SAME
        // `definitions` list S22/S23 render flat and per-kind; pinned here to show S29 neither adds,
        // drops, nor reorders, and that its total agrees with the number of lines S22 enumerates.
        let src = r"\begin{document}\section{Introduction}\label{sec:intro}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}

\begin{equation}\label{eq:e}E=mc^2\end{equation}

First \label{dup} here.

Second \label{dup} there.

See Section~\ref{sec:intro}, \eqref{eq:e}, \eqref{eq:ghost}, \nameref{sec:intro}, \nameref{fig:p}, and \cite{a,b} plus \cite{c,ghost}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\bibitem{b} Author B.
\bibitem{c} Author C.
\bibitem{a} Author A again.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");

        // A handful of prior renderers — byte-for-byte unchanged.
        // S22 flat winning label definitions.
        assert_eq!(
            doc.label_definitions(),
            "\\label{sec:intro}\n\\label{fig:p}\n\\label{eq:e}\n\\label{dup}"
        );
        // S25 per-kind label-definition counts.
        assert_eq!(
            doc.label_kind_counts(),
            "section: 1\nfigure: 1\nequation: 1\ninline: 1"
        );
        // S27 dangling-reference count — exactly one (`\eqref{eq:ghost}`).
        assert_eq!(doc.unresolved_reference_count(), "1");
        // S28 resolved-reference count — exactly two (`\ref{sec:intro}`, `\eqref{eq:e}`).
        assert_eq!(doc.resolved_reference_count(), "2");

        // And S29 itself: exactly FOUR distinct label keys are defined (`sec:intro`, `fig:p`, `eq:e`,
        // `dup`), so the count is "4" — agreeing with the four lines S22 enumerates for the same doc.
        // The later `\label{dup}` is a duplicate (S20), not a fifth definition.
        assert_eq!(doc.label_definition_count(), "4");
        assert_eq!(
            doc.label_definition_count(),
            doc.label_definitions().lines().count().to_string()
        );
    }

    // ---------------------------------------------------------------------------------------------
    // LTXDOC03 S30 — single-integer TOTAL of the bibliography entries (`bibliography_entry_count`).
    // The CITATION-side twin of S29's `label_definition_count`: one decimal line = `.len()` of the
    // SAME winning `entries` list S19's `bibliography_entries` renders flat. It completes the totals
    // family — S27/S28 count the two reference tables, S29 counts the label definitions, S30 counts
    // the bibliography entries. Being a COUNT renderer, its empty value is the honest number "0", NOT
    // a `(no …)` marker. A later duplicate `\bibitem` is in `duplicate_entries` (S16), never
    // `entries`, so it is excluded — the count is exactly the number of lines S19 lists.
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn s30_multiple_entries_count_the_integer() {
        // Three distinct `\bibitem` keys → the count is exactly "3".
        let src = r"\begin{document}\begin{thebibliography}{9}
\bibitem{a} Author A.\bibitem{b} Author B.\bibitem{c} Author C.\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.bibliography_entry_count(), "3");
        // Cross-check: the count equals the number of lines S19 enumerates (two views of the SAME
        // winning `entries` list).
        assert_eq!(
            doc.bibliography_entry_count(),
            doc.bibliography_entries().lines().count().to_string()
        );
    }

    #[test]
    fn s30_one_entry_counts_one() {
        // A single `\bibitem` → "1".
        let src = r"\begin{document}\begin{thebibliography}{9}\bibitem{only} Solo.\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.bibliography_entry_count(), "1");
        assert_eq!(doc.bibliography_entries(), "[1] only");
    }

    #[test]
    fn s30_duplicate_bibitem_counted_once_matching_s19() {
        // A key `\bibitem{dup}` defined twice: only the FIRST (winning) entry is in `entries`; the
        // later one is a DUPLICATE (S16's domain), never a second `entries` row. So the count is the
        // number of DISTINCT keys, exactly matching the number of lines S19 lists.
        let src = r"\begin{document}\begin{thebibliography}{9}
\bibitem{a} Author A.\bibitem{dup} First.\bibitem{dup} Second.\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");
        // Two distinct keys: `a` and `dup`.
        assert_eq!(doc.bibliography_entry_count(), "2");
        // The later `\bibitem{dup}` is a duplicate, surfaced by S16, not counted here.
        assert_eq!(doc.duplicate_bibliography_entries(), "\\bibitem{dup}");
        // The count MUST equal the number of lines S19 lists (two views of the SAME list).
        assert_eq!(
            doc.bibliography_entry_count(),
            doc.bibliography_entries().lines().count().to_string()
        );
        assert_eq!(doc.bibliography_entries(), "[1] a\n[2] dup");
    }

    #[test]
    fn s30_no_entries_counts_zero() {
        // A document with NO `\bibitem` at all → "0" (the count of an empty `entries` list). A COUNT
        // renderer's honest value for an empty list is the number "0", never a `(no …)` marker.
        let src = r"\begin{document}Just some text with no bibliography.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.bibliography_entry_count(), "0");
        // And S19 (the list view) shows its fixed marker for the same doc — the divergence is intended.
        assert_eq!(doc.bibliography_entries(), "(no bibliography entries)");
    }

    #[test]
    fn s30_is_additive_leaves_s1_s29_outputs_unchanged() {
        // Same representative doc as the S29 additive test. S30 changes NONE of the S1-S29 outputs —
        // it only reads `resolve_citations`. Its count is a second view of the SAME `entries` list S19
        // renders flat; pinned here alongside S19/S22/S27/S28/S29 to show S30 neither adds, drops, nor
        // reorders, and that its total agrees with the number of lines S19 enumerates.
        let src = r"\begin{document}\section{Introduction}\label{sec:intro}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}

\begin{equation}\label{eq:e}E=mc^2\end{equation}

First \label{dup} here.

Second \label{dup} there.

See Section~\ref{sec:intro}, \eqref{eq:e}, \eqref{eq:ghost}, \nameref{sec:intro}, \nameref{fig:p}, and \cite{a,b} plus \cite{c,ghost}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\bibitem{b} Author B.
\bibitem{c} Author C.
\bibitem{a} Author A again.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");

        // A handful of prior renderers — byte-for-byte unchanged.
        // S19 flat winning bibliography entries (three distinct keys; the later `\bibitem{a}` loses).
        assert_eq!(doc.bibliography_entries(), "[1] a\n[2] b\n[3] c");
        // S22 flat winning label definitions.
        assert_eq!(
            doc.label_definitions(),
            "\\label{sec:intro}\n\\label{fig:p}\n\\label{eq:e}\n\\label{dup}"
        );
        // S27 dangling-reference count — exactly one (`\eqref{eq:ghost}`).
        assert_eq!(doc.unresolved_reference_count(), "1");
        // S28 resolved-reference count — exactly two (`\ref{sec:intro}`, `\eqref{eq:e}`).
        assert_eq!(doc.resolved_reference_count(), "2");
        // S29 label-definition count — exactly four distinct label keys.
        assert_eq!(doc.label_definition_count(), "4");

        // And S30 itself: exactly THREE distinct `\bibitem` keys are defined (`a`, `b`, `c`), so the
        // count is "3" — agreeing with the three lines S19 enumerates for the same doc. The later
        // `\bibitem{a}` is a duplicate (S16), not a fourth entry.
        assert_eq!(doc.bibliography_entry_count(), "3");
        assert_eq!(
            doc.bibliography_entry_count(),
            doc.bibliography_entries().lines().count().to_string()
        );
    }

    // LTXDOC03 S31 — single-integer TOTAL of the resolved citations (`citation_count`). The exact
    // resolved-CITATION-side twin of S28's `resolved_reference_count`: one decimal line = `.len()` of
    // the SAME resolved `\cite`-key list S15's `citations_by_source` renders grouped by source. It
    // extends the totals family onto the resolved-citation table — S27/S28 count the two reference
    // tables, S29 the label definitions, S30 the bibliography entries, S31 the resolved citations.
    // Being a COUNT renderer, its empty value is the honest number "0", NOT a `(no …)` marker. A
    // dangling `\cite{ghost}` is in `unresolved` (S17), never `resolved`, so it is excluded.
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn s31_multiple_resolved_citations_count_the_integer() {
        // `\cite{a,b}` then `\cite{c}` against a bibliography defining a, b, c → three keys resolve,
        // so the count is exactly "3".
        let src = r"\begin{document}See \cite{a,b} and \cite{c}.
\begin{thebibliography}{9}\bibitem{a} A.\bibitem{b} B.\bibitem{c} C.\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.citation_count(), "3");
    }

    #[test]
    fn s31_one_resolved_citation_counts_one() {
        // A single resolved `\cite{only}` → "1".
        let src = r"\begin{document}See \cite{only}.
\begin{thebibliography}{9}\bibitem{only} Solo.\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.citation_count(), "1");
    }

    #[test]
    fn s31_dangling_citation_excluded_from_count() {
        // `\cite{a, ghost}`: `a` is defined (resolves), `ghost` is not (dangles, S17's domain). Only
        // the ONE resolved key is counted; the dangling `ghost` is excluded by construction.
        let src = r"\begin{document}See \cite{a, ghost}.
\begin{thebibliography}{9}\bibitem{a} A.\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.citation_count(), "1");
    }

    #[test]
    fn s31_no_resolved_citations_counts_zero() {
        // A document with NO `\cite` at all → "0" (the count of an empty `resolved` list). A COUNT
        // renderer's honest value for an empty list is the number "0", never a `(no …)` marker.
        let src = r"\begin{document}Just some text with no citations.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.citation_count(), "0");
        // And S15 (the list view) shows its fixed marker for the same doc — the divergence is intended.
        assert_eq!(doc.citations_by_source(), "(no resolved citations)");
    }

    #[test]
    fn s31_every_citation_dangling_counts_zero() {
        // Every cited key dangles (no matching `\bibitem`) → all go to `unresolved` (S17), so the
        // resolved-citation count is "0", the same honest-zero as the "no `\cite` at all" case.
        let src = r"\begin{document}See \cite{ghost, phantom}.
\begin{thebibliography}{9}\bibitem{real} R.\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.citation_count(), "0");
    }

    #[test]
    fn s31_is_additive_leaves_s1_s30_outputs_unchanged() {
        // Same representative doc as the S30 additive test. S31 changes NONE of the S1-S30 outputs — it
        // only reads `resolve_citations`. Its count is a second view of the SAME `resolved` list S15
        // renders per-source; pinned here alongside S27/S28/S29/S30 to show S31 neither adds, drops, nor
        // reorders. The body cites `\cite{a,b}` and `\cite{c,ghost}` against a bibliography defining a,
        // b, c — so `a`, `b`, `c` resolve (three) and `ghost` dangles.
        let src = r"\begin{document}\section{Introduction}\label{sec:intro}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}

\begin{equation}\label{eq:e}E=mc^2\end{equation}

First \label{dup} here.

Second \label{dup} there.

See Section~\ref{sec:intro}, \eqref{eq:e}, \eqref{eq:ghost}, \nameref{sec:intro}, \nameref{fig:p}, and \cite{a,b} plus \cite{c,ghost}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\bibitem{b} Author B.
\bibitem{c} Author C.
\bibitem{a} Author A again.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");

        // Prior totals-family renderers — byte-for-byte unchanged.
        // S27 dangling-reference count — exactly one (`\eqref{eq:ghost}`).
        assert_eq!(doc.unresolved_reference_count(), "1");
        // S28 resolved-reference count — exactly two (`\ref{sec:intro}`, `\eqref{eq:e}`).
        assert_eq!(doc.resolved_reference_count(), "2");
        // S29 label-definition count — exactly four distinct label keys.
        assert_eq!(doc.label_definition_count(), "4");
        // S30 bibliography-entry count — exactly three distinct `\bibitem` keys (`a`, `b`, `c`).
        assert_eq!(doc.bibliography_entry_count(), "3");

        // And S31 itself: exactly THREE citation keys resolve (`a`, `b`, `c`); the one dangling `ghost`
        // is in `unresolved` (S17), not counted here.
        assert_eq!(doc.citation_count(), "3");
    }

    // LTXDOC03 S32 — single-integer TOTAL of the unresolved (dangling) citations
    // (`unresolved_citation_count`). The exact unresolved-CITATION-side twin of S27's
    // `unresolved_reference_count`, and the dangling sibling of S31's resolved `citation_count`: one
    // decimal line = `.len()` of the SAME dangling `\cite`-key list S17's `unresolved_citations_by_source`
    // renders grouped by source. S31 + S32 PARTITION every per-key `\cite` record (each key routes to
    // exactly one of `resolved`/`unresolved`). Being a COUNT renderer, its empty value is the honest
    // number "0", NOT a `(no …)` marker. A resolved `\cite{a}` is in `resolved` (S15/S31), never
    // `unresolved`, so it is excluded.
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn s32_multiple_dangling_citations_count_the_integer() {
        // `\cite{ghost, phantom}` then `\cite{spook}` against a bibliography defining only `real` →
        // three keys dangle, so the count is exactly "3".
        let src = r"\begin{document}See \cite{ghost, phantom} and \cite{spook}.
\begin{thebibliography}{9}\bibitem{real} R.\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.unresolved_citation_count(), "3");
    }

    #[test]
    fn s32_one_dangling_citation_counts_one() {
        // A single dangling `\cite{ghost}` (no matching `\bibitem`) → "1".
        let src = r"\begin{document}See \cite{ghost}.
\begin{thebibliography}{9}\bibitem{real} R.\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.unresolved_citation_count(), "1");
    }

    #[test]
    fn s32_resolved_citation_excluded_from_count() {
        // `\cite{a, ghost}`: `a` is defined (resolves, S15/S31's domain), `ghost` is not (dangles). Only
        // the ONE dangling key is counted; the resolved `a` is excluded by construction.
        let src = r"\begin{document}See \cite{a, ghost}.
\begin{thebibliography}{9}\bibitem{a} A.\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.unresolved_citation_count(), "1");
    }

    #[test]
    fn s32_no_dangling_citations_counts_zero() {
        // A document with NO `\cite` at all → "0" (the count of an empty `unresolved` list). A COUNT
        // renderer's honest value for an empty list is the number "0", never a `(no …)` marker.
        let src = r"\begin{document}Just some text with no citations.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.unresolved_citation_count(), "0");
        // And S17 (the list view) shows its fixed marker for the same doc — the divergence is intended.
        assert_eq!(doc.unresolved_citations_by_source(), "(no unresolved citations)");
    }

    #[test]
    fn s32_every_citation_resolved_counts_zero() {
        // Every cited key resolves (a matching `\bibitem` for each) → all go to `resolved` (S15/S31), so
        // the dangling-citation count is "0", the same honest-zero as the "no `\cite` at all" case.
        let src = r"\begin{document}See \cite{a, b}.
\begin{thebibliography}{9}\bibitem{a} A.\bibitem{b} B.\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.unresolved_citation_count(), "0");
    }

    #[test]
    fn s32_mixed_partitions_with_s31_over_the_cite_keys() {
        // `\cite{a,b}` (both defined) then `\cite{c,ghost}` (only `c` defined) → `a`,`b`,`c` resolve
        // (three) and `ghost` dangles (one). S32 counts the dangling ONE; and S31 + S32 partition the
        // four total keys — proving each `\cite` key routes to exactly one of `resolved`/`unresolved`.
        let src = r"\begin{document}See \cite{a,b} then \cite{c,ghost}.
\begin{thebibliography}{9}\bibitem{a} A.\bibitem{b} B.\bibitem{c} C.\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.unresolved_citation_count(), "1");
        // S31 still reports the three resolved keys — unchanged by S32.
        assert_eq!(doc.citation_count(), "3");
        // The two totals partition all four cited keys.
        let resolved: usize = doc.citation_count().parse().unwrap();
        let dangling: usize = doc.unresolved_citation_count().parse().unwrap();
        assert_eq!(resolved + dangling, 4);
    }

    #[test]
    fn s32_is_additive_leaves_s1_s31_outputs_unchanged() {
        // Same representative doc as the S31 additive test. S32 changes NONE of the S1-S31 outputs — it
        // only reads `resolve_citations`. Its count is a second view of the SAME `unresolved` list S17
        // renders per-source; pinned here alongside S27/S28/S29/S30/S31 to show S32 neither adds, drops,
        // nor reorders. The body cites `\cite{a,b}` and `\cite{c,ghost}` against a bibliography defining
        // a, b, c — so `a`, `b`, `c` resolve (three) and `ghost` dangles (one).
        let src = r"\begin{document}\section{Introduction}\label{sec:intro}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}

\begin{equation}\label{eq:e}E=mc^2\end{equation}

First \label{dup} here.

Second \label{dup} there.

See Section~\ref{sec:intro}, \eqref{eq:e}, \eqref{eq:ghost}, \nameref{sec:intro}, \nameref{fig:p}, and \cite{a,b} plus \cite{c,ghost}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\bibitem{b} Author B.
\bibitem{c} Author C.
\bibitem{a} Author A again.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");

        // A couple of representative earlier *list* renderers — byte-for-byte unchanged.
        // S19 flat winning bibliography entries (three distinct keys; the later `\bibitem{a}` loses).
        assert_eq!(doc.bibliography_entries(), "[1] a\n[2] b\n[3] c");
        // S22 flat winning label definitions (four distinct keys, first-definition-wins on `dup`).
        assert_eq!(
            doc.label_definitions(),
            "\\label{sec:intro}\n\\label{fig:p}\n\\label{eq:e}\n\\label{dup}"
        );

        // Prior totals-family renderers — byte-for-byte unchanged.
        // S27 dangling-reference count — exactly one (`\eqref{eq:ghost}`).
        assert_eq!(doc.unresolved_reference_count(), "1");
        // S28 resolved-reference count — exactly two (`\ref{sec:intro}`, `\eqref{eq:e}`).
        assert_eq!(doc.resolved_reference_count(), "2");
        // S29 label-definition count — exactly four distinct label keys.
        assert_eq!(doc.label_definition_count(), "4");
        // S30 bibliography-entry count — exactly three distinct `\bibitem` keys (`a`, `b`, `c`).
        assert_eq!(doc.bibliography_entry_count(), "3");
        // S31 resolved-citation count — exactly three keys resolve (`a`, `b`, `c`).
        assert_eq!(doc.citation_count(), "3");

        // And S32 itself: exactly ONE citation key dangles (`ghost`); the three resolved keys `a`, `b`,
        // `c` are in `resolved` (S15/S31), not counted here.
        assert_eq!(doc.unresolved_citation_count(), "1");
    }

    // LTXDOC03 S33 — single-integer TOTAL of the duplicate ("multiply defined") `\bibitem`s
    // (`duplicate_bibliography_count`). The warning-side companion of S30/S31/S32: one decimal line =
    // `.len()` of the SAME losing-duplicate `\bibitem` list S16's `duplicate_bibliography_entries`
    // renders as one `\bibitem{key}` warning line each. S30 counts the winning `entries` and S33 the
    // losing `duplicate_entries` — together they PARTITION every `\bibitem` inside a `thebibliography`
    // (each `\bibitem` routes to exactly one of `entries`/`duplicate_entries`). Being a COUNT renderer,
    // its empty value is the honest number "0", NOT a `(no …)` marker. The winning first `\bibitem` of a
    // key is in `entries` (S19/S30), never `duplicate_entries`, so it is excluded.
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn s33_multiple_duplicate_bibitems_count_the_integer() {
        // `\bibitem{a}` defined three times and `\bibitem{b}` twice → the 2nd and 3rd `a` lose (two)
        // and the 2nd `b` loses (one) → three losing duplicates, so the count is exactly "3".
        let src = r"\begin{document}\begin{thebibliography}{9}
\bibitem{a} First A.\bibitem{b} First B.\bibitem{a} Second A.\bibitem{a} Third A.\bibitem{b} Second B.\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.duplicate_bibliography_count(), "3");
    }

    #[test]
    fn s33_no_duplicates_all_distinct_counts_zero() {
        // Every `\bibitem` key is distinct → no duplicate wins → "0" (the count of an empty
        // `duplicate_entries` list). A COUNT renderer's honest value for an empty list is "0".
        let src = r"\begin{document}\begin{thebibliography}{9}\bibitem{a} A.\bibitem{b} B.\bibitem{c} C.\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.duplicate_bibliography_count(), "0");
        // And S16 (the list view) shows its fixed marker for the same doc — the divergence is intended.
        assert_eq!(
            doc.duplicate_bibliography_entries(),
            "(no duplicate bibliography entries)"
        );
    }

    #[test]
    fn s33_no_bibitems_at_all_counts_zero() {
        // A document with NO `\bibitem` at all → "0", the same honest-zero as the "all distinct" case.
        let src = r"\begin{document}Just some text with no bibliography.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.duplicate_bibliography_count(), "0");
    }

    #[test]
    fn s33_one_duplicate_counts_one() {
        // A single key `\bibitem{smith}` defined twice → only the SECOND loses → "1".
        let src = r"\begin{document}\begin{thebibliography}{9}
\bibitem{smith} First Smith.\bibitem{jones} Jones.\bibitem{smith} Second Smith.\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.duplicate_bibliography_count(), "1");
        // The count MUST equal the number of warning lines S16 lists (two views of the SAME list).
        assert_eq!(
            doc.duplicate_bibliography_count(),
            doc.duplicate_bibliography_entries().lines().count().to_string()
        );
        assert_eq!(doc.duplicate_bibliography_entries(), "\\bibitem{smith}");
    }

    #[test]
    fn s33_partitions_with_s30_over_the_bibitems() {
        // `a` twice, `b` once, `c` twice → winners are the three distinct keys (S30 = "3"); losers are
        // the 2nd `a` and 2nd `c` (S33 = "2"). Together they partition all FIVE `\bibitem`s — proving
        // each `\bibitem` routes to exactly one of `entries`/`duplicate_entries`. Also cross-checked
        // against the number of warning lines S16 emits.
        let src = r"\begin{document}\begin{thebibliography}{9}
\bibitem{a} A1.\bibitem{b} B.\bibitem{a} A2.\bibitem{c} C1.\bibitem{c} C2.\end{thebibliography}\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.duplicate_bibliography_count(), "2");
        // S30 winners — unchanged by S33.
        assert_eq!(doc.bibliography_entry_count(), "3");
        // S33 == number of S16 warning lines (two views of the SAME `duplicate_entries` list).
        assert_eq!(
            doc.duplicate_bibliography_count(),
            doc.duplicate_bibliography_entries().lines().count().to_string()
        );
        // Winners + losers partition all five `\bibitem`s.
        let winners: usize = doc.bibliography_entry_count().parse().unwrap();
        let losers: usize = doc.duplicate_bibliography_count().parse().unwrap();
        assert_eq!(winners + losers, 5);
    }

    #[test]
    fn s33_is_additive_leaves_s1_s32_outputs_unchanged() {
        // Same representative doc as the S32 additive test. S33 changes NONE of the S1-S32 outputs — it
        // only reads `resolve_citations`. Its count is a second view of the SAME `duplicate_entries` list
        // S16 renders per-source; pinned here alongside S16/S19/S22/S27/S28/S29/S30/S31/S32 to show S33
        // neither adds, drops, nor reorders. The bibliography defines a, b, c once each and `a` a second
        // time — so `a` has exactly ONE losing duplicate.
        let src = r"\begin{document}\section{Introduction}\label{sec:intro}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}

\begin{equation}\label{eq:e}E=mc^2\end{equation}

First \label{dup} here.

Second \label{dup} there.

See Section~\ref{sec:intro}, \eqref{eq:e}, \eqref{eq:ghost}, \nameref{sec:intro}, \nameref{fig:p}, and \cite{a,b} plus \cite{c,ghost}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\bibitem{b} Author B.
\bibitem{c} Author C.
\bibitem{a} Author A again.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");

        // A couple of representative earlier *list* renderers — byte-for-byte unchanged.
        // S16 duplicate `\bibitem` warnings (the later `\bibitem{a}` is the sole losing duplicate).
        assert_eq!(doc.duplicate_bibliography_entries(), "\\bibitem{a}");
        // S19 flat winning bibliography entries (three distinct keys; the later `\bibitem{a}` loses).
        assert_eq!(doc.bibliography_entries(), "[1] a\n[2] b\n[3] c");
        // S22 flat winning label definitions (four distinct keys, first-definition-wins on `dup`).
        assert_eq!(
            doc.label_definitions(),
            "\\label{sec:intro}\n\\label{fig:p}\n\\label{eq:e}\n\\label{dup}"
        );

        // Prior totals-family renderers — byte-for-byte unchanged.
        // S27 dangling-reference count — exactly one (`\eqref{eq:ghost}`).
        assert_eq!(doc.unresolved_reference_count(), "1");
        // S28 resolved-reference count — exactly two (`\ref{sec:intro}`, `\eqref{eq:e}`).
        assert_eq!(doc.resolved_reference_count(), "2");
        // S29 label-definition count — exactly four distinct label keys.
        assert_eq!(doc.label_definition_count(), "4");
        // S30 bibliography-entry count — exactly three distinct `\bibitem` keys (`a`, `b`, `c`).
        assert_eq!(doc.bibliography_entry_count(), "3");
        // S31 resolved-citation count — exactly three keys resolve (`a`, `b`, `c`).
        assert_eq!(doc.citation_count(), "3");
        // S32 unresolved-citation count — exactly one key dangles (`ghost`).
        assert_eq!(doc.unresolved_citation_count(), "1");

        // And S33 itself: exactly ONE `\bibitem` is a losing duplicate (the later `\bibitem{a}`); the
        // three winning first `\bibitem`s `a`, `b`, `c` are in `entries` (S19/S30), not counted here.
        assert_eq!(doc.duplicate_bibliography_count(), "1");
        // S30 winners + S33 losers partition all four `\bibitem`s in the bibliography.
        let winners: usize = doc.bibliography_entry_count().parse().unwrap();
        let losers: usize = doc.duplicate_bibliography_count().parse().unwrap();
        assert_eq!(winners + losers, 4);
    }

    // LTXDOC03 S34 — single-integer TOTAL of the duplicate ("multiply defined") `\label`s
    // (`duplicate_label_count`). The label-side twin of S33's `duplicate_bibliography_count`: one
    // decimal line = `.len()` of the SAME losing-duplicate `\label` list S20's
    // `duplicate_label_definitions` renders as one `\label{key}` warning line each. S29 counts the
    // winning `definitions` and S34 the losing `duplicates` — together they PARTITION every `\label`
    // (each `\label` routes to exactly one of `definitions`/`duplicates`). Being a COUNT renderer, its
    // empty value is the honest number "0", NOT a `(no …)` marker. The winning first `\label` of a key
    // is in `definitions` (S22/S29), never `duplicates`, so it is excluded.
    // ---------------------------------------------------------------------------------------------

    #[test]
    fn s34_multiple_duplicate_labels_count_the_integer() {
        // `\label{a}` defined three times and `\label{b}` twice → the 2nd and 3rd `a` lose (two) and
        // the 2nd `b` loses (one) → three losing duplicates, so the count is exactly "3".
        let src = r"\begin{document}A \label{a} one.

B \label{b} two.

A' \label{a} again.

A'' \label{a} thrice.

B' \label{b} again.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.duplicate_label_count(), "3");
    }

    #[test]
    fn s34_no_duplicates_all_distinct_counts_zero() {
        // Every `\label` key is distinct → no duplicate loses → "0" (the count of an empty `duplicates`
        // list). A COUNT renderer's honest value for an empty list is "0".
        let src = r"\begin{document}One \label{a} here, two \label{b} there, three \label{c} yonder.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.duplicate_label_count(), "0");
        // And S20 (the list view) shows its fixed marker for the same doc — the divergence is intended.
        assert_eq!(
            doc.duplicate_label_definitions(),
            "(no duplicate label definitions)"
        );
    }

    #[test]
    fn s34_no_labels_at_all_counts_zero() {
        // A document with NO `\label` at all → "0", the same honest-zero as the "all distinct" case.
        let src = r"\begin{document}Just some text with no labels.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.duplicate_label_count(), "0");
    }

    #[test]
    fn s34_one_duplicate_counts_one() {
        // A single key `\label{dup}` defined twice → only the SECOND loses → "1".
        let src = r"\begin{document}First \label{dup} here.

Second \label{dup} there.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.duplicate_label_count(), "1");
        // The count MUST equal the number of warning lines S20 lists (two views of the SAME list).
        assert_eq!(
            doc.duplicate_label_count(),
            doc.duplicate_label_definitions().lines().count().to_string()
        );
        assert_eq!(doc.duplicate_label_definitions(), "\\label{dup}");
    }

    #[test]
    fn s34_partitions_with_s29_over_the_labels() {
        // `a` twice, `b` once, `c` twice → winners are the three distinct keys (S29 = "3"); losers are
        // the 2nd `a` and 2nd `c` (S34 = "2"). Together they partition all FIVE `\label`s — proving each
        // `\label` routes to exactly one of `definitions`/`duplicates`. Also cross-checked against the
        // number of warning lines S20 emits.
        let src = r"\begin{document}A \label{a} one.

B \label{b} two.

A' \label{a} again.

C \label{c} three.

C' \label{c} again.\end{document}";
        let doc = parse_document(src).expect("parse");
        assert_eq!(doc.duplicate_label_count(), "2");
        // S29 winners — unchanged by S34.
        assert_eq!(doc.label_definition_count(), "3");
        // S34 == number of S20 warning lines (two views of the SAME `duplicates` list).
        assert_eq!(
            doc.duplicate_label_count(),
            doc.duplicate_label_definitions().lines().count().to_string()
        );
        // Winners + losers partition all five `\label`s.
        let winners: usize = doc.label_definition_count().parse().unwrap();
        let losers: usize = doc.duplicate_label_count().parse().unwrap();
        assert_eq!(winners + losers, 5);
    }

    #[test]
    fn s34_is_additive_leaves_s1_s33_outputs_unchanged() {
        // Same representative doc as the S33 additive test. S34 changes NONE of the S1-S33 outputs — it
        // only reads `resolve_references`. Its count is a second view of the SAME `duplicates` list S20
        // renders per-source; pinned here alongside S16/S19/S20/S22/S29/S30/S31/S32/S33 to show S34
        // neither adds, drops, nor reorders. The body defines `dup` twice (one losing duplicate) and the
        // bibliography defines `a` twice (one losing `\bibitem`).
        let src = r"\begin{document}\section{Introduction}\label{sec:intro}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}

\begin{equation}\label{eq:e}E=mc^2\end{equation}

First \label{dup} here.

Second \label{dup} there.

See Section~\ref{sec:intro}, \eqref{eq:e}, \eqref{eq:ghost}, \nameref{sec:intro}, \nameref{fig:p}, and \cite{a,b} plus \cite{c,ghost}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\bibitem{b} Author B.
\bibitem{c} Author C.
\bibitem{a} Author A again.
\end{thebibliography}
\end{document}";
        let doc = parse_document(src).expect("parse");

        // A couple of representative earlier *list* renderers — byte-for-byte unchanged.
        // S20 duplicate `\label` warnings (the later `\label{dup}` is the sole losing duplicate).
        assert_eq!(doc.duplicate_label_definitions(), "\\label{dup}");
        // S22 flat winning label definitions (four distinct keys, first-definition-wins on `dup`).
        assert_eq!(
            doc.label_definitions(),
            "\\label{sec:intro}\n\\label{fig:p}\n\\label{eq:e}\n\\label{dup}"
        );
        // S19 flat winning bibliography entries (three distinct keys; the later `\bibitem{a}` loses).
        assert_eq!(doc.bibliography_entries(), "[1] a\n[2] b\n[3] c");

        // Prior totals-family renderers — byte-for-byte unchanged.
        // S27 dangling-reference count — exactly one (`\eqref{eq:ghost}`).
        assert_eq!(doc.unresolved_reference_count(), "1");
        // S28 resolved-reference count — exactly two (`\ref{sec:intro}`, `\eqref{eq:e}`).
        assert_eq!(doc.resolved_reference_count(), "2");
        // S29 label-definition count — exactly four distinct label keys.
        assert_eq!(doc.label_definition_count(), "4");
        // S30 bibliography-entry count — exactly three distinct `\bibitem` keys (`a`, `b`, `c`).
        assert_eq!(doc.bibliography_entry_count(), "3");
        // S31 resolved-citation count — exactly three keys resolve (`a`, `b`, `c`).
        assert_eq!(doc.citation_count(), "3");
        // S32 unresolved-citation count — exactly one key dangles (`ghost`).
        assert_eq!(doc.unresolved_citation_count(), "1");
        // S33 duplicate-bibliography count — exactly one losing `\bibitem` (the later `\bibitem{a}`).
        assert_eq!(doc.duplicate_bibliography_count(), "1");

        // And S34 itself: exactly ONE `\label` is a losing duplicate (the later `\label{dup}`); the
        // four winning first `\label`s are in `definitions` (S22/S29), not counted here.
        assert_eq!(doc.duplicate_label_count(), "1");
        // S29 winners + S34 losers partition all five `\label`s in the document.
        let winners: usize = doc.label_definition_count().parse().unwrap();
        let losers: usize = doc.duplicate_label_count().parse().unwrap();
        assert_eq!(winners + losers, 5);
    }
}
