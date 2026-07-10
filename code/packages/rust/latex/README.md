# latex

A **full-fidelity LaTeX parser** (documents *and* math, not a math subset), built as a
standalone, reusable Rust crate. It turns LaTeX source into a faithful AST that any
consumer can use — a reasoning engine, a computer-algebra system, a renderer. It is the
first frontend of the pluggable [`math-frontend`](../math-frontend) framework (it will
implement `MathFrontend` in a later layer) and is useful on its own.

Spec: [`code/specs/LTX01-full-latex-parser.md`](../../../specs/LTX01-full-latex-parser.md);
framework: [`code/specs/PFE01-pluggable-parser-frontends.md`](../../../specs/PFE01-pluggable-parser-frontends.md).

## Honest scope

LaTeX rests on TeX, whose macro layer is Turing-complete. This crate parses the full LaTeX
**surface** and supports the macro mechanisms authors actually use (`\newcommand`/`\def`).
The programmable TeX tail — runtime `\catcode` reassignment, `\expandafter`/`\csname`,
`\if…` programming, external `\input` — is the **documented asymptote**, surfaced as an
explicit "unsupported" node rather than mis-parsed.

## Why a catcode state machine

A character's meaning in LaTeX depends on its **category code** and the scanner's state:
`\` begins a control sequence, `%` skips to end of line, `$` toggles math, a blank line is
a paragraph break. The tokenizer is therefore a hand-written, catcode-driven state machine
with a **text-mode-primary** mode stack (LaTeX starts in text mode; math is entered by
`$`/`\(`/`\[`/`$$`) — the inverse of a math-only tokenizer. (This mirrors how
`grammar-tools` hand-writes its own `.tokens` scanner; the pattern is established here.)

## Status / roadmap (conformance ladder)

| Layer | Contents | Status |
|-------|----------|--------|
| **L0 tokenizer** | catcode state machine → flat `Token` stream w/ byte spans | ✅ |
| **L1 structural** | groups, `\cmd[opt]{arg}`, `\begin{env}…\end{env}`, text runs, raw math islands, `to_latex()` round-trip | ✅ |
| **L2 math** | math AST (frac, binom, roots, scripts, big ops, functions, accents, `\left\right` fences, relations, `\overset`/`\underset`/`\stackrel` stacking, `\xrightarrow`-family extensible labelled arrows, `\overbrace`/`\underbrace` horizontal braces, and `\overrightarrow`/`\underrightarrow`-family stretchy over/under-arrow accents), precedence-climbing parser, `to_latex()` round-trip | ✅ |
| **L3 environments** | math env family — `matrix`/`pmatrix`/`bmatrix`/`vmatrix`/`cases`/`aligned`/`align` plus `array`/`subarray` (mandatory `{col-spec}`) split on `&` and `\\` → `MathNode::Matrix`, round-trip; nesting + scripts | ✅ |
| **L4 macros** | `\newcommand`/`\renewcommand`/`\providecommand` with positional `#1`..`#9`; bounded recursive expansion via `expand()` (L4a) | ✅ |
| **L5 text breadth** | `\verb`/`verbatim` raw (L5a/b) + text accents `\'e`/`\c{c}` via `recognize_accents` (L5c) + sectioning/refs/preamble/font via `recognize_structure` (L5d) | ✅ |
| **L6 frontend** | `LatexMath` implements `math-frontend::MathFrontend` — lifts `MathNode` → neutral `MathExpr`; LaTeX is plugin #1 via `registry()` (default-on `frontend` feature) | ✅ |
| **D1 doc tables/lists** | document-mode `tabular`/`tabular*` grids (split on `&`/`\\`) → `NodeKind::Tabular` and `itemize`/`enumerate`/`description` (split on `\item`) → `NodeKind::List`, via the opt-in `recognize_tables` pass; total, round-trip | ✅ |
| **D2 Document skeleton** | hierarchical `Document` model: preamble/body split at `\begin{document}`, `\documentclass`/`\usepackage` classified, body lowered to a **flat** `Vec<Block>` (headings → zero-body `Block::Section`; paragraphs/lists/tables/display-math/environments; inline runs → `Vec<Inline>`); `parse_document`/`build_document` + `Document::to_latex` round-trip; body spans precise per node as of 0.34.0 (S3) | ✅ |
| **D3 sectioning fold** | folds the flat block stream into the **nested sectioning forest**: each heading OWNS the run of following blocks up to the next heading of same-or-higher level (`\part > \chapter > \section > … > \subparagraph`, via `rank(level)`); deeper headings nest. A trailing `\label{key}` is hoisted onto its section's new `label` field. Applies to every block-list (top-level + environment/list/table bodies). Total & panic-free; `to_latex` fixed point; `flatten(fold(flat)) == flat` property test; folded-section span = union of heading ∪ children spans | ✅ |
| **D4 metadata** | extracts `\title`/`\author{A \and B}`/`\date` and the `abstract` env into a typed `Metadata` record on `Document`, as an **additive projection** — the underlying nodes stay in `preamble`/`body`, so `to_latex` round-trips unchanged and re-parsing repopulates the same `Metadata` (fixed point). Both preamble and body scanned; first title/date wins; every `\author` (each `\and`-split) contributes; `\maketitle` is a metadata no-op. Total & panic-free; never fabricated. (Inline normalization — `\textbf`/`\emph`/`\texttt`/`$…$`/`\ref`/accents — already lands in D2/D3's `lower_inline`.) | ✅ |
| **D5 floats/code/display-math** | specializes the generic environment fold by name: `figure`/`figure*` → `Block::Figure` and `table`/`table*` → the inner `Block::Table`, each with `\caption{…}` → `Caption` + a hoisted `\label`; `verbatim`/`lstlisting` → `Block::CodeBlock` (raw text kept unparsed); `equation`/`align`/`gather`/… → `Block::DisplayMath` (source kept, delegated to the math frontend on demand); `quote`/`quotation` → `Block::Quote`; any other env → recursed `Block::Environment`. `to_latex` fixed point; total & panic-free; body spans precise per node as of 0.34.0 (S3) | ✅ |
| **D6 provenance API (capstone)** | the byte-provenance surface: `Document::walk()` — a pre-order, depth-first `impl Iterator<Item = NodeRef>` over every body block + nested inline; `Document::node_at(byte)` — the innermost walked node whose span contains a source byte, returned as `Provenance { node, span }`; `NodeRef::{span,kind}`. A capstone real-paper corpus (article + abstract + tabular + itemize + inline/display math + figure+caption+label + `\cite`) proves a `to_latex` fixed point, a non-panicking `walk()`, and **byte coverage**: every non-whitespace body-region byte is owned by ≥1 walked node. Panic-free (`saturating_sub`); spans region-coarse at D6, made precise by LTXDOC02 S3, and `node_at` resolves to the true body leaf as of S4 (below). | ✅ |
| **LTXDOC02 S1 — spanned L1 nodes** | `Node` restructured to `{ kind: NodeKind, span: Span }`; `parse()` threads each token's exact byte span onto the node it builds, so `&src[node.span()]` slices back to the node's own source (`\textbf{x}`, `{…}` incl. braces, `$…$` incl. delimiters, `\begin{env}…\end{env}`, a `Text` run's exact chars). Span is orthogonal to shape; `PartialEq` ignores it (round-trip = fixed point **modulo spans**); `Unsupported`'s bespoke tuple folded onto the uniform `Node.span`. `to_latex` unchanged. The one parser-level rung of the precise-spans arc (recognition-pass + Document-fold precision = S2/S3). | ✅ |
| **LTXDOC02 S2 — spanned recognition passes** | the opt-in recognition passes now give each *synthesised* node the exact union of its constituents' real S1 spans: `recognize_structure` (`Section`/`CrossRef`/`Preamble`/`Styled` = recognizing command ∪ each argument), `recognize_accents` (`Accent` = command ∪ argument), `recognize_tables` (`Tabular` = `\begin{tabular}…\end{tabular}` ∪ every cell; `List` = `\begin{env}…\end{env}` ∪ every item). `&src[node.span()]` slices back to the exact source extent for a Section (heading through owned body), an Accent, a Tabular, and an itemize List. Unions over real child spans (never substring search); `to_latex` + round-trip-modulo-spans unchanged. Document-fold precision = S3. | ✅ |
| **LTXDOC02 S3 — precise Document fold** | `build_document` reads each source `Node`'s carried, precise span instead of the coarse enclosing `region`, so **every body `Block`/`Inline` span is now the node's tight source range**: `&src[inline.span]` slices back to exactly a `Text` run's word, a `\textbf{…}`, an inline `$…$`, a `\cite{…}`. Composites union their children's real spans (`Paragraph` = ∪ its inlines; `Section` = heading ∪ owned body; `List`/`Tabular`/`Environment`/`Figure` = the `\begin…\end` extent; captioned `table` float = tabular ∪ float; `DocListItem` = term ∪ body; `Caption` = ∪ its content). The `region` parameter is deleted from the fold helpers (a fallback-only seed survives on `lower_blocks`). Preamble/`DocumentClass`/`Package` stay honestly preamble-region-coarse (classified, not walked). `to_latex` + round-trip-modulo-spans unchanged; precise `node_at` + coverage capstone = S4/S5. | ✅ |
| **LTXDOC02 S4 — precise `node_at`, region-coarse caveat retired** | with S3's tight body spans, `Document::node_at(byte)` **formally** resolves to the **true per-token leaf** — the narrowest node whose *precise* span contains the byte (ties → deepest in pre-order): a byte inside `widgets` → the `Text` run owning `widgets` (not the enclosing `Paragraph`/`Section`); a byte inside a `\section` title → the title inline (not the whole `Section`). Docs-and-tests rung (no `node_at`/`walk` logic change): retired the region-coarse hedging on `node_at`/`Provenance`/`walk`/module note for **body** nodes, kept the honest coarse note on `Preamble`/`DocumentClass`/`Package`. New leaf-resolution tests + an honest body byte-coverage test (every non-whitespace body byte resolves to a node whose precise span contains it, on a representative input; whole-corpus tightest-leaf capstone = S5). | ✅ |
| **LTXDOC02 S5 — precise byte-coverage capstone (arc COMPLETE)** | the capstone `capstone_every_body_byte_resolves_to_tightest_covering_node` proves, over the same LTXDOC01 D6 representative corpus, that **every** non-whitespace body byte (a) resolves (`node_at(b).is_some()`) AND (b) resolves to the **tightest-covering** walked node — no *other* walked node whose span is a strict subset also contains the byte. Honest, not overclaimed: the load-bearing gate is tightest-covering, **not** "always a `Text` leaf" — structural bytes (`\section`/`\item`/`\begin{…}` machinery, inter-child delimiters) legitimately resolve to their enclosing composite, which is the tightest cover there (a soft signal records that the *majority* of content bytes still land on leaves). No `node_at`/parser/fold logic change (S1–S4 already made spans precise + `node_at` leaf-resolving); pure test rung. Corpus fixed/bounded ⇒ O(len), not a DoS. **Completes the LTXDOC02 precise-per-token-spans arc.** | ✅ |
| **LTXDOC03 S1 — cross-reference resolution (label table + `\ref` binding)** | `Document::resolve_references() -> ReferenceResolution` — a pure, additive pass (no parser/fold/`walk`/`node_at`/span change) that binds each cross-reference to the `\label` that defines it, **with byte spans on both sides**. Collects the label table from hoisted section/table/figure labels (`Block::…{ label: Some(k) }`) and inline `\label{k}` (`Inline::CrossRef`); resolves the reference family `{ref, eqref, pageref}` against it → `ResolvedRef` (ref-span **and** target def-span + `LabelKind`) or `UnresolvedRef` (dangling key + ref-span); reports `Duplicate` keys with **first-def-wins**. `\cite` is a separate table, bound by S2. The static analogue of LaTeX's two-pass `.aux` machinery, binding *structure* not numbers/pages. Total & panic-free, reuses the bounded `walk` (no new recursion). | ✅ |
| **LTXDOC03 S2 — `\cite` → bibliography binding** | `Document::resolve_citations() -> CitationResolution` — the *parallel* pass to S1 for the **citation** family, binding each `\cite` key to the `\bibitem` that defines it, **with byte spans on both sides**. Collects the bibliography table from every `\bibitem{k}` inside a `thebibliography` environment (`BibEntry { key, span }`, span = the tight `\bibitem{k}` construct), first-entry-wins with `DuplicateBib`; then, for each `\cite` (the `CITE_COMMAND` family), splits `target` on commas into individual trimmed keys and resolves each → `ResolvedCite` (the shared `\cite` `cite_span` **and** the entry's `entry_span`) or `UnresolvedCite` (dangling key + `cite_span`). A multi-key `\cite{a,b,c}` yields one record **per key**, all sharing that `\cite`'s span; `\cite[note]{k}` keeps the note out of the key. External `.bib`/BibTeX databases and citation numbering stay out of scope (in-document `thebibliography` only). Disjoint from and non-interfering with S1. Total & panic-free, reuses the bounded `walk` + a `MAX_DEPTH`-bounded env descent. | ✅ |
| **LTXDOC03 S3 — target → `NodeRef` exposure** | The depth-add on S1+S2: both bound each target's **bytes** (a `Span`) but not the target **node**. `Document::node_for_span(span) -> Option<NodeRef>` returns the walked body node whose span **exactly equals** `span` (half-open equality of start **and** end; *equality*, not `node_at`'s containment) — or `None` for a span that is no walked node's own (empty doc, un-walked preamble/metadata, fabricated span). Thin accessors `ref_target_node(&ResolvedRef)`, `cite_target_node(&ResolvedCite)`, `label_def_node(&LabelDef)` take a resolved record and hand back the node, so a caller can read its `kind()` and — for a `Block` — descend into its children (a `\ref`→section enumerates its paragraphs; a `\ref`→figure reaches its caption). **Verified reachability:** every S1/S2 target span matches exactly one walked node (zero collisions); a `\ref`→section/figure/table yields a `NodeRef::Block`, an inline `\eqref`→`\label` a `NodeRef::Inline` (`CrossRef`), and — the once-uncertain case — a `\cite`→`\bibitem` **is** walked (an `Inline::Raw` inside the `thebibliography`), so `cite_target_node` returns `Some`, not `None`. Purely additive (S1/S2 result types keep owned `Span`s, no lifetimes; the `NodeRef` is fetched on demand); tie-break = first-in-pre-order (defensive, does not fire). Numbering + external BibTeX remain out of scope. Total & panic-free, reuses the bounded `walk`. | ✅ |
| **LTXDOC03 S4 — document numbering (hierarchical sections + flat float counters)** | `Document::number_labels() -> Numbering` — the number a `\ref` *prints*, computed in one `walk` (LaTeX's second `.aux` pass, done statically). **Section numbers** are hierarchical with deeper-reset: a numbered `\section`…`\subparagraph` increments its depth's counter, resets deeper depths to `0`, and renders the dotted join from the top level down (`1`, `1.1`, `1.2`, `1.2.1`, `2`); a starred `\section*` (`numbered == false`) fires no counter and is skipped. **Float counters** are flat and independent: every `figure` advances a running figure counter (`1, 2, …`), every `table` its own (`1, 2, …`) — labeled *or not* (a `\label` only captures the value; an unlabeled figure between two labeled ones takes `2`). Missing-parent rule: a document that starts deep renders honest leading `0`s (a lone `\subsection` → `0.1`), a plain top-level `\section` → `1`. Returns one owned `NumberedLabel { key, kind, number }` per defined numberable label, with `number_for(key)`; `ref_number(&ResolvedRef) -> Option<String>` ties S1 resolution to S4 numbering (`\ref{sec:intro}` → `"1.2"`). Equation numbers, citation `[1]` numbers, and other counters deferred to S5+. Pure, additive, tree-unchanged; fixed 7-slot counter array (no unchecked indexing). Total & panic-free, reuses the bounded `walk`. | ✅ |
| **LTXDOC03 S5 — citation numbering (bracketed bibliography numbers)** | `Document::number_citations() -> CitationNumbering` — the bracketed number a `\cite` *prints* (`[2]`), the citation-family analogue of S4's `ref_number`. In the default numeric/unsorted style each `\bibitem` is numbered by its **listing position**, so S5 numbers S2's already-ordered winning `entries` by index: `entries[0]` → `[1]`, `entries[1]` → `[2]`, …. A first-`\bibitem`-wins **duplicate** is in `duplicate_entries`, not `entries`, so it consumes **no** number and the entries after it are unshifted (`a, b, c` + a later dup `\bibitem{a}` keeps `c` at `[3]`). A **dangling** `\cite` is in S2's `unresolved`, so it has no entry and `number_for` returns `None` (LaTeX's `[?]`). Returns one owned `NumberedCitation { key, ordinal, number }` per numbered entry, with `number_for(key) -> Option<&str>`; `cite_number(&ResolvedCite) -> Option<String>` ties S2 resolution to S5 numbering (`\cite{foo}` → `"[2]"`). Bracket style single-sourced in one helper. Equation numbers (blocked on `DisplayMath` carrying no label field), author-year/natbib sorted styles, and external `.bib` remain future rungs. Pure, additive, tree-unchanged. Total & panic-free. | ✅ |
| **LTXDOC03 S6 — the cross-reference report (consumer composing S1/S2/S4/S5)** | `Document::cross_reference_report() -> CrossReferenceReport` — the consumer rung that proves the passes **compose**: it walks S1's resolved `\ref`s and S2's resolved `\cite`s and assembles an owned, plain-data report where each entry carries its rendered **number** (S4/S5) alongside key/command/kind. **No new AST walk** — it numbers each family **once** (`number_labels`/`number_citations`) then looks each key up (never per-item re-numbering). Produces `refs: Vec<RefEntry { key, command, kind, number }>` (one per resolved **and numbered** `\ref`, in S1 order — a resolved `\ref` to an *unnumbered bare-inline* label is still **omitted**; an S7 equation label is included), `cites: Vec<CiteEntry { key, number }>` (one per resolved `\cite` key, in S2 order), and `dangling_refs`/`dangling_cites: Vec<String>` (S1/S2 `unresolved` keys — LaTeX's `??`/`[?]`, surfaced **separately**). `CrossReferenceReport::to_plain_text() -> String` renders a stable, pinned string (`\ref{k} -> Section 1.2` / `\cite{k} -> [2]` lines, optional `Dangling …:` footers, joined by `\n`, no trailing newline; empty → `(no cross-references)`). Pure composition, reads S1–S5 unchanged, tree untouched. Total & panic-free. | ✅ |
| **LTXDOC03 S7 — equation-label lifting** | `Block::DisplayMath` gains a `label: Option<String>` and `LabelKind` gains an `Equation` variant. For a **non-starred** display-math environment (`equation`/`align`/`gather`/`multline`/`eqnarray` — not the starred forms), the D5 lowering now **lifts** the `\label{key}` out of the env body onto the block (removing it from `source`, no duplication) and registers it as a real `LabelKind::Equation` definition in the same pass as section/figure/table labels. So an `\eqref{eq:e}`/`\ref{eq:e}` to an in-equation label now **resolves** and is **included** in the S6 report (`\ref{eq:e} -> Equation ?`) instead of being omitted. The equation **number** (`\theequation`) is deferred to S8: the report carries the placeholder `EQUATION_NUMBER_PLACEHOLDER` (`"?"`). `to_latex()` re-emits a lifted-label equation as `\begin{equation}…\label{…}\end{equation}` so the round-trip fixed point holds. Starred forms and `\[…\]`/`$$…$$` keep `label: None`. Pure, additive; total & panic-free. | ✅ |
| **LTXDOC03 S8 — equation numbering** | `Counters` gains a flat `equation: u32` field and a `step_equation()` method (mirroring `step_figure`/`step_table` — pre-increment, saturating, one monotonic run **independent** of section/figure/table). In the `Block::DisplayMath { label: Some(key), .. }` arm of `number_labels`, S7's placeholder is replaced with `counters.step_equation().to_string()`, so a lifted equation label now carries a **real** sequential number in document order (`1`, `2`, …). The S6 report prints `\ref{eq:e} -> Equation 1` (was `Equation ?`). Equation numbering is independent of the section/figure/table counters. **Limitation:** because `Block::DisplayMath` carries no `numbered` flag (the D5 lowering sets `label: None` for both starred envs and `\[…\]`/`$$…$$` islands), only **labelled** equations step the counter — an unlabelled numbered equation between two labelled ones is not yet counted (needs an AST change, deferred). `\eqref` parenthesisation (`(1)` vs `1`) is also deferred to a later slice; S8 is counter-only. Pure, additive, tree-unchanged; total & panic-free. | ✅ |
| **LTXDOC03 S9 — `\eqref` parenthesisation** | `CrossReferenceReport::to_plain_text` now mirrors amsmath's `\eqref` for the one case that matters: a resolved reference whose `command == "eqref"` **and** `kind == LabelKind::Equation` renders `\eqref{eq:e} -> Equation (1)` — the `\eqref` spelling is kept and the number is **parenthesised**. Every other reference — all `\ref`, all `\pageref`, and any `\eqref` to a **non-equation** kind — is byte-for-byte unchanged from S8: canonical `\ref` prefix, bare number (`\ref{sec:intro} -> Section 1.2`). `RefEntry.command` (the surface spelling) was already retained by S1, so S9 is a pure rendering split in one `format!` — no AST change, no new field, no re-numbering; `to_latex()` stays a fixed point. Pure, additive; total & panic-free. | ✅ |
| **LTXDOC03 S10 — distinct `\pageref` rendering** | `CrossReferenceReport::to_plain_text` gains a **third** rendering branch so a resolved `\pageref` no longer renders identically to a `\ref`. A `\pageref{key}` asks "what **page** is the target on" — a different question from `\ref`'s "what **number** is the target" — but the crate has **no page model**, so it cannot compute a real page number. A resolved reference whose `command == "pageref"` (to **any** target kind) now renders `\pageref{sec:i} -> page ?` — the `\pageref` spelling is kept and the kind/number are replaced by the fixed literal placeholder `page ?` (the `?` mirrors LaTeX's own `??` for an unresolved page reference; it means "page number not modelled", not the kind, not the number). Branch precedence: (1) `\eqref` to Equation → parenthesised (S9); (2) `\pageref` any kind → `page ?` (S10); (3) else → `\ref{key} -> Kind N` (S8). So `\ref` and `\eqref` outputs are byte-for-byte unchanged; only `\pageref` lines change. Pure rendering branch — no AST change, no re-numbering, `to_latex()` still a fixed point. Total & panic-free. | ✅ |
| **LTXDOC03 S11 — grouped-by-kind cross-reference report** | `CrossReferenceReport::to_plain_text_by_kind() -> String` — a **new, separate** method that renders the **same** resolved references as `to_plain_text` but **grouped under fixed-order kind subheadings** (Sections, Figures, Tables, Equations, Inline — regardless of source order), each subheading followed by two-space-indented ref lines in the report's existing pre-order. Kinds with zero resolved refs are omitted entirely. The per-line rendering is the **identical** S8/S9/S10 rule — factored into a shared private `render_resolved_ref(&RefEntry)` that **both** `to_plain_text` and `to_plain_text_by_kind` call, so the flat and grouped reports can never drift (`to_plain_text` output is byte-for-byte unchanged). Only resolved refs are grouped (no citations, no dangling footers); zero resolved refs → the fixed marker `(no resolved references)`. A `\pageref` groups under its **target kind**. Pure report-assembly over data the report already holds — no AST/struct/numbering change, `to_latex()` still a fixed point. Total & panic-free. | ✅ |
| **LTXDOC03 S12 — List of Figures / List of Tables index** | `Document::list_of_floats() -> String` — a **new** method that renders the document's **List of Figures** and **List of Tables** (LaTeX's `\listoffigures` / `\listoftables`, as plain text) directly from the floats. A single document-order walk threads the **same** `Counters` float counters as `number_labels`, so each float's line number equals its S4 flat figure/table number (a labeled float's List-of number and its `\ref` number agree; figures numbered `1, 2, …`, tables independently from `1`). Each line is `<n>. <caption text>`, where the caption text is the plain rendering of the float's `\caption{…}` inlines (text/code verbatim, space as a single space, font-wrapper content recursively, trimmed — the same descent the caption-reaching test proves), via a private `caption_text(&Option<Caption>)` helper; a float with **no** `\caption` renders the fixed placeholder `(no caption)` so every float still gets a numbered line. The `List of Figures` heading is emitted only when there is ≥1 figure, `List of Tables` only when ≥1 table; a document with no floats → the fixed marker `(no floats)`. Lines joined by `\n`, no trailing newline. Real LaTeX gates these on `\listoffigures` / `\listoftables` commands (not parser-recognised here), so — like S11 — S12 is a method the caller invokes; every S1–S11 output is byte-for-byte unchanged. Pure assembly over existing blocks + the existing float walk — no AST/grammar/counter change, `to_latex()` still a fixed point. Total & panic-free. | ✅ |
| **LTXDOC03 S13 — `\nameref` resolution to a target's name** | `Document::resolve_namerefs() -> String` — a **new** method that resolves every `\nameref{key}` to the **name** (title/caption text) of its target — the name-valued sibling of `\ref` (number) and `\pageref` (page). `\nameref` is **not** a `REF_COMMAND`, so it appears in neither the resolved nor unresolved ref table (an AST probe confirms it lowers to `Inline::CrossRef { command: "nameref", .. }`); S13 reads the same S1 `\label` table but answers "*what is it called?*", so it is purely additive. One document-order walk collects each `nameref` cross-ref; the key is resolved against the winning label table and the name read from its defining node via the S3 `label_def_node` accessor — a `Section` → its title inlines flattened, a `Figure`/`Table` → its `\caption` via the shared `caption_text` helper (S12's flatten factored into a module-level `flatten_inlines_to_text`, reused by both so they can never drift). An `Equation`/`Inline` target (a number, not a title) → `(no name)`; an undefined key → `(undefined nameref: <key>)`; no `\nameref` at all → `(no namerefs)`. Each line is `\nameref{<key>} -> <name>`, joined by `\n`, no trailing newline. Every S1–S12 output is byte-for-byte unchanged. Total & panic-free. | ✅ |
| **LTXDOC03 S14 — per-kind census of the numbered-label table** | `Document::list_summary() -> String` — a **new**, read-only method that renders a compact per-kind **count** of the document's numbered labels: how many sections, figures, tables, and equations carry a `\label`. It is a pure tally of the rows `number_labels()` returns, grouped by `LabelKind`, so it can never drift from the S4 numbering it summarises. Only the four numberable kinds reach that table — a bare inline `\label{…}` (`LabelKind::Inline`) is not numbered and is counted nowhere. One line per non-zero kind in the **fixed order** `Sections`, `Figures`, `Tables`, `Equations` (deterministic, not source order), formatted `<Kind>: <count>` with a **fixed plural** label regardless of count (a single section still prints `Sections: 1`); a kind with count 0 is **omitted** (mirroring S11). Lines joined by `\n`, no trailing newline. A document with **no** numbered label at all → the fixed marker `(no labels)`. E.g. two labeled sections + a figure + a table + an equation → `Sections: 2\nFigures: 1\nTables: 1\nEquations: 1`. Every S1–S13 output is byte-for-byte unchanged, `to_latex()` still a fixed point. Total & panic-free. | ✅ |
| **LTXDOC03 S15 — resolved citations grouped by their source `\cite`** | `Document::citations_by_source() -> String` — a **new**, read-only method that renders the resolved citations **grouped by the source `\cite` they came from** — the citation-family parallel of S11's `to_plain_text_by_kind` and S13's `resolve_namerefs`. It reads only `resolve_citations().resolved` and re-assembles the per-key rows S2 flattened out of each multi-key `\cite`, grouping them back by `cite_span` in **first-appearance order** (source order of the `\cite`s) with keys kept in their left-to-right order. One line per source `\cite`: `\cite{` + the group's **resolved** keys joined by `", "` + `}`. A **dangling** key never entered `resolved`, so it is excluded — `\cite{a,ghost}` where only `a` resolves renders `\cite{a}` (we reconstruct from resolved keys rather than slice `&src[cite_span]`, which would still show `ghost`). Lines joined by `\n`, no trailing newline. A document with **no** resolved citations (none present, or every key dangling) → the fixed marker `(no resolved citations)`. E.g. `\cite{a,b}` (both defined) then `\cite{c}` → `\cite{a, b}\n\cite{c}`. Every S1–S14 output is byte-for-byte unchanged, `to_latex()` still a fixed point. Total & panic-free. | ✅ |
| **LTXDOC03 S16 — duplicate (multiply-defined) `\bibitem` entries** | `Document::duplicate_bibliography_entries() -> String` — a **new**, read-only method that surfaces the **multiply-defined** `\bibitem`s — LaTeX's *"Citation `key' multiply defined"* warnings — the citation-family parallel of S6's *"Dangling citations"* footer (for the *other* bibliography warning). These were already computed by S2 (`resolve_citations().duplicate_entries`) but rendered by **no** method until now. S2 collects every `\bibitem` in `walk` pre-order; the **first** of each key wins and every **later** `\bibitem` of an already-defined key is a losing duplicate. S16 emits one line per duplicate in that pre-order (**not** re-sorted, **not** de-duplicated — a key defined three times yields two lines), each reconstructed from its key as `\bibitem{<key>}` (no source slicing). Lines joined by `\n`, no trailing newline. A document with **no** duplicates (no bibliography, or every key once) → the fixed marker `(no duplicate bibliography entries)`. E.g. `smith` defined twice + `jones` once → `\bibitem{smith}` (only the loser). Every S1–S15 output is byte-for-byte unchanged, `to_latex()` still a fixed point. Total & panic-free. | ✅ |
| **LTXDOC03 S17 — unresolved (dangling) citations grouped by source `\cite`** | `Document::unresolved_citations_by_source() -> String` — a **new**, read-only method that renders the **dangling** citations **grouped by the source `\cite` they came from** — the DANGLING-key mirror of S15's `citations_by_source`, and the per-`\cite` view of S6's flat *"Dangling citations"* footer. It reads only `resolve_citations().unresolved` and re-assembles the per-key rows S2 flattened out of each multi-key `\cite`, grouping the dangling keys back by `cite_span` in **first-appearance order** (source order) with keys kept left-to-right. One line per source `\cite` with ≥1 dangling key: `\cite{` + the group's **dangling** keys joined by `", "` + `}`. A **resolved** key never entered `unresolved`, so `\cite{a,ghost}` where only `a` resolves renders `\cite{ghost}` (reconstructed from keys, no source slicing). Lines joined by `\n`, no trailing newline. A document with **no** unresolved citations → the fixed marker `(no unresolved citations)`. E.g. `\cite{known,ghost}` (only `known` defined) then `\cite{x,y}` (neither) → `\cite{ghost}\n\cite{x, y}`. Every S1–S16 output is byte-for-byte unchanged, `to_latex()` still a fixed point. Total & panic-free. | ✅ |
| **LTXDOC03 S18 — unresolved (dangling) references grouped by source `\ref`** | `Document::unresolved_references_by_source() -> String` — a **new**, read-only method that renders the **dangling** references, one reconstructed `\<command>{key}` line each — the `\ref`-family parallel of S17's dangling-CITATION report, and a **distinct** view from S6's flat *"Dangling references: k1, k2"* footer (S18 is **command-aware**, so `\eqref`/`\pageref` render as themselves, not flattened to `\ref`). It reads only `resolve_references().unresolved` (a `Vec<UnresolvedRef { key, command, ref_span }>`) and groups by `ref_span` in **first-appearance order** (source order); because each `\ref`/`\eqref`/`\pageref` takes exactly one key, every group is a single entry emitting one line: `\` + the ref's own `command` + `{` + its `key` + `}` (reconstructed from the owned strings, no source slicing). A **resolved** `\ref` never entered `unresolved`, so it is excluded. Lines joined by `\n`, no trailing newline. A document with **no** unresolved references → the fixed marker `(no unresolved references)`. E.g. `\eqref{eq:ghost}` then `\pageref{p:ghost}` (neither defined) → `\eqref{eq:ghost}\n\pageref{p:ghost}`. Every S1–S17 output is byte-for-byte unchanged, `to_latex()` still a fixed point. Total & panic-free. | ✅ |
| **LTXDOC03 S19 — numbered winning-bibliography-entry list** | `Document::bibliography_entries() -> String` — a **new**, read-only method that renders the **winning** bibliography entries as a **numbered list** — the rendered bibliography a reader sees, and the table citations resolve against. A **distinct** view over `resolve_citations()`: S16 (`duplicate_bibliography_entries`) renders the **losing** `duplicate_entries` as `\bibitem{key}` lines, S15 (`citations_by_source`) renders per-source *resolved cite keys*; S19 renders the **winning** `entries`. It reads only `resolve_citations().entries` (the first `\bibitem` of each distinct key, body pre-order; later re-definitions live in `duplicate_entries`) and numbers it **1-based**, one line per entry as `[n] key` (reconstructed from the owned key, no source slicing). The `[n] key` shape is deliberately distinct from S16's `\bibitem{key}` lines. A `\bibitem{dup}` written twice appears **once** — the winner. Lines joined by `\n`, no trailing newline. A document with **no** bibliography entries → the fixed marker `(no bibliography entries)`. E.g. `smith` (twice) + `jones` → `[1] smith\n[2] jones`. Every S1–S18 output is byte-for-byte unchanged, `to_latex()` still a fixed point. Total & panic-free. | ✅ |
| **LTXDOC03 S20 — losing duplicate `\label` definitions** | `Document::duplicate_label_definitions() -> String` — a **new**, read-only method that surfaces the **multiply-defined** `\label`s — LaTeX's *"Label `key' multiply defined"* warnings — the **label-family mirror of S16's** `duplicate_bibliography_entries` (which surfaces the losing `\bibitem` duplicates). It reads only `resolve_references().duplicates`: S1 collects every `\label` in pre-order; the **first** of each key wins (into `definitions`, what `\ref`/`\eqref`/`\pageref` resolve against) and every **later** `\label` of an already-defined key is a losing duplicate. S20 emits one line per duplicate in that pre-order (**not** re-sorted, **not** de-duplicated — a key defined three times yields two lines), each reconstructed from its key as `\label{<key>}` (no source slicing; the `\label{…}` form is right for any `LabelKind`). Lines joined by `\n`, no trailing newline. A document with **no** duplicate labels (every key once, or none) → the fixed marker `(no duplicate label definitions)`. E.g. `dup` defined twice + `once` once → `\label{dup}` (only the loser). Every S1–S19 output is byte-for-byte unchanged, `to_latex()` still a fixed point. Total & panic-free. | ✅ |
| **LTXDOC03 S21 — resolved references grouped by source `\ref`** | `Document::resolved_references_by_source() -> String` — a **new**, read-only method that renders the **resolved** (successfully-matched) references, one reconstructed `\<command>{key}` line each — the **RESOLVED mirror of S18's** `unresolved_references_by_source` (which renders the *dangling* half of the same split), **command-aware** so `\eqref`/`\pageref` render as themselves, not flattened to `\ref`. It reads only `resolve_references().resolved` (a `Vec<ResolvedRef { key, command, ref_span, target_span, target_kind }>`) and groups by `ref_span` in **first-appearance order** (source order); because each `\ref`/`\eqref`/`\pageref` takes exactly one key, every group is a single entry emitting one line: `\` + the ref's own `command` + `{` + its `key` + `}` (reconstructed from the owned strings, no source slicing). A **dangling** `\ref` never entered `resolved`, so it is excluded (it lives in S18). Lines joined by `\n`, no trailing newline. A document with **no** resolved references (every ref dangles, or none) → the fixed marker `(no resolved references)`. E.g. `\ref{sec:intro}`, `\eqref{eq:main}`, `\pageref{sec:intro}` (all defined) → `\ref{sec:intro}\n\eqref{eq:main}\n\pageref{sec:intro}`. Every S1–S20 output is byte-for-byte unchanged, `to_latex()` still a fixed point. Total & panic-free. | ✅ |
| **LTXDOC03 S22 — winning `\label` definitions** | `Document::label_definitions() -> String` — a **new**, read-only method that renders the **winning** label definitions — the `\label{key}` definitions references resolve against, one `\label{key}` line per distinct key — the **label-family analogue of S19's** `bibliography_entries` (which renders the winning `\bibitem` entries) and the **winning-side counterpart of S20's** `duplicate_label_definitions` (which renders the *losing* duplicate `\label`s). It reads only `resolve_references().definitions`: S1 collects every `\label` in pre-order; the **first** of each key wins (into `definitions`, one row per distinct key) and every **later** `\label` of an already-defined key goes to `duplicates` (S20's domain). S22 emits one line per winning definition in that pre-order (**not** re-sorted, **not** de-duplicated — none needed, since `definitions` already holds one row per distinct key), each reconstructed from its key as `\label{<key>}` (no source slicing; the `\label{…}` form is right for any `LabelKind`). A `\label{dup}` written twice appears **once** — the winner. Lines joined by `\n`, no trailing newline. A document with **no** label definitions → the fixed marker `(no label definitions)`. E.g. `sec:intro` (section), `eq:main` (equation), then a re-used `sec:intro` → `\label{sec:intro}\n\label{eq:main}` (winner once). Every S1–S21 output is byte-for-byte unchanged, `to_latex()` still a fixed point. Total & panic-free. | ✅ |
| **LTXDOC03 S23 — winning `\label` definitions grouped by kind** | `Document::label_definitions_by_kind() -> String` — a **new**, read-only method that renders the **winning** `\label` definitions **grouped by their `LabelKind`** — a per-kind census — the **by-kind grouping companion of S22's** `label_definitions` (which lists the same winning definitions *flat*); S22 and S23 are two views of the one winning `definitions` list. It reads only `resolve_references().definitions` and groups by kind in a **fixed, document-independent order** (the enum declaration order: `Section`, `Table`, `Figure`, `Equation`, `Inline` — iterated as an explicit slice, not a hash map, so the order is deterministic like S17/S18's `Vec`-of-groups). Within each kind, definitions keep their existing pre-order. Each line is `[<kind>] \label{<key>}` — `<kind>` from `LabelKind::as_str()` (`"section"`/`"table"`/`"figure"`/`"equation"`/`"inline"`), `<key>` from the owned key (no source slicing). A kind with no definitions contributes no lines (no empty `[table]` header). Lines joined by `\n`, no trailing newline. A document with **no** label definitions → the **same** fixed marker `(no label definitions)` S22 uses. E.g. `sec:intro` (section), `eq:main` (equation), `note` (inline) → `[section] \label{sec:intro}\n[equation] \label{eq:main}\n[inline] \label{note}`. Every S1–S22 output is byte-for-byte unchanged, `to_latex()` still a fixed point. Total & panic-free. | ✅ |
| **LTXDOC03 S24 — resolved references grouped by target kind** | `Document::resolved_references_by_kind() -> String` — a **new**, read-only method that renders the **resolved** `\ref`/`\eqref`/`\pageref` references **grouped by the `LabelKind` they resolved TO** — a per-kind census — the **by-kind grouping companion of S21's** `resolved_references_by_source` (which lists the same resolved refs *flat*, in source order); S21 and S24 are two views of the one `resolved` list. It mirrors S23's `label_definitions_by_kind` idiom but over the **resolved-references** list, and stays **command-aware** like S21. It reads only `resolve_references().resolved` (a `Vec<ResolvedRef { key, command, ref_span, target_span, target_kind }>`) and groups by `target_kind` in a **fixed, document-independent order** (the enum declaration order: `Section`, `Table`, `Figure`, `Equation`, `Inline` — the SAME slice S23 uses, iterated explicitly, not a hash map, so the order is deterministic). Within each kind, refs keep their existing pre-order. Each line is `[<kind>] \<command>{<key>}` — `<kind>` from the ref's `target_kind.as_str()`, `<command>` the ref's own (so `\eqref`/`\pageref` render as themselves), `<key>` from the owned key (no source slicing). A **dangling** `\ref` never entered `resolved`, so it is excluded (it lives in S18). A kind with no resolved refs contributes no lines (no empty `[table]` header). Lines joined by `\n`, no trailing newline. A document with **no** resolved references → the **same** fixed marker `(no resolved references)` S21 uses. E.g. `\ref{sec:intro}` (section), `\eqref{eq:main}` (equation), `\pageref{sec:intro}` (section) → `[section] \ref{sec:intro}\n[section] \pageref{sec:intro}\n[equation] \eqref{eq:main}`. Every S1–S23 output is byte-for-byte unchanged, `to_latex()` still a fixed point. Total & panic-free. | ✅ |
| **LTXDOC03 S25 — per-kind census (counts) of the winning `\label` definitions** | `Document::label_kind_counts() -> String` — a **new**, read-only method that renders a **per-kind CENSUS** of the winning `\label` definitions: one `<kind>: <n>` line per `LabelKind` that has at least one winning definition, carrying the integer **count** (not a list) — the **count companion of S23's** `label_definitions_by_kind` (which renders one `[kind] \label{key}` *line per definition*). S22's flat `label_definitions`, S23's grouped list, and S25's counts are three views of the one winning `definitions` list; S25 collapses each kind's group to a single tally line (it is to S23 what S14's `list_summary` is to a full enumeration). It reads only `resolve_references().definitions` (one row per distinct key — the WINNER; a `\label{dup}` written twice counts **once**, its later copy being a `Duplicate` in S20's domain) and counts by kind in a **fixed, document-independent order** (the enum declaration order: `Section`, `Table`, `Figure`, `Equation`, `Inline` — the SAME slice S23/S24 use, iterated explicitly, not a hash map, so the order is deterministic). Each line is `<kind>: <n>` — `<kind>` from `LabelKind::as_str()` (the SAME tag S23 renders: `"section"`/`"table"`/`"figure"`/`"equation"`/`"inline"`), `<n>` the decimal count (no source slicing). A kind with a zero count contributes no line (no bare `table: 0`). Lines joined by `\n`, no trailing newline. A document with **no** label definitions → the **same** fixed marker `(no label definitions)` S22/S23 use. E.g. `sec:intro` (section), `eq:a`/`eq:b` (equations), `note` (inline) → `section: 1\nequation: 2\ninline: 1`. Every S1–S24 output is byte-for-byte unchanged, `to_latex()` still a fixed point. Total & panic-free. | ✅ |
| **LTXDOC03 S26 — per-kind census (counts) of the resolved references** | `Document::resolved_reference_kind_counts() -> String` — a **new**, read-only method that renders a **per-kind CENSUS** of the RESOLVED `\ref`/`\eqref`/`\pageref` references: one `<kind>: <n>` line per `LabelKind` that has at least one resolved ref, carrying the integer **count** (not a list) — the **count companion of S24's** `resolved_references_by_kind` (which renders one `[kind] \<command>{key}` *line per resolved ref*). S21's flat `resolved_references_by_source`, S24's grouped list, and S26's counts are three views of the one `resolved` list; S26 collapses each kind's group to a single tally line (it is to S24 what S25's `label_kind_counts` is to S23). It reads only `resolve_references().resolved` (each a `ResolvedRef` carrying the `target_kind` — the kind of the label it bound to; a dangling `\ref` lives in `unresolved`, S18's domain, and is excluded by construction — never a spurious `<kind>: 0`) and counts by `target_kind` in a **fixed, document-independent order** (the enum declaration order: `Section`, `Table`, `Figure`, `Equation`, `Inline` — the SAME slice S23/S24/S25 use, iterated explicitly, not a hash map, so the order is deterministic). Each line is `<kind>: <n>` — `<kind>` from `LabelKind::as_str()` (the SAME tag S24 renders: `"section"`/`"table"`/`"figure"`/`"equation"`/`"inline"`), `<n>` the decimal count (no source slicing). A kind with a zero count contributes no line (no bare `table: 0`). Lines joined by `\n`, no trailing newline. A document with **no** resolved references → the **same** fixed marker `(no resolved references)` S21/S24 use. E.g. `\ref{sec:a}`, `\ref{sec:b}` (two sections), `\eqref{eq:e}` (equation) → `section: 2\nequation: 1`. Every S1–S25 output is byte-for-byte unchanged, `to_latex()` still a fixed point. Total & panic-free. | ✅ |
| **LTXDOC03 S27 — single-integer total of the unresolved (dangling) references** | `Document::unresolved_reference_count() -> String` — a **new**, read-only method that renders the decimal **COUNT** of the UNRESOLVED (dangling) `\ref`/`\eqref`/`\pageref` references — the ones no `\label` defines (LaTeX's *"Reference `key' undefined"*, the `??`) — as one integer line. It is the **count-total companion of S18's** `unresolved_references_by_source` (which renders one `\<command>{key}` *line per dangling ref*): S18 and S27 are two views of the one `unresolved` list; S27 collapses the whole list to a single `.len()` tally. It is the count-total sibling of the census family (S25 `label_kind_counts`, S26 `resolved_reference_kind_counts`), but for the UNRESOLVED refs — which carry **no** `target_kind` (a dangling ref bound to nothing), so a per-kind census is not viable and a single total is the clean move. It reads only `resolve_references().unresolved.len()` (a resolved `\ref{sec:i}` lives in `resolved`, S21's domain, and is excluded by construction) — never a `target_kind`, no source slicing at all. Being a COUNT renderer, its empty case (every ref resolves, or none at all) is the honest number `"0"` — **not** a `(no …)` marker (that discipline belongs to the *list* renderers). One line, no trailing newline. E.g. `\ref{sec:i}` (resolves) + `\ref{nope}` + `\ref{gone}` (both dangle) → `2`. Every S1–S26 output is byte-for-byte unchanged, `to_latex()` still a fixed point. Total & panic-free. | ✅ |
| **LTXDOC03 S28 — single-integer total of the resolved references** | `Document::resolved_reference_count() -> String` — a **new**, read-only method that renders the decimal **COUNT** of the RESOLVED `\ref`/`\eqref`/`\pageref` references — the ones some `\label` defines — as one integer line. It is the **count-total companion of S21's** `resolved_references_by_source` **and S24's** `resolved_references_by_kind` (which render one `\<command>{key}` *line per resolved ref*, flat in source order or grouped by target kind): S21/S24 and S28 are two views of the one `resolved` list; S28 collapses the whole list to a single `.len()` tally. It is the exact resolved-side **twin of S27's** `unresolved_reference_count` — together S28 + S27 split every reference into the pair (resolved, dangling), so their totals sum to the total reference count. It reads only `resolve_references().resolved.len()` (a dangling `\ref{nope}` lives in `unresolved`, S18/S27's domain, and is excluded by construction) — never a `target_kind`, no source slicing at all, so section/table/equation references all fold into one total. Being a COUNT renderer, its empty case (every ref dangles, or none at all) is the honest number `"0"` — **not** a `(no resolved references)` marker (that discipline belongs to the *list* renderers; this mirrors S27). One line, no trailing newline. E.g. `\ref{sec:i}` (resolves) + `\pageref{sec:i}` (resolves) + `\ref{nope}` (dangles) → `2`. Every S1–S27 output is byte-for-byte unchanged, `to_latex()` still a fixed point. Total & panic-free. | ✅ |
| **LTXDOC03 S29 — single-integer total of the label definitions** | `Document::label_definition_count() -> String` — a **new**, read-only method that renders the decimal **COUNT** of the winning label definitions — the distinct `\label` keys the document defines — as one integer line. It is the **count-total companion of S22's** `label_definitions` **and S23's** `label_definitions_by_kind` (which render one `\label{key}` *line per winning definition*, flat in source order or grouped by kind): S22/S23 and S29 are two views of the one winning `definitions` list; S29 collapses the whole list to a single `.len()` tally. It is the exact label-definition-side **analogue** of the reference-side totals S27's `unresolved_reference_count` and S28's `resolved_reference_count`, and the count-total sibling of the census family (S25 `label_kind_counts`) but over the *whole* definition list rather than per-kind. It reads only `resolve_references().definitions.len()` (a later duplicate `\label{dup}` lives in `duplicates`, S20's domain, and is excluded by construction — the count is exactly the number of lines S22 lists) — never a `kind`, no source slicing at all, so section/figure/equation/inline labels all fold into one total. Being a COUNT renderer, its empty case (no `\label` at all) is the honest number `"0"` — **not** a `(no label definitions)` marker (that discipline belongs to the *list* renderers; this mirrors S27/S28). One line, no trailing newline. E.g. `sec:intro` (section) + `eq:main` (equation) + a re-used `sec:intro` (duplicate) → `2`. Every S1–S28 output is byte-for-byte unchanged, `to_latex()` still a fixed point. Total & panic-free. | ✅ |
| **LTXDOC03 S30 — single-integer total of the bibliography entries** | `Document::bibliography_entry_count() -> String` — a **new**, read-only method that renders the decimal **COUNT** of the winning bibliography entries — the distinct `\bibitem` keys the document defines inside a `thebibliography` environment — as one integer line. It is the **count-total companion of S19's** `bibliography_entries` (which renders one `[n] key` *line per winning entry*, 1-based in source order): S19 and S30 are two views of the one winning `entries` list; S30 collapses the whole list to a single `.len()` tally. It is the exact **citation-side analogue of S29's** `label_definition_count`, completing the *totals family* — S27's `unresolved_reference_count` and S28's `resolved_reference_count` count the two reference tables, S29 counts the label definitions, and S30 counts the bibliography entries. It reads only `resolve_citations().entries.len()` (a later duplicate `\bibitem{dup}` lives in `duplicate_entries`, S16's domain, and is excluded by construction — the count is exactly the number of lines S19 lists) — no source slicing at all. Being a COUNT renderer, its empty case (no `\bibitem` at all) is the honest number `"0"` — **not** a `(no bibliography entries)` marker (that discipline belongs to the *list* renderer S19; this mirrors S27/S28/S29). One line, no trailing newline. E.g. `a` + `b` + `c` + a re-used `a` (duplicate) → `3`. Every S1–S29 output is byte-for-byte unchanged, `to_latex()` still a fixed point. Total & panic-free. | ✅ |
| **LTXDOC03 S31 — single-integer total of the resolved citations** | `Document::citation_count() -> String` — a **new**, read-only method that renders the decimal **COUNT** of the RESOLVED `\cite` keys — the ones some `\bibitem` defines — as one integer line. It is the **count-total companion of S15's** `citations_by_source` (which renders the resolved keys *grouped by their source `\cite`*): S15 and S31 are two views of the one `resolved` list; S31 collapses the whole list to a single `.len()` tally. It is the exact resolved-**citation-side twin of S28's** `resolved_reference_count`, extending the *totals family* onto the resolved-citation table — S27's `unresolved_reference_count` and S28's `resolved_reference_count` count the two reference tables, S29 counts the label definitions, S30 counts the bibliography entries, and S31 counts the resolved citations. It reads only `resolve_citations().resolved.len()` (a dangling `\cite{ghost}` lives in `unresolved`, S17's domain, and is excluded by construction) — never a `cite_span`/`entry_span`, no source slicing at all, so every resolved key folds into one total. Being a COUNT renderer, its empty case (every cited key dangling, or none at all) is the honest number `"0"` — **not** a `(no resolved citations)` marker (that discipline belongs to the *list* renderer S15; this mirrors S27/S28/S29/S30). One line, no trailing newline. E.g. `\cite{a,b}` (both defined) + `\cite{c,ghost}` (only `c` defined) → `3`. Every S1–S30 output is byte-for-byte unchanged, `to_latex()` still a fixed point. Total & panic-free. | ✅ |
| **LTXDOC03 S32 — single-integer total of the unresolved (dangling) citations** | `Document::unresolved_citation_count() -> String` — a **new**, read-only method that renders the decimal **COUNT** of the UNRESOLVED (dangling) `\cite` keys — the ones **no** `\bibitem` defines — as one integer line. It is the **count-total companion of S17's** `unresolved_citations_by_source` (which renders the dangling keys *grouped by their source `\cite`*): S17 and S32 are two views of the one `unresolved` list; S32 collapses the whole list to a single `.len()` tally. It is the exact unresolved-**citation-side twin of S27's** `unresolved_reference_count`, and the **dangling sibling of S31's** resolved `citation_count`. Together S31 and S32 **partition** every per-key `\cite` record — `citation_count + unresolved_citation_count` equals the number of cited keys, because `resolve_citations()` routes each key into exactly one of `resolved`/`unresolved`. It reads only `resolve_citations().unresolved.len()` (a resolved `\cite{a}` lives in `resolved`, S15/S31's domain, and is excluded by construction) — never a `cite_span`/dangling `key`, no source slicing at all, so every dangling key folds into one total. Being a COUNT renderer, its empty case (every cited key resolving, or none at all) is the honest number `"0"` — **not** a `(no unresolved citations)` marker (that discipline belongs to the *list* renderer S17; this mirrors S27/S28/S29/S30/S31). One line, no trailing newline. E.g. `\cite{a,b}` (both defined) + `\cite{c,ghost}` (only `c` defined) → `1`. Every S1–S31 output is byte-for-byte unchanged, `to_latex()` still a fixed point. Total & panic-free. | ✅ |

The low-level ladder is **complete** (L0–L6). 🎉 The hierarchical **Document** layer (LTXDOC01) is
now **complete too** — D1–D6 all shipped, taking LaTeX → `Document` AST **end-to-end**: source →
tables/lists (D1) → preamble/body skeleton (D2) → sectioning forest (D3) → metadata + inline
normalization (D4) → floats/code/display-math (D5) → provenance API + byte-coverage capstone (D6).
The **precise per-token spans** arc (LTXDOC02) is now **complete** 🎉: **S1 shipped spanned L1 nodes**
(`parse()` retains the exact byte range it already computed for each node), **S2 shipped spanned
recognition passes** (each synthesised `Section`/`CrossRef`/`Preamble`/`Styled`/`Accent`/`Tabular`/
`List` node carries the exact union of its constituents' spans), **S3 shipped the precise
Document fold** (every body `Block`/`Inline` span is now the node's tight source range, composites
union their children's real spans, and the coarse `region` plumbing is deleted), **S4 retired
the region-coarse caveat for body nodes** (`node_at(byte)` now formally resolves to the true
per-token leaf; preamble/metadata stay honestly coarse), and **S5 shipped the precise byte-coverage
capstone** — over the representative corpus, every non-whitespace body byte resolves to the
*tightest-covering* walked node (no strictly-narrower walked node also covers it), stated honestly as
tightest-covering rather than leaf-only.

The **document-feature** arc (LTXDOC03) now builds on that precise-span foundation: **S1 ships
cross-reference resolution** (`Document::resolve_references`) — a pure, additive pass that binds each
`\ref`/`\eqref`/`\pageref` to the `\label` that defines it, with byte spans on both sides, reporting
duplicate and dangling references. **S2 ships `\cite` → bibliography binding**
(`Document::resolve_citations`) — the parallel pass that binds each `\cite` key to the `\bibitem`
inside a `thebibliography` environment, multi-key `\cite{a,b,c}` aware (one binding per key, all
sharing the `\cite` span), first-entry-wins for duplicate `\bibitem`s, and dangling `\cite`s
reported. **S3 lifts both bindings from bytes to nodes** (`Document::node_for_span` +
`ref_target_node`/`cite_target_node`/`label_def_node`) — given a resolved reference/citation, hand
back the actual walked `NodeRef` so a consumer can read its `kind()` and descend into a `Block`'s
children (a verified reachability check confirmed every target — including the `\cite`→`\bibitem`
inside `thebibliography` — is a walked node). **S4 assigns the numbers a `\ref` prints**
(`Document::number_labels` + `ref_number`) — hierarchical section numbers with deeper-reset (`1.2.1`,
`\section*` skipped) and independent flat figure/table counters (every float advancing its counter,
labeled or not), so `\ref{sec:intro}` resolves to `"1.2"`. External `.bib`/BibTeX databases stay out
of scope (in-document `thebibliography` only). **S5 assigns the bracketed number a `\cite` prints**
(`Document::number_citations` + `cite_number`) — the citation-family analogue of `ref_number`:
numbering S2's listing-ordered `entries` by index so the first `\bibitem` is `[1]`, the second `[2]`,
… (first-`\bibitem`-wins duplicates consume no number; dangling `\cite`s are unnumbered), so
`\cite{foo}` resolves to `"[2]"`. **S6 assembles the cross-reference report**
(`Document::cross_reference_report`) — the consumer that **composes** S1/S2/S4/S5 into one owned
artifact: every resolved `\ref` and `\cite` with its rendered number (`\ref{sec:intro} ->
Section 1.2`, `\cite{smith} -> [2]`), dangling refs/cites surfaced separately, plus a stable
`to_plain_text()` rendering — adding no new AST walk (each family numbered once, then looked up).
**S7 lifts equation labels**: a non-starred display-math env's `\label` is now pulled onto its
`Block::DisplayMath { label }` and registered as a `LabelKind::Equation` definition, so an `\eqref`
to it resolves and is included in the S6 report instead of omitted. **S8 numbers equations**: a flat
`step_equation()` counter (independent of section/figure/table, mirroring `step_figure`) assigns each
lifted equation label a real sequential number in document order, so the S6 report prints
`\ref{eq:e} -> Equation 1` (was `Equation ?`). Only **labelled** equations step the counter (the AST
carries no `numbered` flag for unlabelled numbered equations — deferred), and `\eqref`
parenthesisation is deferred to a later slice. Author-year/natbib sorted styles and external `.bib`
remain future rungs.

## The Document layer (LTXDOC01)

Above the flat `Vec<Node>` sits a reusable, hierarchical `Document` AST — the write-once fold every
consumer (renderers, format conversion, structure queries, the ADJ byte-provenance pipeline) shares
instead of re-deriving the hierarchy. `parse_document` composes the shipped LTX01 passes then folds:

```rust
use latex::{parse_document, Block, Inline};

let doc = parse_document(
    r"\documentclass{article}\begin{document}\section{Intro}Hello \textbf{world}.\end{document}",
).unwrap();

assert_eq!(doc.preamble.document_class.unwrap().class, "article");
// D3: the heading OWNS the following blocks — its `body` is the nested sectioning forest.
if let Block::Section { body, .. } = &doc.body[0] {
    assert!(matches!(body[0], Block::Paragraph(..)));         // "Hello world." is owned by Intro
}
assert_eq!(doc.to_latex().is_empty(), false);                // round-trips (modulo spans)
```

### Document metadata (LTXDOC01 D4)

`Document::metadata` is a typed `Metadata { title, authors, date, abstract_ }` record — ask "what is
the title / who are the authors / what is the abstract?" without walking the tree:

```rust
use latex::parse_document;

let doc = parse_document(
    r"\title{Paper}\author{Alice \and Bob}\begin{document}\maketitle\begin{abstract}An abstract.\end{abstract}Body.\end{document}",
).unwrap();

assert_eq!(doc.metadata.authors.len(), 2);           // \and split → two entries
assert!(doc.metadata.title.is_some());
assert!(doc.metadata.abstract_.is_some());
```

Metadata is an **additive projection**: the `\title`/`\author`/`\date` commands and the `abstract`
environment are *not* removed — they still live in `preamble`/`body`, so `to_latex` round-trips
unchanged and re-parsing repopulates the identical `Metadata` (a fixed point). Absent directives
leave the fields `None`/empty — never fabricated.

### Floats, code & display math (LTXDOC01 D5)

D5 gives the semantic block kinds a real paper uses, by specializing the environment fold on the
`\begin{env}` name:

```rust
use latex::{parse_document, Block};

let doc = parse_document(concat!(
    r"\begin{document}",
    r"\begin{figure}\includegraphics{plot.png}\caption{A plot}\label{fig:p}\end{figure}",
    r"\begin{table}\begin{tabular}{lc}a & b \\ c & d\end{tabular}\caption{Grid}\label{tab:g}\end{table}",
    r"\begin{equation}E = mc^2\end{equation}",
    r"\end{document}",
)).unwrap();

// A `figure` float: caption + label lifted, the \includegraphics body preserved.
assert!(doc.body.iter().any(|b| matches!(b, Block::Figure { caption: Some(_), label: Some(_), .. })));
// A `table` float attaches its \caption/\label to the *inner* tabular's Block::Table.
assert!(doc.body.iter().any(|b| matches!(b, Block::Table { caption: Some(_), .. })));
// A named display-math environment keeps its source string (unparsed here).
assert!(doc.body.iter().any(|b| matches!(b, Block::DisplayMath { .. })));
```

- **`figure`/`figure*`** → `Block::Figure { content, caption, label }` — the `\caption{…}` becomes a
  `Caption`, a following `\label{…}` is hoisted, and everything else (e.g. `\includegraphics`) stays
  in `content`.
- **`table`/`table*`** → the inner `Block::Table` with the float's caption/label attached (a float
  with no tabular degrades to a `Block::Figure`, so nothing is lost).
- **`verbatim`/`lstlisting`** → `Block::CodeBlock` — the body is kept **unparsed** (code is source,
  not marked-up LaTeX).
- **`equation`/`align`/`gather`/`displaymath`/…** → `Block::DisplayMath { source, label }` — the
  inner LaTeX is kept as a source string, delegated to the math frontend on demand (LTXDOC01 never
  parses math itself). For a **non-starred** env (LTXDOC03 S7), a `\label{key}` in the body is lifted
  into `label: Some(key)` (removed from `source`) — a real `LabelKind::Equation` definition; starred
  forms and `\[…\]`/`$$…$$` keep `label: None`.
- **`quote`/`quotation`** → `Block::Quote`; any other environment stays a recursed
  `Block::Environment`.

Caption/label extraction mirrors D3's `\label` hoist and never drops float content; `to_latex`
re-emits the `figure`/`table` wrapper (with `\caption`/`\label`), `verbatim` fences, and `$$…$$`
so `parse(to_latex(d)).strip_spans() == d.strip_spans()` remains a fixed point.

**Body spans are precise as of 0.34.0 (LTXDOC02 S3), and `node_at` resolves to the true body leaf as
of 0.35.0 (S4):** every body block/inline span is the source
node's own tight byte range (`&src[node.span]` slices back to exactly its source), and a composite's
span is the **union** of its children's real spans — so every child span ⊆ its parent ⊆ the
`Document` span still holds, and now the leaves are tight. Only `Preamble` / `DocumentClass` /
`Package` remain honestly preamble-region-coarse (the preamble is classified out of directives, not
walked as per-node body content).

### The provenance API — walk / node_at (LTXDOC01 D6, arc complete)

D6 exposes the byte-provenance surface the ADJ reasoning pipeline consumes — "which node owns this
source byte?" and "visit every node in order":

```rust
use latex::parse_document;

let d = parse_document(r"\begin{document}\section{Intro}Body text.\end{document}").unwrap();

// walk(): pre-order, depth-first over every body Block + nested Inline.
let kinds: Vec<&str> = d.walk().map(|n| n.kind()).collect();
// e.g. ["Section", "Text", "Paragraph", "Text", …] — a parent precedes its children.

// node_at(byte): the true per-token leaf — the narrowest node whose precise span contains the byte.
if let Some(p) = d.node_at(35) {
    let _ = (p.node.kind(), p.span); // NodeRef + its span
}
assert!(d.node_at(usize::MAX).is_none()); // out of range → None, never panics
```

**Granularity.** As of 0.35.0 (S4) body spans are precise and `node_at(byte)` **formally resolves to
the true per-token leaf** — the narrowest node whose precise span contains the byte (ties → the
deepest node in pre-order). A byte inside `widgets` resolves to the `Text` run owning `widgets`, not
to the enclosing `Paragraph`/`Section`; a byte inside a `\section` title resolves to the title
inline, not the whole `Section`. This holds for **body** nodes (the ones `walk` visits);
`Preamble`/`DocumentClass`/`Package` stay honestly preamble-region-coarse (classified out of
directives, not walked, and `node_at` never resolves into them). The region-scoped capstone
byte-coverage test asserts every non-whitespace byte inside the document **body region** is owned by
≥1 walked node; and as of 0.36.0 (**S5**, arc complete) the whole-corpus capstone
`capstone_every_body_byte_resolves_to_tightest_covering_node` strengthens this to
**tightest-covering** — every non-whitespace body byte resolves to the innermost walked node, with
no strictly-narrower walked node also covering it (stated honestly as tightest-covering, since
structural bytes legitimately resolve to their enclosing composite rather than a `Text` leaf).

### Cross-reference resolution (LTXDOC03 S1)

Built on top of the precise spans, `Document::resolve_references()` binds each cross-reference to the
`\label` that defines it — the static, single-pass analogue of LaTeX's two-pass `.aux` machinery, but
carrying the defining node's **source bytes** rather than its number/page. It is pure analysis: it
changes nothing about the parser, the fold, `walk`, `node_at`, any span, or the `to_latex`
round-trip.

```rust
use latex::{parse_document, LabelKind};

let src = r"\begin{document}\section{Intro}\label{sec:intro}

See Section~\ref{sec:intro}, and also \ref{missing}.\end{document}";
let doc = parse_document(src).unwrap();
let res = doc.resolve_references();

// The `\label` is a definition (a section-kind label).
assert_eq!(res.definitions[0].kind, LabelKind::Section);

// The `\ref{sec:intro}` RESOLVES: both spans slice back to real source.
let r = &res.resolved[0];
assert_eq!(&src[r.ref_span.start..r.ref_span.end], r"\ref{sec:intro}");
assert!(src[r.target_span.start..r.target_span.end].starts_with(r"\section{Intro}"));

// The `\ref{missing}` is dangling (LaTeX's "Reference `missing' undefined").
assert_eq!(res.unresolved[0].key, "missing");
```

The reference family is `{ref, eqref, pageref}`; `\cite` resolves against a *separate* table
(the bibliography — see S2 below), and a multiply-defined key is reported as a `Duplicate` with
**first-def-wins** for resolution. A resolved `\ref` therefore points at the exact defining node's
source bytes — the source→source correlation the ADJ byte-provenance pipeline audits.

### `\cite` → bibliography binding (LTXDOC03 S2)

The parallel pass, `Document::resolve_citations()`, binds each `\cite` key to the `\bibitem` that
defines it inside a `thebibliography` environment — the citation-family analogue of the S1 label
pass, likewise carrying byte spans on both sides and never computing a citation number. It is
disjoint from and non-interfering with S1 (the two read separate command families).

```rust
use latex::parse_document;

let src = r"\begin{document}See \cite{a,b} and \cite{ghost}.

\begin{thebibliography}{9}
\bibitem{a} Author A. First. 2001.
\bibitem{b} Author B. Second. 2002.
\end{thebibliography}\end{document}";
let doc = parse_document(src).unwrap();
let res = doc.resolve_citations();

// `\cite{a,b}` is ONE construct with two keys → two bindings sharing the same `cite_span`.
assert_eq!(res.resolved.len(), 2);
assert_eq!(res.resolved[0].cite_span, res.resolved[1].cite_span);
// Each resolves to its own `\bibitem` span.
assert_eq!(&src[res.resolved[0].entry_span.start..res.resolved[0].entry_span.end], r"\bibitem{a}");

// `\cite{ghost}` is dangling (LaTeX's "Citation `ghost' undefined").
assert_eq!(res.unresolved[0].key, "ghost");
```

A multi-key `\cite{a,b,c}` splits on commas and resolves each key independently (all sharing the
one `\cite` span); `\cite[note]{k}` keeps the `[note]` out of the key; a duplicate `\bibitem` is a
`DuplicateBib` with **first-entry-wins**. Only an **in-document** `thebibliography`/`\bibitem`
bibliography is bound — an external `.bib`/BibTeX database and citation numbering/sorting stay out of
scope (a `\cite` whose key lives only in a `.bib` is reported unresolved here).

### Target → `NodeRef` exposure (LTXDOC03 S3)

S1/S2 bind each target's **bytes** (a `Span`); S3 hands back the actual target **node**, so a
consumer can read its `kind()` and — for a `Block` — descend into its children. The primitive is
`Document::node_for_span(span)` (the walked node whose span **exactly equals** `span`, else `None`);
the ergonomic accessors take a resolved record and return its node.

```rust
use latex::{parse_document, NodeRef, Block};

let src = r"\begin{document}\section{Intro}\label{sec:intro}

First paragraph. See Section~\ref{sec:intro}.\end{document}";
let doc = parse_document(src).unwrap();
let refs = doc.resolve_references();

// S1 bound the ref's *bytes*; S3 hands back the *node*.
let r = &refs.resolved[0];
let node = doc.ref_target_node(r).expect("the resolved target is a walked node");
assert_eq!(node.kind(), "Section");

// It is the REAL section block — descend into the paragraphs it owns.
if let NodeRef::Block(Block::Section { body, .. }) = node {
    assert!(body.iter().any(|b| matches!(b, Block::Paragraph(..))));
}

// A span that is no walked node's own → `None` (never a panic).
assert!(doc.node_for_span(latex::Span::new(9000, 9001)).is_none());
```

`cite_target_node(&ResolvedCite)` returns the `\bibitem` node (an `Inline::Raw`, kind `"Raw"` — the
bibitem inside `thebibliography` **is** walked) and `label_def_node(&LabelDef)` returns a
definition's node. Every S1/S2 target span matches exactly one walked node, so these return `Some`
for any genuine reference/citation; `None` is reserved for the honest edge (empty docs, un-walked
preamble/metadata, fabricated spans). The lookup is span-**equality** (not `node_at`'s containment),
with a first-in-pre-order tie-break that no real target hits. The S1/S2 result types are unchanged
(they keep owned `Span`s) — the `NodeRef` is fetched on demand, so S3 is purely additive.

### Document numbering (LTXDOC03 S4)

S1–S3 bind each `\ref` to its target's bytes and node, but not the **number** it prints. S4 assigns
those numbers — LaTeX's second `.aux` pass, done in one walk over the parsed `Document`:
hierarchical **section** numbers (`1`, `1.1`, `1.2`, `2`, with deeper-reset on each coarser step, and
`\section*` skipped), and independent flat **figure**/**table** counters (each `1, 2, 3, …`, every
float advancing its counter whether labeled or not). `Document::ref_number` is the payoff — the number
a resolved `\ref` prints:

```rust
use latex::parse_document;

let src = r"\begin{document}\section{Intro}\label{sec:intro}

\subsection{Scope}\label{sec:scope}

See Section~\ref{sec:scope}.\end{document}";
let doc = parse_document(src).unwrap();

// Number every defined label, then look one up:
let num = doc.number_labels();
assert_eq!(num.number_for("sec:intro"), Some("1"));
assert_eq!(num.number_for("sec:scope"), Some("1.1")); // subsection under section 1

// The payoff: a resolved `\ref` → the number it prints.
let refs = doc.resolve_references();
let r = refs.resolved.iter().find(|r| r.key == "sec:scope").unwrap();
assert_eq!(doc.ref_number(r), Some("1.1".to_string())); // \ref{sec:scope} → "1.1"
```

`number_labels()` returns a `Numbering` of one `NumberedLabel { key, kind, number }` per defined,
numberable label (section/figure/table). A document that starts deep applies the documented
missing-parent rule (a lone leading `\subsection` numbers `0.1`). Bare-inline labels and other
counters (enumerate/theorem/…) are deferred to S5+; a lifted **equation** label (S7) is numbered by a
flat `step_equation()` counter (S8), independent of section/figure/table, so it prints a real
sequential number (`1`, `2`, …). Numbering is pure, additive analysis — it never
mutates the tree, so the S1/S2/S3 outputs are unchanged.

### Citation numbering (LTXDOC03 S5)

S2 binds each `\cite` to its `\bibitem`, but not the **bracketed number** it prints (`[2]`). S5
assigns those — the citation-family analogue of S4's `ref_number`. In the default numeric/unsorted
style each `\bibitem` is numbered by its **listing position**, so S5 numbers S2's already-ordered
winning entries by index (`entries[0]` → `[1]`, `entries[1]` → `[2]`, …). `Document::cite_number` is
the payoff — the bracketed number a resolved `\cite` prints:

```rust
use latex::parse_document;

let src = r"\begin{document}As shown in \cite{knuth} and \cite{lamport}.

\begin{thebibliography}{9}
\bibitem{knuth} Knuth, D. The TeXbook. 1984.
\bibitem{lamport} Lamport, L. LaTeX. 1986.
\end{thebibliography}\end{document}";
let doc = parse_document(src).unwrap();

// Number every bibliography entry, then look one up:
let num = doc.number_citations();
assert_eq!(num.number_for("knuth"), Some("[1]"));   // first \bibitem
assert_eq!(num.number_for("lamport"), Some("[2]")); // second \bibitem

// The payoff: a resolved `\cite` → the number it prints.
let cites = doc.resolve_citations();
let c = cites.resolved.iter().find(|c| c.key == "lamport").unwrap();
assert_eq!(doc.cite_number(c), Some("[2]".to_string())); // \cite{lamport} → "[2]"
```

`number_citations()` returns a `CitationNumbering` of one `NumberedCitation { key, ordinal, number }`
per numbered entry. A first-`\bibitem`-wins duplicate consumes no number (later entries stay
unshifted); a dangling `\cite` has no entry, so `number_for` returns `None` (LaTeX's `[?]`). The
equation **counter** (`\theequation`, S8), author-year/natbib sorted styles, and external `.bib`
databases remain future rungs. Like S4, S5 is pure, additive analysis — it never mutates the tree, so
the S1–S4 outputs are unchanged.

### The cross-reference report (LTXDOC03 S6)

S1–S5 each produced their own result type; S6 is the **consumer** that composes them into one
auditable artifact. `Document::cross_reference_report()` walks S1's resolved `\ref`s and S2's resolved
`\cite`s and returns an owned report where each entry carries its rendered number (from S4/S5). It
adds no new AST walk — it numbers each family once and looks each key up:

```rust
use latex::parse_document;

let src = r"\begin{document}\section{Intro}\label{sec:intro}

See Section~\ref{sec:intro} and \cite{lamport}. Also \ref{nope} and \cite{ghost}.

\begin{thebibliography}{9}
\bibitem{knuth} Knuth, D. The TeXbook. 1984.
\bibitem{lamport} Lamport, L. LaTeX. 1986.
\end{thebibliography}\end{document}";
let doc = parse_document(src).unwrap();

let rep = doc.cross_reference_report();

// Each resolved reference carries its S4 number, kind, and command.
assert_eq!(rep.refs.len(), 1);
assert_eq!(rep.refs[0].key, "sec:intro");
assert_eq!(rep.refs[0].number, "1"); // \ref{sec:intro} → Section 1

// Each resolved citation carries its S5 bracketed number.
assert_eq!(rep.cites.len(), 1);
assert_eq!(rep.cites[0].key, "lamport");
assert_eq!(rep.cites[0].number, "[2]"); // the second \bibitem

// Dangling refs/cites are surfaced separately (LaTeX's `??` / `[?]`).
assert_eq!(rep.dangling_refs, vec!["nope".to_string()]);
assert_eq!(rep.dangling_cites, vec!["ghost".to_string()]);

// A stable, human-readable rendering (pinned format):
assert_eq!(
    rep.to_plain_text(),
    "\\ref{sec:intro} -> Section 1\n\
     \\cite{lamport} -> [2]\n\
     Dangling references: nope\n\
     Dangling citations: ghost",
);
```

`cross_reference_report()` returns a `CrossReferenceReport { refs, cites, dangling_refs,
dangling_cites }` with `RefEntry { key, command, kind, number }` and `CiteEntry { key, number }`. A
resolved `\ref` to an *unnumbered bare-inline* label is omitted from `refs` (it has no S4 number —
deferred); an **equation** label (S7) is included with its real number (S8), e.g. `Equation 1`. Like S1–S5, S6 is
pure, additive analysis — it reads the prior passes and mutates nothing, so their outputs are unchanged.

### Equation-label lifting (LTXDOC03 S7)

A `\label` inside a **non-starred** display-math environment used to be swallowed into the
`Block::DisplayMath` source string, so an `\eqref` to it resolved (S1) but had no number and was
**omitted** from the S6 report. S7 lifts it out onto the block as a real `LabelKind::Equation`
definition, so the reference is now resolvable and reported:

```rust
use latex::{parse_document, Block, LabelKind};

let src = r"\begin{document}\begin{equation} E = mc^2 \label{eq:e} \end{equation}
See \ref{eq:e}.\end{document}";
let doc = parse_document(src).unwrap();

// The `\label` is lifted onto the block; `source` no longer contains it.
let dm = doc.body.iter().find(|b| matches!(b, Block::DisplayMath { .. })).unwrap();
assert!(matches!(dm, Block::DisplayMath { source, label: Some(k), .. }
    if source == "E = mc^2" && k == "eq:e"));

// It is a real Equation-kind definition, so the `\ref` resolves and is REPORTED.
let rep = doc.cross_reference_report();
assert_eq!(rep.refs.len(), 1);
assert_eq!(rep.refs[0].kind, LabelKind::Equation);
assert_eq!(rep.refs[0].number, "1"); // real `\theequation` value (S8)
assert_eq!(rep.to_plain_text(), "\\ref{eq:e} -> Equation 1");
```

Starred forms (`equation*`, …) and `\[…\]`/`$$…$$` islands keep `label: None` (unnumbered in LaTeX,
so unchanged). The equation **number** (`\theequation`) arrives in S8 (a real `1`, `2`, …), and its
amsmath parenthesisation in S9 (below). `to_latex()` re-emits a lifted-label equation as
`\begin{equation}…\label{…}\end{equation}`, so the round-trip fixed point holds.

### `\eqref` parenthesisation (LTXDOC03 S9)

amsmath's `\eqref{eq:e}` typesets the equation number **parenthesised** — `(1)` — where a plain
`\ref{eq:e}` typesets a bare `1`. S9 makes the S6 report mirror that surface distinction for the one
case that matters: an `\eqref` to an **equation** keeps its `\eqref` spelling and parenthesises the
number; everything else is byte-for-byte the S8 line.

```rust
use latex::parse_document;

let src = r"\begin{document}\begin{equation} E = mc^2 \label{eq:e} \end{equation}
See \eqref{eq:e} and \ref{eq:e}.\end{document}";
let doc = parse_document(src).unwrap();

// The `\eqref` parenthesises; the sibling `\ref` to the same equation stays bare.
assert_eq!(
    doc.cross_reference_report().to_plain_text(),
    "\\eqref{eq:e} -> Equation (1)\n\
     \\ref{eq:e} -> Equation 1",
);
```

Only `command == "eqref"` **and** `kind == LabelKind::Equation` diverges under S9: all `\ref` and any
`\eqref` to a non-equation kind still render with the canonical `\ref` prefix and a bare number
(`\ref{sec:intro} -> Section 1.2`). (`\pageref` diverges separately under S10, below.)
`RefEntry.command` (the surface spelling) was already retained by
S1, so S9 is a pure rendering split — no AST change, no re-numbering, `to_latex()` still a fixed point.

### distinct `\pageref` rendering (LTXDOC03 S10)

A `\pageref{key}` asks "what **page** is the target on" — a fundamentally different question from
`\ref`'s "what **number** is the target". Through S9 the report conflated the two: a resolved
`\pageref` rendered **identically** to a `\ref` (`\ref{sec:i} -> Section 1`). The crate has **no page
model**, so it cannot compute a real page number, but S10 renders the page family *honestly and
distinctly*: a resolved `\pageref` (to **any** target kind) keeps its `\pageref` spelling and prints
the fixed literal placeholder `page ?` (the `?` mirrors LaTeX's own `??` for an unresolved page
reference — it means "page number not modelled", not the kind, not the number).

```rust
use latex::parse_document;

let src = r"\begin{document}\section{Intro}\label{sec:i} \ref{sec:i} \pageref{sec:i}\end{document}";
let doc = parse_document(src).unwrap();

// The `\ref` is unchanged (bare number); the `\pageref` now diverges to the honest placeholder.
assert_eq!(
    doc.cross_reference_report().to_plain_text(),
    "\\ref{sec:i} -> Section 1\n\
     \\pageref{sec:i} -> page ?",
);
```

Branch precedence in the render loop is (1) `\eqref` to Equation → parenthesised (S9); (2) `\pageref`
any kind → `page ?` (S10); (3) else → `\ref{key} -> Kind N` (S8). A `\pageref` ignores the
eqref/Equation special-case entirely — a `\pageref` to an equation still renders `page ?`, never
`Equation (1)`. So `\ref` and `\eqref` outputs are byte-for-byte unchanged; only `\pageref` lines
change. Pure rendering branch — no AST change, no re-numbering, `to_latex()` still a fixed point.

### grouped-by-kind cross-reference report (LTXDOC03 S11)

`to_plain_text` renders resolved references in flat source order. `to_plain_text_by_kind` is a
**separate, sibling** method that renders the **same** resolved-ref lines **grouped under fixed-order
kind subheadings** — Sections, Figures, Tables, Equations, Inline — so a reader can answer "which
sections / figures / equations does this document cross-reference?" at a glance. The kind order is
fixed regardless of source order; within a kind group the refs keep their source (pre-order) order; a
kind with zero resolved refs is omitted entirely (no empty subheading). The per-line rendering is the
**identical** S8/S9/S10 rule — a shared `render_resolved_ref` helper backs **both** methods, so the
flat and grouped reports can never drift.

```rust
use latex::parse_document;

let src = r"\begin{document}\section{Intro}\label{sec:intro}
\section{Methods}\label{sec:methods}
\begin{figure}\caption{A plot}\label{fig:plot}\end{figure}
\begin{equation} a = 1 \label{eq:e} \end{equation}
See \ref{sec:intro}, \ref{sec:methods}, \ref{fig:plot}, \eqref{eq:e}.\end{document}";
let doc = parse_document(src).unwrap();

// Grouped under fixed-order kind subheadings, two-space-indented ref lines in pre-order.
assert_eq!(
    doc.cross_reference_report().to_plain_text_by_kind(),
    "Sections:\n\
     \x20 \\ref{sec:intro} -> Section 1\n\
     \x20 \\ref{sec:methods} -> Section 2\n\
     Figures:\n\
     \x20 \\ref{fig:plot} -> Figure 1\n\
     Equations:\n\
     \x20 \\eqref{eq:e} -> Equation (1)",
);
```

Only resolved references are grouped (citations and dangling footers are **not** included — the flat
`to_plain_text` remains the full report); a `\pageref` groups under its **target kind** (a `\pageref`
to a section sits under `Sections:`). A report with zero resolved refs renders the fixed marker
`(no resolved references)` — the S11 analogue of `to_plain_text`'s `(no cross-references)`. Pure
report-assembly over data the report already holds — no AST/struct/numbering change, `to_latex()`
still a fixed point.

### `\nameref` resolution to a target's name (LTXDOC03 S13)

`\ref` prints a target's **number** ("Section 1"); `\pageref` prints its **page**. The `nameref`
package's `\nameref` prints its **name** — a section's title, a float's caption text. `resolve_namerefs`
is a **new** method that answers exactly that: it walks the body, finds every `\nameref{key}`, resolves
the key against the same S1 `\label` table `\ref` uses, and renders the target's name. Because
`\nameref` is **not** a `REF_COMMAND`, it appears in neither the resolved nor the unresolved reference
table — S13 reads the same table but asks a different question, so it changes no S1–S12 output.

```rust
use latex::parse_document;

let src = r"\begin{document}\section{Introduction}\label{sec:intro}
\begin{figure}\caption{A plot}\label{fig:p}\end{figure}

See \nameref{sec:intro}, \nameref{fig:p}, and \nameref{nope}.\end{document}";
let doc = parse_document(src).unwrap();

// One `\nameref{key} -> <name>` line per reference, in body order.
assert_eq!(
    doc.resolve_namerefs(),
    "\\nameref{sec:intro} -> Introduction\n\
     \\nameref{fig:p} -> A plot\n\
     \\nameref{nope} -> (undefined nameref: nope)",
);
```

A `Section` target resolves to its title text; a `Figure`/`Table` to its `\caption` text (via the
**same** `caption_text` descent S12's List-of-Floats uses, so the two agree). An `Equation`/inline-label
target has a number, not a name, so it renders `(no name)`; an undefined key renders
`(undefined nameref: <key>)`; a document with no `\nameref` at all renders `(no namerefs)`. Pure
assembly over existing blocks + the S1 label table — no AST/grammar change, `to_latex()` still a fixed
point.

### per-kind census of numbered labels (LTXDOC03 S14)

`number_labels` assigns each numbered label a *number*; `list_summary` answers the coarser question
*"how many of each kind are there?"*. It is a pure tally of that same table, grouped by kind — one
line per non-zero kind in the fixed order `Sections`, `Figures`, `Tables`, `Equations` (never source
order), each `<Kind>: <count>` with a fixed plural label; zero-count kinds are omitted; no numbered
label at all → `(no labels)`.

```rust
use latex::parse_document;

let src = r"\begin{document}\section{One}\label{sec:a}
\section{Two}\label{sec:b}
\begin{figure}\includegraphics{p.png}\caption{A plot}\label{fig:p}\end{figure}
\begin{table}\begin{tabular}{lc}a & b\end{tabular}\caption{Data}\label{tab:d}\end{table}
\begin{equation}\label{eq:e}E=mc^2\end{equation}\end{document}";
let doc = parse_document(src).unwrap();
assert_eq!(
    doc.list_summary(),
    "Sections: 2\nFigures: 1\nTables: 1\nEquations: 1",
);
```

A bare inline `\label{…}` is not numbered, so it never appears in this census; a document whose only
label is such an inline label renders the `(no labels)` marker. Purely additive — reuses
`number_labels`, mutates nothing, leaves every S1–S13 output and the `to_latex()` fixed point
unchanged.

### resolved citations grouped by their source `\cite` (LTXDOC03 S15)

S2's `resolve_citations` flattens a multi-key `\cite{a,b}` into *several* `ResolvedCite` rows (one per
key, all sharing that `\cite`'s `cite_span`). `citations_by_source` reads only that `resolved` list and
re-assembles it: it groups the rows back by `cite_span` — in first-appearance order, i.e. source order
of the `\cite`s — and emits one line per `\cite`, reconstructed as `\cite{` + the group's resolved keys
joined by `", "` + `}`. A dangling key never entered `resolved`, so it is excluded by construction.

```rust
use latex::parse_document;

let src = r"\begin{document}
See \cite{a,b} and \cite{c,ghost}.
\begin{thebibliography}{9}
\bibitem{a} Author A.
\bibitem{b} Author B.
\bibitem{c} Author C.
\end{thebibliography}
\end{document}";
let doc = parse_document(src).unwrap();
assert_eq!(
    doc.citations_by_source(),
    "\\cite{a, b}\n\\cite{c}",   // both of {a,b} resolve; only `c` of {c,ghost} does
);
```

A document with no resolved citations — none present, or every cited key dangling — renders the fixed
`(no resolved citations)` marker. Purely additive — reuses `resolve_citations`, mutates nothing, leaves
every S1–S14 output and the `to_latex()` fixed point unchanged.

### duplicate (multiply-defined) bibliography entries (LTXDOC03 S16)

S2's `resolve_citations` collects every `\bibitem` in pre-order: the **first** of each key wins (it is
the entry citations resolve against), and every **later** `\bibitem` of an already-defined key becomes
a losing duplicate in `duplicate_entries` — LaTeX's *"Citation `key' multiply defined"* warning.
`duplicate_bibliography_entries` reads only that list and emits one line per losing duplicate, in the
existing pre-order (not re-sorted, not de-duplicated), each reconstructed from its key as
`\bibitem{<key>}`.

```rust
use latex::parse_document;

let src = r"\begin{document}\cite{smith}.
\begin{thebibliography}{9}
\bibitem{smith} First Smith. 1990.
\bibitem{jones} Jones. 1991.
\bibitem{smith} Second Smith. 1992.
\end{thebibliography}\end{document}";
let doc = parse_document(src).unwrap();
assert_eq!(
    doc.duplicate_bibliography_entries(),
    "\\bibitem{smith}",   // only the SECOND `\bibitem{smith}` loses; `jones` (once) is not listed
);
```

A document with no duplicates — no bibliography, or every key defined exactly once — renders the fixed
`(no duplicate bibliography entries)` marker. Purely additive — reuses `resolve_citations`, mutates
nothing, leaves every S1–S15 output and the `to_latex()` fixed point unchanged.

### unresolved (dangling) citations grouped by source `\cite` (LTXDOC03 S17)

The DANGLING-key mirror of S15's `citations_by_source`, and the per-`\cite` view of S6's flat
*"Dangling citations"* footer. S2's `resolve_citations` flattens every `\cite` into per-key rows,
splitting them into the **resolved** keys and the **unresolved** (dangling) keys, each tagged with the
citing `\cite`'s `cite_span`. `unresolved_citations_by_source` reads only that `unresolved` list and
groups the dangling keys back by `cite_span` in **first-appearance order** (source order of the
`\cite`s), emitting one line per source `\cite` that has ≥1 dangling key: `\cite{` + its dangling keys
joined by `", "` + `}`. Because only dangling keys are in `unresolved`, a `\cite{a, ghost}` where `a`
resolves renders `\cite{ghost}`.

```rust
use latex::parse_document;

let src = r"\begin{document}
See \cite{known, ghost} and \cite{x, y}.
\begin{thebibliography}{9}
\bibitem{known} Author K.
\end{thebibliography}
\end{document}";
let doc = parse_document(src).unwrap();
assert_eq!(
    doc.unresolved_citations_by_source(),
    "\\cite{ghost}\n\\cite{x, y}",   // first drops resolved `known`; second is fully dangling
);
```

A document with no unresolved citations — every cited key resolves, or none present — renders the fixed
`(no unresolved citations)` marker. Purely additive — reuses `resolve_citations`, mutates nothing,
leaves every S1–S16 output and the `to_latex()` fixed point unchanged.

### unresolved (dangling) references grouped by source `\ref` (LTXDOC03 S18)

The `\ref`-family parallel of S17's dangling-CITATION report, and a **distinct** view from S6's flat
*"Dangling references: k1, k2"* footer: S18 reconstructs each dangling reference on **its own line**,
**command-aware**. S3's `resolve_references` walks every `\ref`/`\eqref`/`\pageref` in body pre-order and
routes the dangling ones into `unresolved` as `UnresolvedRef { key, command, ref_span }`.
`unresolved_references_by_source` reads only that `unresolved` list and groups by `ref_span` in
**first-appearance order** (source order). Because each reference takes exactly **one** key, every group
is a single entry emitting one line: `\` + the reference's own `command` + `{` + its `key` + `}`. Rebuilt
from the ref's own `command`, a dangling `\eqref{eq:x}` renders `\eqref{eq:x}` and a dangling
`\pageref{p}` renders `\pageref{p}` — never a hard-coded `\ref`.

```rust
use latex::parse_document;

let src = r"\begin{document}
See \eqref{eq:ghost} on \pageref{p:ghost}.
\end{document}";
let doc = parse_document(src).unwrap();
assert_eq!(
    doc.unresolved_references_by_source(),
    "\\eqref{eq:ghost}\n\\pageref{p:ghost}",   // each command preserved, one per line, source order
);
```

A document with no unresolved references — every reference resolves, or none present — renders the fixed
`(no unresolved references)` marker. Purely additive — reuses `resolve_references`, mutates nothing,
leaves every S1–S17 output and the `to_latex()` fixed point unchanged.

### numbered winning-bibliography-entry list (LTXDOC03 S19)

The **winning** bibliography entries rendered as a **numbered list** — the rendered bibliography a reader
actually sees, and the table citations resolve against. A **distinct** view over `resolve_citations()`:
S16 (`duplicate_bibliography_entries`) renders the **losing** duplicates as `\bibitem{key}` warning
lines, and S15 (`citations_by_source`) renders per-source *resolved cite keys* — S19 renders the
**winning** `entries` themselves. S2 collects the first `\bibitem{key}` of each distinct key into
`resolve_citations().entries` (body pre-order; later re-definitions go to `duplicate_entries`, never
here). `bibliography_entries` numbers that list **1-based** and emits one line per winning entry as
`[n] key` (reconstructed from the owned key — no source slicing). The `[n] key` shape is deliberately
distinct from S16's `\bibitem{key}` lines, so the winning list never looks like the losing-duplicate
report even when keys overlap. A `\bibitem{dup}` written twice appears **once** — the winner.

```rust
use latex::parse_document;

let src = r"\begin{document}
\begin{thebibliography}{9}
\bibitem{smith} First Smith.
\bibitem{jones} Jones.
\bibitem{smith} Second Smith.
\end{thebibliography}
\end{document}";
let doc = parse_document(src).unwrap();
assert_eq!(
    doc.bibliography_entries(),
    "[1] smith\n[2] jones",   // numbered, 1-based, pre-order; the duplicate `smith` wins once
);
```

A document with no bibliography entries — no `thebibliography`, or an empty one — renders the fixed
`(no bibliography entries)` marker. Purely additive — reuses `resolve_citations`, mutates nothing, leaves
every S1–S18 output and the `to_latex()` fixed point unchanged.

### losing duplicate `\label` definitions (LTXDOC03 S20)

The **losing** duplicate `\label` definitions — LaTeX's *"Label `key' multiply defined"* warnings —
the **label-family mirror of S16's** `duplicate_bibliography_entries` (which renders the losing
`\bibitem` duplicates). S1 splits every `\label` into the **winning** first definition of each key
(`resolve_references().definitions`, what `\ref`/`\eqref`/`\pageref` resolve against) and the **losing**
later re-definitions (`duplicates`, in body pre-order). `duplicate_label_definitions` reads only that
`duplicates` list and emits one line per losing duplicate — **not** re-sorted, **not** de-duplicated, so
every *"multiply defined"* warning gets its own line — each reconstructed from its owned key as
`\label{<key>}` (no source slicing; the `\label{…}` form is correct for any `LabelKind`, whether the
re-`\label`ed node was a section, figure, equation, or bare inline label).

```rust
use latex::parse_document;

let src = r"\begin{document}First \label{dup} here.

Second \label{dup} there.

And \label{once}.\end{document}";
let doc = parse_document(src).unwrap();
assert_eq!(
    doc.duplicate_label_definitions(),
    "\\label{dup}",   // only the losing second `\label{dup}`; the first wins, `once` isn't a dup
);
```

A document with no duplicate labels — every key defined once, or none at all — renders the fixed
`(no duplicate label definitions)` marker. Purely additive — reuses `resolve_references`, mutates
nothing, leaves every S1–S19 output and the `to_latex()` fixed point unchanged.

### resolved references grouped by source `\ref` (LTXDOC03 S21)

The **resolved** (successfully-matched) references — the **RESOLVED mirror of S18's**
`unresolved_references_by_source` (which renders the *dangling* half of the same split). S3 splits every
`\ref`/`\eqref`/`\pageref` into the **resolved** references (`resolve_references().resolved`, those a
`\label` defines) and the **unresolved** (dangling) ones (`unresolved`). `resolved_references_by_source`
reads only that `resolved` list and groups by `ref_span` in **first-appearance order** (source order),
emitting one line per resolved reference — reconstructed from its **own** `command` and `key` as
`\<command>{<key>}` (no source slicing) — so a resolved `\eqref{eq:main}` renders `\eqref{eq:main}` and a
resolved `\pageref{sec:intro}` renders `\pageref{sec:intro}`; the command is **never** flattened to
`\ref`. A dangling `\ref` never entered `resolved`, so it is excluded (it lives in S18).

```rust
use latex::parse_document;

let src = r"\begin{document}\section{Intro}\label{sec:intro}
See \ref{sec:intro} and \eqref{eq:main} and \pageref{sec:intro}. Also \ref{nope}.
\begin{equation}\label{eq:main}x=1\end{equation}
\end{document}";
let doc = parse_document(src).unwrap();
assert_eq!(
    doc.resolved_references_by_source(),
    // three resolved refs, command preserved, in source order; the dangling `\ref{nope}` is excluded
    "\\ref{sec:intro}\n\\eqref{eq:main}\n\\pageref{sec:intro}",
);
```

A document with no resolved references — every reference dangles, or there are none at all — renders the
fixed `(no resolved references)` marker. Purely additive — reuses `resolve_references`, mutates nothing,
leaves every S1–S20 output and the `to_latex()` fixed point unchanged.

### winning `\label` definitions (LTXDOC03 S22)

The **winning** label definitions — the `\label{key}` definitions references resolve against — the
**label-family analogue of S19's** `bibliography_entries` (which renders the winning `\bibitem` entries)
and the **winning-side counterpart of S20's** `duplicate_label_definitions` (which renders the *losing*
duplicate `\label`s). S1 splits every `\label` into the **winning** first definition of each key
(`resolve_references().definitions`, one row per distinct key, in pre-order) and the **losing** later
re-definitions (`duplicates`). `label_definitions` reads only that `definitions` list and emits one line
per winning definition, in pre-order — reconstructed from its owned `key` as `\label{<key>}` (no source
slicing; the `\label{…}` form is right for any `LabelKind`). No re-sorting and no de-duplication are
needed, because `definitions` already holds exactly one row per distinct key; a `\label{dup}` written
twice appears **once** here (its losing second definition lives in S20).

```rust
use latex::parse_document;

let src = r"\begin{document}\section{Intro}\label{sec:intro}
\begin{equation}\label{eq:main} x=1 \end{equation}
\subsection{Dup}\label{sec:intro}\end{document}";
let doc = parse_document(src).unwrap();
assert_eq!(
    doc.label_definitions(),
    // the winning key `sec:intro` appears once; the later re-`\label`ed `sec:intro` is a duplicate (S20)
    "\\label{sec:intro}\n\\label{eq:main}",
);
```

A document with no label definitions renders the fixed `(no label definitions)` marker. Purely
additive — reuses `resolve_references`, mutates nothing, leaves every S1–S21 output and the `to_latex()`
fixed point unchanged.

### winning `\label` definitions grouped by kind (LTXDOC03 S23)

The **winning** `\label` definitions **grouped by their `LabelKind`** — a per-kind census — the
**by-kind grouping companion of S22's** `label_definitions` (which lists the same winning definitions
*flat*, one `\label{key}` per line in pure pre-order). S22 and S23 are two *views* of the one winning
`definitions` list `resolve_references()` produces. `label_definitions_by_kind` reads only that
`definitions` list and groups it by kind in a **fixed, document-independent order** — the `LabelKind`
enum declaration order (`Section`, `Table`, `Figure`, `Equation`, `Inline`), iterated as an explicit
slice rather than a hash map, so the group order is deterministic (the same `Vec`-of-groups discipline
S17/S18 use). Within each kind, definitions keep their existing pre-order. Each line is
`[<kind>] \label{<key>}` — the `<kind>` tag from `LabelKind::as_str()`, the `<key>` from the owned key
(no source slicing). A kind with no definitions contributes no lines (no empty `[table]` header).

```rust
use latex::parse_document;

let src = r"\begin{document}\section{Intro}\label{sec:intro}
\begin{equation}\label{eq:main} x=1 \end{equation}
\label{note}\end{document}";
let doc = parse_document(src).unwrap();
assert_eq!(
    doc.label_definitions_by_kind(),
    // grouped in the fixed kind order: section, then equation, then inline
    "[section] \\label{sec:intro}\n[equation] \\label{eq:main}\n[inline] \\label{note}",
);
```

A document with no label definitions renders the **same** fixed `(no label definitions)` marker S22
uses (S23 groups the identical list). Purely additive — reuses `resolve_references`, mutates nothing,
leaves every S1–S22 output and the `to_latex()` fixed point unchanged.

### resolved references grouped by target kind (LTXDOC03 S24)

The **resolved** `\ref`/`\eqref`/`\pageref` references **grouped by the `LabelKind` they resolved TO**
— a per-kind census — the **by-kind grouping companion of S21's** `resolved_references_by_source`
(which lists the same resolved refs *flat*, one `\<command>{key}` per line in source pre-order). S21
and S24 are two *views* of the one `resolved` list `resolve_references()` produces. It is the exact
`resolved`-refs analogue of S23's `label_definitions_by_kind` (same `const KIND_ORDER`, same
`flat_map`/`filter` pass), and stays **command-aware** like S21. `resolved_references_by_kind` reads
only that `resolved` list and groups it by `target_kind` in a **fixed, document-independent order** —
the `LabelKind` enum declaration order (`Section`, `Table`, `Figure`, `Equation`, `Inline`, the SAME
slice S23 uses), iterated as an explicit slice rather than a hash map, so the group order is
deterministic. Within each kind, refs keep their existing pre-order. Each line is
`[<kind>] \<command>{<key>}` — the `<kind>` tag from the ref's `target_kind.as_str()`, the `<command>`
the ref's own (so `\eqref`/`\pageref` render as themselves), the `<key>` from the owned key (no source
slicing). A **dangling** `\ref` never entered `resolved`, so it is excluded (it lives in S18). A kind
with no resolved refs contributes no lines (no empty `[table]` header).

