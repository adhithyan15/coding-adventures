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
   matching `\label`, but not every `\label` is *numbered*: S4 numbers sections, figure/table floats,
   and (as of S7, §13) **non-starred display-math equation labels**. A **bare-inline `\label`** (a
   `\label` not lifted onto any block — `LabelKind::Inline`) is still deliberately unnumbered. So a
   `\ref{eq:x}` to a *bare-inline* `\label{eq:x}` *resolves* yet has **no** S4 number, and S6 **omits**
   it from `refs` (it is neither *dangling* — its label exists — nor *renderable*). An **equation**
   label lifted out of a `\begin{equation}` (S7) *is* numbered — with the `EQUATION_NUMBER_PLACEHOLDER`
   (`"?"`) until S8 — so an `\eqref` to it is **included** in `refs` (`\ref{eq:e} -> Equation ?`).
   Citations have no analogous gap — a winning `\bibitem` is always S5-numbered — so every resolved
   `\cite` becomes a `CiteEntry`.
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

## 13. S7 — equation-label lifting

### 13.1 The gap S7 closes

S1 *resolves* a `\ref`/`\eqref` to a `\label` that sits **inside** a display-math environment
(`\begin{equation} E=mc^2 \label{eq:e} \end{equation}`), but that reference had **no** S4 number, so
S6 (§12.2, rule 2) **omitted** it from `refs` — it was neither dangling nor renderable. The root cause
is in the D5 lowering: `Block::DisplayMath` keeps its whole env body as one raw `source: String`, and
`render_nodes(&body)` renders the `\label{eq:e}` **into** that string. The label is therefore swallowed
— it never becomes a real label definition, so it has no `LabelKind`, no counter, and no report row.

Empirically (the D5 lowering, before S7): `\begin{equation} E=mc^2 \label{eq:e} \end{equation}` →
`DisplayMath { source: "E=mc^2 \\label{eq:e}" }`, and the label table has **no** entry for `eq:e`.

### 13.2 What S7 does (and does not)

S7 fixes exactly this one gap for a **non-starred** display-math environment — `equation`, `align`,
`gather`, `multline`, `eqnarray` (the numbered forms). It does **not** touch the starred forms
(`equation*`, `align*`, `gather*`, `multline*`, `eqnarray*`) or `displaymath`, which are **unnumbered**
in LaTeX, nor the `\[…\]`/`$$…$$` islands (which lower via `NodeKind::Math`).

- **Lift the `\label` out of the env body.** In `lower_environment`, a numbered display-math env's body
  is scanned for the first `\label{key}` (a `NodeKind::CrossRef { command: "label", target }` at the
  LTX01 node level); it is **removed** from the body, its key recovered verbatim via
  `document_to_latex(target)`, and the remaining nodes are rendered to `source`. So the `\label` is
  **not** left duplicated in `source`.
- **`Block::DisplayMath` carries the lifted key.** A new field `label: Option<String>`, mirroring how
  `Block::Figure`/`Block::Table` carry their lifted `\label`. It is `Some` **only** for the non-starred
  named-env path; starred envs, `displaymath`, and `\[…\]`/`$$…$$` set `label: None` (no behaviour
  change for them).
- **A new numbered `LabelKind::Equation`.** `as_str()` → `"equation"`, `kind_display_name` → `"Equation"`.
  The lifted label is registered as a real `LabelDef` in the **same** collection pass
  (`block_label`/`collect_definitions`) that registers section/figure/table labels, so an
  `\eqref{eq:e}`/`\ref{eq:e}` now **resolves** to it (S1) and reaches the S6 report.
- **Included in the S6 report (no longer omitted).** `number_labels` records an `Equation` row so that
  `Numbering::number_for(key)` returns `Some`, and `cross_reference_report()` therefore emits a
  `RefEntry` (rendered `\ref{eq:e} -> Equation ?`) instead of dropping it.

### 13.3 Numbering deferred to S8

S7 is **just** the lifting + a resolvable `LabelKind::Equation`; it does **not** wire the equation
counter (`\theequation`). Because a report row requires a number, the `Equation` row carries the
placeholder `EQUATION_NUMBER_PLACEHOLDER` — the constant `"?"` (echoing LaTeX's `??` for an
as-yet-unresolved number), exported from the crate root. S8 will replace this with the real per-equation
counter value. This is consistent with S6's existing honesty rule: a *bare-inline* `\label` (a `\label`
not lifted onto any block) is still `LabelKind::Inline`, still unnumbered, and still omitted from `refs`.

### 13.4 Round-trip fixed point preserved

`Document::to_latex()` previously re-emitted every `DisplayMath` as `$$source$$`. A lifted-label
equation re-emitted that way would **drop** the label (the `$$…$$` form lowers via `NodeKind::Math` →
`label: None`), breaking the round-trip. So `to_latex` now re-emits a *lifted-label* `DisplayMath` as
`\begin{equation}<source> \label{<key>}\end{equation}`, and a label-free one as `$$source$$` (unchanged).
Re-parsing then re-lifts to an equal AST, so `parse(doc.to_latex())` is a fixed point (modulo spans).

### 13.5 Public API (added in S7)

```rust
// references.rs
pub enum LabelKind { Section, Table, Figure, Equation /* new */, Inline }
pub const EQUATION_NUMBER_PLACEHOLDER: &str = "?";

// document.rs
pub enum Block {
    DisplayMath { source: String, label: Option<String> /* new */, span: Span },
    // …
}
```

Both are exported from the crate root. The change is **additive**: no S1–S6 result type is removed and
no existing behaviour changes, except that a resolved equation-label reference stops being omitted.

### 13.6 Verification (S7)

`cargo test -p latex` green (4 new S7 tests: an `equation` env lifts `\label` onto the block with the
exact `source` `"E = mc^2"` and `label: Some("eq:e")`; a starred `equation*` yields `label: None` and
registers **no** equation label; a lifted equation label is a `LabelKind::Equation` def whose
`as_str()` is `"equation"`; an `\eqref`/`\ref` to it is **included** in `cross_reference_report()` with
number `"?"`, rendering `\ref{eq:e} -> Equation ?`; and a `to_latex()` round-trip re-lifts to an equal
AST). `cargo clippy -p latex --all-targets -- -D warnings` clean; downstream `cargo test -p adj-lang
-p adj-lang-cli` green; `cargo build -p latex --no-default-features` builds. No `cargo fmt`, no grammar
regen, no new dependencies.

## 14. S8 — equation numbering

### 14.1 The gap S8 closes

S7 (§13) made a `\ref`/`\eqref` to a display-math `\label` *resolve* and appear in the S6 report, but
it left the equation **number** unwired: the `Equation` row carried the placeholder
`EQUATION_NUMBER_PLACEHOLDER` (`"?"`, §13.3), so `cross_reference_report()` rendered
`\ref{eq:e} -> Equation ?`. S8 wires the real `\theequation` counter so the report prints
`\ref{eq:e} -> Equation 1`.

### 14.2 What S8 does — a flat equation counter

The numbering walk (`Document::number_labels`) already threads a single `Counters` state through the
pre-order `walk`, with a **hierarchical** section counter (`step_section`, deeper-reset) and two
**flat** float counters (`step_figure`/`step_table`, one monotonic run each, incremented for *every*
float in document order). S8 adds a third flat counter of exactly the same shape for equations:

- **New `Counters` field.** `equation: u32`, initialised to `0` in `Counters::new()` alongside
  `figure`/`table`.
- **New `step_equation(&mut self) -> u32`.** Mirrors `step_figure`/`step_table` **exactly**:
  pre-increment (saturating, so a pathological run never wraps or panics) and return the new value. So
  the first equation numbers `1`, the next `2`, in pure document order — **independent** of the
  section/figure/table counters (the `article` default, where `\theequation` is a flat run and is not
  reset per section).
- **The `Block::DisplayMath { label: Some(key), .. }` arm.** The single line
  `record(key, LabelKind::Equation, EQUATION_NUMBER_PLACEHOLDER.to_string())` becomes
  `record(key, LabelKind::Equation, counters.step_equation().to_string())`. Nothing else in the walk
  changes; the `Equation` row now carries a real sequential number, so `Numbering::number_for(key)`
  returns `Some("1")` and the S6 report renders `\ref{eq:e} -> Equation 1`.

### 14.3 LaTeX-fidelity limitation — unlabelled numbered equations

In real LaTeX **every** non-starred display equation consumes the equation counter whether or not it
carries a `\label` — exactly like figures/tables, where the `\label` only *captures* an
already-stepped value. Our AST, however, only marks the **labelled** non-starred case:
`Block::DisplayMath` carries **no** `numbered: bool` flag, and the D5 lowering (§13.2) sets
`label: None` for *both* starred envs (`equation*`, …) **and** unlabelled islands (`\[…\]`, `$$…$$`).
So an unlabelled-but-numbered `equation` env is, at the numbering walk, indistinguishable from an
unnumbered island — there is no state to key a step on. S8 therefore steps the counter **only** for
labelled equations, in keeping with the "don't invent AST fields" constraint.

**Consequence:** if an unlabelled numbered equation sits between two labelled ones, the second
labelled equation's number is one lower than a full LaTeX run would assign. Closing this gap requires
adding `numbered: bool` to `Block::DisplayMath` (an AST change) so the numbering walk can step for
unlabelled numbered equations too, mirroring the figure/table "every float advances the counter"
rule; that is deferred to a later slice.

### 14.4 `\eqref` parenthesisation — deferred

S8 is **counter-only**. The S6 report (§12.4) renders every reference as `\ref{key} -> Kind number`
(canonical `\ref` spelling, bare number), so `\eqref` does **not** yet parenthesise to `(1)`. That
surface distinction — amsmath's `\eqref` wrapping the number in parentheses — is a later slice and is
out of scope for S8.

### 14.5 `EQUATION_NUMBER_PLACEHOLDER` retained

The constant stays defined and re-exported from the crate root (§13.5). Although its former **code**
use in the numbering arm is replaced by the real counter, it is still referenced by the module's
intra-doc links and remains available to the deferred `\eqref` work. It is a `pub` item, so removing
its sole code use does not make it dead code.

### 14.6 Public API (added in S8)

No public type or signature changes: `Counters`/`step_equation` are private to `references.rs`, and
`number_labels`/`Numbering`/`cross_reference_report` keep their existing signatures. The only
observable change is that an `Equation` row's `number` is now a real sequential value (`"1"`, `"2"`,
…) instead of the placeholder `"?"`. The change is **additive** and byte-for-byte compatible with
S1–S7 except for that number.

### 14.7 Verification (S8)

`cargo test -p latex` green (5 new/updated tests: a single lifted equation label numbers `"1"`; two in
document order number `"1"` then `"2"`; the equation counter is independent of the section counter (a
`\section` before the equation leaves it `"1"`); the equation counter is independent of the figure
counter (a figure between two equations leaves them `"1"`/`"2"` while the figure is its own `"1"`); the
`to_latex()` round-trip for a labelled equation is still a fixed point; and the updated S7 report test
now asserts `\ref{eq:e} -> Equation 1`). `cargo clippy -p latex --all-targets -- -D warnings` clean;
downstream `cargo test -p adj-lang -p adj-lang-cli` green; `cargo build -p latex --no-default-features`
builds. No `cargo fmt`, no grammar regen, no new dependencies.

## 15. S9 — `\eqref` parenthesisation

### 15.1 The gap S9 closes

S8 (§14) gave a lifted equation label a real number, so the S6 report prints `Equation 1`. But it
rendered **every** reference identically — `\ref{key} -> Kind number` (canonical `\ref` spelling, bare
number) — regardless of the surface command (§14.4). amsmath's `\eqref{eq:e}`, however, typesets the
equation number **parenthesised** as `(1)`, whereas a plain `\ref{eq:e}` typesets a bare `1`. Through
S8 that surface distinction was lost in the report. S9 closes it for the one case that matters.

### 15.2 What S9 does — an amsmath-faithful rendering split

S9 is a **rendering-only** change confined to `CrossReferenceReport::to_plain_text` (§12.4). The
single resolved-refs `format!` becomes a two-branch split, keyed on the *surface command* + *target
kind* already carried by `RefEntry`:

- **`\eqref` to an equation** — a `RefEntry` with `command == "eqref"` **and**
  `kind == LabelKind::Equation` renders `\eqref{eq:e} -> Equation (1)`: the `\eqref` spelling is kept
  and the number is **parenthesised**, mirroring how amsmath's `\eqref` typesets `(1)`.
- **Everything else** — all `\ref`, all `\pageref` (superseded by S10, §16), and any `\eqref` to a
  **non-equation** kind — renders exactly as through S8: the canonical `\ref` prefix and a **bare**
  number, `\ref{sec:intro} -> Section 1.2`. The `\ref` prefix names the *binding*, not the surface
  command, unchanged.

Only the one amsmath case diverges; the else-branch is byte-for-byte the S8 line. The `\cite`,
dangling-ref, dangling-cite, and empty-report branches of `to_plain_text` are untouched.

### 15.3 `RefEntry.command` was already retained by S1

No struct or AST change is needed. `RefEntry` has carried `pub command: String` (the surface spelling
`"ref"` / `"eqref"` / `"pageref"`) since S1, and `cross_reference_report` (§12) has always populated it
via `command: r.command.clone()`. S9 simply *reads* that field it had been ignoring, so it is a pure
rendering split in one `format!` — no new field, no numbering change, no AST walk.

### 15.4 Additive, and `to_latex()` unchanged

S9 touches only report rendering, not the tree: `to_latex()` remains a fixed point (S9 does not touch
the AST at all). Every S1–S8 output byte is unchanged **except** the one thing S9 is about — the
report line for an `\eqref`-to-equation reference, which gains its `\eqref` prefix and parentheses.

### 15.5 Public API (added in S9)

No public type or signature changes: `RefEntry`, `CrossReferenceReport`, `cross_reference_report`, and
`to_plain_text` keep their existing shapes. The only observable change is the rendered text of an
`\eqref`-to-equation line (now `\eqref{eq:e} -> Equation (1)`). The change is **additive** and
byte-for-byte compatible with S1–S8 except for that one line.

### 15.6 Verification (S9)

`cargo test -p latex` green (3 new S9 tests: an `\eqref` to an equation parenthesises its number while
a sibling `\ref` to the same equation stays bare (`\eqref{eq:e} -> Equation (1)` then
`\ref{eq:e} -> Equation 1`); a lone `\ref` to an equation is a bare number (`\ref{eq:e} -> Equation 1`,
unchanged from S8); two `\eqref`s to two equations number sequentially and each parenthesises
(`\eqref{eq:a} -> Equation (1)` then `\eqref{eq:b} -> Equation (2)`) — plus the updated S7 report test,
whose first (`\eqref`) line now asserts `\eqref{eq:e} -> Equation (1)`).
`cargo clippy -p latex --all-targets -- -D warnings` clean; downstream
`cargo test -p adj-lang -p adj-lang-cli` green; `cargo build -p latex --no-default-features` builds. No
`cargo fmt`, no grammar regen, no new dependencies.

## 16. S10 — distinct `\pageref` rendering

### 16.1 The gap S10 closes

