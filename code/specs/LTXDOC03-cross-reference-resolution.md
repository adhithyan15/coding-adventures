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

## 9. S3 — target → `NodeRef` exposure

S1 and S2 each bind a cross-reference to the **bytes** of its target: a `ResolvedRef` carries
`target_span: Span`, a `ResolvedCite` carries `entry_span: Span`, a `LabelDef`/`BibEntry` carries
`span: Span`. A `Span` is a source-byte range you can *slice* (`&src[span]`) — but it is not the
target **node**, so a consumer cannot ask "what *kind* of node is this?" or "walk into the section's
paragraphs / the figure's caption". S3 is the natural depth-add on S1+S2: given a resolved target's
span, hand back the actual walked `NodeRef`, so the caller can read its `kind()` and — for a
`NodeRef::Block` — descend into its children. No new parsing, no numbering, no I/O — pure, additive
analysis over the existing `Document::walk`.

### 9.1 Scope

- **In (S3):** a span→node lookup primitive `Document::node_for_span(span) -> Option<NodeRef>`, and
  three thin accessors that take a resolved S1/S2 record and return its node —
  `ref_target_node(&ResolvedRef)`, `cite_target_node(&ResolvedCite)`, `label_def_node(&LabelDef)`.
- **Out (S3):** everything S1/S2 already excluded — citation/reference **numbering**, **external
  BibTeX**, natbib families, hyperref anchors — remains deferred to later rungs. S3 changes no parser,
  fold, `walk`, `node_at`, or span logic, and does **not** alter the S1/S2 result types.

### 9.2 The lookup: span-equality against the body walk

`node_for_span` walks the body (`Document::walk`, pre-order) and returns the node whose
`span()` **exactly equals** the requested span — half-open equality of *both* `start` and `end`. This
is deliberately **equality**, not **containment**: a resolved S1/S2 target span is, by construction,
some walked node's own span (S1 recorded a block's or a cross-ref's span; S2 recorded a `\bibitem`'s
`Inline::Raw` span), so the node we want is the one that *is* that span, not merely one that
*encloses* it. Containment — "which leaf owns this arbitrary byte?" — is `Document::node_at`'s job
(S4), a different question. The lookup is O(nodes): one reuse of the bounded `walk`, no new recursion.

**Tie-break (defensive).** If two walked nodes ever shared an identical span, `node_for_span` returns
the **first in pre-order** — the outermost/earliest, since `walk` yields a parent before its children
and an earlier sibling before a later one. The exploratory parse (below) found **zero** such
collisions among real targets, so this rule is documented for determinism, not because callers reach
it.

### 9.3 Reachability (confirmed against the parsed AST, not assumed)

An exploratory parse over a document exercising every target family confirmed that **every** S1/S2
target span corresponds to **exactly one** walked node — there were zero pairs of distinct walked
nodes sharing an identical span:

| target | walked node it resolves to | reachable? |
|--------|----------------------------|------------|
| `\ref`→`\section` (hoisted label) | the `Block::Section` | **yes** — `NodeRef::Block`, `kind()=="Section"` |
| `\ref`→`figure` float | the `Block::Figure` | **yes** — `kind()=="Figure"` |
| `\ref`→`table` float | the `Block::Table` | **yes** — `kind()=="Table"` |
| `\eqref`→ inline `\label` | the `Inline::CrossRef` | **yes** — `NodeRef::Inline`, `kind()=="CrossRef"` |
| `\cite`→`\bibitem` | the `Inline::Raw` `\bibitem` command | **yes** — `kind()=="Raw"` (see below) |

**The `\bibitem` target *is* walked** — the one genuinely uncertain case, so it was *verified*, not
assumed. A `\bibitem{key}` sits inside a `thebibliography` `Block::Environment`, whose body `walk`
descends into; the `\bibitem` survives D2 as an `Inline::Raw` command inside a `Block::Paragraph`,
which `walk` visits. Its span therefore matches a walked node and `cite_target_node` returns
`Some(NodeRef::Inline(..))` (`kind()=="Raw"`), **not** `None`.