```rust
use latex::parse_document;

let src = r"\begin{document}\section{Intro}\label{sec:intro}
\begin{equation}\label{eq:main} x=1 \end{equation}
\ref{sec:intro} \eqref{eq:main} \pageref{sec:intro}\end{document}";
let doc = parse_document(src).unwrap();
assert_eq!(
    doc.resolved_references_by_kind(),
    // grouped in the fixed kind order: both section refs (in pre-order), then the equation ref
    "[section] \\ref{sec:intro}\n[section] \\pageref{sec:intro}\n[equation] \\eqref{eq:main}",
);
```

A document with no resolved references (every ref dangles, or there are none) renders the **same** fixed
`(no resolved references)` marker S21 uses. Purely additive — reuses `resolve_references`, mutates
nothing, leaves every S1–S23 output and the `to_latex()` fixed point unchanged.

### per-kind census (counts) of the winning `\label` definitions (LTXDOC03 S25)

A **per-kind CENSUS** of the winning `\label` definitions — one `<kind>: <n>` line per `LabelKind`
that has at least one winning definition, carrying the integer **count** rather than a list — the
**count companion of S23's** `label_definitions_by_kind` (which renders one `[kind] \label{key}` line
*per definition*, grouped by kind). S22's flat `label_definitions`, S23's grouped list, and S25's
counts are three *views* of the one winning `definitions` list `resolve_references()` produces; S25
collapses each kind's group to a single tally line. It is to S23 what S14's `list_summary`
(`"Sections: 1"`) is to a full enumeration: a numeric summary. `label_kind_counts` reads only that
`definitions` list — one row per distinct key, the WINNER (a `\label{dup}` written twice counts
**once**, its later copy being a `Duplicate` in S20's domain) — and counts by kind in a **fixed,
document-independent order** (the `LabelKind` enum declaration order: `Section`, `Table`, `Figure`,
`Equation`, `Inline`, the SAME slice S23/S24 use), iterated as an explicit slice rather than a hash
map, so the line order is deterministic. Each line is `<kind>: <n>` — the `<kind>` tag from
`LabelKind::as_str()` (the SAME tag S23 renders), the `<n>` the decimal count (no source slicing). A
kind with a zero count contributes no line (no bare `table: 0`).

```rust
use latex::parse_document;

let src = r"\begin{document}\section{Intro}\label{sec:intro}
\begin{equation}\label{eq:a} x=1 \end{equation}
\begin{equation}\label{eq:b} y=2 \end{equation}
\label{note}\end{document}";
let doc = parse_document(src).unwrap();
assert_eq!(
    doc.label_kind_counts(),
    // one count line per non-empty kind, in the fixed kind order (Table/Figure omitted — zero)
    "section: 1\nequation: 2\ninline: 1",
);
```

A document with no label definitions renders the **same** fixed `(no label definitions)` marker
S22/S23 use (S25 counts the identical list). Purely additive — reuses `resolve_references`, mutates
nothing, leaves every S1–S24 output and the `to_latex()` fixed point unchanged.

### per-kind census (counts) of the resolved references (LTXDOC03 S26)

A **per-kind CENSUS** of the RESOLVED `\ref`/`\eqref`/`\pageref` references — one `<kind>: <n>` line
per `LabelKind` that has at least one resolved ref, carrying the integer **count** rather than a list —
the **count companion of S24's** `resolved_references_by_kind` (which renders one `[kind] \<command>{key}`
line *per resolved ref*, grouped by the kind each ref bound to). S21's flat `resolved_references_by_source`,
S24's grouped list, and S26's counts are three *views* of the one `resolved` list `resolve_references()`
produces; S26 collapses each kind's group to a single tally line. It is to S24 what S25's
`label_kind_counts` is to S23: a numeric summary. `resolved_reference_kind_counts` reads only that
`resolved` list — each entry a `ResolvedRef` carrying the `target_kind` (the kind of the label it bound
to; a dangling `\ref` lives in `unresolved`, S18's domain, and is excluded by construction — never a
spurious `<kind>: 0`) — and counts by `target_kind` in a **fixed, document-independent order** (the
`LabelKind` enum declaration order: `Section`, `Table`, `Figure`, `Equation`, `Inline`, the SAME slice
S23/S24/S25 use), iterated as an explicit slice rather than a hash map, so the line order is
deterministic. Each line is `<kind>: <n>` — the `<kind>` tag from `LabelKind::as_str()` (the SAME tag
S24 renders), the `<n>` the decimal count (no source slicing). A kind with a zero count contributes no
line (no bare `table: 0`).

