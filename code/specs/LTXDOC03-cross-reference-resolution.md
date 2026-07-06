# LTXDOC03 — cross-reference resolution (label table + `\ref` binding; `\cite` → bibliography)

## 1. Motivation

LTXDOC01 built the LaTeX → `Document` AST end-to-end (D1–D6), and LTXDOC02 made every body node's
byte `Span` **precise** — `node_at(byte)` resolves to the true per-token leaf, and `&src[node.span]`
slices back to exactly that node's source. That precise-span surface is a *substrate*: it says
"here is a `\ref{sec:intro}` at bytes `[a, b)`" and "here is a `\section`…`\label{sec:intro}` at
bytes `[c, d)`", but it does **not** connect the two. A consumer that wants to answer "what does this
`\ref` point at?" still has to build the correspondence itself.

LTXDOC03 is the first **document-feature** arc on top of that substrate — features a real LaTeX
consumer needs, not just structure/provenance plumbing. **S1** is cross-reference **resolution**: a
pure analysis pass over a parsed `Document` that binds each cross-reference to the `\label` that
defines it, with byte spans on **both** sides. It is the piece that turns "two independent spans" into
"a resolved edge from the reference's bytes to the defining node's bytes" — exactly the source→source
correlation the ADJ byte-provenance north star audits against. **S2** (§8) extends the same binding
to the *other* cross-reference family — `\cite`↔`\bibitem` — resolving citations against the
in-document bibliography.

## 2. The LaTeX `.aux` two-pass model this mimics

LaTeX resolves cross-references in **two passes** through an auxiliary `.aux` file:

- **First `latex` run.** Every `\label{key}` writes a `\newlabel{key}{…}` line into `document.aux`
  (recording the number and page the label points at). Every `\ref{key}`/`\pageref{key}` is left as a
  placeholder — on the first run the label table is not yet available.
- **Second run.** LaTeX reads `document.aux` back **before** typesetting, so by the time it meets a
  `\ref{key}` the label table is already populated and the reference can be filled in.

The two diagnostic outcomes LTXDOC03 S1 mirrors:

- a key that never got a `\newlabel` prints `??` and warns *"Reference `key' … undefined"* → an
  **unresolved** (dangling) reference;
- a key that got two `\newlabel`s warns *"Label `key' multiply defined"* → a **duplicate** definition.

LTXDOC03 S1 is the **static, single-pass analogue** over an already-parsed `Document`. We do **not**
run LaTeX and we do **not** compute numbers/pages — we bind *structure*: for each reference we answer
"**which defining node**, at **which source bytes**, does this reference point at?". The `.aux`
`\newlabel{key}{…}` line becomes a `LabelDef { key, kind, span }` row recording the defining node's
**source bytes** rather than its number/page.

## 3. Invariants (hard gates)

1. **Totality / no panic.** `resolve_references` is infallible: no `unwrap`/`expect`, no unchecked
   indexing, all data is plain owned/`Copy` values.
2. **Additive & non-destructive.** The pass is pure analysis — it mutates nothing. The parser, the D2
   fold, `Document::walk`, `Document::node_at`, and **every existing span** are untouched.
3. **Round-trip fixed point preserved.** Because nothing is mutated, `to_latex` and the
   round-trip-modulo-spans fixed point are exactly as before.