**When `None` is returned.** `node_for_span` returns `None` for any span that is *not* some walked
body node's own span: a span from an empty document, a preamble/metadata span (those regions are
classified out of directives and deliberately **not** walked — see `Document::walk`), or any
caller-fabricated span landing between nodes. This is a **documented, total** outcome — never a panic.
Because every *resolved* S1/S2 target span is a walked node's span, the accessors never return `None`
for a genuinely resolved reference/citation in a well-formed document; `None` is reserved for the
honest edge.

### 9.4 Public API

```rust
impl Document {
    /// The walked body node whose span EXACTLY equals `span`, else `None`. First-in-pre-order on tie.
    pub fn node_for_span(&self, span: Span) -> Option<NodeRef<'_>>;
    /// The target node of a resolved reference (= `node_for_span(r.target_span)`).
    pub fn ref_target_node(&self, r: &ResolvedRef) -> Option<NodeRef<'_>>;
    /// The `\bibitem` node of a resolved citation (= `node_for_span(c.entry_span)`).
    pub fn cite_target_node(&self, c: &ResolvedCite) -> Option<NodeRef<'_>>;
    /// The defining node of a label definition (= `node_for_span(d.span)`).
    pub fn label_def_node(&self, d: &LabelDef) -> Option<NodeRef<'_>>;
}
```

**Additivity & borrowing.** The S1/S2 result types (`ResolvedRef`, `ResolvedCite`, `LabelDef`,
`BibEntry`, …) are **unchanged** — they still carry only owned `Span`s (no lifetimes), so a resolution
still outlives any borrow of the source. A `NodeRef` borrows the doc, so it cannot live on those owned
types; instead it is fetched **on demand** through these `Document` methods, keeping S3 purely
additive and lifetime-clean.

### 9.5 Verification (S3)

`cargo test -p latex` green (9 new `references` S3 tests); `cargo clippy -p latex --all-targets
-- -D warnings` clean; downstream `cargo test -p adj-lang -p adj-lang-cli` green; `cargo build
-p latex --no-default-features` builds. No `cargo fmt`, no grammar regen. The load-bearing tests
assert the **actual node**, not just `.is_some()`: a `\ref`→section returns the real `Block::Section`
and we descend into its title inlines + owned paragraph; a `\ref`→figure reaches its caption text; an
inline `\eqref` returns the `CrossRef` inline whose span slices back to `\label{eq:x}`; a
`\cite`→`\bibitem` returns the walked `Inline::Raw` whose span slices back to exactly
`\bibitem{key}`. The suite also pins: `label_def_node` returns the defining node, a non-matching span
→ `None` (no panic), `node_for_span(target_span)` agrees with `ref_target_node`, empty-document
lookups → `None`, and a regression that exercising the S3 accessors leaves S1's `resolve_references`
and S2's `resolve_citations` outputs byte-for-byte unchanged.

## 10. S4 — document numbering (hierarchical sections + flat float counters)

S1 bound each `\ref` to its target's **bytes**; S3 lifted that to the target **node**. Neither gives
the rendered **number** a `\ref` prints — the "1.2" in "see Section~1.2", the "3" in "Figure~3". S4
assigns those numbers: it is the static, single-pass analogue of LaTeX's **second `.aux` pass**. On
the first `latex` run each `\refstepcounter` (fired by every numbered `\section`/`figure`/`table`/…)
writes the counter value into `document.aux` next to the label; on the second run `\ref{key}` reads it
back. S4 does the same binding **in one walk** over the already-parsed `Document` — no `.aux` file, no
second parse — computing each numbered target's value directly. S3's target-node exposure is what
unblocks it: the counters are assigned by walking exactly the nodes S3 made addressable.

### 10.1 The two counter models