S9 (§15) split the report rendering so an `\eqref` to an equation parenthesises its number, but it
left the **page** reference family conflated with the **number** reference family. A `\pageref{key}`
asks a fundamentally different question from `\ref{key}`: `\ref` asks "what **number** is the target"
(a section number, figure number, …), while `\pageref` asks "what **page** is the target printed on".
LaTeX resolves these against two different values in the `.aux` `\newlabel` record. Through S9 the S6
report ignored the distinction and rendered a resolved `\pageref` **identically** to a `\ref` —
`\ref{key} -> Kind number` — so `\pageref{sec:i}` printed `\ref{sec:i} -> Section 1`, silently
answering the *number* question for a command that asked the *page* question. S10 closes that gap.

### 16.2 What S10 does — an honest page placeholder

The crate has **no page model**: it parses and numbers *structure*, never lays out pages, so it cannot
compute a real page number for any label. S10 therefore does not invent one — it renders the page
family **distinctly and honestly** with a fixed placeholder. S10 is a **rendering-only** change
confined to `CrossReferenceReport::to_plain_text` (§12.4), adding a **third** branch to the
resolved-refs loop, tried in this precedence order:

1. **`\eqref` to an equation** — `command == "eqref"` **and** `kind == LabelKind::Equation` renders
   `\eqref{eq:e} -> Equation (1)` (S9, §15.2, unchanged).
2. **`\pageref` to any kind** — a `RefEntry` with `command == "pageref"` (regardless of target kind —
   Section/Table/Figure/Equation/Inline) renders `\pageref{sec:i} -> page ?`: the `\pageref` spelling
   is kept and the kind/number are replaced by the fixed literal placeholder `page ?`. The `?` mirrors
   LaTeX's own `??` for an unresolved page reference (and the S7 number-placeholder `"?"`): it means
   "page number not modelled" — **not** the kind, **not** the number. Because a page reference is
   about *location*, not *identity*, the target's kind and number are irrelevant to this rendering.
3. **Everything else** — all `\ref` and any `\eqref` to a **non-equation** kind — renders exactly as
   through S8: the canonical `\ref` prefix and a **bare** number, `\ref{sec:intro} -> Section 1.2`.

Precedence matters. Branch (1) before (2) is moot (an `\eqref` is never a `\pageref`), but (2) before
(3) is what makes **every** `\pageref` diverge from the S8 `\ref` line. So `\ref` and `\eqref` outputs
are byte-for-byte unchanged from S9; **only** `\pageref` lines change (previously `\ref{key} -> Kind
N`, now `\pageref{key} -> page ?`). The `\cite`, dangling-ref, dangling-cite, and empty-report
branches of `to_plain_text` are untouched.

### 16.3 `RefEntry.command` was already retained by S1

As with S9, no struct or AST change is needed. `RefEntry` has carried `pub command: String` (the
surface spelling `"ref"` / `"eqref"` / `"pageref"`) since S1, and `cross_reference_report` (§12) has
always populated it. S10 simply reads `command == "pageref"` — a field it had been folding into the
else-branch — so it is a pure rendering branch in one loop, no new field, no numbering change, no AST
walk.

### 16.4 Additive, and `to_latex()` unchanged

S10 touches only report rendering, not the tree: `to_latex()` remains a fixed point (S10 does not
touch the AST at all). Every S1–S9 output byte is unchanged **except** the one thing S10 is about — a
resolved `\pageref` line, which now reads `\pageref{key} -> page ?` instead of `\ref{key} -> Kind N`.

### 16.5 What is DEFERRED (honest boundary)

A **real page number** is out of scope: it requires a page-layout model (line/page breaking, float
placement, `\pagebreak`, class geometry) the crate does not have and does not intend to add at this
rung. The `page ?` placeholder is the honest terminus for the page family until such a model exists,
exactly as `Equation ?` was S7's honest terminus before S8 wired the equation counter.

### 16.6 Public API (added in S10)

No public type or signature changes: `RefEntry`, `CrossReferenceReport`, `cross_reference_report`, and
`to_plain_text` keep their existing shapes. The only observable change is the rendered text of a
resolved `\pageref` line (now `\pageref{key} -> page ?`). The change is **additive** and byte-for-byte
compatible with S1–S9 except for that one line.

### 16.7 Verification (S10)

`cargo test -p latex` green (3 new S10 tests: a `\pageref` to a labelled section renders
`\pageref{sec:i} -> page ?` (not `\ref{sec:i} -> Section 1`); a doc with both `\ref` and `\pageref` to
the same section renders `\ref{sec:i} -> Section 1` then `\pageref{sec:i} -> page ?`, proving `\ref` is
unchanged and `\pageref` now diverges; a `\pageref` to a labelled equation still renders
`\pageref{eq:e} -> page ?`, proving `\pageref` ignores the S9 eqref/Equation special-case).
`cargo clippy -p latex --all-targets -- -D warnings` clean; downstream
`cargo test -p adj-lang -p adj-lang-cli` green; `cargo build -p latex --no-default-features` builds. No
`cargo fmt`, no grammar regen, no new dependencies.

## 17. S11 — grouped-by-kind cross-reference report

### 17.1 The gap S11 closes

Through S10 the only rendering of the cross-reference report was `CrossReferenceReport::to_plain_text`
(§12.4, S6): resolved references, one line each, in **flat source (pre-order)** order — the order the
`\ref`s appear in the body. That is the right default for an audit trail, but it answers "in what order
are things referenced?" rather than "**which sections / figures / equations** does this document
cross-reference?". A reader who wants the latter must eyeball the flat list and mentally bucket it by
kind. S11 closes that gap with a **new, separate** rendering that does the bucketing.

### 17.2 What S11 does — a new sibling method, `to_plain_text_by_kind`

S11 adds a **new public method** `CrossReferenceReport::to_plain_text_by_kind(&self) -> String`. It
renders the **same** resolved references `to_plain_text` does, but **grouped under fixed-order kind
subheadings** instead of flat source order. It does **not** touch `to_plain_text`: S11 is purely
additive, a second lens on the same `refs` data.

The exact format, pinned:

1. **Fixed kind order.** Groups are emitted in the fixed order **Sections, Figures, Tables, Equations,
   Inline** (`S11_KIND_ORDER`) — *regardless* of the order the references appear in the source.
2. **Non-empty groups only.** For each kind with **≥1** resolved ref, a subheading line — the
   **pluralised capitalised** kind name plus a colon (`Sections:`, `Figures:`, `Tables:`,
   `Equations:`, `Inline:`, from the new `kind_group_heading` helper) — followed by **one line per
   ref**. A kind with **zero** resolved refs is **omitted entirely** (no bare subheading).
3. **Two-space-indented ref lines, shared rendering.** Each ref line is indented by **two spaces** then
   rendered by the **shared** `render_resolved_ref(&RefEntry) -> String` helper — the *identical*
   per-command rule `to_plain_text` uses (§16.2 S8/S9/S10): an `\eqref` to an equation →
   `\eqref{eq:e} -> Equation (1)`; a `\pageref` (any kind) → `\pageref{key} -> page ?`; else
   `\ref{key} -> Kind N`. Within a kind group the refs keep the report's existing **pre-order** (a
   filter of `refs` for that kind, preserving order). A `\pageref` groups under its **target kind**
   (e.g. a `\pageref` to a section appears in the `Sections:` group).
4. **Resolved refs only.** Citations (`cites`) and the dangling footers (`dangling_refs`,
   `dangling_cites`) are **not** included — this method stays focused on the kind-grouped resolved
   refs; the flat `to_plain_text` remains the place for the full report.
5. **Distinct empty marker.** If there are **zero** resolved refs at all, the method returns the fixed
   string `"(no resolved references)"` — the S11 analogue of `to_plain_text`'s `"(no cross-references)"`
   — so the output is never the empty string.

Lines are joined by a single `\n` with **no trailing newline** and no trailing whitespace on any line.
Example:

```text
Sections:
  \ref{sec:intro} -> Section 1
  \ref{sec:methods} -> Section 2
Figures:
  \ref{fig:plot} -> Figure 1
Equations:
  \eqref{eq:e} -> Equation (1)
```

### 17.3 The shared `render_resolved_ref` helper (no-drift guarantee)

To guarantee the flat (S6) and grouped (S11) reports can never diverge on how a single ref line looks,
the three precedence-ordered per-command renderings are **factored** out of `to_plain_text`'s loop into
a private `render_resolved_ref(&RefEntry) -> String`. **Both** `to_plain_text` and
`to_plain_text_by_kind` call it, so a `\ref`/`\eqref`/`\pageref` line is byte-for-byte the same wherever
it appears. `to_plain_text` was refactored to call the helper **only** because its output stays
byte-for-byte identical — the existing S6–S10 tests (`report_to_plain_text_renders_the_exact_pinned_string`,
the `s9_`/`s10_` tests) pass unchanged, proving additivity.

### 17.4 Additive, and `to_latex()` unchanged

S11 is a pure report-assembly method over data the report already holds (`refs` with their `kind`
`LabelKind`). No AST/grammar/counter change, no new struct or field, no new dependency, no `unsafe`, no
I/O. `to_latex()` remains a fixed point (S11 does not touch the AST). Every S1–S10 output byte is
unchanged — `to_plain_text` included.

### 17.5 Public API (added in S11)

One new method: `CrossReferenceReport::to_plain_text_by_kind(&self) -> String`. No existing type or
signature changes; `RefEntry`, `CrossReferenceReport`, `cross_reference_report`, and `to_plain_text`
keep their shapes and outputs.

### 17.6 Verification (S11)

`cargo test -p latex` green (5 new S11 tests: sections + a figure + an equation grouped in the fixed
Sections/Figures/Equations order with two-space-indented lines and the real S4 numbers; the grouped
lines obey the S9 `\eqref`-parenthesises and S10 `\pageref` → `page ?` rules, with a `\pageref` grouped
under its target kind's group; a doc with no resolved refs → the `(no resolved references)` marker; a
doc with only a dangling `\ref` and a `\cite` → still the marker, proving citations/dangling are
excluded; and a guard that `to_plain_text()` still returns its exact prior pinned string). All prior
S1–S10 tests pass unchanged. `cargo clippy -p latex --all-targets -- -D warnings` clean; downstream
`cargo test -p adj-lang -p adj-lang-cli` green; `cargo build -p latex --no-default-features` builds. No
`cargo fmt`, no grammar regen, no new dependencies.

## 18. S12 — a List of Figures / List of Tables index (`list_of_floats`)

### 18.1 Motivation

S1–S11 answer *"where does this reference point, and what number/kind is its target?"* S12 asks the
**dual** question a reader browsing the front matter asks: *"what figures and tables does this document
contain, in order?"* — LaTeX's `\listoffigures` / `\listoftables`. Those two commands print a numbered
table of every float's caption, in document order. S12 renders that index as plain text.

### 18.2 Why a new method, not a gated render

Real LaTeX only prints these lists where the author writes `\listoffigures` / `\listoftables`. But
those are **not** parser-recognised commands in this crate (a grep finds no lowering for them), so
there is no `Block` to gate on. Exactly as S11 did with its grouped report, S12 is therefore exposed as
a **new public method** the caller invokes directly, rendering the index from the document's floats.
Being a brand-new method, S12 is **additive by construction**: it reads existing blocks, mutates
nothing, and leaves every S1–S11 output — including `to_latex()`'s round-trip fixed point — byte-for-
byte unchanged.

### 18.3 What S12 does — `Document::list_of_floats(&self) -> String`

A single document-order walk (`Document::walk`) threads the **same** `Counters` float counters that
`number_labels` (S4) uses:

- **Every** `Block::Figure` steps the flat figure counter and emits a line `<n>. <caption text>`, where
  `<n>` is the counter's value at that float. Because it is the *same* counter walk as S4, a labeled
  figure's List-of number equals its `\ref` number — the two renderings can never drift.
- **Every** `Block::Table` does the same against the independent flat table counter (numbered from `1`).
- **Caption text** is the plain rendering of the float's `\caption{…}` inlines, via a private
  `caption_text(&Option<Caption>) -> String` helper: `Inline::Text`/`Inline::Code` runs verbatim,
  `Inline::Space` as a single space, and the text *inside* font wrappers (`Strong`/`Emph`/`Styled`)
  recursively; math / cross-ref / accent inlines contribute no plain text. The result is trimmed. This
  is the same descent the `ref_target_node_for_figure_reaches_its_caption` test exercises.
- A float carrying **no** `\caption` renders the fixed placeholder `(no caption)`, so every float still
  gets its own numbered line and the numbering stays aligned with the real float count.

### 18.4 Assembly rules

- The `List of Figures` heading + its lines are emitted **only** when there is ≥1 figure.
- The `List of Tables` heading + its lines are emitted **only** when there is ≥1 table.
- If the document has **no** floats at all, the method returns the fixed marker `"(no floats)"`.
- Lines are joined by `\n` with **no** trailing newline and no trailing whitespace.

Example (two captioned figures, one captioned table):

```text
List of Figures
1. First plot
2. Second plot
List of Tables
1. Data table
```

### 18.5 Public API (added in S12)

One new method: `Document::list_of_floats(&self) -> String`, plus a private `caption_text` helper. No
existing type, field, counter, or signature changes; no AST or grammar change; no new dependency, no
`unsafe`, no I/O.

### 18.6 Verification (S12)

`cargo test -p latex` green (3 new S12 tests: two figures + a table listed in order with the exact
`List of Figures … List of Tables` string; an uncaptioned figure rendering `1. (no caption)`; a
float-free document returning `(no floats)`). All prior S1–S11 tests pass unchanged. `cargo clippy -p
latex --all-targets -- -D warnings` clean; downstream `cargo test -p adj-lang -p adj-lang-cli` green;
`cargo build -p latex --no-default-features` builds. No `cargo fmt`, no grammar regen, no new
dependencies.

## 19. S13 — `\nameref` resolution to a target's name (`resolve_namerefs`)

### 19.1 Motivation

`\ref` prints a target's **number** ("Section 1"), `\pageref` prints its **page**. The `nameref`
package's `\nameref{key}` prints its **name** — a section's *title*, a float's *caption text*. It is the
name-valued sibling of the number- and page-valued references S1–S12 already model. S13 answers the
question `\nameref` asks: *what is this label's target called?*

### 19.2 Why a new method, not a change to `REF_COMMANDS`

`"nameref"` is deliberately **not** in `REF_COMMANDS = ["ref", "eqref", "pageref"]`. An AST probe
confirms `\nameref{sec:intro}` lowers to `Inline::CrossRef { command: "nameref", target: "sec:intro",
note: None, span }` — the key is fully recoverable — and that `resolve_references()` returns it in
**neither** the resolved **nor** the unresolved table (it is not a `REF_COMMAND`, so the S1 resolver
skips it entirely). Adding `"nameref"` to `REF_COMMANDS` would change S1–S12 output (a `\nameref` would
suddenly appear in the resolved/unresolved tables and the S6 report), violating additivity. So S13 is a
**new public method** that reads the same S1 `\label` table but answers a *different* question, leaving
every S1–S12 output — including `to_latex()`'s round-trip fixed point — byte-for-byte unchanged.

### 19.3 What S13 does — `Document::resolve_namerefs(&self) -> String`

A single document-order walk (`Document::walk`) collects every `Inline::CrossRef` whose `command` is
`"nameref"`. Each key is resolved against the **winning** label table (`ReferenceResolution::definition`,
the same first-definition-wins table `\ref` resolves against, built once via `resolve_references`), then
the target's **name** is read from its defining node (reached by the S3 `label_def_node` accessor):

- a `Block::Section` target → its `title` inlines flattened to visible text via a module-level
  `flatten_inlines_to_text(&[Inline]) -> String` (`Text`/`Code` verbatim, `Space` → one space,
  `Strong`/`Emph`/`Styled` recursed, trimmed);