4. **No new recursion.** The pass reuses the existing bounded `Document::walk` traversal (whose depth
   is capped upstream by the parser's `MAX_DEPTH`); it introduces no new unbounded recursion.
5. **No `unsafe`.** Unchanged from LTXDOC01/02.

## 4. Scope

- **In (S1):** a label table collected from both definition sources (hoisted section/table/figure
  labels and inline `\label{…}`); duplicate-definition detection with **first-def-wins**; resolution
  of the **reference family** `{ref, eqref, pageref}` against the table (resolved → ref-span + def-span
  + kind; unresolved → dangling key + ref-span); a public `Document::resolve_references` returning
  `Clone`-able plain data.
- **In (S2 — see §8):** a bibliography table collected from every `\bibitem{key}` inside a
  `thebibliography` environment; duplicate-entry detection with **first-entry-wins**; resolution of
  the **citation family** `{cite}` against the table, splitting a multi-key `\cite{a,b,c}` into
  per-key bindings; a public `Document::resolve_citations` returning `Clone`-able plain data.
- **Out (deferred to later rungs):**
  - **`\cite` was deferred in S1; it is now bound in S2 (§8).** The bullet remains only to note that
    the *S1* label pass still treats `\cite` as neither a resolvable reference nor a dangling one —
    citations are the *separate* S2 table.
  - **Numbers/pages / citation numbering.** We bind *structure* (the defining node + its bytes), not
    the rendered "3.2" / page number / citation number a full `.aux` + BibTeX pass would compute.
  - **External BibTeX databases.** Only an **in-document** `thebibliography`/`\bibitem` bibliography
    is bound (S2). A `.bib` file read via `\bibliography{…}`, or an un-`\input`-ed `.bbl`, is out of
    scope — no file I/O, no BibTeX parse.
  - **Forward-reference ordering nuance, `\nameref`, `cleveref`'s `\cref`, natbib
    `\citep`/`\citet`, hyperref anchors.** All later slices; S1 is the core `\ref`↔`\label` binding
    and S2 the core `\cite`↔`\bibitem` binding.

## 5. Public API

A method on `Document`, plus the plain record types it returns (all `Clone`, `PartialEq`, `Eq`; spans
are `Copy`, keys are owned `String`s copied out of the document so the resolution outlives any borrow):

```rust
impl Document {
    pub fn resolve_references(&self) -> ReferenceResolution;
}

pub const REF_COMMANDS: [&str; 3] = ["ref", "eqref", "pageref"];

pub enum LabelKind { Section, Table, Figure, Inline }   // + LabelKind::as_str()

pub struct LabelDef      { pub key: String, pub kind: LabelKind, pub span: Span }
pub struct Duplicate     { pub key: String, pub kind: LabelKind, pub span: Span }
pub struct ResolvedRef   { pub key: String, pub command: String,
                           pub ref_span: Span, pub target_span: Span, pub target_kind: LabelKind }
pub struct UnresolvedRef { pub key: String, pub command: String, pub ref_span: Span }

pub struct ReferenceResolution {
    pub definitions: Vec<LabelDef>,     // winning (first) defs, pre-order — the label table
    pub duplicates:  Vec<Duplicate>,    // later (losing) defs of already-defined keys
    pub resolved:    Vec<ResolvedRef>,  // refs that found a definition
    pub unresolved:  Vec<UnresolvedRef>,// dangling refs
}                                       // + ReferenceResolution::definition(key) -> Option<&LabelDef>
```

**Definition sources** (both collected in `Document::walk` pre-order):

1. **Hoisted labels** — LTXDOC01's D3/D5 folds hoist a trailing `\label{key}` off a
   `\section`/`table`/`figure` into that `Block`'s `label: Option<String>`. When `Some`, the defining
   node is that block; its span is the block's span; the kind is `Section`/`Table`/`Figure`.
2. **Inline `\label{key}`** — any `\label` not hoisted survives as an `Inline::CrossRef` with
   `command == "label"`; the key is its `target`, the span its cross-ref span, the kind `Inline`.

**First-def-wins.** If a key is defined more than once, the **first** definition (in walk pre-order)
is the winner references resolve against; every later definition is recorded as a `Duplicate`.

**Reference family only.** Only `REF_COMMANDS = {ref, eqref, pageref}` resolve. `\label` *defines*
(it is a definition source, never a reference); `\cite` is deferred (§4). Both are excluded from the
resolved/unresolved tables.

## 6. Verification

`cargo test -p latex` green (10 new `references` tests); `cargo clippy -p latex --all-targets
-- -D warnings` clean; downstream `cargo test -p adj-lang -p adj-lang-cli` green; `cargo build
-p latex --no-default-features` builds. No `cargo fmt`, no grammar regen. The load-bearing test is
that a **resolved target span slices back to the exact expected source substring** (`&src[def_span]`
starts with the defining `\section{…}` / covers the whole `\begin{figure}…\end{figure}` / equals the
inline `\label{…}`), and the ref-span slices back to exactly the `\ref{…}` construct — not merely
`.is_some()`. The suite also pins: figure/table/section/inline kinds, `\eqref` and `\pageref`
resolving, a dangling `\ref` recorded with the correct ref-span, first-def-wins under a duplicate,
`\cite` excluded from both ref tables, and an empty/label-free document yielding empty results without
panicking.

## 7. Payoff

A resolved `\ref` now points at the **exact defining node's source bytes**. Combined with LTXDOC02's
precise spans, the LaTeX → `Document` surface can answer, for any reference, "this `\ref{sec:intro}` at
bytes `[a, b)` binds to the `\section` at bytes `[c, d)`" — a byte-faithful, auditable edge from the
citing text to the cited structure. That is precisely the source→source correlation the ADJ
byte-provenance pipeline (the north-star consumer) audits against: a claim that quotes "as shown in
Section 3" can be traced, through the resolved reference, back to the specific section it names — not
by string-matching, but by the document's own cross-reference graph. `\cite`/bibliography binding
(S2, §8) extends the same guarantee to citations.

## 8. S2 — `\cite` → bibliography binding

S2 is the **parallel pass** to S1 for the *other* cross-reference family. LaTeX keeps two
independent cross-reference tables: the `\label`/`\ref` table (S1, the `.aux` `\newlabel`/`\ref`
machinery) and the **bibliography** — `\bibitem{key}` entries inside a `thebibliography` environment,
cited by `\cite{key}`. Where S1 binds `\ref`↔`\label`, S2 binds `\cite`↔`\bibitem`, with byte spans
on **both** sides, and is the static single-pass analogue of LaTeX's `\bibcite` `.aux` dance (it
binds *structure*, never a citation number/sort order — that is BibTeX/`.bbl` territory).

### 8.1 Scope