```rust
use latex::parse_document;

let src = r"\begin{document}\section{One}\label{sec:a}
\section{Two}\label{sec:b}
\begin{equation}\label{eq:e} E=mc^2 \end{equation}
\ref{sec:a} \ref{sec:b} \eqref{eq:e}\end{document}";
let doc = parse_document(src).unwrap();
assert_eq!(
    doc.resolved_reference_kind_counts(),
    // one count line per non-empty kind, in the fixed kind order (Table/Figure/Inline omitted — zero)
    "section: 2\nequation: 1",
);
```

A document with no resolved references (all dangling, or none at all) renders the **same** fixed
`(no resolved references)` marker S21/S24 use (S26 counts the identical list). Purely additive — reuses
`resolve_references`, mutates nothing, leaves every S1–S25 output and the `to_latex()` fixed point
unchanged.

### single-integer total of the unresolved (dangling) references (LTXDOC03 S27)

The decimal **COUNT** of the UNRESOLVED (dangling) `\ref`/`\eqref`/`\pageref` references — the ones no
`\label` defines (LaTeX's *"Reference `key' undefined"*, the `??`) — as one integer line. It is the
**count-total companion of S18's** `unresolved_references_by_source` (which renders one `\<command>{key}`
line *per dangling ref*, in body pre-order): S18 and S27 are two *views* of the one `unresolved` list
`resolve_references()` produces; S27 collapses the whole list to a single `.len()` tally. It is the
count-total sibling of the census family (S25's `label_kind_counts`, S26's
`resolved_reference_kind_counts`), but for the UNRESOLVED refs — which carry **no** `target_kind` (a
dangling ref bound to nothing), so a per-kind census is not viable and a single total is the clean move.
`unresolved_reference_count` reads only `resolve_references().unresolved.len()` — a resolved `\ref{sec:i}`
lives in `resolved` (S21's domain) and is excluded by construction — never a `target_kind`, with no
source slicing at all.

```rust
use latex::parse_document;

let src = r"\begin{document}\section{Intro}\label{sec:i}
\ref{sec:i} \ref{nope} \ref{gone}\end{document}";
let doc = parse_document(src).unwrap();
assert_eq!(
    doc.unresolved_reference_count(),
    // two refs dangle (`nope`, `gone`); the resolved `\ref{sec:i}` is excluded
    "2",
);
```

Being a COUNT renderer, a document with no dangling references (every ref resolves, or none at all)
renders the honest number `"0"` — **not** a `(no …)` marker (that discipline belongs to the *list*
renderers S18/S21/S24, whose empty case has no lines to show). Purely additive — reuses
`resolve_references`, mutates nothing, leaves every S1–S26 output and the `to_latex()` fixed point
unchanged.

### single-integer total of the resolved references (LTXDOC03 S28)

The decimal **COUNT** of the RESOLVED `\ref`/`\eqref`/`\pageref` references — the ones some `\label`
defines — as one integer line. It is the **count-total companion of S21's** `resolved_references_by_source`
**and S24's** `resolved_references_by_kind` (which render one `\<command>{key}` line *per resolved ref*,
flat in source order or grouped by target kind): S21/S24 and S28 are two *views* of the one `resolved`
list `resolve_references()` produces; S28 collapses the whole list to a single `.len()` tally. It is the
exact resolved-side **twin of S27's** `unresolved_reference_count` — together S28 + S27 split every
reference into the pair (resolved, dangling), so their totals sum to the total reference count.
`resolved_reference_count` reads only `resolve_references().resolved.len()` — a dangling `\ref{nope}`
lives in `unresolved` (S18/S27's domain) and is excluded by construction — never a `target_kind`, with no
source slicing at all, so section/table/equation references all fold into one total.

```rust
use latex::parse_document;

let src = r"\begin{document}\section{Intro}\label{sec:i}
\ref{sec:i} \pageref{sec:i} \ref{nope}\end{document}";
let doc = parse_document(src).unwrap();
assert_eq!(
    doc.resolved_reference_count(),
    // two refs resolve (`\ref{sec:i}`, `\pageref{sec:i}`); the dangling `\ref{nope}` is excluded
    "2",
);
```

Being a COUNT renderer, a document with no resolved references (every ref dangles, or none at all)
renders the honest number `"0"` — **not** a `(no resolved references)` marker (that discipline belongs to
the *list* renderers S21/S24, whose empty case has no lines to show; this mirrors S27). Purely additive —
reuses `resolve_references`, mutates nothing, leaves every S1–S27 output and the `to_latex()` fixed point
unchanged.

### single-integer total of the label definitions (LTXDOC03 S29)

The decimal **COUNT** of the winning label definitions — the distinct `\label` keys the document defines —
as one integer line. It is the **count-total companion of S22's** `label_definitions` **and S23's**
`label_definitions_by_kind` (which render one `\label{key}` line *per winning definition*, flat in source
order or grouped by kind): S22/S23 and S29 are two *views* of the one winning `definitions` list
`resolve_references()` produces; S29 collapses the whole list to a single `.len()` tally. It is the exact
label-definition-side **analogue** of the reference-side totals S27's `unresolved_reference_count` and
S28's `resolved_reference_count`, and the count-total sibling of the census family (S25 `label_kind_counts`)
but over the *whole* definition list rather than per-kind. `label_definition_count` reads only
`resolve_references().definitions.len()` — a later duplicate `\label{dup}` lives in `duplicates` (S20's
domain) and is excluded by construction, so the count is exactly the number of lines S22 lists — never a
`kind`, with no source slicing at all, so section/figure/equation/inline labels all fold into one total.

```rust
use latex::parse_document;

let src = r"\begin{document}\section{Intro}\label{sec:intro}
\begin{equation}\label{eq:main}x=1\end{equation}
\subsection{Dup}\label{sec:intro}\end{document}";
let doc = parse_document(src).unwrap();
assert_eq!(
    doc.label_definition_count(),
    // two distinct keys defined (`sec:intro`, `eq:main`); the later `\label{sec:intro}` is a duplicate
    "2",
);
```

Being a COUNT renderer, a document with no label definitions at all renders the honest number `"0"` —
**not** a `(no label definitions)` marker (that discipline belongs to the *list* renderers S22/S23, whose
empty case has no lines to show; this mirrors S27/S28). Purely additive — reuses `resolve_references`,
mutates nothing, leaves every S1–S28 output and the `to_latex()` fixed point unchanged.

### single-integer total of the bibliography entries (LTXDOC03 S30)

The decimal **COUNT** of the winning bibliography entries — the distinct `\bibitem` keys the document defines
inside a `thebibliography` environment — as one integer line. It is the **count-total companion of S19's**
`bibliography_entries` (which renders one `[n] key` line *per winning entry*, 1-based in source order): S19
and S30 are two *views* of the one winning `entries` list `resolve_citations()` produces; S30 collapses the
whole list to a single `.len()` tally. It is the exact **citation-side analogue of S29's**
`label_definition_count`, completing the *totals family* — S27's `unresolved_reference_count` and S28's
`resolved_reference_count` count the two reference tables, S29 counts the label definitions, and S30 counts
the bibliography entries. `bibliography_entry_count` reads only `resolve_citations().entries.len()` — a later
duplicate `\bibitem{dup}` lives in `duplicate_entries` (S16's domain) and is excluded by construction, so the
count is exactly the number of lines S19 lists, with no source slicing at all.

```rust
use latex::parse_document;

let src = r"\begin{document}\begin{thebibliography}{9}
\bibitem{a} Author A.\bibitem{b} Author B.\bibitem{a} Author A again.\end{thebibliography}\end{document}";
let doc = parse_document(src).unwrap();
assert_eq!(
    doc.bibliography_entry_count(),
    // two distinct keys defined (`a`, `b`); the later `\bibitem{a}` is a duplicate
    "2",
);
```

Being a COUNT renderer, a document with no bibliography entries at all renders the honest number `"0"` —
**not** a `(no bibliography entries)` marker (that discipline belongs to the *list* renderer S19, whose empty
case has no lines to show; this mirrors S27/S28/S29). Purely additive — reuses `resolve_citations`, mutates
nothing, leaves every S1–S29 output and the `to_latex()` fixed point unchanged.

### single-integer total of the resolved citations (LTXDOC03 S31)

The decimal **COUNT** of the resolved `\cite` keys — the ones some `\bibitem` defines — as one integer line.
It is the **count-total companion of S15's** `citations_by_source` (which renders the resolved keys *grouped
by their source `\cite`*): S15 and S31 are two *views* of the one `resolved` list `resolve_citations()`
produces; S31 collapses the whole list to a single `.len()` tally. It is the exact resolved-**citation-side
twin of S28's** `resolved_reference_count`, extending the *totals family* onto the resolved-citation table —
S27's `unresolved_reference_count` and S28's `resolved_reference_count` count the two reference tables, S29
counts the label definitions, S30 counts the bibliography entries, and S31 counts the resolved citations.
`citation_count` reads only `resolve_citations().resolved.len()` — a dangling `\cite{ghost}` lives in
`unresolved` (S17's domain) and is excluded by construction — never a `cite_span`/`entry_span`, no source
slicing at all, so every resolved key folds into one total.

```rust
use latex::parse_document;

let src = r"\begin{document}See \cite{a,b} plus \cite{c,ghost}.
\begin{thebibliography}{9}\bibitem{a} A.\bibitem{b} B.\bibitem{c} C.\end{thebibliography}\end{document}";
let doc = parse_document(src).unwrap();
assert_eq!(
    doc.citation_count(),
    // three keys resolve (`a`, `b`, `c`); the dangling `ghost` is excluded
    "3",
);
```

Being a COUNT renderer, a document with no resolved citations at all (every cited key dangling, or none at
all) renders the honest number `"0"` — **not** a `(no resolved citations)` marker (that discipline belongs to
the *list* renderer S15, whose empty case has no lines to show; this mirrors S27/S28/S29/S30). Purely
additive — reuses `resolve_citations`, mutates nothing, leaves every S1–S30 output and the `to_latex()` fixed
point unchanged.

### single-integer total of the unresolved (dangling) citations (LTXDOC03 S32)

The decimal **COUNT** of the unresolved (dangling) `\cite` keys — the ones **no** `\bibitem` defines — as one
integer line. It is the **count-total companion of S17's** `unresolved_citations_by_source` (which renders
the dangling keys *grouped by their source `\cite`*): S17 and S32 are two *views* of the one `unresolved`
list `resolve_citations()` produces; S32 collapses the whole list to a single `.len()` tally. It is the exact
unresolved-**citation-side twin of S27's** `unresolved_reference_count`, and the **dangling sibling of S31's**
resolved `citation_count`. Together S31 and S32 **partition** every per-key `\cite` record —
`citation_count + unresolved_citation_count` is exactly the number of cited keys, because
`resolve_citations()` routes each key into exactly one of `resolved`/`unresolved`. `unresolved_citation_count`
reads only `resolve_citations().unresolved.len()` — a resolved `\cite{a}` lives in `resolved` (S15/S31's
domain) and is excluded by construction — never a `cite_span`/dangling `key`, no source slicing at all, so
every dangling key folds into one total.

```rust
use latex::parse_document;

let src = r"\begin{document}See \cite{a,b} plus \cite{c,ghost}.
\begin{thebibliography}{9}\bibitem{a} A.\bibitem{b} B.\bibitem{c} C.\end{thebibliography}\end{document}";
let doc = parse_document(src).unwrap();
assert_eq!(
    doc.unresolved_citation_count(),
    // one key dangles (`ghost`); the three resolved keys `a`, `b`, `c` are excluded (they are S31's "3")
    "1",
);
```

Being a COUNT renderer, a document with no dangling citations at all (every cited key resolving, or none at
all) renders the honest number `"0"` — **not** a `(no unresolved citations)` marker (that discipline belongs
to the *list* renderer S17, whose empty case has no lines to show; this mirrors S27/S28/S29/S30/S31). Purely
additive — reuses `resolve_citations`, mutates nothing, leaves every S1–S31 output and the `to_latex()` fixed
point unchanged.

## Usage

```rust
use latex::{parse, NodeKind};

let doc = parse(r"Let $x$ be \textbf{bold}.").unwrap();
assert!(matches!(doc[0].kind, NodeKind::Text(_)));                       // "Let"
assert!(doc.iter().any(|n| matches!(n.kind, NodeKind::Math { .. })));    // $x$
// every node carries its exact source byte span (LTXDOC02 S1):
let src = r"Let $x$ be \textbf{bold}.";
let d = parse(src).unwrap();
assert_eq!(&src[d[0].span().start..d[0].span().end], "Let");
// round-trips: parsing the rendered AST yields the same AST
assert_eq!(parse(&latex::document_to_latex(&doc)).unwrap(), doc);
```

### Math (L2)

Each `$…$` island keeps its **raw** inner source at L1; the math grammar parses it on
demand into a `MathNode` tree with full operator precedence:

```rust
use latex::{parse_math, MathNode};

// fractions, big operators with bounds, roots, scripts, fences — all supported
let m = parse_math(r"\sum_{i=1}^{n} i").unwrap();
assert!(matches!(m, MathNode::BigOp { .. }));

// precedence-aware round-trip: re-parsing the rendered AST yields the same AST
let e = parse_math(r"\left(\frac{a}{b}\right)^2").unwrap();
assert_eq!(parse_math(&e.to_latex()).unwrap(), e);

// parse an island found in a document directly
let doc = parse(r"area is $\pi r^2$").unwrap();
let area = doc.iter().find_map(|n| n.parsed_math()).unwrap().unwrap();
assert!(matches!(area, MathNode::Bin(..)));   // π · r²  (implicit multiplication)
```

### Environments (L3)

The math environment family parses into `MathNode::Matrix { env, col_spec, rows }` — `&`
splits columns, `\\` splits rows. Supported: `matrix`/`pmatrix`/`bmatrix`/`Bmatrix`/`vmatrix`/
`Vmatrix`/`smallmatrix`, `cases`, the alignment environments (`aligned`/`align`/…), and the
general `array`/`subarray` grids. Cells hold arbitrary math, environments nest, and a matrix
is an atom (so `…^2` attaches):

```rust
use latex::{parse_math, MathNode};

let m = parse_math(r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}").unwrap();
if let MathNode::Matrix { env, col_spec, rows } = &m {
    assert_eq!(env, "pmatrix");
    assert_eq!(*col_spec, None);        // pmatrix takes no column-spec argument
    assert_eq!(rows.len(), 2);          // two rows
    assert_eq!(rows[0].len(), 2);       // two columns
}
assert_eq!(parse_math(&m.to_latex()).unwrap(), m);   // round-trips
```

`array` and `subarray` carry a **mandatory column-spec argument** — `\begin{array}{l|cr}` —
captured verbatim on `col_spec` (`Some("l|cr")`) so the node round-trips. Alignment is
presentation, so the neutral `MathExpr` lowering **drops** `col_spec`: an `array` and the
equivalent `pmatrix` lower to the same `MathExpr::Matrix`. The text-mode `tabular` family and
document-mode list environments are a later layer; an unknown `\begin{…}` (or an `array`
missing its column-spec) is rejected with a spanned error, never mis-parsed.

### Macros (L4a)

`parse` stays purely structural; `expand` is an **opt-in pass** over the document tree that
registers `\newcommand`/`\renewcommand`/`\providecommand` (positional `#1`..`#9`) and replaces
later uses by their substituted, recursively-expanded bodies. Definitions vanish from the
output, just like in LaTeX:

```rust
use latex::{parse, expand, document_to_latex};

let doc = parse(r"\newcommand{\sq}[1]{#1^2} area \sq{r}").unwrap();
let expanded = expand(doc).unwrap();
assert_eq!(document_to_latex(&expanded), "area r^2");
```

Expansion is **bounded** — a recursive macro (`\newcommand{\a}{\a}\a`) or an expansion bomb
errors via a depth + work-budget guard rather than hanging or overflowing. Deferred to later
sub-rungs: optional arguments with a default (`[n][default]`), TeX-style `\def`, and a
built-in starter set; `#n` inside a math island is not substituted in L4a.

### Verbatim (L5a/L5b)

`\verb<delim>…<delim>` (and the `\verb*` visible-space variant) read their body **raw** — the
tokenizer suspends catcodes inside, so `{ } $ # \` are literal — producing a `NodeKind::Verb`
that round-trips:

```rust
use latex::{parse, Node};

let doc = parse(r"call \verb|x{y}$z| now").unwrap();
assert!(matches!(doc[1].kind, NodeKind::Verb { delim: '|', .. }));   // body "x{y}$z" kept verbatim
```

The **`verbatim` environment** (and `verbatim*`) reads its whole body raw — newlines included —
up to the matching `\end{verbatim}`, producing a `NodeKind::VerbatimEnv` that also round-trips:

```rust
use latex::{parse, Node};

let doc = parse("\\begin{verbatim}let x = {1};\n$y$\\end{verbatim}").unwrap();
assert!(matches!(doc[0].kind, NodeKind::VerbatimEnv { .. }));   // body kept literal, $/{} not special
```

Only `verbatim`/`verbatim*` divert to raw scanning; every other `\begin{…}` is parsed
structurally. An unterminated `\verb` (or a `*`/space delimiter, or a body past the line end)
and an unterminated `verbatim` environment are spanned errors — never a mis-parse.

### Text accents (L5c)

`recognize_accents` is an opt-in pass (like `expand`) that folds an accent control sequence
and the character it accents into a `NodeKind::Accent` — both spellings, `\'e` and `\'{e}`,
recognize to the same node and round-trip:

```rust
use latex::{parse, recognize_accents, Node};

let doc = recognize_accents(parse(r"caf\'e").unwrap());
assert!(matches!(doc[1].kind, NodeKind::Accent { .. }));   // é over `e`; "caf" stays text
```

Recognized: `\'  \`  \^  \"  \~  \=  \.` and `\u \v \H \c \d \b \r \t`. A dangling accent (no
accent-able char after it) is left as a plain command — never dropped.

### Document structure (L5d)

`recognize_structure` is the second opt-in classification pass (like `recognize_accents`). It
turns the *generic* commands L1 produces into **semantic** structure nodes — headings,
cross-references, preamble directives, and argument-form font commands — while leaving L1's
round-trip intact:

```rust
use latex::{parse, recognize_structure, Node, SectionLevel};

let doc = recognize_structure(parse(r"\section*{Intro} see \ref{fig:1}").unwrap());
assert!(matches!(doc[0].kind, NodeKind::Section { level: SectionLevel::Section, starred: true, .. }));
assert!(doc.iter().any(|n| matches!(n.kind, NodeKind::CrossRef { .. })));   // \ref{fig:1}
```

Recognized:

- **`NodeKind::Section`** — `\part`/`\chapter`/`\section`/`\subsection`/`\subsubsection`/
  `\paragraph`/`\subparagraph`, the starred `\section*{…}` form (the `*` sibling is folded),
  and the optional short TOC title `\section[Short]{Title}`;
- **`NodeKind::CrossRef`** — `\label`/`\ref`/`\eqref`/`\pageref`/`\autoref`/`\nameref`/`\cite`/
  `\citep`/`\citet` (the `\cite[note]{key}` optional is kept);
- **`NodeKind::Preamble`** — `\documentclass`/`\usepackage`/`\RequirePackage` with `[options]`;
- **`NodeKind::Styled`** — argument-form font commands (`\textbf`, `\textit`, `\texttt`, `\emph`,
  `\underline`, …).

A command that does not match its expected shape (a sectioning command with no title, a
cross-ref with no key) is left as a plain command — never dropped or mis-folded. Font
*declarations* (`\bfseries`, `\itshape`, `\large`, …) also stay plain commands: their effect is
positional (until end of group), so wrapping them in an argument node would misrepresent them.
The pass is idempotent and round-trips: `recognize_structure(parse(&n.to_latex())) == [n]`.
(The two passes — `recognize_accents` and `recognize_structure` — are independent and compose.)

### Document-mode tables & lists (D1)

`recognize_tables` is the third opt-in classification pass (like the two above). It folds the
*generic* environments L1 produces for document-mode `tabular`/`tabular*` grids and the
`itemize`/`enumerate`/`description` list environments into structured nodes — splitting a table
body on the `&` alignment tab and the `\\` row break, and a list body on `\item`:

```rust
use latex::{parse, recognize_tables, Node, ListKind};

let table = recognize_tables(parse(r"\begin{tabular}{lc}a & b \\ c & d\end{tabular}").unwrap());
assert!(matches!(table[0].kind, NodeKind::Tabular { .. }));   // 2×2 grid, col_spec = Some("lc")

let list = recognize_tables(parse(r"\begin{itemize}\item one\item two\end{itemize}").unwrap());
assert!(matches!(list[0].kind, NodeKind::List { kind: ListKind::Itemize, .. }));
```

Recognized:

- **`NodeKind::Tabular { col_spec, rows }`** — `tabular`/`tabular*`; `rows[r][c]` is the node
  sequence of cell `c` in row `r`; `col_spec` is the column spec captured verbatim (`None` if
  absent). A `tabular*` `{width}` argument is dropped, keeping the trailing `{colspec}`.
- **`NodeKind::List { kind, items }`** — `itemize`/`enumerate`/`description`; each `ListItem` carries
  its `\item[term]` optional `label` and the `body` up to the next `\item`.

The pass is **total and infallible**: ragged rows (differing cell counts) are preserved exactly,
and a list with stray content before its first `\item` is left as a generic `NodeKind::Environment`
— never an error here (truly malformed input — unbalanced braces, `\begin`/`\end` mismatch — is
already rejected by the L1 parser with a spanned error, upstream of this pass). It is idempotent
and round-trips: `recognize_tables(parse(&n.to_latex())) == [n]`. All three recognition passes
are independent and compose.

### Pluggable frontend (L6)

The capstone: `LatexMath` implements the [`math-frontend`](../math-frontend) `MathFrontend`
trait, so LaTeX math plugs into the shared, notation-agnostic registry. `parse` runs the math
grammar and **lowers** the LaTeX-shaped `MathNode` into the neutral `MathExpr` — two source
strings that mean the same math produce the same tree, so a consumer lowers *one* AST and gets
every notation for free:

```rust
use latex::registry;                       // a FrontendRegistry with LaTeX installed
use math_frontend::{MathExpr, BinOp};

let reg = registry();
assert_eq!(reg.names(), ["latex"]);

// \times, \cdot, and juxtaposition all normalize to the same neutral Mul:
let a = reg.parse("latex", r"a \times b").unwrap();
assert_eq!(a, reg.parse("latex", "ab").unwrap());
assert!(matches!(a, MathExpr::Bin(BinOp::Mul, _, _)));
```

Lowering drops *presentation* and keeps *meaning*: fence style → `Group`, matrix delimiter →
`Matrix`, `a^n` → `Pow`, `a_i` → `Subscript`, accents → `Call`; numbers stay **exact**
(`MathExpr::Number`, never `f64`). `\pm`/`\mp` lower to `BinOp::PlusMinus`/`MinusPlus` (the ± / ∓
pair operators) and `\binom{n}{k}` to `MathExpr::Binom` — every LaTeX math construct the grammar
parses now has a faithful neutral counterpart (the two former gaps were closed by extending the
`math-frontend` neutral AST, not by faking them here). The adapter sits behind the default-on
**`frontend`** feature; build with `--no-default-features` for the zero-dependency L0–L5 parser
alone.

The low-level `tokenize` is also public. Tokens and errors carry half-open byte `Span`s;
all of `parse`, `parse_math`, and `tokenize` return spanned errors rather than panicking,
and recursion is depth-guarded so adversarial nesting errors instead of overflowing.

### Deep-tree drop safety

`MathNode` is a recursive `Box`-owning tree, so a naive (compiler-derived) destructor would
recurse once per level. The parser's `MAX_DEPTH` bounds *nesting*, but left-associative
chains — `1+1+1+…`, juxtaposition `aaa…` — are built in loops with no per-term depth charge,
so they produce O(n)-deep trees that `parse_math` happily returns (it builds iteratively).
Dropping such a tree would overflow the stack: an **uncatchable abort**. To prevent it,
`MathNode` implements `Drop` explicitly, dismantling the tree with a **heap worklist** (each
boxed child is moved onto a `Vec`, replaced in place by a cheap leaf, then popped) so the
generated destructor recurses at most one trivial level — O(1) stack depth at any size. The
neutral `math_frontend::MathExpr` does the same. (Consequence: because `MathNode: Drop`, you
cannot move fields out of an owned `MathNode` in a by-value `match` — borrow with `match &node`
and lift children via `mem::replace`/`Option::take`.)

## Tests

```
cargo test -p latex
cargo clippy -p latex -- -D warnings
```