1. **Hierarchical section counters (deeper-reset).** `\part` … `\subparagraph` share a nested counter
   family: incrementing a coarser counter **resets every finer one to 0**. The printed number is the
   dotted join of the counters from the top level down to that heading's depth: `1`, `1.1`, `1.2`,
   `1.2.1`, `2`. `rank(level)` (0 = `\part` … 6 = `\subparagraph`, mirroring the D3 sectioning-fold
   rank) is the array index a heading increments and the depth its number is joined to.
2. **Flat float counters (no reset, no hierarchy).** `figure` and `table` each own an *independent*
   running counter that only increments: figures `1, 2, 3, …`, tables their **own** `1, 2, 3, …` (a
   table after two figures is `1`, not `3`).

**Every float consumes a counter — labeled or not.** LaTeX advances a float's counter every time the
float appears; a `\label` merely *captures* the current value. So an unlabeled figure between two
labeled ones still takes a number: the labeled ones read `1` and `3`, the unlabeled one having taken
`2`. S4 walks **every** `Block::Figure`/`Block::Table` and advances its counter, exposing the value
only for the labeled ones. **Starred `\section*`** (`numbered == false`) is the opposite: it fires no
counter and is **skipped**, so the following numbered section keeps the number it would have had.

### 10.2 The missing-parent rule (a document that starts deep)

A well-formed document opens with a top-level heading, but nothing stops an author writing a
`\subsection` before any `\section`. An exploratory parse confirmed such a document parses fine (a
lone `Block::Section { level: Subsection, numbered: true, .. }`), and its parent `\section` counter
sits at its initial **0**. **S4's rule: treat a missing parent as 0** — we render from the `\section`
depth (rank 2, the article default top-numbered level) down to the heading, surfacing the un-opened
parent slots' honest `0`. So a lone leading `\subsection` numbers **`0.1`**, a lone `\subsubsection`
**`0.0.1`**; a plain top-level `\section` is just **`1`** (it *is* the reference depth, no parent to
zero-fill). This is total (no panic), deterministic, and honestly reflects "no parent section opened
yet" rather than silently inventing a `1` the source never wrote — the faithful, auditable choice for
a byte-provenance model. (It is a documented convention, not a LaTeX fact: the point is that S4 picks
a rule and sticks to it on degenerate input rather than crashing.)

### 10.3 Public API

```rust
impl Document {
    /// One `NumberedLabel { key, kind, number }` per DEFINED numberable label (section/figure/table),
    /// in pre-order. Inline/equation labels carry no S4 counter and are omitted (deferred to S5).
    pub fn number_labels(&self) -> Numbering;
    /// The number a resolved `\ref` prints — `number_labels().number_for(r.key)`; ties S1 → S4.
    pub fn ref_number(&self, r: &ResolvedRef) -> Option<String>;
}

pub struct Numbering { pub labels: Vec<NumberedLabel> }
impl Numbering { pub fn number_for(&self, key: &str) -> Option<&str>; }

pub struct NumberedLabel { pub key: String, pub kind: LabelKind, pub number: String }
```

`Numbering`/`NumberedLabel` are dedicated, owned-`String` result types mirroring S1's
`ReferenceResolution` and S2's `CitationResolution`, so a numbering outlives any borrow of the source.
`ref_number` is the **payoff**: `\ref{sec:intro}` → `"1.2"`, closing the loop from S1 resolution to
S4 numbering. Only the **first** definition of a duplicated key is numbered (matching S1's
first-definition-wins).

### 10.4 What is DEFERRED (honest boundary, mirroring S1/S2/S3)

- **Equation numbers** — `equation`/`align` bodies are opaque `Block::DisplayMath` source strings
  (never parsed here); numbering needs an equation counter threaded through the math island. S5.
- **Citation `[1]` numbers** — the order-of-first-appearance number a numeric bibliography style
  prints; a citation-order traversal over S2's resolution. A separate rung.