- a `Block::Figure`/`Block::Table` target → its `\caption` text via the **shared** `caption_text`
  helper (so a `\nameref` and the S12 List-of-Floats entry read the *same* caption; an uncaptioned
  float yields the `(no caption)` marker `caption_text` returns);
- a `LabelKind::Equation`/`LabelKind::Inline` target → the fixed marker `(no name)` (an equation or a
  bare `\label` has a *number*, not a title — the honest boundary);
- a key that **no** `\label` defines → the fixed placeholder `(undefined nameref: <key>)` (the
  name-valued analogue of LaTeX's `??`).

To keep S12's caption descent and S13's title descent from ever diverging, the flatten logic previously
nested inside `caption_text` is factored into the module-level `flatten_inlines_to_text`, which **both**
`caption_text` and `resolve_namerefs` call.

### 19.4 Assembly rules

- One line per `\nameref`, in body pre-order, formatted `\nameref{<key>} -> <name>` (mirroring the S6
  `\ref{k} -> …` arrow).
- Lines are joined by `\n` with **no** trailing newline.
- A document with **no** `\nameref` at all returns the fixed marker `(no namerefs)`.

Example (a section, a captioned figure, and an undefined key):

```text
\nameref{sec:intro} -> Introduction
\nameref{fig:p} -> A plot
\nameref{nope} -> (undefined nameref: nope)
```

### 19.5 Public API (added in S13)

One new method: `Document::resolve_namerefs(&self) -> String`, plus a private `nameref_name(&LabelDef)`
helper and the module-level `flatten_inlines_to_text` (factored out of `caption_text`). No existing
type, field, counter, or signature changes; `REF_COMMANDS` is **unchanged**; no AST or grammar change;
no new dependency, no `unsafe`, no I/O.

### 19.6 Verification (S13)

`cargo test -p latex` green (5 new S13 tests: a section+figure resolving to `Introduction` / `A plot`;
an undefined key rendering `(undefined nameref: nope)`; an equation + inline label both rendering
`(no name)`; a `\ref`-only document returning `(no namerefs)`; and an additivity check that the two
`\namerefs` stay out of the resolved/unresolved ref tables and `list_of_floats` is unchanged). All prior
S1–S12 tests pass unchanged. `cargo clippy -p latex --all-targets -- -D warnings` clean; downstream
`cargo test -p adj-lang -p adj-lang-cli` green; `cargo build -p latex --no-default-features` builds. No
`cargo fmt`, no grammar regen, no new dependencies.

---

## 20. S14 — per-kind census of the numbered-label table (`list_summary`)

### 20.1 Motivation

S1–S13 answer *"where does this reference point, what number/name is its target?"* — always per
reference or per label. S14 asks the coarser, aggregate question a table-of-contents or a document
overview needs: *"how many numbered labels of each kind does this document have?"* — a count of
sections vs figures vs tables vs equations that carry a `\label`. It is the census over the table S4
already built.

### 20.2 Why a new method — additive by construction

S14 adds a **new public method** `Document::list_summary(&self) -> String`. It is a pure, read-only
tally of the rows `Document::number_labels()` returns, grouped by `LabelKind`. It reuses that table
verbatim (never re-deriving counts from a fresh walk), so the census can never drift from the S4
numbering it summarises. It mutates nothing and changes no S1–S13 output — including `to_latex()`'s
round-trip fixed point — byte-for-byte.

### 20.3 What S14 counts

Only four kinds ever reach `number_labels()`: a numbered `\section` (`LabelKind::Section`), a
`figure` (`Figure`), a `table` (`Table`), and a non-starred display `equation` label (`Equation`).
A bare inline `\label{…}` (`LabelKind::Inline`) is **not** numbered — the numbering pass records no
`Inline` rows — so it never appears in the table and is therefore **counted nowhere**. (Confirmed by
reading the numbering pass and by an exploratory tally over a mixed fixture: a doc with two labeled
sections, a figure, a table, an equation, and one bare inline `\label` yields exactly five numbered
rows — Section, Section, Figure, Table, Equation — the inline label omitted.)

### 20.4 The exact rendering contract — `Document::list_summary(&self) -> String`

- One line per kind whose count is **≥ 1**, in this **fixed order** (deterministic, *not* document
  order): **Sections, Figures, Tables, Equations**.
- Each line is exactly `<Kind>: <count>` — `Sections: n`, `Figures: n`, `Tables: n`, `Equations: n`
  — with a **fixed plural** label regardless of `n` (a single section still prints `Sections: 1`,
  never `Section: 1`).
- A kind whose count is **0 is omitted** entirely, mirroring S11's "kinds with 0 refs are omitted"
  convention.
- Lines are joined by `\n` with **no** trailing newline (matching S11 `to_plain_text_by_kind`, S12
  `list_of_floats`, S13 `resolve_namerefs`).
- If **all** counts are 0 (the document defines no numbered label at all), the fixed marker
  `(no labels)` is returned, so the output is never the empty string.

Example (two labeled sections, one labeled figure, one labeled table, one labeled equation):

```text
Sections: 2
Figures: 1
Tables: 1
Equations: 1
```

A document whose only label is a bare inline `\label{marker}` renders `(no labels)` (that label is
not numbered). A document with only three labeled sections renders just `Sections: 3` (the other
three kinds are omitted).

### 20.5 Public API (added in S14)

One new method: `Document::list_summary(&self) -> String`. No existing type, field, counter, or
signature changes; `number_labels` and every S1–S13 method are unchanged; no AST or grammar change;
no new dependency, no `unsafe`, no I/O.

### 20.6 Verification (S14)

`cargo test -p latex` green (4 new S14 tests: a doc counting all four kinds
`Sections: 2\nFigures: 1\nTables: 1\nEquations: 1`; a sections-only doc `Sections: 3` with zero-count
kinds omitted; an equation/inline-only doc returning the `(no labels)` marker; and an additivity
check that `to_plain_text`, `to_plain_text_by_kind`, `list_of_floats`, and `resolve_namerefs` all
still produce their exact prior strings). All prior S1–S13 tests pass unchanged. `cargo clippy -p
latex --all-targets -- -D warnings` clean; downstream `cargo test -p adj-lang -p adj-lang-cli` green;
`cargo build -p latex --no-default-features` builds. No `cargo fmt`, no grammar regen, no new
dependencies.

## 21. S15 — resolved citations grouped by their source `\cite` (`citations_by_source`)

### 21.1 Motivation

S2's `resolve_citations` returns `resolved: Vec<ResolvedCite>` **flattened per key** — a multi-key
`\cite{a,b}` yields one row for `a` and one for `b`, each carrying that single `\cite`'s `cite_span`.
That per-key shape is right for a resolver but loses, at a glance, *which keys travelled together in
one `\cite`*. S15 answers the citation-family analogue of the question S11's `to_plain_text_by_kind`
answers for references: *"grouped by the construct they came from, what resolved?"* — it re-assembles
the per-key rows back into one line per source `\cite`.

### 21.2 Why a new method — additive by construction

S15 adds a **new public method** `Document::citations_by_source(&self) -> String`. It is a pure,
read-only re-assembly of `resolve_citations().resolved` — grouping the rows by `cite_span`. It reuses
that table verbatim (never re-walking the body or re-resolving keys), so the grouping can never drift
from the S2 resolution it summarises. It mutates nothing and changes no S1–S14 output — including
`to_latex()`'s round-trip fixed point — byte-for-byte. Like S11–S14 it is a method the caller invokes
directly.

### 21.3 The grouping and ordering rule

Every `ResolvedCite` from one `\cite` shares that `\cite`'s `cite_span`, so grouping `resolved` by
`cite_span` reconstructs "which resolved keys came from which `\cite`". Because `resolved` is already
in body pre-order (and within one multi-key `\cite` in left-to-right key order), the **first
appearance** of each distinct `cite_span` fixes that group's position — so the groups come out in
source order of the `\cite`s, and keys within a group keep their left-to-right order. The
implementation uses a `Vec<(Span, Vec<&str>)>` (not a hash map) precisely to keep that first-appearance
order deterministic.

### 21.4 The exact rendering contract — `Document::citations_by_source(&self) -> String`

- One line per source `\cite` that resolved **≥ 1** key, in first-appearance (source) order of the
  `\cite`s.
- Each line is the citing command **reconstructed from its resolved keys**: `\cite{` + the group's
  resolved keys joined by `", "` + `}`.
- The keys are **only the resolved ones**, in their original left-to-right order. A **dangling** key
  (one no `\bibitem` defines) never entered `resolved`, so it is **excluded**: a `\cite{a,ghost}`
  where only `a` resolves renders `\cite{a}`, not `\cite{a,ghost}`. We reconstruct from resolved keys
  rather than slice the raw `&src[cite_span]` precisely because the source text would still contain the
  dangling `ghost`; the reconstruction shows exactly what *bound*.
- Lines are joined by `\n` with **no** trailing newline (matching S11 `to_plain_text_by_kind`, S12
  `list_of_floats`, S13 `resolve_namerefs`, S14 `list_summary`).
- If there are **no** resolved citations (none present, or every cited key dangling), the fixed marker
  `(no resolved citations)` is returned, so the output is never the empty string.

Example (`\cite{smith2020, jones2019}` both defined, then `\cite{a, ghost}` with only `a` defined):

```text
\cite{smith2020, jones2019}
\cite{a}
```

### 21.5 Public API (added in S15)

One new method: `Document::citations_by_source(&self) -> String`. No existing type, field, counter, or
signature changes; `resolve_citations` and every S1–S14 method are unchanged; no AST or grammar
change; no new dependency, no `unsafe`, no I/O.

### 21.6 Verification (S15)

`cargo test -p latex` green (4 new S15 tests: a doc grouping a multi-key `\cite{a,b}` and a separate
`\cite{c}` into `\cite{a, b}\n\cite{c}`; a partial `\cite{a,ghost}` rendering only `\cite{a}`; an
all-dangling doc returning the `(no resolved citations)` marker; and an additivity check that
`to_plain_text`, `to_plain_text_by_kind`, `list_of_floats`, `resolve_namerefs`, and `list_summary` all
still produce their exact prior strings). All prior S1–S14 tests pass unchanged. `cargo clippy -p latex
--all-targets -- -D warnings` clean; downstream `cargo test -p adj-lang -p adj-lang-cli` green; `cargo
build -p latex --no-default-features` builds. No `cargo fmt`, no grammar regen, no new dependencies.

## 22. S16 — duplicate (multiply-defined) bibliography entries (`duplicate_bibliography_entries`)

### 22.1 Motivation

S2's `resolve_citations` returns `duplicate_entries: Vec<DuplicateBib>` — every `\bibitem` that
redefines an already-defined key, i.e. LaTeX's *"Citation `key' multiply defined"* warning. S6's flat
report surfaces the *dangling* bibliography warning (undefined citations, a "Dangling citations:"
footer), but the *other* bibliography warning — the multiply-defined `\bibitem`s — was computed by S2
yet rendered by **no** method. S16 fills that gap: it is the citation-family analogue of the dangling
footer, for duplicate definitions.

### 22.2 Why a new method — additive by construction

S16 adds a **new public method** `Document::duplicate_bibliography_entries(&self) -> String`. It is a
pure, read-only render of `resolve_citations().duplicate_entries`. It reuses that table verbatim (never
re-walking the body or re-collecting `\bibitem`s), so the report can never drift from the S2 resolution
it summarises. It mutates nothing and changes no S1–S15 output — including `to_latex()`'s round-trip
fixed point — byte-for-byte. Like S11–S15 it is a method the caller invokes directly.

### 22.3 The ordering and multiplicity rule

S2 collects every `\bibitem{key}` inside a `thebibliography` in `Document::walk` **pre-order**. The
**first** `\bibitem` of each key wins (it is the entry citations resolve against, in `entries`); every
**later** `\bibitem` of an already-defined key is a losing duplicate, appended to `duplicate_entries`
in that same pre-order. S16 renders `duplicate_entries` **in that order, unchanged** — it does **not**
re-sort. It also does **not** de-duplicate: if a key is defined three times, the second and third both
lose, so `duplicate_entries` holds two rows and S16 emits two lines — one per *"multiply defined"*
warning LaTeX would raise. The winning first `\bibitem` is never listed (it is an entry, not a
duplicate).

### 22.4 The exact rendering contract — `Document::duplicate_bibliography_entries(&self) -> String`

- One line per losing duplicate `\bibitem`, in the existing pre-order of `duplicate_entries` (source
  order of the offending `\bibitem`s, **not** re-sorted).
- Each line is the offending command **reconstructed from its key**: `\bibitem{` + the duplicate's key
  + `}`. We reconstruct from the owned key rather than slice the raw `&src[span]` (matching S13
  `resolve_namerefs` and S15 `citations_by_source`), so the render needs no source borrow and can never
  index out of bounds.
- Every losing `\bibitem` yields its own line — **no** de-duplication (a key defined three times yields
  two lines).
- Lines are joined by `\n` with **no** trailing newline (matching S11 `to_plain_text_by_kind`, S12
  `list_of_floats`, S13 `resolve_namerefs`, S14 `list_summary`, S15 `citations_by_source`).
- If there are **no** duplicate entries (no bibliography, or every key defined exactly once), the fixed
  marker `(no duplicate bibliography entries)` is returned, so the output is never the empty string.

Example (`thebibliography` defining `smith` twice and `jones` once):

```text
\begin{thebibliography}{9}
\bibitem{smith} First Smith. 1990.
\bibitem{jones} Jones. 1991.
\bibitem{smith} Second Smith. 1992.
\end{thebibliography}
```

only the *second* `\bibitem{smith}` loses, so the report is the single line:

```text
\bibitem{smith}
```

### 22.5 Public API (added in S16)

One new method: `Document::duplicate_bibliography_entries(&self) -> String`. No existing type, field,
counter, or signature changes; `resolve_citations` and every S1–S15 method are unchanged; no AST or
grammar change; no new dependency, no `unsafe`, no I/O.

### 22.6 Verification (S16)

`cargo test -p latex` green (4 new S16 tests: a bibliography defining `smith` twice and `jones` once
rendering `\bibitem{smith}` (only the loser); two distinct keys each defined twice rendering
`\bibitem{a}\n\bibitem{b}` in pre-order; a no-duplicate bibliography returning the
`(no duplicate bibliography entries)` marker; and an additivity check that `to_plain_text`,
`to_plain_text_by_kind`, `list_of_floats`, `resolve_namerefs`, `list_summary`, and
`citations_by_source` all still produce their exact prior strings). All prior S1–S15 tests pass
unchanged. `cargo clippy -p latex --all-targets -- -D warnings` clean; downstream `cargo test -p
adj-lang -p adj-lang-cli` green; `cargo build -p latex --no-default-features` builds. No `cargo fmt`,
no grammar regen, no new dependencies.

## 23. S17 — unresolved (dangling) citations grouped by source `\cite` (`unresolved_citations_by_source`)

### 23.1 Motivation

S2's `resolve_citations` returns `unresolved: Vec<UnresolvedCite>` — every `\cite` key that matched no
`\bibitem`, i.e. LaTeX's *"Citation `key' undefined"* warning (the `[?]` in the output). S6's flat
report already surfaces these dangling keys as a single "Dangling citations:" footer, and S15's
`citations_by_source` groups the *resolved* keys per source `\cite`. S17 fills the remaining cell of
that 2×2: it groups the *dangling* keys per source `\cite` — the DANGLING-key mirror of S15, and a
distinct new per-`\cite` view of the same information S6 renders flat.

### 23.2 Why a new method — additive by construction

S17 adds a **new public method** `Document::unresolved_citations_by_source(&self) -> String`. It is a
pure, read-only render of `resolve_citations().unresolved`. It reuses that table verbatim (never
re-walking the body or re-resolving citations), so the report can never drift from the S2 resolution it
summarises. It mutates nothing and changes no S1–S16 output — including `to_latex()`'s round-trip fixed
point — byte-for-byte. Like S11–S16 it is a method the caller invokes directly.

### 23.3 The grouping and ordering rule

S2 flattens every `\cite` into per-key rows, splitting them into resolved keys and unresolved
(dangling) keys, each row tagged with the citing `\cite`'s own `cite_span` (shared by every key of a
multi-key `\cite`). S17 groups the dangling keys by their shared `cite_span`, preserving the
**first-appearance order** of the cite_spans (source order of the `\cite`s) via a `Vec<(Span,
Vec<&str>)>` — **not** a hash map — so the order is deterministic. Keys within a group stay in their
existing left-to-right order. Because `unresolved` holds **only** the dangling keys, a resolved key of
a mixed `\cite` never appears — the exact analogue of how S15 shows only the *resolved* keys.