- **In (S2):** a bibliography table collected from every `\bibitem{key}` inside a `thebibliography`
  environment (span = the `\bibitem{key}` construct's own tight bytes); duplicate-entry detection
  with **first-entry-wins**; resolution of the **citation family** `{cite}` against the table, with a
  multi-key `\cite{a,b,c}` split on commas into per-key bindings (each carrying that one `\cite`'s
  span); a public `Document::resolve_citations` returning `Clone`-able plain data.
- **Out (S2):** external `.bib`/BibTeX databases and un-`\input`-ed `.bbl` files (no file I/O, no
  BibTeX parse — in-document `thebibliography`/`\bibitem` only); citation **numbering**/sorting;
  natbib `\citep`/`\citet`/`\citeauthor` (only plain `\cite` is folded to `Inline::CrossRef` today).

### 8.2 How the constructs surface (confirmed against the parsed AST)

- `\begin{thebibliography}{9}…\end{thebibliography}` → `Block::Environment { name:
  "thebibliography", body, span }`.
- `\bibitem{key}` → an `Inline::Raw(Node { kind: Command { name: "bibitem", arguments: [[key]] } },
  span)` — a *generic* command (`recognize_structure` does not fold `\bibitem`), **not** an
  `Inline::CrossRef`. The key is `arguments[0]` rendered back to source; the span covers exactly
  `\bibitem{key}`. The trailing author/title/year prose parses as *separate* sibling `Text`/`Space`
  inlines with no delimiter marking entry boundaries, so the entry span is the `\bibitem{key}`
  construct only (the honest, tightest attributable range).
- `\cite{a,b,c}` → **one** `Inline::CrossRef { command: "cite", target: "a,b,c" }`; the multi-key
  list is a single comma-joined `target` string. `\cite[p. 3]{key}` keeps the `[p. 3]` in the
  cross-ref's separate `note` field with `target == "key"`.

### 8.3 Public API

A method on `Document`, plus the plain record types it returns (all `Clone`, `PartialEq`, `Eq`; spans
are `Copy`, keys are owned `String`s; the aggregate derives `Default`):

```rust
impl Document {
    pub fn resolve_citations(&self) -> CitationResolution;
}

pub const CITE_COMMAND: &str = "cite";

pub struct BibEntry       { pub key: String, pub span: Span }              // \bibitem{key}, span = its bytes
pub struct DuplicateBib   { pub key: String, pub span: Span }              // later, losing \bibitem
pub struct ResolvedCite   { pub key: String, pub cite_span: Span, pub entry_span: Span }
pub struct UnresolvedCite { pub key: String, pub cite_span: Span }

pub struct CitationResolution {
    pub entries:           Vec<BibEntry>,        // winning (first) \bibitems, pre-order — the table
    pub duplicate_entries: Vec<DuplicateBib>,    // later (losing) \bibitems of already-defined keys
    pub resolved:          Vec<ResolvedCite>,    // per-key bindings that found an entry
    pub unresolved:        Vec<UnresolvedCite>,  // per-key dangling citations
}                                                // + CitationResolution::entry(key) -> Option<&BibEntry>
```

**Multi-key, shared cite-span.** One `\cite{a,b,c}` yields one `ResolvedCite`/`UnresolvedCite`
**per key**, all sharing that `\cite`'s `cite_span`, so a caller sees per-key resolution *and* which
source `\cite` each key came from (group by `cite_span`). Empty keys (from `\cite{a,,b}` / a trailing
comma / `\cite{}`) are skipped.

**First-entry-wins.** If a key is defined by more than one `\bibitem`, the **first** (in walk
pre-order) is the winner citations resolve against; every later one is a `DuplicateBib`.

**Citation family only.** Only `\cite` (`CITE_COMMAND`) resolves against the bibliography;
`\ref`/`\eqref`/`\pageref`/`\label` are the S1 label family and are excluded. The two passes read
disjoint command families and produce disjoint result types, so they never interfere.

### 8.4 Verification (S2)

`cargo test -p latex` green (10 new `references` S2 tests); `cargo clippy -p latex --all-targets
-- -D warnings` clean; downstream `cargo test -p adj-lang -p adj-lang-cli` green; `cargo build
-p latex --no-default-features` builds. No `cargo fmt`, no grammar regen. The load-bearing test is
that a **resolved entry span slices back to exactly `\bibitem{key}`** and the **cite span slices back
to exactly the `\cite{…}` construct** — not merely `.is_some()`. The suite also pins: a multi-key
`\cite{a,b}` → two bindings sharing the `\cite` span with distinct entry spans, a mixed
`\cite{known,unknown}` → one resolved + one unresolved from the same span, `\cite[p. 3]{key}`
resolving with the note not conflated into the key, a dangling `\cite{ghost}` recorded, first-entry-
wins under a duplicate `\bibitem`, a `\cite` with no bibliography reported unresolved without
panicking, an empty document yielding empty results, and a regression that S1's `resolve_references`
and S2's `resolve_citations` coexist on a document with **both** families without disturbing each
other.