- **Other `\label`-able counters** — `enumerate` item numbers, theorem/footnote counters — each needs
  a per-environment counter context. S5+.

### 10.5 Verification (S4)

`cargo test -p latex` green (12 new `references` S4 tests); `cargo clippy -p latex --all-targets
-- -D warnings` clean; downstream `cargo test -p adj-lang -p adj-lang-cli` green; `cargo build
-p latex --no-default-features` builds. No `cargo fmt`, no grammar regen. The tests assert the
**actual number string**: nested `\section`/`\subsection`/`\subsection`/`\section` → `1`/`1.1`/`1.2`/
`2` (deeper-reset); a `\section*` between two numbered sections leaves the second `2` not `3`; two
labeled figures → `1`/`2` and a labeled table → `1` (independent counter); an unlabeled figure between
two labeled ones makes the second labeled one `3` (every float advances); `ref_number` for
`\ref{s:b}` (a subsection under a section) → `"1.1"` (the load-bearing S1→S4 payoff); an undefined key
→ `None` (no panic); an empty document → empty numbering; a lone leading `\subsection` → `0.1` (the
missing-parent rule); a plain top-level `\section` → `1`; an inline `\label` is **not** numbered
(deferred); and a regression that numbering leaves the S1/S2/S3 outputs byte-for-byte unchanged (the
tree is never mutated).

## 11. S5 — citation numbering (bracketed bibliography numbers)

S4 numbered **sections and floats** — the counters a `\ref` most commonly prints — but explicitly
left **citations** unnumbered. S5 fills that gap: it assigns the bracketed number LaTeX prints for a
`\cite` (the `[2]` in "as shown in [2]") over the bibliography **S2 already resolved**. It is the
citation-family analogue of S4's `ref_number`: where S4 tied S1 resolution to a section/float number,
S5 ties S2 resolution to a bibliography number.

**Why citations, not equations, for this rung.** Equation numbering was the other candidate, but the
AST does not cleanly model it: an equation body is kept as an opaque `Block::DisplayMath { source:
String, span }` — a **raw unparsed string** with **no** `label` field, so an equation's `\label` is
buried inside `source`, not a resolvable label def. Per-equation numbering would need fuzzy string
heuristics. Citation numbering, by contrast, is well-defined and fully testable over S2's clean owned
data, so S5 does citations; equation numbering stays a documented future rung (§11.4).

### 11.1 The LaTeX citation-numbering model

In the default **numeric, unsorted** bibliography style (`plain`-family, a hand-written
`thebibliography`, or `unsrt`), every `\bibitem` is numbered by its **position in the list**: the
first `\bibitem` is `[1]`, the second `[2]`, …. On the first `latex` run each `\bibitem{key}` writes a
`\bibcite{key}{n}` line into `document.aux`; on the second run each `\cite{key}` reads `n` back and
prints it **in square brackets**. A multi-key `\cite{a,c}` prints both numbers (`[1, 3]`); a `\cite`
whose key has no `\bibitem` prints the tell-tale `[?]` and warns *"Citation `key' undefined"*.

S5 is the static, single-pass, in-document analogue: a flat counter over S2's **already-ordered**
winning entry list. No `.aux`, no second parse — S5 numbers `CitationResolution::entries` by their
index. An exploratory parse confirmed `entries` is in `\bibitem` **listing order** (`entries[0]` is
the first `\bibitem` in the source), so entry `entries[i]` renders as `[i + 1]`.

### 11.2 The three rules (each confirmed against S2's data)

1. **Listing-order numbering.** `entries[i]` → `i + 1`, rendered bracketed. Because S2 collects
   `\bibitem`s in pre-order, the index matches LaTeX's list position exactly.
2. **First-`\bibitem`-wins duplicates consume no number.** A key defined by two `\bibitem`s puts the
   first in `entries` and the second in `duplicate_entries` — the losing duplicate is **not** in
   `entries`, so it never advances the counter. Confirmed by exploration: with entries `a, b, c` and a
   later duplicate `\bibitem{a}`, `entries == [a, b, c]` and `c` is still `[3]` (not pushed to `[4]`).
   This mirrors LaTeX: a re-declared `\bibitem` is numbered the same as the first, consuming no slot.
