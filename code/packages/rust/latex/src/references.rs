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
}