### 23.4 The exact rendering contract — `Document::unresolved_citations_by_source(&self) -> String`

- One line per source `\cite` that has **at least one** dangling key, in the first-appearance order of
  the cite_spans (source order of the `\cite`s, **not** re-sorted).
- Each line is `\cite{` + that group's **dangling** keys joined by `", "` + `}`, reconstructed from the
  owned keys rather than sliced from `&src[span]` (matching S13 `resolve_namerefs`, S15
  `citations_by_source`, S16 `duplicate_bibliography_entries`), so the render needs no source borrow and
  can never index out of bounds.
- A `\cite{a, ghost}` where `a` resolves and `ghost` dangles renders `\cite{ghost}` (only the dangling
  key), because `unresolved` contains only the dangling keys.
- Lines are joined by `\n` with **no** trailing newline (matching S11 `to_plain_text_by_kind`, S12
  `list_of_floats`, S13 `resolve_namerefs`, S14 `list_summary`, S15 `citations_by_source`, S16
  `duplicate_bibliography_entries`).
- If there are **no** unresolved citations (every cited key resolves, or none present), the fixed marker
  `(no unresolved citations)` is returned, so the output is never the empty string.

Example (body citing `\cite{a, ghost}` where `a` resolves, then `\cite{x, y}` where neither is
defined):

```text
\cite{ghost}
\cite{x, y}
```

the first line drops the resolved `a` and keeps only the dangling `ghost`; the second reunites both
dangling keys of the fully-dangling `\cite` on one comma-space-joined line, in source order.

### 23.5 Public API (added in S17)

One new method: `Document::unresolved_citations_by_source(&self) -> String`. No existing type, field,
counter, or signature changes; `resolve_citations` and every S1–S16 method are unchanged; no AST or
grammar change; no new dependency, no `unsafe`, no I/O.

### 23.6 Verification (S17)

`cargo test -p latex` green (6 new S17 tests: a single dangling `\cite{ghost}` rendering `\cite{ghost}`;
a mixed `\cite{known, ghost}` (only `known` defined) rendering `\cite{ghost}`; a fully-dangling
`\cite{x, y}` rendering `\cite{x, y}` (both keys, one line); two distinct dangling `\cite`s rendering
`\cite{ghost1}\n\cite{ghost2}` in source order; an all-resolved document returning the
`(no unresolved citations)` marker; and an additivity check that `to_plain_text`,
`to_plain_text_by_kind`, `list_of_floats`, `resolve_namerefs`, `list_summary`, `citations_by_source`,
and `duplicate_bibliography_entries` all still produce their exact prior strings). All prior S1–S16
tests pass unchanged. `cargo clippy -p latex --all-targets -- -D warnings` clean; downstream `cargo test
-p adj-lang -p adj-lang-cli` green; `cargo build -p latex --no-default-features` builds. No `cargo fmt`,
no grammar regen, no new dependencies.

## 24. S18 — unresolved (dangling) references grouped by source `\ref` (`unresolved_references_by_source`)

### 24.1 Motivation

S3's `resolve_references` returns `unresolved: Vec<UnresolvedRef>` — every `\ref`/`\eqref`/`\pageref`
key that matched no `\label`, i.e. LaTeX's *"Reference `key' undefined"* warning (the `??` in the
output). Each `UnresolvedRef` carries not just the dangling `key` and its `ref_span` but the `command`
that was written (`"ref"` / `"eqref"` / `"pageref"`). S6's flat report already surfaces these dangling
keys as a single "Dangling references: k1, k2" footer, and S17 gives the citation family its per-source
dangling view. S18 is the `\ref`-family parallel of S17 — but it is a **distinct** view from S6's
footer: it reconstructs each dangling reference on **its own line**, and it is **command-aware**, so a
dangling `\eqref` renders `\eqref{…}` and a dangling `\pageref` renders `\pageref{…}` rather than being
flattened into an undifferentiated `\ref`-shaped comma list.

### 24.2 Why a new method — additive by construction

S18 adds a **new public method** `Document::unresolved_references_by_source(&self) -> String`. It is a
pure, read-only render of `resolve_references().unresolved`. It reuses that table verbatim (never
re-walking the body or re-resolving references), so the report can never drift from the S3 resolution it
summarises. It mutates nothing and changes no S1–S17 output — including `to_latex()`'s round-trip fixed
point — byte-for-byte. Like S11–S17 it is a method the caller invokes directly.

### 24.3 The grouping and ordering rule

S3 walks every `\ref`/`\eqref`/`\pageref` in body pre-order and splits them into resolved references and
unresolved (dangling) references, each dangling row recorded as `UnresolvedRef { key, command, ref_span
}`. S18 groups the dangling references by their shared `ref_span`, preserving the **first-appearance
order** of the ref_spans (source order of the references) via a `Vec<(Span, Vec<&UnresolvedRef>)>` —
**not** a hash map — so the order is deterministic and the code reads identically to S17's grouping.
Unlike a multi-key `\cite`, a `\ref`/`\eqref`/`\pageref` takes exactly **one** key, so every group holds
a single entry; the structural mirror of S17 is kept only for readability, and each group emits exactly
one line. Because `unresolved` holds **only** the dangling references, a `\ref` that resolves to a
`\label` never appears — the exact analogue of how S17 shows only the *dangling* keys.

### 24.4 The exact rendering contract — `Document::unresolved_references_by_source(&self) -> String`

- One line per dangling reference, in the first-appearance order of the ref_spans (source order of the
  references, **not** re-sorted).
- Each line is `\` + that reference's own `command` + `{` + its `key` + `}`, reconstructed from the
  owned `command`/`key` `String`s rather than sliced from `&src[span]` (matching S13 `resolve_namerefs`,
  S15 `citations_by_source`, S17 `unresolved_citations_by_source`), so the render needs no source borrow
  and can never index out of bounds. The command is taken from the reference's **own** `command` field —
  it is **never** hard-coded to `\ref`, so a dangling `\eqref{eq:x}` renders `\eqref{eq:x}` and a
  dangling `\pageref{p}` renders `\pageref{p}`.
- A `\ref` that resolves to a `\label` never entered `unresolved`, so it is excluded by construction.
- Lines are joined by `\n` with **no** trailing newline (matching S15 `citations_by_source`, S17
  `unresolved_citations_by_source`).
- If there are **no** unresolved references (every reference resolves, or none present), the fixed marker
  `(no unresolved references)` is returned, so the output is never the empty string.

Example (body with `\eqref{eq:ghost}` and `\pageref{p:ghost}`, neither defined by a `\label`):

```text
\eqref{eq:ghost}
\pageref{p:ghost}
```

each line preserves the command it was written with, one dangling reference per line, in source order.
This is distinct from S6's flat *"Dangling references: eq:ghost, p:ghost"* footer, which drops the
per-reference command and comma-joins the keys.

### 24.5 Public API (added in S18)

One new method: `Document::unresolved_references_by_source(&self) -> String`. No existing type, field,
counter, or signature changes; `resolve_references` and every S1–S17 method are unchanged; no AST or
grammar change; no new dependency, no `unsafe`, no I/O.

### 24.6 Verification (S18)

`cargo test -p latex` green (6 new S18 tests: a single dangling `\ref{nope}` rendering `\ref{nope}`; a
dangling `\eqref{eq:ghost}` and `\pageref{p:ghost}` rendering `\eqref{eq:ghost}\n\pageref{p:ghost}` with
each command preserved; two distinct dangling `\ref`s rendering `\ref{nope1}\n\ref{nope2}` in source
order; a resolved `\ref` excluded so a fully-resolved document returns the `(no unresolved references)`
marker; a reference-free document returning the same marker; and an additivity check that
`to_plain_text`, `to_plain_text_by_kind`, `list_of_floats`, `resolve_namerefs`, `list_summary`,
`citations_by_source`, `duplicate_bibliography_entries`, and `unresolved_citations_by_source` all still
produce their exact prior strings). All prior S1–S17 tests pass unchanged. `cargo clippy -p latex
--all-targets -- -D warnings` clean; downstream `cargo test -p adj-lang -p adj-lang-cli` green; `cargo
build -p latex --no-default-features` builds. No `cargo fmt`, no grammar regen, no new dependencies.

## 25. S19 — numbered winning-bibliography-entry list (`bibliography_entries`)

### 25.1 Motivation

S2's `resolve_citations` returns `entries: Vec<BibEntry>` — the **winning** bibliography entries, one row
per distinct citation key (the first `\bibitem{key}` seen), in body pre-order. This is the table
citations resolve against, and it is exactly what a reader sees rendered as the document's bibliography.
Yet no method rendered it: S16 (`duplicate_bibliography_entries`) renders the **losing**
`duplicate_entries` as `\bibitem{key}` warning lines, and S15 (`citations_by_source`) renders the
per-source *resolved cite keys* — neither shows the winning entries themselves. S19 fills the remaining
cell of that view-matrix by rendering `entries` as a **numbered list**, the way a bibliography actually
looks.

### 25.2 Why a new method — additive by construction

S19 adds a **new public method** `Document::bibliography_entries(&self) -> String`. It is a pure,
read-only render of `resolve_citations().entries`. It reuses that table verbatim (never re-walking the
body or re-collecting entries), so the report can never drift from the S2 resolution it summarises. It
mutates nothing and changes no S1–S18 output — including `to_latex()`'s round-trip fixed point —
byte-for-byte. Like S11–S18 it is a method the caller invokes directly.

### 25.3 The numbering and ordering rule

S2 collects every `\bibitem{key}` inside a `thebibliography` environment in body pre-order, keeping the
**first** `\bibitem` of each distinct key in `entries` and routing every later re-definition into
`duplicate_entries` (never into `entries`). S19 numbers `entries` **1-based** in that existing pre-order,
emitting one line per winning entry. Because `entries` already holds only the first `\bibitem` of each
key, a `\bibitem{dup}` written twice appears **once** — the winner — exactly as a real bibliography
renders one line per key; its losing re-definitions remain the S16 view.

### 25.4 The exact rendering contract — `Document::bibliography_entries(&self) -> String`

- One numbered line per winning entry, **1-based**, in the existing pre-order (**not** re-sorted): the
  n-th entry renders `format!("[{}] {}", n, entry.key)` → `[1] smith2020`, `[2] jones2019`, ….
- Each line is reconstructed from the entry's owned `key` `String` rather than sliced from `&src[span]`
  (matching S13 `resolve_namerefs`, S15 `citations_by_source`, S16 `duplicate_bibliography_entries`, and
  S17/S18's dangling reports), so the render needs no source borrow and can never index out of bounds.
- The `[n] key` shape is chosen **deliberately** so the winning list reads as a *rendered bibliography*
  and is visually distinct from S16's `\bibitem{key}` losing-duplicate lines — the two never look alike
  even when they list overlapping keys.
- Lines are joined by `\n` with **no** trailing newline (matching every S11–S18 renderer).
- If there are **no** bibliography entries (no `thebibliography`, or an empty one), the fixed marker
  `(no bibliography entries)` is returned, so the output is never the empty string.

Example (`thebibliography` defining `smith` twice and `jones` once):

```text
[1] smith
[2] jones
```

the second `\bibitem{smith}` is a duplicate (the S16 view), not a second entry, so the winning list is
two lines. This is distinct from S16's `\bibitem{smith}` losing-duplicate line, which renders the loser.

### 25.5 Public API (added in S19)

One new method: `Document::bibliography_entries(&self) -> String`. No existing type, field, counter, or
signature changes; `resolve_citations` and every S1–S18 method are unchanged; no AST or grammar change;
no new dependency, no `unsafe`, no I/O.

### 25.6 Verification (S19)

`cargo test -p latex` green (5 new S19 tests: two distinct entries rendering `[1] a\n[2] b`; a duplicate
key winning once with a peer rendering `[1] dup\n[2] other`; three entries numbered in pre-order
rendering `[1] x\n[2] y\n[3] z`; a bibliography-free document returning the `(no bibliography entries)`
marker; and an additivity check that `to_plain_text`, `to_plain_text_by_kind`, `list_of_floats`,
`resolve_namerefs`, `list_summary`, `citations_by_source`, `duplicate_bibliography_entries`,
`unresolved_citations_by_source`, and `unresolved_references_by_source` all still produce their exact
prior strings). All prior S1–S18 tests pass unchanged. `cargo clippy -p latex --all-targets -- -D
warnings` clean; downstream `cargo test -p adj-lang -p adj-lang-cli` green; `cargo build -p latex
--no-default-features` builds. No `cargo fmt`, no grammar regen, no new dependencies.

## 26. S20 — losing duplicate `\label` definitions (`duplicate_label_definitions`)

### 26.1 Motivation

S1's `resolve_references` returns `duplicates: Vec<Duplicate>` — the **losing** later re-definitions of
an already-defined label key, one row per multiply-defined `\label`, in body pre-order. This is exactly
LaTeX's *"Label `key' multiply defined"* warning list: the first `\label{key}` seen wins (into
`definitions`, the table `\ref`/`\eqref`/`\pageref` resolve against) and every later `\label` of that key
is a loser. Yet no method rendered it. S16 (`duplicate_bibliography_entries`) does the *citation-family*
equivalent — it renders the losing `\bibitem` duplicates — but its `\label` mirror was missing. S20 fills
that cell by rendering `duplicates` as the losing-`\label` report, the exact label-family parallel of S16.

### 26.2 Why a new method — additive by construction

S20 adds a **new public method** `Document::duplicate_label_definitions(&self) -> String`. It is a pure,
read-only render of `resolve_references().duplicates`. It reuses that table verbatim (never re-walking the
body or re-collecting labels), so the report can never drift from the S1 resolution it summarises. It
mutates nothing and changes no S1–S19 output — including `to_latex()`'s round-trip fixed point —
byte-for-byte. Like S11–S19 it is a method the caller invokes directly.

### 26.3 The ordering rule

S1 collects every `\label` (hoisted onto a section/table/figure/equation, or a bare inline `\label`) in
body pre-order, keeping the **first** of each distinct key in `definitions` and routing every later
re-definition of an already-defined key into `duplicates` (never into `definitions`). S20 renders
`duplicates` verbatim in that existing pre-order — **not** re-sorted and **not** de-duplicated — so a key
defined three times yields two lines, and every *"multiply defined"* warning gets its own line, exactly
like S16. The winning first definition of each key stays in `definitions` and is never in this report.