3. **Dangling `\cite`s are unnumbered.** A `\cite{missing}` whose key has no `\bibitem` is in S2's
   `unresolved`, so it carries no `ResolvedCite` and there is no entry to number — `number_for`
   returns `None` (the `[?]` case), never a panic.

The bracket rendering (`n` → `"[n]"`) is single-sourced in one `render_cite_number` helper, so the
bracket convention lives in exactly one place.

### 11.3 Public API

```rust
impl Document {
    /// One `NumberedCitation { key, ordinal, number }` per numbered bibliography key, in `\bibitem`
    /// listing order. Losing duplicates and dangling cites are omitted.
    pub fn number_citations(&self) -> CitationNumbering;
    /// The bracketed number a resolved `\cite` prints — `number_citations().number_for(c.key)`; ties
    /// S2 → S5. `None` for a non-entry key (total, never a panic).
    pub fn cite_number(&self, c: &ResolvedCite) -> Option<String>;
}

pub struct CitationNumbering { pub entries: Vec<NumberedCitation> }
impl CitationNumbering { pub fn number_for(&self, key: &str) -> Option<&str>; }

pub struct NumberedCitation { pub key: String, pub ordinal: usize, pub number: String }
```

`CitationNumbering`/`NumberedCitation` are dedicated, owned-`String` result types mirroring S4's
`Numbering`/`NumberedLabel` (owned `String`s + `Copy` ordinal), so a numbering outlives any borrow of
the source. `cite_number` is the **payoff**: `\cite{foo}` → `"[2]"`, closing the loop from S2
resolution to S5 numbering. (A caller numbering *many* cites should call `number_citations` once and
reuse it; `cite_number` re-numbers per call.)

### 11.4 What is DEFERRED (honest boundary, mirroring S1–S4)

- **Equation numbers** — blocked on the AST shape: an equation body is an opaque
  `Block::DisplayMath` **raw source string** with no `label` field (the `\label` is buried inside the
  string), so per-equation numbering would need fuzzy string heuristics. A future rung.
- **Author-year / natbib sorted styles** — `plainnat`/`abbrvnat`/`alpha` renumber, re-*label*
  (`[Smith2020]`, `[Smi20]`), and often **sort** the entry list, changing the number a key prints. S5
  models only the listing-order numeric style; sorted/author-year styles are a later rung.
- **External `.bib` databases** — as with S2, only an in-document `thebibliography` is numbered; a
  `\bibliography{refs}` reading an external `.bib`/`.bbl` is not (no file I/O, no BibTeX parsing). A
  key that lives only in an external database is *unresolved* by S2, hence unnumbered here.

### 11.5 Verification (S5)

`cargo test -p latex` green (7 new `references` S5 tests); `cargo clippy -p latex --all-targets
-- -D warnings` clean; downstream `cargo test -p adj-lang -p adj-lang-cli` green; `cargo build
-p latex --no-default-features` builds. No `cargo fmt`, no grammar regen. The tests assert the
**actual bracketed string**: three `\bibitem`s → `number_for("a")=="[1]"`, `"b"=="[2]"`, `"c"=="[3]"`
(listing order); `cite_number` for a resolved `\cite{b}` → `"[2]"` (the load-bearing S2→S5 payoff); a
multi-key `\cite{a,c}` numbers its resolved records to `"[1]"` and `"[3]"`; a dangling `\cite{missing}`
→ `number_for("missing")` is `None` (no panic); a later duplicate `\bibitem{a}` does not renumber or
shift the others (`b` stays `"[2]"`, `c` stays `"[3]"`) and consumes no number; an empty document /
document with no `thebibliography` → empty numbering; and a regression that citation numbering leaves
the S1/S2/S3/S4 outputs byte-for-byte unchanged (the tree is never mutated).

