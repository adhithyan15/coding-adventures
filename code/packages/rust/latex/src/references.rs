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
//! - **`\cite` is deferred (out of scope for S1).** A citation resolves against a **bibliography**
//!   (`.bib` / `thebibliography`), an entirely separate table from the `\label` one, so binding it
//!   is a later rung. The ref pass therefore treats `\cite` as **neither** a resolvable reference
//!   **nor** a dangling one — it is simply not a `\label`-table reference. (`\label` is likewise not
//!   a *reference*: it *defines*, it does not *use*.)
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

use crate::document::{Block, Document, Inline, NodeRef};
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
}