### 26.4 The exact rendering contract — `Document::duplicate_label_definitions(&self) -> String`

- One line per losing duplicate, in the existing pre-order (**not** re-sorted, **not** de-duplicated),
  each rendering `format!("\\label{{{}}}", dup.key)` → `\label{dup}`.
- Each line is reconstructed from the duplicate's owned `key` `String` rather than sliced from
  `&src[span]` (matching S13 `resolve_namerefs`, S15 `citations_by_source`, S16
  `duplicate_bibliography_entries`, and S17–S19's reports), so the render needs no source borrow and can
  never index out of bounds. Labels are always *defined* by `\label{…}`, so `\label{key}` is the correct
  reconstruction regardless of the duplicate's `LabelKind` — a re-`\label`ed section, figure, equation,
  or bare inline label all render the same `\label{key}` form.
- Lines are joined by `\n` with **no** trailing newline (matching every S11–S19 renderer).
- If there are **no** duplicate labels (every key defined once, or no labels at all), the fixed marker
  `(no duplicate label definitions)` is returned, so the output is never the empty string.

Example (body writing `\label{dup}` twice and `\label{once}` once):

```text
\label{dup}
```

only the second `\label{dup}` loses; the first `dup` wins (into `definitions`) and `once` is defined once,
so the report is the single losing line. This is the label-family mirror of S16's `\bibitem{key}` view.

### 26.5 Public API (added in S20)

One new method: `Document::duplicate_label_definitions(&self) -> String`. No existing type, field,
counter, or signature changes; `resolve_references` and every S1–S19 method are unchanged; no AST or
grammar change; no new dependency, no `unsafe`, no I/O.

### 26.6 Verification (S20)

`cargo test -p latex` green (4 new S20 tests: a `\label{dup}` written twice rendering `\label{dup}` (only
the loser); two distinct keys each multiply-defined rendering `\label{alpha}\n\label{beta}` in pre-order;
labels all defined once returning the `(no duplicate label definitions)` marker; and an additivity check
that `to_plain_text`, `to_plain_text_by_kind`, `list_of_floats`, `resolve_namerefs`, `list_summary`,
`citations_by_source`, `duplicate_bibliography_entries`, `unresolved_citations_by_source`,
`unresolved_references_by_source`, and `bibliography_entries` all still produce their exact prior
strings). All prior S1–S19 tests pass unchanged. `cargo clippy -p latex --all-targets -- -D warnings`
clean; downstream `cargo test -p adj-lang -p adj-lang-cli` green; `cargo build -p latex
--no-default-features` builds. No `cargo fmt`, no grammar regen, no new dependencies.

## 27. S21 — resolved references grouped by source `\ref` (`resolved_references_by_source`)

### 27.1 Motivation

S3's `resolve_references` returns `resolved: Vec<ResolvedRef>` — every `\ref`/`\eqref`/`\pageref` that
bound to a real `\label`, one row per resolved reference, in body pre-order. S18
(`unresolved_references_by_source`) already renders the **dangling** half of that split — the references
that matched no `\label`, LaTeX's *"Reference `key' undefined"* — as one command-aware `\<command>{key}`
line each. But the **resolved** half — the references that *did* match — had no by-source renderer: S6
reports the flat *"Dangling references"* footer, S18 the dangling per-line view, but nothing renders the
successfully-matched references on their own lines with their commands preserved. S21 fills that cell by
rendering `resolved` as the resolved-reference report, the exact RESOLVED mirror of S18's dangling view.

### 27.2 Why a new method — additive by construction

S21 adds a **new public method** `Document::resolved_references_by_source(&self) -> String`. It is a pure,
read-only render of `resolve_references().resolved`. It reuses that table verbatim (never re-walking the
body or re-resolving references), so the report can never drift from the S3 resolution it summarises. It
mutates nothing and changes no S1–S20 output — including `to_latex()`'s round-trip fixed point —
byte-for-byte. Like S11–S20 it is a method the caller invokes directly.

### 27.3 The exact rendering contract — `Document::resolved_references_by_source(&self) -> String`

- Read `resolve_references().resolved` and group its entries by their shared `ref_span`, preserving the
  **first-appearance order** of the ref_spans (source order of the references) — a `Vec` of
  `(ref_span, refs)`, **not** a hash map, so the order is deterministic and the code reads identically to
  S18's grouping. A `\ref`/`\eqref`/`\pageref` takes exactly **one** key, so every group holds a single
  entry and emits exactly one line.
- One line per resolved reference: `format!("\\{}{{{}}}", r.command, r.key)` → `\` + the reference's own
  `command` + `{` + its `key` + `}`. Reconstructed from the owned `command`/`key` `String`s rather than
  sliced from `&src[span]` (matching S13 `resolve_namerefs`, S15 `citations_by_source`, and S17/S18's
  reports), so the render needs no source borrow and can never index out of bounds. Because the line is
  rebuilt from the ref's **own** `command`, a resolved `\eqref{eq:main}` renders `\eqref{eq:main}` and a
  resolved `\pageref{sec:intro}` renders `\pageref{sec:intro}` — the command is **never** hard-coded to
  `\ref`. Dangling references never entered `resolved`, so they are excluded by construction (a
  `\ref{nope}` with no `\label` appears in S18, not here).
- Lines are joined by `\n` with **no** trailing newline (matching every S11–S20 renderer).
- If there are **no** resolved references (every reference dangles, or none at all), the fixed marker
  `(no resolved references)` is returned, so the output is never the empty string.

Example (body defining `\label{sec:intro}` and `\label{eq:main}`, then writing `\ref{sec:intro}`,
`\eqref{eq:main}`, `\pageref{sec:intro}`, and a dangling `\ref{nope}`):

```text
\ref{sec:intro}
\eqref{eq:main}
\pageref{sec:intro}
```

each resolved reference on its own line, command preserved, in source order; the dangling `\ref{nope}` is
excluded (it lives in S18). This is the RESOLVED mirror of S18's dangling `\<command>{key}` view.

### 27.4 Public API (added in S21)

One new method: `Document::resolved_references_by_source(&self) -> String`. No existing type, field,
counter, or signature changes; `resolve_references` and every S1–S20 method are unchanged; no AST or
grammar change; no new dependency, no `unsafe`, no I/O.

### 27.5 Verification (S21)

`cargo test -p latex` green (5 new S21 tests: a resolved `\ref`+`\eqref`+`\pageref` rendering each as its
own command-aware line in source order; a doc with no references returning the `(no resolved references)`
marker; a doc whose only reference dangles returning the same marker; a mixed doc listing only the
resolved reference and excluding the dangling `\ref{nope}`; and an additivity check that `to_plain_text`,
`to_plain_text_by_kind`, `list_of_floats`, `resolve_namerefs`, `list_summary`, `citations_by_source`,
`duplicate_bibliography_entries`, `unresolved_citations_by_source`, `unresolved_references_by_source`,
`bibliography_entries`, and `duplicate_label_definitions` all still produce their exact prior strings,
which also pins the `\n`-join with no trailing newline on a multi-ref case). All prior S1–S20 tests pass
unchanged. `cargo clippy -p latex --all-targets -- -D warnings` clean; downstream `cargo test -p adj-lang
-p adj-lang-cli` green; `cargo build -p latex --no-default-features` builds. No `cargo fmt`, no grammar
regen, no new dependencies.

## 28. S22 — winning label definitions (`label_definitions`)

### 28.1 Motivation

S1's `resolve_references` returns `definitions: Vec<LabelDef>` — the **winning** label definitions, one
row per **distinct** key (the first `\label` of each key seen), in `walk` pre-order. This is precisely the
label table `\ref`/`\eqref`/`\pageref` resolve against. S20 (`duplicate_label_definitions`) already
renders the **losing** half of the `\label` split — the later re-definitions of an already-defined key,
LaTeX's *"Label `key' multiply defined"* — one `\label{key}` line each. But the **winning** half — the
definitions references actually bind to — had no renderer: S19 renders the winning `\bibitem` entries for
the citation family, yet the exact `\label` analogue was missing. S22 fills that cell by rendering
`definitions` as the winning-label report, the label-family analogue of S19 and the winning-side
counterpart of S20.

### 28.2 Why a new method — additive by construction

S22 adds a **new public method** `Document::label_definitions(&self) -> String`. It is a pure, read-only
render of `resolve_references().definitions`. It reuses that table verbatim (never re-walking the body or
re-collecting definitions), so the report can never drift from the S1 resolution it summarises. It mutates
nothing and changes no S1–S21 output — including `to_latex()`'s round-trip fixed point — byte-for-byte.
Like S11–S21 it is a method the caller invokes directly.

### 28.3 The ordering rule

`definitions` is already in `walk` pre-order with **one row per distinct key**: S1's `collect_definitions`
keeps the **first** `\label` of each key as the winner and routes every later re-definition of an
already-defined key into `duplicates` (never into `definitions`). S22 renders `definitions` in that
existing pre-order — **not** re-sorted and **not** de-duplicated, because no de-duplication is needed
(the list already holds exactly one row per distinct key). A `\label{dup}` written twice therefore appears
**once** here — the winner; its losing second definition lives in S20.

### 28.4 The exact rendering contract — `Document::label_definitions(&self) -> String`

- Read `resolve_references().definitions` in its existing pre-order and emit one line per winning
  definition: `format!("\\label{{{}}}", def.key)` → `\label{` + the definition's `key` + `}`.
  Reconstructed from the owned `key` `String` rather than sliced from `&src[span]` (matching S13
  `resolve_namerefs`, S15 `citations_by_source`, S16 `duplicate_bibliography_entries`, S19
  `bibliography_entries`, and S20 `duplicate_label_definitions`), so the render needs no source borrow and
  can never index out of bounds. `\label{key}` is the correct reconstruction for any `LabelKind` (a
  section, figure, equation, or bare inline label all render the same `\label{key}` form).
- One line per winning definition, in the existing pre-order — **not** re-sorted, **not** de-duplicated
  (none needed, since `definitions` already holds one row per distinct key).
- Lines are joined by `\n` with **no** trailing newline (matching every S11–S21 renderer).
- If there are **no** label definitions, the fixed marker `(no label definitions)` is returned, so the
  output is never the empty string.

Example (body defining `\label{sec:intro}` on a section, `\label{eq:main}` on an equation, then re-using
`\label{sec:intro}` on a later subsection):

```text
\label{sec:intro}
\label{eq:main}
```

only the *first* `\label{sec:intro}` wins, so the winning key `sec:intro` appears **once**; its later
re-definition is a duplicate (surfaced by S20, not a second `definitions` row). This is the winning-label
analogue of S19's numbered `\bibitem` list and the winning-side counterpart of S20's losing-duplicate
view.

### 28.5 Public API (added in S22)

One new method: `Document::label_definitions(&self) -> String`. No existing type, field, counter, or
signature changes; `resolve_references` and every S1–S21 method are unchanged; no AST or grammar change;
no new dependency, no `unsafe`, no I/O.

### 28.6 Verification (S22)

`cargo test -p latex` green (5 new S22 tests: two distinct labels rendering `\label{k1}\n\label{k2}` in
source pre-order; a doc with no labels returning the `(no label definitions)` marker; a `\label{dup}`
written twice rendering `\label{dup}` **once** (the winner) and cross-checked against S20's losing side;
a three-label case pinning the `\n`-join with no trailing newline; and an additivity check that
`to_plain_text`, `to_plain_text_by_kind`, `list_of_floats`, `resolve_namerefs`, `list_summary`,
`citations_by_source`, `duplicate_bibliography_entries`, `unresolved_citations_by_source`,
`unresolved_references_by_source`, `bibliography_entries`, `duplicate_label_definitions`, and
`resolved_references_by_source` all still produce their exact prior strings). All prior S1–S21 tests pass
unchanged. `cargo clippy -p latex --all-targets -- -D warnings` clean; downstream `cargo test -p adj-lang
-p adj-lang-cli` green; `cargo build -p latex --no-default-features` builds. No `cargo fmt`, no grammar
regen, no new dependencies.

## 29. S23 — winning label definitions grouped by kind (`label_definitions_by_kind`)

### 29.1 Motivation

S22 (`label_definitions`) renders the **winning** `\label` definitions **flat** — one `\label{key}` line
per distinct key, in `walk` pre-order. But each winning definition also carries a `LabelKind`
(`Section`, `Table`, `Figure`, `Equation`, or `Inline`), and a reader often wants the definitions
organised **by that kind** — a per-kind census answering "which keys are sections? which are equations?
which are bare inline labels?" at a glance. The citation/reference families already have their by-source
groupings (S15/S17/S18 `citations_by_source`, `unresolved_citations_by_source`,
`unresolved_references_by_source`); the label family had a flat winning-definitions list (S22) but no
by-kind grouping. S23 fills that cell by rendering the *same* winning `definitions` list grouped by
`LabelKind`, the by-kind companion of S22.

### 29.2 Why a new method — additive by construction

S23 adds a **new public method** `Document::label_definitions_by_kind(&self) -> String`. It is a pure,
read-only render of `resolve_references().definitions` — a *second view* of the exact list S22 renders
flat. It reuses that table verbatim (never re-walking the body or re-collecting definitions), so the
report can never drift from the S1 resolution it summarises, and grouping never adds, drops, or reorders
definitions relative to what `resolve_references` produced. It mutates nothing and changes no S1–S22
output — including `to_latex()`'s round-trip fixed point — byte-for-byte. Like S11–S22 it is a method the
caller invokes directly.

### 29.3 The ordering rule

The output is ordered by a **fixed, document-independent** kind order — the `LabelKind` enum declaration
order: `Section`, `Table`, `Figure`, `Equation`, `Inline`. S23 iterates that order as an explicit slice
(**not** a hash map keyed by kind), so the group order is deterministic and never depends on document
content or hash iteration order — the same `Vec`-of-groups discipline S17/S18 use to avoid hash-order
nondeterminism. **Within** each kind, the definitions keep their existing `definitions` pre-order;
grouping never reorders within a kind. A kind with **no** definitions produces **no** lines (there is
never an empty `[table]` group for a doc with no tables).

### 29.4 The exact rendering contract — `Document::label_definitions_by_kind(&self) -> String`

- Iterate the fixed kind order (`Section`, `Table`, `Figure`, `Equation`, `Inline`). For each kind, take
  the definitions of that kind from `resolve_references().definitions` in their existing pre-order and
  emit one line each: `format!("[{}] \\label{{{}}}", def.kind.as_str(), def.key)` → `[` + the kind tag +
  `] \label{` + the definition's `key` + `}`. The kind tag comes from `LabelKind::as_str()`
  (`"section"`/`"table"`/`"figure"`/`"equation"`/`"inline"`); the key is reconstructed from the owned
  `key` `String` rather than sliced from `&src[span]` (matching S13 `resolve_namerefs`, S19
  `bibliography_entries`, S20 `duplicate_label_definitions`, and S22 `label_definitions`), so the render
  needs no source borrow and can never index out of bounds.
- The `[kind]` prefix makes the per-kind census visible on every line while staying
  one-line-per-definition; it is a **report annotation**, not round-trippable LaTeX (S23 is a *report*).
- A kind with no definitions contributes no lines and no empty header.
- Lines are joined by `\n` with **no** trailing newline (matching every S11–S22 renderer).
- If there are **no** label definitions at all, the fixed marker `(no label definitions)` is returned —
  the **same** marker S22 uses (S23 groups the identical list), so the output is never the empty string.

Example (body defining a section label `sec:intro`, an equation label `eq:main`, and a bare inline label
`note`):

```text
[section] \label{sec:intro}
[equation] \label{eq:main}
[inline] \label{note}
```

`sec:intro` (kind `Section`) leads even though `note` is also a definition, because `Section` sorts
before `Equation` and `Inline` in the fixed kind order. This is the by-kind grouping companion of S22's
flat winning-definitions list — two views of the one `definitions` list.

### 29.5 Public API (added in S23)

One new method: `Document::label_definitions_by_kind(&self) -> String`. No existing type, field, counter,
or signature changes; `resolve_references` and every S1–S22 method are unchanged; no AST or grammar
change; no new dependency, no `unsafe`, no I/O.

### 29.6 Verification (S23)

`cargo test -p latex` green (6 new S23 tests: a section+equation+inline case rendering grouped in the
fixed kind order; a source-order-reversed case proving the fixed kind order pulls the section ahead of an
earlier inline; two same-kind inline labels grouped together in pre-order under `inline`; a doc with no
labels returning the `(no label definitions)` marker; a section+figure+inline case pinning the `\n`-join
with no trailing newline; and an additivity check that `to_plain_text`, `to_plain_text_by_kind`,
`list_of_floats`, `resolve_namerefs`, `list_summary`, `citations_by_source`,
`duplicate_bibliography_entries`, `unresolved_citations_by_source`, `unresolved_references_by_source`,
`bibliography_entries`, `duplicate_label_definitions`, `resolved_references_by_source`, and S22's flat
`label_definitions` all still produce their exact prior strings). All prior S1–S22 tests pass unchanged.
`cargo clippy -p latex --all-targets -- -D warnings` clean; downstream `cargo test -p adj-lang -p
adj-lang-cli` green; `cargo build -p latex --no-default-features` builds. No `cargo fmt`, no grammar
regen, no new dependencies.

## 30. S24 — resolved references grouped by target kind (`resolved_references_by_kind`)

### 30.1 Motivation

S21 (`resolved_references_by_source`) renders the **resolved** `\ref`/`\eqref`/`\pageref` references
**flat** — one reconstructed `\<command>{key}` line per resolved reference, in `walk` pre-order. But each
resolved reference also carries a `target_kind` (`Section`, `Table`, `Figure`, `Equation`, or `Inline`) —
the `LabelKind` of the definition it bound to — and a reader often wants the resolved references
organised **by that kind**: a per-kind census answering "which of my references land on sections? which
on equations? which on bare inline labels?" at a glance. The label family already got its by-kind
grouping in S23 (`label_definitions_by_kind`, the by-kind companion of S22's flat `label_definitions`);
S24 brings the *same* discipline to the **resolved-references** family — it is to S21 exactly what S23 is
to S22.

### 30.2 Why a new method — additive by construction

S24 adds a **new public method** `Document::resolved_references_by_kind(&self) -> String`. It is a pure,
read-only render of `resolve_references().resolved` — a *second view* of the exact list S21 renders flat.
It reuses that list verbatim (never re-walking the body or re-resolving references), so the report can
never drift from the S1 resolution it summarises, and grouping never adds, drops, or reorders references
relative to what `resolve_references` produced. It mutates nothing and changes no S1–S23 output —
including `to_latex()`'s round-trip fixed point — byte-for-byte. Like S11–S23 it is a method the caller
invokes directly.

### 30.3 The ordering rule

The output is ordered by a **fixed, document-independent** kind order — the `LabelKind` enum declaration
order: `Section`, `Table`, `Figure`, `Equation`, `Inline` (the **same** `const KIND_ORDER` slice S23
uses). S24 iterates that order as an explicit slice (**not** a hash map keyed by kind), so the group
order is deterministic and never depends on document content or hash iteration order — the same
`Vec`-scan discipline S17/S18/S23 use to avoid hash-order nondeterminism. **Within** each kind, the
resolved references keep their existing `resolved` pre-order; grouping never reorders within a kind. A
kind with **no** resolved references produces **no** lines (there is never an empty `[table]` group for a
doc that references no tables).

### 30.4 The exact rendering contract — `Document::resolved_references_by_kind(&self) -> String`

- Iterate the fixed kind order (`Section`, `Table`, `Figure`, `Equation`, `Inline`). For each kind, take
  the resolved references whose `target_kind` is that kind from `resolve_references().resolved` in their
  existing pre-order and emit one line each:
  `format!("[{}] \\{}{{{}}}", r.target_kind.as_str(), r.command, r.key)` → `[` + the kind tag + `] \` +
  the ref's own `command` + `{` + its `key` + `}`. The kind tag comes from `LabelKind::as_str()`
  (`"section"`/`"table"`/`"figure"`/`"equation"`/`"inline"`); the `command` is the reference's **own**
  (so a resolved `\eqref` renders `\eqref` and a resolved `\pageref` renders `\pageref`, never
  hard-coded to `\ref` — matching S21's command-awareness); the key is reconstructed from the owned `key`
  `String` rather than sliced from `&src[span]` (matching S21 `resolved_references_by_source` and S23
  `label_definitions_by_kind`), so the render needs no source borrow and can never index out of bounds
  (`ref_span`/`target_span` are unused).
- The `[kind]` prefix makes the per-kind census visible on every line while staying one-line-per-ref; it
  is a **report annotation**, not round-trippable LaTeX (S24 is a *report*).
- Dangling references never entered `resolved`, so they are excluded by construction (a `\ref{nope}` with
  no `\label` appears in S18's `unresolved_references_by_source`, not here).
- A kind with no resolved references contributes no lines and no empty header.
- Lines are joined by `\n` with **no** trailing newline (matching every S11–S23 renderer).
- If there are **no** resolved references at all (every reference dangles, or there are none), the fixed
  marker `(no resolved references)` is returned — the **same** marker S21 uses (S24 groups the identical
  list), so the output is never the empty string.

Example (body defining a section label `sec:intro` and an equation label `eq:main`, then writing
`\ref{sec:intro}`, `\eqref{eq:main}`, and `\pageref{sec:intro}` — all of which resolve):

```text
[section] \ref{sec:intro}
[section] \pageref{sec:intro}
[equation] \eqref{eq:main}
```

Both references to `sec:intro` (kind `Section`) lead — in their pre-order, `\ref` before `\pageref` —
even though the `\eqref` appears between them in source, because `Section` sorts before `Equation` in the
fixed kind order. This is the by-kind grouping companion of S21's flat resolved-references list — two
views of the one `resolved` list.

### 30.5 Public API (added in S24)

One new method: `Document::resolved_references_by_kind(&self) -> String`. No existing type, field,
counter, or signature changes; `resolve_references` and every S1–S23 method are unchanged; no AST or
grammar change; no new dependency, no `unsafe`, no I/O.

### 30.6 Verification (S24)

`cargo test -p latex` green (6 new S24 tests: a `\ref`-to-section + `\eqref`-to-equation case rendering
grouped in the fixed kind order; a source-order-reversed case proving the fixed kind order pulls the
section ahead of an earlier equation; two same-section refs (`\ref` then `\pageref`) grouped together in
pre-order under `section`, command-aware; a dangling-only and a no-references case both returning the
`(no resolved references)` marker; a section+equation case with a trailing dangling `\ref{nope}` pinning
the `\n`-join with no trailing newline and confirming the dangling ref is excluded; and an additivity
check that `to_plain_text`, `to_plain_text_by_kind`, `list_of_floats`, `resolve_namerefs`,
`list_summary`, `citations_by_source`, `duplicate_bibliography_entries`,
`unresolved_citations_by_source`, `unresolved_references_by_source`, `bibliography_entries`,
`duplicate_label_definitions`, `resolved_references_by_source`, `label_definitions`, and S23's grouped
`label_definitions_by_kind` all still produce their exact prior strings). All prior S1–S23 tests pass
unchanged. `cargo clippy -p latex --all-targets -- -D warnings` clean; downstream `cargo test -p adj-lang
-p adj-lang-cli` green; `cargo build -p latex --no-default-features` builds. No `cargo fmt`, no grammar
regen, no new dependencies.

## 31. S25 — per-kind census (counts) of the winning label definitions (`label_kind_counts`)

### 31.1 Motivation

S23 (`label_definitions_by_kind`) groups the **winning** `\label` definitions by their `LabelKind` and
renders one **line per definition** (`[kind] \label{key}`). But a reader often wants not the enumeration
but the **tally** — "how many sections do I define? how many equations? how many bare inline labels?" —
a per-kind *count* answered at a glance, without scanning the individual keys. S14 (`list_summary`)
already provides exactly this shape (`"Sections: 1"`) over S4's numbered-node census; S25 brings the same
numeric-summary discipline to the **label-definitions** family. It is to S23 what a count is to a full
list: the *same* winning `definitions` list, collapsed to one line per kind.

### 31.2 Why a new method — additive by construction

S25 adds a **new public method** `Document::label_kind_counts(&self) -> String`. It is a pure, read-only
render of `resolve_references().definitions` — a *third view* of the exact list S22 renders flat and S23
groups by kind. It reuses that list verbatim (never re-walking the body or re-resolving), so the report
can never drift from the S1 resolution it summarises, and counting never adds, drops, or reorders
definitions relative to what `resolve_references` produced. It mutates nothing and changes no S1–S24
output — including `to_latex()`'s round-trip fixed point — byte-for-byte. Like S11–S24 it is a method the
caller invokes directly.

### 31.3 The ordering rule

The output is ordered by a **fixed, document-independent** kind order — the `LabelKind` enum declaration
order: `Section`, `Table`, `Figure`, `Equation`, `Inline` (the **same** `const KIND_ORDER` slice S23/S24
use). S25 iterates that order as an explicit slice (**not** a hash map keyed by kind), so the line order
is deterministic and never depends on document content or hash iteration order — the same `Vec`-scan
discipline S17/S18/S23/S24 use to avoid hash-order nondeterminism. A kind with a **zero** count produces
**no** line (there is never a bare `table: 0` for a doc with no table labels).

### 31.4 The exact rendering contract — `Document::label_kind_counts(&self) -> String`

- Iterate the fixed kind order (`Section`, `Table`, `Figure`, `Equation`, `Inline`). For each kind, count
  the `resolve_references().definitions` whose `kind` is that kind and — only if the count is **at least
  one** — emit one line `format!("{}: {}", kind.as_str(), count)` → the kind tag + `": "` + the decimal
  count. The kind tag comes from `LabelKind::as_str()` (`"section"`/`"table"`/`"figure"`/`"equation"`/
  `"inline"`) — the **same** kind string S23 renders — and there is **no** source slicing at all (only the
  `kind` field is read; keys/spans are unused), so the render needs no source borrow and can never index
  out of bounds.
- Only the **winning** definitions are counted. `definitions` holds one row per distinct key (the first
  `\label` of each key); a `\label{dup}` written twice contributes **one** to its kind's count, because
  its later re-definition is a `Duplicate` (S20's domain), never a second row in `definitions`.
- A kind with a zero count contributes no line and no empty header.
- Lines are joined by `\n` with **no** trailing newline (matching every S11–S24 renderer).
- If there are **no** winning label definitions at all, the fixed marker `(no label definitions)` is
  returned — the **same** marker S22/S23 use (S25 counts the identical list), so the output is never the
  empty string.

Example (body defining a section label `sec:intro`, two equation labels `eq:a`/`eq:b`, and a bare inline
label `note`):

```text
section: 1
equation: 2
inline: 1
```

The `section` count leads, then `equation`, then `inline`, in the fixed kind order; the `table` and
`figure` kinds have zero definitions and so contribute no lines. This is the count companion of S23's
per-definition grouping — a third view of the one winning `definitions` list.

### 31.5 Public API (added in S25)

One new method: `Document::label_kind_counts(&self) -> String`. No existing type, field, counter, or
signature changes; `resolve_references` and every S1–S24 method are unchanged; no AST or grammar change;
no new dependency, no `unsafe`, no I/O.

### 31.6 Verification (S25)

`cargo test -p latex` green (6 new S25 tests: a section + two-equation + inline case rendering the
per-kind counts in the fixed kind order with the zero-count kinds omitted; a single-kind case
(`inline: 2`); a no-labels case returning the `(no label definitions)` marker; a duplicate-`\label` case
proving only the WINNING definition is counted (the loser routed to S20's `duplicate_label_definitions`);
a section + figure + inline case pinning the `\n`-join with no trailing newline; and an additivity check
that `to_plain_text`, `to_plain_text_by_kind`, `list_of_floats`, `resolve_namerefs`, `list_summary`,
`citations_by_source`, `duplicate_bibliography_entries`, `unresolved_citations_by_source`,
`unresolved_references_by_source`, `bibliography_entries`, `duplicate_label_definitions`,
`resolved_references_by_source`, `label_definitions`, S23's `label_definitions_by_kind`, and S24's
`resolved_references_by_kind` all still produce their exact prior strings). All prior S1–S24 tests pass
unchanged. `cargo clippy -p latex --all-targets -- -D warnings` clean; downstream `cargo test -p adj-lang
-p adj-lang-cli` green; `cargo build -p latex --no-default-features` builds. No `cargo fmt`, no grammar
regen, no new dependencies.

## 32. S26 — per-kind census (counts) of the resolved references (`resolved_reference_kind_counts`)

### 32.1 Motivation

S24 (`resolved_references_by_kind`) groups the **resolved** `\ref`/`\eqref`/`\pageref` references by the
`LabelKind` they resolved TO and renders one **line per resolved reference** (`[kind] \<command>{key}`).
But a reader often wants not the enumeration but the **tally** — "how many of my references land on
sections? how many on equations? how many on bare inline labels?" — a per-kind *count* answered at a
glance, without scanning the individual keys. S25 (`label_kind_counts`) already brought exactly this
numeric-summary discipline to the **label-definitions** family (a `<kind>: <n>` tally over the winning
`definitions` list); S26 brings the same shape to the **resolved-references** family. It is to S24 what
S25 is to S23: the *same* `resolved` list, collapsed to one line per kind.

### 32.2 Why a new method — additive by construction

S26 adds a **new public method** `Document::resolved_reference_kind_counts(&self) -> String`. It is a
pure, read-only render of `resolve_references().resolved` — a *third view* of the exact list S21 renders
flat and S24 groups by kind. It reuses that list verbatim (never re-walking the body or re-resolving), so
the report can never drift from the S1 resolution it summarises, and counting never adds, drops, or
reorders references relative to what `resolve_references` produced. It mutates nothing and changes no
S1–S25 output — including `to_latex()`'s round-trip fixed point — byte-for-byte. Like S11–S25 it is a
method the caller invokes directly.

### 32.3 The ordering rule

The output is ordered by a **fixed, document-independent** kind order — the `LabelKind` enum declaration
order: `Section`, `Table`, `Figure`, `Equation`, `Inline` (the **same** `const KIND_ORDER` slice
S23/S24/S25 use). S26 iterates that order as an explicit slice (**not** a hash map keyed by kind), so the
line order is deterministic and never depends on document content or hash iteration order — the same
`Vec`-scan discipline S17/S18/S23/S24/S25 use to avoid hash-order nondeterminism. A kind with a **zero**
count produces **no** line (there is never a bare `table: 0` for a doc that references no tables).

### 32.4 The exact rendering contract — `Document::resolved_reference_kind_counts(&self) -> String`

- Iterate the fixed kind order (`Section`, `Table`, `Figure`, `Equation`, `Inline`). For each kind, count
  the `resolve_references().resolved` refs whose `target_kind` is that kind and — only if the count is
  **at least one** — emit one line `format!("{}: {}", kind.as_str(), count)` → the kind tag + `": "` + the
  decimal count. The kind tag comes from `LabelKind::as_str()` (`"section"`/`"table"`/`"figure"`/
  `"equation"`/`"inline"`) — the **same** kind string S24 renders — and there is **no** source slicing at
  all (only the `target_kind` field is read; keys/commands/spans are unused for the count), so the render
  needs no source borrow and can never index out of bounds.
- Only the **resolved** references are counted. A dangling `\ref{nope}` lives in
  `resolve_references().unresolved` (S18's domain), never in `resolved`, so it is excluded by construction
  and never contributes a spurious `<kind>: 0` line.
- A kind with a zero count contributes no line and no empty header.
- Lines are joined by `\n` with **no** trailing newline (matching every S11–S25 renderer).
- If there are **no** resolved references at all (every reference dangles, or there are none), the fixed
  marker `(no resolved references)` is returned — the **same** marker S21/S24 use (S26 counts the
  identical list), so the output is never the empty string.

Example (body defining two section labels `sec:a`/`sec:b` and one equation label `eq:e`, then writing
`\ref{sec:a}`, `\ref{sec:b}`, and `\eqref{eq:e}`, all of which resolve):

```text
section: 2
equation: 1
```

The `section` count leads, then `equation`, in the fixed kind order; the `table`, `figure`, and `inline`
kinds have zero resolved refs and so contribute no lines. This is the count companion of S24's per-ref
grouping — a third view of the one `resolved` list.

### 32.5 Public API (added in S26)

One new method: `Document::resolved_reference_kind_counts(&self) -> String`. No existing type, field,
counter, or signature changes; `resolve_references` and every S1–S25 method are unchanged; no AST or
grammar change; no new dependency, no `unsafe`, no I/O.

### 32.6 Verification (S26)

`cargo test -p latex` green (6 new S26 tests: a section + two-equation + inline case rendering the
per-kind counts in the fixed kind order with the zero-count kinds omitted; a single-kind case
(`section: 2`, two refs to one section); a two-section-labels case proving multiple resolved refs to the
same kind aggregate (`section: 2`); an all-dangling / no-references case returning the
`(no resolved references)` marker (cross-checked against S18's `unresolved_references_by_source`); a
section + equation + dangling-ref case pinning the `\n`-join with no trailing newline and the dangling
ref's exclusion; and an additivity check that `to_plain_text`, `to_plain_text_by_kind`, `list_of_floats`,
`resolve_namerefs`, `list_summary`, `citations_by_source`, `duplicate_bibliography_entries`,
`unresolved_citations_by_source`, `unresolved_references_by_source`, `bibliography_entries`,
`duplicate_label_definitions`, `resolved_references_by_source`, `label_definitions`, S23's
`label_definitions_by_kind`, S24's `resolved_references_by_kind`, and S25's `label_kind_counts` all still
produce their exact prior strings). All prior S1–S25 tests pass unchanged. `cargo clippy -p latex
--all-targets -- -D warnings` clean; downstream `cargo test -p adj-lang -p adj-lang-cli` green; `cargo
build -p latex --no-default-features` builds. No `cargo fmt`, no grammar regen, no new dependencies.

## 33. S27 — single-integer total of the unresolved (dangling) references (`unresolved_reference_count`)

### 33.1 Motivation

S18 (`unresolved_references_by_source`) enumerates the **unresolved** (dangling) `\ref`/`\eqref`/`\pageref`
references — the ones no `\label` defines (LaTeX's *"Reference `key' undefined"*, the `??`) — one **line
per dangling reference** (`\<command>{key}`, in body pre-order). But a reader often wants not the
enumeration but the **total** — "how many of my references dangle?" — a single number answered at a
glance, without scanning the individual keys. S25 (`label_kind_counts`) and S26
(`resolved_reference_kind_counts`) already brought a numeric-summary discipline to the label-definitions
and resolved-references families, respectively, as *per-kind* censuses. But the unresolved refs carry
**no** `target_kind` — a dangling ref bound to no definition, so there is nothing to group by; a per-kind
census is not viable. The clean move is a **single total**: S27 collapses the whole `unresolved` list to
its decimal `.len()`. It is the count-total sibling of the census family, but for the UNRESOLVED refs.

### 33.2 Why a new method — additive by construction

S27 adds a **new public method** `Document::unresolved_reference_count(&self) -> String`. It is a pure,
read-only render of `resolve_references().unresolved.len()` — a *second view* of the exact list S18
renders per-source. It reuses that list verbatim (never re-walking the body or re-resolving), so the
report can never drift from the S1 resolution it summarises, and counting never adds, drops, or reorders
references relative to what `resolve_references` produced. It mutates nothing and changes no S1–S26 output
— including `to_latex()`'s round-trip fixed point — byte-for-byte. Like S11–S26 it is a method the caller
invokes directly.

### 33.3 The rendering rule

The output is the decimal `.len()` of the `unresolved` list, rendered as its `String`, **always** on a
single line with **no** trailing newline. There is no ordering question (a single integer has no order)
and no per-kind grouping (a dangling ref carries no `target_kind` to group by) — only `.len()` is read,
with **no** source slicing at all. Being a **count** renderer, its empty case is the honest number `"0"`
— **not** a `(no …)` marker. The `(no …)` marker discipline belongs to the *list* renderers
S18/S21/S24, whose empty case has no lines to show; a total count of zero *is* a number, so `"0"` is its
truthful value.

### 33.4 The exact rendering contract — `Document::unresolved_reference_count(&self) -> String`

- Read `resolve_references().unresolved.len()` and render it with `.to_string()` → the decimal count, one
  line, no trailing newline. There is **no** source slicing and **no** `target_kind` read at all (a
  dangling ref never carries one), so the render needs no source borrow and can never index out of bounds.
- Only the **unresolved** references are counted. A resolved `\ref{sec:i}` lives in
  `resolve_references().resolved` (S21's domain), never in `unresolved`, so it is excluded by construction.
- The empty case (every ref resolves, or there are none at all) returns the honest number `"0"` — **not**
  a `(no …)` marker, because S27 is a count renderer.

Example (body defining `\label{sec:i}`, then writing `\ref{sec:i}` (resolves), `\ref{nope}` (dangles), and
`\ref{gone}` (dangles)):

```text
2
```

Two references dangle; the one resolved `\ref{sec:i}` is excluded. This is the count-total companion of
S18's per-source list — a second view of the one `unresolved` list; the count equals the number of lines
S18 would enumerate.

### 33.5 Public API (added in S27)

One new method: `Document::unresolved_reference_count(&self) -> String`. No existing type, field, counter,
or signature changes; `resolve_references` and every S1–S26 method are unchanged; no AST or grammar
change; no new dependency, no `unsafe`, no I/O.

### 33.6 Verification (S27)

`cargo test -p latex` green (6 new S27 tests: two-danglers-plus-one-resolved returning `"2"`
(cross-checked against S18's `unresolved_references_by_source`); an all-refs-resolve case returning `"0"`
(cross-checked against S18's `(no unresolved references)` marker); a no-references-at-all case returning
`"0"`; a mixed-kinds-of-danglers case (`\ref`/`\eqref`/`\pageref`, none defined) returning the integer
`"3"`; a cross-check that the count equals the number of lines S18 enumerates (`"4"`); and an additivity
check that a handful of prior renderers — `list_summary`, `bibliography_entries`, `label_definitions`,
`unresolved_references_by_source`, S25's `label_kind_counts`, and S26's `resolved_reference_kind_counts` —
all still produce their exact prior strings, with S27 returning `"1"` (agreeing with the single line S18
enumerates)). All prior S1–S26 tests pass unchanged. No `cargo fmt`, no grammar regen, no new
dependencies.

## 34. S28 — single-integer total of the resolved references (`resolved_reference_count`)

### 34.1 Motivation

S21 (`resolved_references_by_source`) and S24 (`resolved_references_by_kind`) enumerate the **resolved**
`\ref`/`\eqref`/`\pageref` references — the ones some `\label` defines — one **line per resolved
reference** (`\<command>{key}`, flat in body pre-order or grouped by the target kind each ref bound to).
But a reader often wants not the enumeration but the **total** — "how many of my references resolve?" — a
single number answered at a glance, without scanning the individual keys. S27
(`unresolved_reference_count`) just gave that single-total discipline to the *unresolved* (dangling) side;
S28 is its exact resolved-side **twin**: it collapses the whole `resolved` list to its decimal `.len()`.
Together S28 + S27 split every reference into the pair (resolved, dangling), so their two totals sum to the
total reference count. Unlike S26 (`resolved_reference_kind_counts`, which *can* census the resolved refs
by the `target_kind` each bound to), S28 takes no per-kind view: section, table, and equation references
all fold into one total — a single number is the clean move.

### 34.2 Why a new method — additive by construction

S28 adds a **new public method** `Document::resolved_reference_count(&self) -> String`. It is a pure,
read-only render of `resolve_references().resolved.len()` — a *second view* of the exact list S21/S24
render per-source and per-kind. It reuses that list verbatim (never re-walking the body or re-resolving),
so the report can never drift from the S1 resolution it summarises, and counting never adds, drops, or
reorders references relative to what `resolve_references` produced. It mutates nothing and changes no
S1–S27 output — including `to_latex()`'s round-trip fixed point — byte-for-byte. Like S11–S27 it is a
method the caller invokes directly.

### 34.3 The rendering rule

The output is the decimal `.len()` of the `resolved` list, rendered as its `String`, **always** on a
single line with **no** trailing newline. There is no ordering question (a single integer has no order)
and no per-kind grouping (S28 reads only the length, never a `target_kind`) — only `.len()` is read, with
**no** source slicing at all. Being a **count** renderer, its empty case is the honest number `"0"` —
**not** a `(no resolved references)` marker, mirroring S27 exactly. The `(no …)` marker discipline belongs
to the *list* renderers S21/S24, whose empty case has no lines to show; a total count of zero *is* a
number, so `"0"` is its truthful value.

### 34.4 The exact rendering contract — `Document::resolved_reference_count(&self) -> String`

- Read `resolve_references().resolved.len()` and render it with `.to_string()` → the decimal count, one
  line, no trailing newline. There is **no** source slicing and **no** `target_kind` read at all, so the
  render needs no source borrow and can never index out of bounds.
- Only the **resolved** references are counted. A dangling `\ref{nope}` lives in
  `resolve_references().unresolved` (S18/S27's domain), never in `resolved`, so it is excluded by
  construction.
- The empty case (every ref dangles, or there are none at all) returns the honest number `"0"` — **not**
  a `(no resolved references)` marker, because S28 is a count renderer (mirroring S27).

Example (body defining `\label{sec:i}`, then writing `\ref{sec:i}` (resolves), `\pageref{sec:i}`
(resolves), and `\ref{nope}` (dangles)):

```text
2
```

Two references resolve; the one dangling `\ref{nope}` is excluded. This is the count-total companion of
S21's per-source list and S24's per-kind grouping — a second view of the one `resolved` list; the count
equals the number of lines S21 would enumerate.

### 34.5 Public API (added in S28)

One new method: `Document::resolved_reference_count(&self) -> String`. No existing type, field, counter,
or signature changes; `resolve_references` and every S1–S27 method are unchanged; no AST or grammar
change; no new dependency, no `unsafe`, no I/O.

### 34.6 Verification (S28)

`cargo test -p latex` green (6 new S28 tests: two-resolved-plus-one-dangling returning `"2"`
(cross-checked against S21's `resolved_references_by_source`, and against S27 returning `"1"` on the same
doc — the two totals split the references); a no-resolvable-refs case returning `"0"` (cross-checked
against S21's `(no resolved references)` marker); a no-references-at-all case returning `"0"`; a
mixed-target-kinds case (section/table/equation) returning the integer `"3"`; a cross-check that the count
equals the number of lines S21 enumerates (`"3"`); and an additivity check that a handful of prior
renderers — `list_summary`, `label_definitions`, `resolved_references_by_source`, S25's `label_kind_counts`,
S26's `resolved_reference_kind_counts`, and S27's `unresolved_reference_count` — all still produce their
exact prior strings, with S28 returning `"2"` (agreeing with the two lines S21 enumerates)). All prior
S1–S27 tests pass unchanged. No `cargo fmt`, no grammar regen, no new dependencies.

## 35. S29 — single-integer total of the label definitions (`label_definition_count`)

### 35.1 Motivation

S22 (`label_definitions`) and S23 (`label_definitions_by_kind`) enumerate the **winning** label
definitions — the distinct `\label` keys the document defines, the table `\ref`/`\eqref`/`\pageref`
resolve against — one **line per winning definition** (`\label{key}`, flat in body pre-order or grouped by
the `LabelKind` each label defines). But a reader often wants not the enumeration but the **total** — "how
many labels does this document define?" — a single number answered at a glance, without scanning the
individual keys. S27 (`unresolved_reference_count`) and S28 (`resolved_reference_count`) gave that
single-total discipline to the two *reference* tables; S29 is their label-definition-side **analogue**: it
collapses the whole winning `definitions` list to its decimal `.len()`. Unlike S25 (`label_kind_counts`,
which *can* census the definitions by the `LabelKind` each defines), S29 takes no per-kind view: section,
figure, equation, and inline labels all fold into one total — a single number is the clean move.

### 35.2 Why a new method — additive by construction

S29 adds a **new public method** `Document::label_definition_count(&self) -> String`. It is a pure,
read-only render of `resolve_references().definitions.len()` — a *second view* of the exact list S22/S23
render flat and per-kind. It reuses that list verbatim (never re-walking the body or re-resolving), so the
report can never drift from the S1 resolution it summarises, and counting never adds, drops, or reorders
definitions relative to what `resolve_references` produced. It mutates nothing and changes no S1–S28
output — including `to_latex()`'s round-trip fixed point — byte-for-byte. Like S11–S28 it is a method the
caller invokes directly.

### 35.3 The rendering rule

The output is the decimal `.len()` of the winning `definitions` list, rendered as its `String`, **always**
on a single line with **no** trailing newline. There is no ordering question (a single integer has no
order) and no per-kind grouping (S29 reads only the length, never a `kind`) — only `.len()` is read, with
**no** source slicing at all. Being a **count** renderer, its empty case is the honest number `"0"` —
**not** a `(no label definitions)` marker, mirroring S27/S28 exactly. The `(no …)` marker discipline
belongs to the *list* renderers S22/S23, whose empty case has no lines to show; a total count of zero *is*
a number, so `"0"` is its truthful value.

### 35.4 The exact rendering contract — `Document::label_definition_count(&self) -> String`

- Read `resolve_references().definitions.len()` and render it with `.to_string()` → the decimal count, one
  line, no trailing newline. There is **no** source slicing and **no** `kind` read at all, so the render
  needs no source borrow and can never index out of bounds.
- Only the **winning** definitions are counted. A later re-definition `\label{dup}` of an already-defined
  key lives in `resolve_references().duplicates` (S20's domain), never in `definitions`, so it is excluded
  by construction — the count is exactly the number of lines S22 lists.
- The empty case (no `\label` at all) returns the honest number `"0"` — **not** a `(no label definitions)`
  marker, because S29 is a count renderer (mirroring S27/S28).

Example (body defining `\label{sec:intro}` (a section), `\label{eq:main}` (an equation), and then re-using
`\label{sec:intro}` on a later subsection (a duplicate)):

```text
2
```

Two distinct keys are defined; the later duplicate `\label{sec:intro}` is excluded. This is the
count-total companion of S22's flat list and S23's per-kind grouping — a second view of the one winning
`definitions` list; the count equals the number of lines S22 would enumerate.

### 35.5 Public API (added in S29)

One new method: `Document::label_definition_count(&self) -> String`. No existing type, field, counter, or
signature changes; `resolve_references` and every S1–S28 method are unchanged; no AST or grammar change; no
new dependency, no `unsafe`, no I/O.

### 35.6 Verification (S29)

`cargo test -p latex` green (6 new S29 tests: a multiple-definitions case returning `"3"` (cross-checked
against S22's `label_definitions`); a duplicate-label case where the later `\label{dup}` is a duplicate
(S20's domain), not a second definition, so the count is the number of distinct keys `"2"` (cross-checked
against S20's `duplicate_label_definitions` and against the number of lines S22 lists); a no-labels case
returning `"0"` (cross-checked against S22's `(no label definitions)` marker); a mixed-document case
(labels + refs + citations) where the count `"3"` counts only the label definitions, unaffected by the
refs/citations; a cross-check that the count equals the number of lines S22 enumerates (`"3"`); and an
additivity check that a handful of prior renderers — S22's `label_definitions`, S25's `label_kind_counts`,
S27's `unresolved_reference_count`, and S28's `resolved_reference_count` — all still produce their exact
prior strings, with S29 returning `"4"` (agreeing with the four lines S22 enumerates)). All prior S1–S28
tests pass unchanged. No `cargo fmt`, no grammar regen, no new dependencies.

## 36. S30 — single-integer total of the bibliography entries (`bibliography_entry_count`)

### 36.1 Motivation

S19 (`bibliography_entries`) enumerates the **winning** bibliography entries — the distinct `\bibitem` keys
the document defines inside a `thebibliography` environment, the table `\cite` resolves against — one
**line per winning entry** (`[n] key`, 1-based in body pre-order). But a reader often wants not the
enumeration but the **total** — "how many bibliography entries does this document define?" — a single
number answered at a glance, without scanning the individual keys. S27 (`unresolved_reference_count`) and
S28 (`resolved_reference_count`) gave that single-total discipline to the two *reference* tables, and S29
(`label_definition_count`) gave it to the label-definition table; S30 is the **citation-side analogue** of
S29, completing the *totals family*: it collapses the whole winning `entries` list to its decimal `.len()`.
Where S29 counts the label definitions, S30 counts the bibliography entries — the last of the four total
renderers (S27/S28 references, S29 labels, S30 bibliography).

### 36.2 Why a new method — additive by construction

S30 adds a **new public method** `Document::bibliography_entry_count(&self) -> String`. It is a pure,
read-only render of `resolve_citations().entries.len()` — a *second view* of the exact list S19 renders
flat. It reuses that list verbatim (never re-walking the body or re-resolving), so the report can never
drift from the S2 resolution it summarises, and counting never adds, drops, or reorders entries relative to
what `resolve_citations` produced. It mutates nothing and changes no S1–S29 output — including
`to_latex()`'s round-trip fixed point — byte-for-byte. Like S11–S29 it is a method the caller invokes
directly.

### 36.3 The rendering rule

The output is the decimal `.len()` of the winning `entries` list, rendered as its `String`, **always** on a
single line with **no** trailing newline. There is no ordering question (a single integer has no order) —
only `.len()` is read, with **no** source slicing at all. Being a **count** renderer, its empty case is the
honest number `"0"` — **not** a `(no bibliography entries)` marker, mirroring S27/S28/S29 exactly. The
`(no …)` marker discipline belongs to the *list* renderer S19, whose empty case has no lines to show; a
total count of zero *is* a number, so `"0"` is its truthful value.

### 36.4 The exact rendering contract — `Document::bibliography_entry_count(&self) -> String`

- Read `resolve_citations().entries.len()` and render it with `.to_string()` → the decimal count, one line,
  no trailing newline. There is **no** source slicing at all, so the render needs no source borrow and can
  never index out of bounds.
- Only the **winning** entries are counted. A later re-definition `\bibitem{dup}` of an already-defined key
  lives in `resolve_citations().duplicate_entries` (S16's domain), never in `entries`, so it is excluded by
  construction — the count is exactly the number of lines S19 lists.
- The empty case (no `\bibitem` at all) returns the honest number `"0"` — **not** a
  `(no bibliography entries)` marker, because S30 is a count renderer (mirroring S27/S28/S29).

Example (a `thebibliography` with `\bibitem{a}`, `\bibitem{b}`, `\bibitem{c}`, and then a re-used
`\bibitem{a}` (a duplicate)):

```text
3
```

Three distinct keys are defined; the later duplicate `\bibitem{a}` is excluded. This is the count-total
companion of S19's flat list — a second view of the one winning `entries` list; the count equals the number
of lines S19 would enumerate.

### 36.5 Public API (added in S30)

One new method: `Document::bibliography_entry_count(&self) -> String`. No existing type, field, counter, or
signature changes; `resolve_citations` and every S1–S29 method are unchanged; no AST or grammar change; no
new dependency, no `unsafe`, no I/O.

### 36.6 Verification (S30)

`cargo test -p latex` green (5 new S30 tests: a multiple-entries case returning `"3"` (cross-checked against
S19's `bibliography_entries`); a single-entry case returning `"1"`; a duplicate-bibitem case where the later
`\bibitem{dup}` is a duplicate (S16's domain), not a second entry, so the count is the number of distinct
keys `"2"` (cross-checked against S16's `duplicate_bibliography_entries` and against the number of lines S19
lists); a no-entries case returning `"0"` (cross-checked against S19's `(no bibliography entries)` marker);
and an additivity check that a handful of prior renderers — S19's `bibliography_entries`, S22's
`label_definitions`, S27's `unresolved_reference_count`, S28's `resolved_reference_count`, and S29's
`label_definition_count` — all still produce their exact prior strings, with S30 returning `"3"` (agreeing
with the three lines S19 enumerates)). All prior S1–S29 tests pass unchanged. No `cargo fmt`, no grammar
regen, no new dependencies.

## 37. S31 — single-integer total of the resolved citations (`citation_count`)

### 37.1 Motivation

S15 (`citations_by_source`) enumerates the **resolved** citations — the `\cite` keys some `\bibitem` defines
— grouped by their source `\cite`. But a reader often wants not the enumeration but the **total** — "how many
citations resolve in this document?" — a single number answered at a glance, without scanning the individual
keys. S27 (`unresolved_reference_count`) and S28 (`resolved_reference_count`) gave that single-total
discipline to the two *reference* tables, S29 (`label_definition_count`) to the label-definition table, and
S30 (`bibliography_entry_count`) to the bibliography table; S31 is the exact resolved-**citation-side twin**
of S28, extending the *totals family* onto the resolved-citation table: it collapses the whole `resolved`
list to its decimal `.len()`. Where S28 counts the resolved references, S31 counts the resolved citations —
the citation-side analogue that pairs with S30 (bibliography entries) to summarise the two citation tables.

### 37.2 Why a new method — additive by construction

S31 adds a **new public method** `Document::citation_count(&self) -> String`. It is a pure, read-only render
of `resolve_citations().resolved.len()` — a *second view* of the exact list S15 renders per-source. It
reuses that list verbatim (never re-walking the body or re-resolving), so the report can never drift from the
S2 resolution it summarises, and counting never adds, drops, or reorders citations relative to what
`resolve_citations` produced. It mutates nothing and changes no S1–S30 output — including `to_latex()`'s
round-trip fixed point — byte-for-byte. Like S11–S30 it is a method the caller invokes directly.

### 37.3 The rendering rule

The output is the decimal `.len()` of the `resolved` list, rendered as its `String`, **always** on a single
line with **no** trailing newline. There is no ordering question (a single integer has no order) — only
`.len()` is read, with **no** source slicing at all. Being a **count** renderer, its empty case is the honest
number `"0"` — **not** a `(no resolved citations)` marker, mirroring S27/S28/S29/S30 exactly. The `(no …)`
marker discipline belongs to the *list* renderer S15, whose empty case has no lines to show; a total count of
zero *is* a number, so `"0"` is its truthful value.

### 37.4 The exact rendering contract — `Document::citation_count(&self) -> String`

- Read `resolve_citations().resolved.len()` and render it with `.to_string()` → the decimal count, one line,
  no trailing newline. There is **no** source slicing at all, so the render needs no source borrow and can
  never index out of bounds.
- Only the **resolved** keys are counted. A dangling `\cite{ghost}` (no matching `\bibitem`) lives in
  `resolve_citations().unresolved` (S17's domain), never in `resolved`, so it is excluded by construction. A
  multi-key `\cite{a,b}` contributes one record per resolved key; `cite_span`/`entry_span` are never read.
- The empty case (every cited key dangling, or no `\cite` at all) returns the honest number `"0"` — **not** a
  `(no resolved citations)` marker, because S31 is a count renderer (mirroring S27/S28/S29/S30).

Example (a body `\cite{a,b}` (both defined) then `\cite{c,ghost}` (only `c` defined), against a bibliography
defining `a`, `b`, `c`):

```text
3
```

Three keys resolve (`a`, `b`, `c`); the one dangling `ghost` is excluded. This is the count-total companion
of S15's per-source list — a second view of the one `resolved` list.

### 37.5 Public API (added in S31)

One new method: `Document::citation_count(&self) -> String`. No existing type, field, counter, or signature
changes; `resolve_citations` and every S1–S30 method are unchanged; no AST or grammar change; no new
dependency, no `unsafe`, no I/O.

### 37.6 Verification (S31)

`cargo test -p latex` green (6 new S31 tests: a multiple-resolved case returning `"3"`; a single-resolved
case returning `"1"`; a dangling-key case where `\cite{a, ghost}` counts only the one resolved key `"1"` (the
`ghost` excluded, S17's domain); a no-citations case returning `"0"` (cross-checked against S15's
`(no resolved citations)` marker); an every-key-dangling case returning `"0"`; and an additivity check that
the prior totals-family renderers — S27's `unresolved_reference_count`, S28's `resolved_reference_count`,
S29's `label_definition_count`, and S30's `bibliography_entry_count` — all still produce their exact prior
strings, with S31 returning `"3"` for `\cite{a,b}` plus `\cite{c,ghost}`). All prior S1–S30 tests pass
unchanged. No `cargo fmt`, no grammar regen, no new dependencies.

## 38. S32 — single-integer total of the unresolved (dangling) citations (`unresolved_citation_count`)

### 38.1 Motivation

S17 (`unresolved_citations_by_source`) enumerates the **unresolved** (dangling) citations — the `\cite` keys
**no** `\bibitem` defines — grouped by their source `\cite`. But a reader often wants not the enumeration but
the **total** — "how many citations dangle in this document?" — a single number answered at a glance, without
scanning the individual keys. S27 (`unresolved_reference_count`) gave that single-total discipline to the
*dangling* side of the reference family, and S31 (`citation_count`) gave it to the *resolved* side of the
citation family; S32 is the exact unresolved-**citation-side twin** of S27, and the **dangling sibling** of
S31, closing the totals family over the citation family: it collapses the whole `unresolved` list to its
decimal `.len()`. Where S27 counts the dangling references, S32 counts the dangling citations. Together S31
and S32 **partition** every per-key `\cite` record — the resolved count plus the dangling count equals the
total number of cited keys, because S2's `resolve_citations` routes each key into exactly one of
`resolved`/`unresolved`.

### 38.2 Why a new method — additive by construction

S32 adds a **new public method** `Document::unresolved_citation_count(&self) -> String`. It is a pure,
read-only render of `resolve_citations().unresolved.len()` — a *second view* of the exact list S17 renders
per-source. It reuses that list verbatim (never re-walking the body or re-resolving), so the report can never
drift from the S2 resolution it summarises, and counting never adds, drops, or reorders citations relative to
what `resolve_citations` produced. It mutates nothing and changes no S1–S31 output — including `to_latex()`'s
round-trip fixed point — byte-for-byte. Like S11–S31 it is a method the caller invokes directly.

### 38.3 The rendering rule

The output is the decimal `.len()` of the `unresolved` list, rendered as its `String`, **always** on a single
line with **no** trailing newline. There is no ordering question (a single integer has no order) — only
`.len()` is read, with **no** source slicing at all. Being a **count** renderer, its empty case is the honest
number `"0"` — **not** a `(no unresolved citations)` marker, mirroring S27/S28/S29/S30/S31 exactly. The
`(no …)` marker discipline belongs to the *list* renderer S17, whose empty case has no lines to show; a total
count of zero *is* a number, so `"0"` is its truthful value.

### 38.4 The exact rendering contract — `Document::unresolved_citation_count(&self) -> String`

- Read `resolve_citations().unresolved.len()` and render it with `.to_string()` → the decimal count, one
  line, no trailing newline. There is **no** source slicing at all, so the render needs no source borrow and
  can never index out of bounds.
- Only the **dangling** keys are counted. A resolved `\cite{a}` (a matching `\bibitem` defines it) lives in
  `resolve_citations().resolved` (S15/S31's domain), never in `unresolved`, so it is excluded by
  construction. A multi-key `\cite{a,b}` contributes one record per dangling key; the `cite_span` and the
  dangling `key` are never read.
- The empty case (every cited key resolving, or no `\cite` at all) returns the honest number `"0"` — **not** a
  `(no unresolved citations)` marker, because S32 is a count renderer (mirroring S27/S28/S29/S30/S31).

Example (a body `\cite{a,b}` (both defined) then `\cite{c,ghost}` (only `c` defined), against a bibliography
defining `a`, `b`, `c`):

```text
1
```

One key dangles (`ghost`); the three resolved keys `a`, `b`, `c` are excluded (they are the `3` S31 reports).
This is the count-total companion of S17's per-source list — a second view of the one `unresolved` list.

### 38.5 Public API (added in S32)

One new method: `Document::unresolved_citation_count(&self) -> String`. No existing type, field, counter, or
signature changes; `resolve_citations` and every S1–S31 method are unchanged; no AST or grammar change; no
new dependency, no `unsafe`, no I/O.

### 38.6 Verification (S32)

`cargo test -p latex` green (7 new S32 tests: a multiple-dangling case returning `"3"`; a single-dangling
case returning `"1"`; a mixed-key case where `\cite{a, ghost}` counts only the one dangling key `"1"` (the
resolved `a` excluded, S15/S31's domain); a no-citations case returning `"0"` (cross-checked against S17's
`(no unresolved citations)` marker); an every-key-resolving case returning `"0"`; a partition check that S31
+ S32 sum to the total cited keys; and an additivity check that the prior totals-family renderers — S27's
`unresolved_reference_count`, S28's `resolved_reference_count`, S29's `label_definition_count`, S30's
`bibliography_entry_count`, and S31's `citation_count` — together with two representative earlier *list*
renderers (S19's `bibliography_entries` and S22's `label_definitions`) all still produce their exact prior
strings, with S32 returning `"1"` for `\cite{a,b}` plus `\cite{c,ghost}`). All prior S1–S31 tests pass
unchanged. No `cargo fmt`, no grammar regen, no new dependencies.