## 12. S6 — the cross-reference report (consumer composing S1/S2/S4/S5)

S1 bound each `\ref`; S2 bound each `\cite`; S4 numbered the labels; S5 numbered the citations. Each
pass produced its own owned result type, but **nothing yet assembles them into a single
consumer-facing artifact**. S6 is that assembly: one method that walks S1's resolved `\ref`s and S2's
resolved `\cite`s and produces an **owned, plain-data report** where each entry carries its rendered
*number* (from S4/S5) alongside its key/command/kind. It is the **payoff rung** — the proof that the
five analysis passes *compose* into an auditable whole, exactly the shape a byte-provenance consumer
wants: "here is every cross-reference in this document, what it points at, and the number it prints."

### 12.1 A pure consumer — no new AST walk

S6 adds **no new walk** of its own. It calls S1 (`resolve_references`) and S2 (`resolve_citations`) —
each of which already reuses the bounded `Document::walk` — and S4 (`number_labels`) / S5
(`number_citations`) to number each family **once**, then *looks each key up* in the resulting number
table. So the whole report costs a *constant* number of the existing bounded passes (never a per-entry
re-numbering — the anti-pattern the S4/S5 convenience methods `ref_number`/`cite_number` warn about
when called in a loop). No new parsing, no AST change, no new recursion.

| family | source pass (binding) | number pass | report entry |
|--------|-----------------------|-------------|--------------|
| `\ref`  | S1 `ResolvedRef { key, command, target_kind }` | S4 `Numbering::number_for(key)` | `RefEntry`  |
| `\cite` | S2 `ResolvedCite { key }`                      | S5 `CitationNumbering::number_for(key)` | `CiteEntry` |

### 12.2 The three rules

1. **Dangling refs/cites surfaced separately.** A `\ref{missing}` (S1's `unresolved`) and a
   `\cite{ghost}` (S2's `unresolved`) — LaTeX's `??` (undefined reference) and `[?]` (undefined
   citation) markers — are **not** dropped and **not** folded in among the resolved entries with a fake
   number. They go in their own `dangling_refs` / `dangling_cites` key vectors. This separation is
   deliberate: two vectors make "resolved vs dangling" a *type-level* fact, rather than burying it in a
   `number: Option<String>` field the caller must remember to check.
2. **A resolved `\ref` whose target is not numbered is omitted.** Every entry in S1's `resolved` has a
   matching `\label`, but not every `\label` is *numbered*: S4 numbers sections and figure/table
   floats, but an **inline `\label`** (typically an equation label) is deliberately unnumbered
   (deferred to a future equation-numbering rung). So a `\ref{eq:x}` to an inline `\label{eq:x}`
   *resolves* yet has **no** S4 number. S6 **omits** such an entry from `refs`, so every row in `refs`
   carries a real number (no placeholder). It is neither *dangling* (its label exists) nor
   *renderable*; it reappears once equation numbering ships. Citations have no analogous gap — a
   winning `\bibitem` is always S5-numbered — so every resolved `\cite` becomes a `CiteEntry`.
3. **Multi-key `\cite` yields one row per key.** S2 already split `\cite{a,b}` into per-key
   `ResolvedCite`s, so S6 emits one `CiteEntry` per key, each numbered independently (`a`→`[1]`,
   `b`→`[2]`).

### 12.3 Public API

```rust
impl Document {
    /// Assemble the cross-reference report: composes S1 (resolve refs) + S2 (resolve cites) + S4
    /// (label numbers) + S5 (citation numbers) into one owned artifact. No new AST walk — each
    /// family is numbered once, then looked up. Total & panic-free; the tree is not mutated.
    pub fn cross_reference_report(&self) -> CrossReferenceReport;
}

pub struct CrossReferenceReport {
    pub refs: Vec<RefEntry>,         // one per resolved & numbered `\ref`, in S1 pre-order
    pub cites: Vec<CiteEntry>,       // one per resolved `\cite` key, in S2 pre-order
    pub dangling_refs: Vec<String>,  // keys of unresolved `\ref` (S1's `unresolved`) — LaTeX's `??`
    pub dangling_cites: Vec<String>, // keys of unresolved `\cite` (S2's `unresolved`) — LaTeX's `[?]`
}
impl CrossReferenceReport {
    /// Render a stable, deterministic, human-readable report (see §12.4 for the pinned format).
    pub fn to_plain_text(&self) -> String;
}

pub struct RefEntry  { pub key: String, pub command: String, pub kind: LabelKind, pub number: String }
pub struct CiteEntry { pub key: String, pub number: String }
```

All plain, owned data (`String`s + `Copy` `LabelKind`), mirroring S4/S5, so the report outlives any
borrow of the source and can be stored/serialized. `RefEntry`, `CiteEntry`, and `CrossReferenceReport`
are exported from the crate root.

### 12.4 The pinned plain-text format

`to_plain_text()` renders a stable string a test can pin byte-for-byte:

- One line per resolved reference, in `refs` order: `\ref{<key>} -> <Kind> <number>` (e.g.
  `\ref{sec:intro} -> Section 1.2`). The command shown is always `\ref` (naming the *binding*, not the
  surface `\eqref`/`\pageref` spelling); `<Kind>` is the capitalised kind name (`Section`/`Table`/
  `Figure`).
- One line per resolved citation, in `cites` order: `\cite{<key>} -> <number>` (e.g.
  `\cite{smith} -> [2]`).
- **Only if** non-empty, a footer `Dangling references: <k1>, <k2>, …` (keys joined by `", "`).
- **Only if** non-empty, a footer `Dangling citations: <k1>, <k2>, …`.

Lines are joined by a single `\n` with **no trailing newline** and no trailing whitespace. An **empty**
report renders the fixed marker `(no cross-references)` (never the empty string). Sections appear in a
fixed order (resolved refs, resolved cites, dangling refs, dangling cites), so the rendering is a pure
function of the report.

### 12.5 What is DEFERRED (honest boundary, inherited from S4/S5)

- **Equation numbers** — a `\ref` to an equation/inline label is *omitted* from `refs` (S4 does not
  number those yet — an equation body is an opaque `Block::DisplayMath` string with no label field), not
  invented.
- **Author-year / natbib sorted citation styles** and **external `.bib`/`.bbl` databases** — out of
  scope at S2/S5, so the report covers only what those passes resolved (in-document `thebibliography`,
  numeric/unsorted style).

### 12.6 Verification (S6)

`cargo test -p latex` green (7 new `references` S6 tests); `cargo clippy -p latex --all-targets
-- -D warnings` clean; downstream `cargo test -p adj-lang -p adj-lang-cli` green; `cargo build
-p latex --no-default-features` builds. No `cargo fmt`, no grammar regen. The tests assert the
**actual composed data and rendered strings**: a doc with both a labeled `\section`+`\ref` and a
`thebibliography`+`\cite` → one `RefEntry` (key `"s:i"`, command `"ref"`, kind `Section`, number
`"1"`) and one `CiteEntry` (key `"b"`, number `"[2]"`), each field exact; `to_plain_text()` renders the
pinned multi-line string; a dangling `\ref{nope}`/`\cite{ghost}` appear in `dangling_refs`/
`dangling_cites` (not in `refs`/`cites`) and in the footer; a multi-key `\cite{a,b}` → two `CiteEntry`s
numbered `[1]`/`[2]`; a resolved-but-unnumbered inline-label `\ref` is omitted from `refs` (and not
dangling); an empty document → an empty report with `to_plain_text()` == `"(no cross-references)"` (no
panic); and a regression that building the report leaves the S1/S2/S3/S4/S5 outputs byte-for-byte
unchanged (the tree is never mutated).
