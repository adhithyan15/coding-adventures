# Changelog — latex

All notable changes to the full-fidelity LaTeX parser crate.

## [0.28.0] — 2026-07-03

### Added — the sectioning fold (LTXDOC01 D3)

D3 folds D2's **flat** body block stream into the **nested sectioning forest**. Where D2 emitted
every heading as a zero-body `Block::Section`, D3's `fold_sections` pass makes each heading OWN the
run of blocks that follow it, up to (but not including) the next heading of the same-or-higher level;
deeper headings nest inside. This runs inside `build_document` for the top-level body **and every
nested block-list** (environment/quote/figure bodies, list items, table cells), because they all
route through the one `lower_blocks` function — so nesting works everywhere.

- **Level ranking.** A new `fn rank(level: SectionLevel) -> u8` gives the hierarchy order
  `Part=0 < Chapter=1 < Section=2 < Subsection=3 < Subsubsection=4 < Paragraph=5 < Subparagraph=6`
  (mirroring `SectionLevel`'s declaration order). A section owns following blocks until it meets a
  heading whose rank is **≤** its own (that heading starts a sibling/ancestor section); strictly
  greater rank nests inside.
- **`Block::Section` gains a `label: Option<String>` field** (per spec §3). `fold_sections` **hoists
  a `\label{key}`** that immediately follows a heading onto this field: the `\label` lowers (via
  `recognize_structure`) to a leading `label` `Inline::CrossRef` on the section's first owned
  paragraph, which the fold peels off (dropping a now-empty paragraph, preserving any following
  text). Only the unambiguous immediately-following case is hoisted; any other `\label` position is
  **left in place in `body`** (never dropped), keeping the fold total. A hoisted label is re-emitted
  by `to_latex` right after the heading so re-parsing + re-folding is a fixed point.
- **Span union.** A folded section's `span` is the union of its heading's (coarse) region span and
  its owned children's spans, so it still satisfies child ⊆ parent ⊆ `Document` (extended
  span-integrity test covers the nested case).
- **Totality preserved.** `fold_sections` is total and panic-free — no `unwrap`/`expect`, no
  unchecked indexing on the block list; recursion folds a strict sub-slice each call, bounded by the
  parser's `MAX_DEPTH`-capped tree. No `unsafe`.
- **Property test.** A `flatten(&[Block])` linearizes the folded forest (heading with emptied body,
  then depth-first its owned blocks); `flatten(fold(flat)) == flat` reproduces D2's pre-fold order.
  Covered case: `\section{A} p1 \subsection{B} p2 \section{C} p3` → A owns `{p1, B{p2}}`, C owns
  `{p3}`; A and C are top-level siblings, B nests in A.

`Cargo.toml` bumped 0.27.0 → 0.28.0. No public API break beyond the added `Section.label` field;
all D2 constructors/matchers/tests updated accordingly.

## [0.27.0] — 2026-07-03

### Added — the hierarchical `Document` layer, preamble/body split (LTXDOC01 D2)

A new `src/document.rs` module lifts LTX01's *flat* `Vec<Node>` into a reusable, hierarchical
`Document` model — the layer above the presentation-shaped node stream. This is a **pure, total
fold** over already-parsed nodes; it touches no lexer/parser/grammar.

- **New public types** (each carries a `token::Span`):
  - `Document { preamble, body: Vec<Block>, span }` — the whole document.
  - `Preamble { document_class, packages, raw: Vec<Node>, span }` — classified directives plus the
    untouched remainder.
  - `DocumentClass { class, options, span }`, `Package { name, options, command, span }`.
  - `Block` — `Section { level, numbered, title, short_title, body, span }` (zero-body in D2),
    `Paragraph`, `List`, `Table`, `DisplayMath`, `Environment`, `Raw(Node, …)`.
  - `Inline` — `Text`, `Space`, `Strong`, `Emph`, `Code`, `Styled`, `Math`, `CrossRef`, `Accent`,
    `Raw(Node, …)`.
  - `DocListItem { term, body, span }` — the D2 analogue of `ListItem`.
- **New public functions:**
  - `build_document(nodes: Vec<Node>, src: &str) -> Document` — the fold in isolation. **Total**:
    never errors, never panics. Splits the node stream at the `document` environment (whole stream
    is preamble if absent), classifies `\documentclass`/`\usepackage`/`\RequirePackage` (matched on
    the `Node::Preamble` variant `recognize_structure` produces), and lowers the body's flat nodes
    into a **flat** `Vec<Block>` (no sectioning nesting yet — every heading is a zero-body
    `Block::Section`; D3 fills the bodies).
  - `parse_document(src: &str) -> Result<Document, ParseError>` =
    `Ok(build_document(recognize_tables(recognize_accents(recognize_structure(parse(src)?))), src))`.
    `recognize_structure` runs first so the fold matches on semantic variants directly; the only
    fallible stage is `parse`.
  - `Document::to_latex()` (+ block/inline renderers) round-trips: `parse_document(&d.to_latex())`
    equals `d` **modulo spans**, compared via a span-stripped `Document::strip_spans()` projection.
- **Span policy (D2 — coarse but honestly nested).** `Document.span = 0..src.len()`; `Preamble.span
  = 0..find("\\begin{document}")` (or `src.len()`); the body region span = end of
  `\begin{document}`..start of `\end{document}` (found by substring search over the source). Every
  block/inline span **defaults to its enclosing region span**, so every child span ⊆ its parent ⊆
  the `Document` span (asserted by a span-integrity test). **Precise per-node byte coverage is
  deferred to D6** once the parser threads token spans through `Node` — a repo-std-#9 divergence
  from the spec's per-node-span ideal; the type carries the field now so later rungs tighten values
  without an API break.
- Re-exported from `lib.rs`: `build_document`, `parse_document`, `Document`, `Preamble`,
  `DocumentClass`, `Package`, `Block`, `Inline`, `DocListItem`. Pipeline doc-comment entry #10 added.
- Tests: preamble/body split on a real document; no-`document`-env fragment (all preamble, empty
  body); `\documentclass`+`\usepackage` classification; a body exercising heading + paragraph +
  itemize + tabular + inline `$…$` + display `\[…\]` + `\ref`; span-integrity (child ⊆ parent ⊆
  document); round-trip modulo spans; totality on junk input; environment recursion.

*No `unsafe`; `cargo clippy -p latex -- -D warnings` clean. `Metadata` (spec §3) is intentionally
deferred to D4 — D2 ships only the skeleton + preamble/body split it specifies.*

## [0.26.0] — 2026-07-03

### Added — document-mode tables & lists via a new `recognize_tables` pass (LTXDOC01 D1)

Closes the L3b gap LTX01 deferred: document-mode `tabular`/`tabular*` grids and the
`itemize`/`enumerate`/`description` list environments now fold into structured AST nodes. This is
a **recognition pass over already-parsed generic environments** — not a parser/lexer change: L1
already parses `\begin{tabular}{lcr}a & b \\ c & d\end{tabular}` as a generic `Node::Environment`
(with `&` → `Text("&")`, `\\` → `Command("\\")`, `\item[t] x` → `Command("item", opt:[t])` + siblings).

- **New `Node` variants** (both span-less, matching the other structural variants):
  - `Node::Tabular { col_spec: Option<String>, rows: Vec<Vec<Vec<Node>>> }` — `rows[r][c]` is cell
    `c` of row `r`; `col_spec` is the column spec captured verbatim (`"lcr"`, `"l|c|r"`), `None` if
    absent. For `tabular*` the `{width}` argument is dropped and the trailing `{colspec}` kept.
  - `Node::List { kind: ListKind, items: Vec<ListItem> }` with new `pub enum ListKind { Itemize,
    Enumerate, Description }` and `pub struct ListItem { label: Option<Vec<Node>>, body: Vec<Node> }`
    (`label` = the `\item[term]` optional term).
- **New `recognize_tables(nodes) -> Vec<Node>` pass** (`src/tables.rs`), mirroring
  `recognize_structure`: a total, infallible fold that recurses into every child node-list and
  splits `tabular` bodies on `&`/`\\` and list bodies on `\item`. Re-exported from `lib.rs` along
  with `ListKind`/`ListItem`.
- **`to_latex` round-trip**: `recognize_tables(parse(&node.to_latex())) == [node]` for tables and
  lists (asserted over a corpus). A recognized `tabular*` round-trips as a plain `tabular` (its
  width was not a column spec — dropping it is faithful to the grid).
- **Totality (spec reconciliation)**: the pass never errors and never panics — ragged rows
  (differing cell counts) are preserved exactly, and a list with stray content before its first
  `\item` is left as a generic `Node::Environment`. The spec's "spanned errors on malformed grids"
  guarantee lives in the **L1 parser** (unbalanced braces / `\begin`/`\end` mismatch, `MAX_DEPTH`
  depth guard), upstream of this total recognition pass; recursion here is bounded by the L1 tree
  depth (no new unbounded recursion, no raw brace counting — `col_spec` comes from parsed argument
  nodes). Per-node byte spans remain the Document layer's job (D2+).
- No `unsafe`; `cargo clippy -p latex -- -D warnings` clean; downstream `adj-lang`/`adj-lang-cli`
  unaffected (the new variants don't reach the math-frontend path).

## [0.25.0] — 2026-07-01

### Changed — comma/semicolon fence lists now keep their delimiters (`Fenced`-of-`Sequence`)

The fence-delimiters arc's remaining slice: a **comma- or semicolon-separated** fence body now
carries its delimiters too. Previously a single-body fence lowered to `MathExpr::Fenced { open,
body, close }` (0.24.0) but a *list* fence (`(a, b)`, `[a, b]`, `\left(a, b\right)`, semicolon
rows) lowered to a bare `MathExpr::Sequence`, **dropping** the surrounding brackets — so `(a, b)`
and `[a, b]` were indistinguishable. Now the frontend adapter wraps the list in a `Fenced` carrying
the open/close strings: `(a, b)` → `Fenced { open: "(", body: Sequence([a, b]), close: ")" }`,
distinct from `[a, b]` → `Fenced { open: "[", … }`. **Both** the delimiters and the list structure
are preserved.

- The change is a one-line relaxation in the neutral-lowering worklist (`frontend.rs`): the
  `MathNode::Fenced` arm now always pushes a `Build::Fenced` wrapper, instead of skipping it when
  the body is a `Sequence`. The inner `Sequence` still builds via its own arm; the `Fenced` build
  wraps it. No new node, no capability change (`fenced_delimiters` + `sequences` already declared).
- Matrices (`pmatrix`/`bmatrix`/…) are unaffected — their delimiter style stays presentation on
  `Matrix` as before; only `\left…\right` / `( ) [ ]` *list* fences are wrapped.
- Tests `comma_fence_lowers_to_sequence` / `semicolon_fence_lowers_to_nested_sequence` become
  `…_lowers_to_fenced_sequence` / `…_lowers_to_fenced_nested_sequence`, asserting the delimiters are
  now carried around the (unchanged) inner sequence. All downstream consumers (`math-frontend`,
  `mathml`, `asciimath`, `unicode-math`, `adj-lang`, `adj-lang-cli`) build and pass unchanged — the
  `adj-lang` adapter already unwraps `Fenced` to its body, so a `Fenced`-of-`Sequence` lowers
  exactly as the bare `Sequence` did (both non-arithmetic).

## [0.24.0] — 2026-06-30

### Changed — fence delimiters preserved as data (frontend adapter)

- The `MathFrontend` adapter now lowers a single-body fence to the new neutral
  **`MathExpr::Fenced { open, body, close }`** (math-frontend 0.7.0) instead of the delimiter-less
  `MathExpr::Group`, carrying the surface delimiters as data: `(x+1)` → `Fenced("(", …, ")")`,
  `[x]` → `Fenced("[", …, "]")`, `\left|x\right|` → `Fenced("|", …, "|")`. An absolute value / norm
  is no longer indistinguishable from a parenthesised group. The latex frontend declares the new
  `fenced_delimiters` capability (via `Capabilities::all()`).
- A **comma-list** fence still lowers to `MathExpr::Sequence` (delimiters dropped, matching MathML
  `<mfenced>` / AsciiMath) — carrying delimiters on sequences is a later slice of this arc.
- The internal `MathNode` and `to_latex` round-trip are unchanged; this only affects the neutral
  lowering. Golden tests updated to assert the preserved delimiters.

## [0.23.0] — 2026-06-30

### Added — down-barb & under harpoon accents (completing the harpoon accent family)

- **Down-barb over-harpoons** (`harpoon` package) — `\overrightharpoondown`, `\overleftharpoondown`,
  the down-barb siblings of the existing up-barb `\overrightharpoonup`/`\overleftharpoonup`. Added
  to `over_arrow_base`, lowering onto `Overset` over the standard amsmath ⇁/↽ relation symbols.
- **Under-harpoons** (`harpoon` package) — `\underrightharpoonup`, `\underleftharpoonup`,
  `\underrightharpoondown`, `\underleftharpoondown`, the under-body mirror of the over-harpoons.
  Added to `under_arrow_base`, lowering onto `Underset` over the plain harpoon symbol.

Both extend the two existing base-name tables (no new machinery, no new AST node, no frontend
change); `to_latex` round-trips through `\overset`/`\underset` over the plain harpoon symbol (an
unknown control word re-parses to a `Sym`). Missing `{body}` is a clean spanned error. With the
prior `\overrightharpoonup`/`\overleftharpoonup` and the `\underrightarrow` family, the crate now
covers all four over/under × up/down harpoon accents.

## [0.22.0] — 2026-06-30

### Added — stretchy UNDER-arrow accents (the mirror of the over-arrow family)

- **Under-arrow accents** (amsmath) — `\underrightarrow`, `\underleftarrow`, `\underleftrightarrow`.
  These are the exact mirror of the existing `\overrightarrow`-family: a stretchy arrow drawn
  **under** the argument rather than over it. Each lowers onto the existing `Underset` node over the
  plain arrow symbol — the same annotation-under-a-body shape as `\underbrace` and an xarrow's
  `[below]` label — so there is no new AST node and no frontend change:
  `\underrightarrow{AB}` → `Underset { under: →, base: AB }` ≡ `\underset{\rightarrow}{AB}`, and
  `to_latex` round-trips through `\underset`. Implemented via a new `under_arrow_base` table mirroring
  `over_arrow_base`; missing `{body}` is a clean spanned error, never a panic.

## [0.21.0] — 2026-06-30

### Added — more stretchy accents & extensible arrows (all onto the existing Overset/Underset node)

- **Extensible harpoons** (mathtools) — `\xrightharpoonup`, `\xrightharpoondown`, `\xleftharpoonup`,
  `\xleftharpoondown`, `\xrightleftharpoons`, `\xleftrightharpoons`, plus `\xLeftrightarrow`. These
  are arrow-like relations with a stretchable label, so they lower **exactly like the xarrows**:
  `\xrightleftharpoons{k}` → `Overset { over: k, base: ⇌ }`, and an optional `[below]` group stacks
  under (`Underset`). Added to the `xarrow_base` table — no new machinery.
- **Over/under paren & group accents** (amsmath/mathtools) — `\overparen`/`\underparen`,
  `\overgroup`/`\undergroup`, alongside the existing `\overbrace`/`\underbrace`. Each draws a
  stretchy parenthesis / group bracket over (under) the body and lowers onto `Overset`/`Underset`
  over the Unicode top/bottom glyph (⏜ U+23DC / ⏝ U+23DD / ⏠ U+23E0 / ⏡ U+23E1), with an optional
  trailing `^{label}` (over) / `_{label}` (under) stacked on the glyph — the same mechanism as
  `\overbrace`. The two hardcoded brace branches were generalised into `over_accent_glyph` /
  `under_accent_glyph` tables (behaviour-identical for the existing braces).
- **No new AST node, no frontend change** — every command reuses the neutral `Overset`/`Underset`
  nodes, so the `math-frontend` lowering and `to_latex` round-trip need no change (the surface
  normalises to `\overset`/`\underset`, which re-parse to the identical tree). 6 new tests (harpoon
  base mapping, harpoon `[below]` underset, the four paren/group accents, label stacking + round-trip
  corpus, missing-body error). `adj-lang` (the only consumer) builds. `latex` 0.20.0 → 0.21.0.

## [0.20.0] — 2026-06-30

### Added — semicolon-separated fences lower to rows (matches MathML `<mfenced>` PR-5)

- A fence whose body contains a top-level semicolon — `(a, b; c, d)`, `\left(a, b; c, d\right)` — is
  now read as **rows**: semicolons are the row separator and commas the within-row (column)
  separator, the classic fenced-matrix notation. `(a, b; c, d)` parses to
  `Sequence([Sequence([a, b]), Sequence([c, d])])` and lowers to the same nested
  `MathExpr::Sequence`. This mirrors the `mathml` crate's PR-5 exactly, extending the write-once
  Sequence node to a second structural frontend's row shape.
- Degenerate shapes are predictable, identical to MathML: a **semicolon-only** fence `(a; b; c)` — a
  column vector with no second column in any row — collapses to the same flat `Sequence([a, b, c])`
  as a comma list; a **ragged** fence `(a; b, c)` stays faithful as
  `Sequence([a, Sequence([b, c])])`. Each cell is still folded to a full relation, so `(x + 1, 2; y)`
  nests the folded `x + 1`. A comma-only fence is unchanged (flat `Sequence`), and a comma/semicolon-
  free fence is still a plain grouped expression.
- **Round-trips** through `to_latex`: a rows `Sequence` (detectable because a comma-list item is
  always a relation, never a *bare* `Sequence`, so a bare-`Sequence` child can only be a row)
  semicolon-joins its rows and comma-joins each row's columns, so `parse_math(n.to_latex()) == n`
  holds for `(a, b; c, d)` and friends. `read_fence_body` records the separator after each relation
  in a bounded loop (never recursion), so a wide grid cannot overflow the stack; a
  leading/trailing/doubled `;` or `,` is a clean spanned error, never a dropped row.
- No new AST node, no shared-crate change (reuses `MathExpr::Sequence`; the neutral node already
  nests, and the frontend lowering already recurses). 10 new tests (rows, semicolon-only flat,
  ragged, folded cells, trailing-`;` error, 4 round-trip corpus entries, frontend nested-lowering).
  `latex` 0.19.0 → 0.20.0.

## [0.19.0] — 2026-06-30

### Added — comma-separated fences lower to the neutral `Sequence` node (third frontend)

- A **comma-separated fence** — `(a, b, c)`, `[x, y, z]`, `\left(p, q\right)` — is now recognised
  as an ordered **list** rather than a single grouped expression. The parser reads the fence body
  with a new `read_fence_body` helper: it parses the first relation, and if the next token is a
  comma it keeps collecting comma-separated items into a new `MathNode::Sequence(items)` AST node.
  A **comma-free** fence — `(a + b)` — is unchanged: it stays a plain grouped expression. Only a
  top-level comma inside the fence triggers a `Sequence`; commas nested inside an inner group are
  the inner group's business.
- **Round-trips** through `to_latex`: a `Sequence` renders its items comma-joined, wrapped by the
  fence that produced it, so `parse_math(n.to_latex()) == n` still holds. `Sequence` participates in
  the iterative-`Drop` `take_children` trampoline, so a pathologically deep nest (tested to 200 000
  levels) drops without a stack overflow — same guarantee as every other node.
- **Neutral-frontend lowering** (`frontend.rs`): a `Sequence` fence lowers to `MathExpr::Sequence`
  (the delimiters are dropped, exactly as for MathML `<mfenced>` comma rows and AsciiMath `(a,b,c)`).
  This makes `latex` the **third** emitter of the neutral `MathExpr::Sequence` node introduced in
  `math-frontend` 0.6.0 — completing write-once-use-many across LaTeX + AsciiMath + MathML: one
  neutral list node, three surface syntaxes, no per-consumer special-casing. A comma-free fence
  still lowers to `MathExpr::Group` as before.
- 9 new tests (parse-to-`Sequence`, comma-free-stays-plain, full-expression items, `\left…\right`
  and bracket fences, 200 000-deep drop, 4 round-trip corpus entries, plus a `frontend.rs` lowering
  test asserting `(a, b, c)` → `MathExpr::Sequence` and `(a + b)` → `MathExpr::Group`).
- `latex` 0.18.0 → 0.19.0.

## [0.18.0] — 2026-06-30

### Added — stretchy over-arrow accents (`\overrightarrow` & friends)

- The parser now accepts the stretchy over-arrow accents `\overrightarrow`, `\overleftarrow`,
  `\overleftrightarrow`, `\overrightharpoonup`, `\overleftharpoonup` — an arrow drawn **over** a
  (possibly multi-token) body, e.g. `\overrightarrow{AB}`. Distinct from `\vec` (a fixed
  single-glyph accent over one symbol). As with `\overbrace` and the xarrows, there is **no new AST
  node**: an over-arrow is an annotation stacked on the body, so it lowers onto the existing
  `Overset` node over the plain arrow symbol — `\overrightarrow{AB}` → `Overset { over: →, base: AB }`,
  identical to `\overset{\rightarrow}{AB}`.
- Reuses the existing `Overset` `to_latex` and neutral-frontend lowering, so **no frontend change**;
  round-trips through `to_latex` (surface normalises to `\overset`). 5 new tests (parse, per-arrow
  base, in-context, round-trip, missing-body error); the `{body}` is mandatory and its absence is a
  clean spanned error, never a panic.
- `latex` 0.17.0 → 0.18.0.

## [0.17.0] — 2026-06-30

### Added — horizontal braces (`\overbrace` / `\underbrace`)

- The parser now accepts `\overbrace{body}` and `\underbrace{body}` — a horizontal brace drawn over
  or under the body, optionally labelled by a trailing `^{label}` (overbrace) / `_{label}`
  (underbrace) that sits over/under the brace. As with the extensible arrows, there is **no new AST
  node**: a brace is exactly an annotation stacked on the body, so it lowers onto the existing
  `Overset`/`Underset` nodes over the Unicode brace glyph (`⏞` U+23DE / `⏟` U+23DF):
  - `\overbrace{a+b}` → `Overset { over: ⏞, base: a+b }`
  - `\overbrace{x+y}^{n}` → `Overset { over: n, base: Overset { over: ⏞, base: x+y } }` (label over
    brace over body); `\underbrace{a}_{k}` is the under-side twin.
- The optional label is consumed inside the brace parse (peeking for `^`/`_`) so it stacks centered
  on the brace, mirroring how the xarrows consume their `[below]`/`{above}` labels — rather than
  being attached as an ordinary raised/lowered script by the postfix mechanism.
- Reuses the existing `Overset`/`Underset` `to_latex` and neutral-frontend lowering, so **no
  frontend change**: `\overbrace`/`\underbrace` reach the neutral `MathExpr::Overset`/`Underset` for
  free. Round-trips through `to_latex` (the surface normalises to `\overset`/`\underset`, which
  re-parse to the identical tree). 7 new tests (parse, labelled, in-context, round-trip, missing-body
  error); the `{body}` is mandatory and its absence is a clean spanned error, never a panic.
- `latex` 0.16.0 → 0.17.0.

## [0.16.0] — 2026-06-30

### Added — extensible / labelled arrows (`\xrightarrow` & friends)

- The parser now accepts the amsmath **extensible arrows** that carry a stretching label:
  `\xrightarrow`, `\xleftarrow`, `\xleftrightarrow`, `\xRightarrow`, `\xLeftarrow`, `\xmapsto`,
  `\xhookrightarrow`, `\xhookleftarrow`. Each takes a **mandatory `{above}` group** (the label set
  over the arrow) and an **optional `[below]` group** (a second label set under it) — the same
  argument shape as in real LaTeX, e.g. `\xrightarrow[\text{below}]{f}`.
- **No new AST node.** A labelled arrow is *exactly* an annotation stacked on the plain arrow
  symbol, so it lowers onto the existing `Overset`/`Underset` nodes: `\xrightarrow{f}` →
  `Overset { over: f, base: \rightarrow }`, and `\xrightarrow[g]{f}` →
  `Underset { under: g, base: Overset { over: f, base: \rightarrow } }`. Because nothing new is
  introduced, the neutral **`MathFrontend` lowering needs zero change** — `\xrightarrow{f}` lowers
  to the identical `MathExpr::Overset` as the explicit `\overset{f}{\rightarrow}`.
- The surface normalises through `to_latex()` to the equivalent `\overset`/`\underset` form, which
  re-parses to the identical tree (round-trip is a fixed point). A missing mandatory `{above}`
  label, or an unterminated optional `[below]` label, is a **spanned `ParseError`, never a panic**.
- 8 new parser tests (`math.rs`) + 1 neutral-lowering test (`frontend.rs`).

## [0.15.0] — 2026-06-30

### Added — the `array` / `subarray` grids (mandatory column-spec)

- The parser now accepts the general **`\begin{array}{cols} … \end{array}`** grid and its
  big-operator-limit cousin **`\begin{subarray}{c} … \end{subarray}`** — the math environments that
  carry a **mandatory column-alignment argument**. This was the documented gap in the matrix family
  (`pmatrix`/`bmatrix`/`cases`/`aligned`/…, all of which take *no* argument): the comment on
  `is_math_env` explicitly parked `array` for "a later layer" because it "need[s] an extra field on
  the node." That field is now here.
- New optional field **`MathNode::Matrix { col_spec: Option<String>, … }`**: the column spec is
  captured **verbatim** (`"cc"`, `"l|cr"`, `"p{3cm}"`, `*{3}{c}`, `>{…}`/`<{…}`/`@{…}` inserts), so
  the node **round-trips** exactly — `to_latex` re-emits `\begin{array}{<spec>}`. It is `None` for
  every environment that takes no argument (a `pmatrix` is unchanged).
- The neutral **lowering drops `col_spec`** (alignment is presentation, PFE01 §2.2): an `array` and
  the equivalent `pmatrix` lower to the **same** `MathExpr::Matrix`. So consumers gain `array` for
  free — no consumer change, no new neutral node, no capability change.
- `read_col_spec` reads the `{…}` argument with a **flat `depth` counter, not recursion**, so it is
  brace-nesting-aware (captures `p{3cm}` whole) yet an adversarial `{{{{…` cannot overflow the
  stack (bounded by input length, like the rest of the tokenizer output). A missing or unterminated
  col-spec is a clean **spanned error**, never a panic.
- Tests (9 new): `array_captures_column_spec_and_cells`, `…_keeps_rules_and_alignment_letters`,
  `…_handles_braced_groups`, `subarray_takes_a_column_spec_too`, `array_round_trips_through_to_latex`,
  `matrix_family_still_has_no_column_spec`, `array_without_column_spec_is_a_spanned_error`,
  `deep_column_spec_braces_do_not_overflow`, and frontend `array_lowers_dropping_column_spec`
  (array ≡ pmatrix after lowering). 150 unit + doc tests pass; clippy `-D warnings` clean;
  downstream `adj-lang` (the only consumer) 86 green.

## [0.14.0] — 2026-06-30

### Added — `\overset` / `\underset` / `\stackrel` → neutral `Overset` / `Underset`

- The parser recognizes **`\overset{over}{base}`**, **`\stackrel{over}{base}`** (amsmath's name
  for the over-set form), and **`\underset{under}{base}`** — two mandatory args (annotation then
  base, the `\frac`/`\binom` shape) — as new `MathNode::Overset` / `MathNode::Underset`, and the L6
  lower emits the neutral **`MathExpr::Overset` / `MathExpr::Underset`** (math-frontend 0.5.0). A
  centered annotation **over/under** the base, **distinct from `Pow`/`Subscript`** (`b^a` / `b_a`).
  This is the LaTeX-side twin of the asciimath over/under-set emitter.
- `to_latex` round-trips both nodes (`parse_math(node.to_latex()) == node`); the iterative
  `take_children` (deep-Drop guard) handles their two children; both stay atoms in the precedence
  table. The lower runs in the existing work-stack trampoline (no new recursion). `capabilities()`
  already advertises `oversets` via `all()`, so emission is conforming with no capability change.
- Tests: parser `overset_underset_parse_and_round_trip`, lower `overset_underset_lower_to_neutral_nodes`
  (overset/underset/stackrel, distinct-from-Pow). 141 unit + doc tests pass; clippy `-D warnings` clean.

## [0.13.0] — 2026-06-29

### Changed — accents lower to the neutral `MathExpr::Accent` (no longer faked as a function call)

The L6 adapter previously lowered a diacritical accent (`\hat{x}`, `\bar{y}`, `\vec{v}`,
`\tilde{a}`, `\dot{x}`, …) to `MathExpr::Call { func: Func::Other(kind), arg }` — a *function
application*, which is the wrong meaning: `\hat{x}` is a mark *over* `x`, not `hat(x)`. Now that
`math-frontend` 0.4.0 provides a neutral **`MathExpr::Accent { accent, body }`** node, the adapter
emits it faithfully.

- `lower()`'s `MathNode::Accent { kind, body }` arm now pushes a new `Build::Accent(kind)` work step
  (instead of `Build::Call(Func::Other(kind))`); its assembler pops the lowered body and produces
  `MathExpr::Accent { accent: kind, body }`. Still inside the iterative trampoline — O(1) call-frame
  depth, stack-safe on deep accent nests.
- `capabilities()` stays `Capabilities::all()` — now **honestly** includes `accents`, which the
  shared conformance harness (math-frontend 0.4.0) enforces (emitting an Accent requires declaring it).
- Test `accent_is_a_named_unary` becomes `accent_lowers_to_neutral_accent_node` (asserts `\hat{x}` →
  `Accent{accent:"hat", x}` and `\vec{v}` → `Accent{accent:"vec", v}`). Tests pass; downstream
  `adj-lang` green (its adapter's catch-all routes an Accent to "unsupported ADJ arithmetic", since a
  diacritic is not computable — behavior unchanged). clippy `-D warnings` clean.

## [0.12.0] — 2026-06-29

### Fixed — deep `MathNode` trees now drop without overflowing the stack

`MathNode` is a recursive `Box`-owning enum, so the compiler-generated destructor recurses
once per level. The parser builds **left-nested** chains via loops with no per-term depth
charge (`parse_add`/`parse_mul`/`parse_relation`), so input like `1+1+1+…` (or a long
juxtaposition `aaa…`) yields an O(n)-deep tree even though `MAX_DEPTH` bounds *nesting*.
Dropping a deep-enough such tree overflowed the stack — an **uncatchable abort**, even
though `parse_math` itself survives (it builds iteratively). This was the pre-existing
latent hazard flagged in the L6 security review.

- **`impl Drop for MathNode`** now dismantles the tree with an explicit **heap worklist**
  instead of the call stack: each node's boxed children are moved onto a `Vec` (replaced
  in place by a cheap `Sym("")` leaf), popped, and repeated, so the generated destructor
  recurses at most one trivial level. O(1) stack depth regardless of tree depth. This
  mirrors `math_frontend::MathExpr`'s `Drop` (added in `math-frontend` 0.3.0).
- **`take_children` helper** does the per-variant child extraction (leaves contribute
  nothing; `Matrix` rows are drained via `mem::take`).
- Because `MathNode` now implements `Drop`, fields can no longer be moved out of an owned
  value in a by-value `match` (E0509). The `frontend.rs` `lower()` trampoline and the
  by-value test matches were rewritten to `match &node`/`match &n` and extract children via
  `mem::replace`/`Option::take`/`mem::take`. No behavior change — purely how ownership is
  threaded.
- New regression tests: `deep_left_nested_tree_drops_without_overflow` (200k-deep `Bin`
  spine), `deep_parsed_chain_drops_without_overflow` (50k-term `+` chain through the real
  parser), `deep_unary_chain_drops_without_overflow` (200k-deep `Unary` spine). 139 tests
  pass; both the default build and `--no-default-features` (zero-dep L0–L5) stay green.

## [0.11.0] — 2026-06-28

### Changed — L6 closes its two honest neutral-AST gaps (± / ∓ and binomials)

The L6 capstone (0.10.0) had to return a spanned `FrontendError` for `\pm`, `\mp`, and
`\binom`, because the `math-frontend` neutral AST could not represent them. `math-frontend`
0.2.0 added `BinOp::PlusMinus`/`MinusPlus` and `MathExpr::Binom`; this release wires the
`latex` adapter to **emit** them, so every LaTeX math construct the L2/L3a grammar parses now
lowers to a faithful neutral node — no faking, no honest-error islands.

- **`\pm` → `BinOp::PlusMinus`, `\mp` → `BinOp::MinusPlus`** (the ± / ∓ pair operators).
  `lower_binop` is now total (infallible) — the two error arms are gone.
- **`\binom{n}{k}` → `MathExpr::Binom(n, k)`** (binomial coefficient, args in source order;
  distinct from `Frac` — no division bar). Lowered via a new `Build::Binom` work-stack step,
  so it stays inside the iterative (stack-safe) trampoline like every other node.
- `capabilities()` stays `Capabilities::all()` — now **honest**, since the adapter genuinely
  emits ± / ∓ and binomials (the shared conformance harness polices this).
- Tests: the former `neutral_gaps_error_honestly` becomes `plusminus_minusplus_and_binom_lower`
  (asserts the three now lower to the right shapes); the conformance corpus keeps `a \pm b`
  and `\binom{n}{k}` (now exercising emission, not error). 136 tests pass; both the default
  build and `--no-default-features` (zero-dep L0–L5) stay green; clippy `-D warnings` clean.
- Crate 0.10.0 → 0.11.0. **LTX01 L6 now has no honest gaps.**

## [0.10.0] — 2026-06-27

### Added — LTX01 L6: `math-frontend` adapter (the ladder's capstone)

- **`LatexMath`** implements `math_frontend::MathFrontend`: `parse(src)` runs the L2/L3a math
  grammar (`parse_math`) and **lowers** the LaTeX-shaped `MathNode` into the notation-agnostic
  `math_frontend::MathExpr`. LaTeX is now the first **pluggable frontend** of the PFE01 framework
  — a consumer (rule engine, CAS, renderer) lowers one neutral tree and gets LaTeX for free.
- **Registration:** `latex::registry()` returns a `FrontendRegistry` with LaTeX installed, and
  `latex::register_latex(&mut reg)` installs it into an existing one. (`math-frontend`'s own
  `with_builtins()` stays empty by design — it cannot depend on this crate without a cycle; the
  wiring lives here.)
- **Neutral lowering** drops presentation, keeps meaning: `\times`/`\cdot`/juxtaposition →
  `Mul`; `\frac`/`\dfrac`/`\tfrac` → `Frac`; every fence style → `Group`; every matrix
  delimiter → `Matrix`; `a^n` → `Pow`, `a_i` → `Subscript`, `a_i^n` → `Pow(Subscript(..),..)`;
  accents (`\hat{x}`) → `Call{Other(kind), arg}`. Numbers stay **exact** (`MathExpr::Number`,
  never `f64`). Declares `Capabilities::all()`; conforms to the shared `check_frontend` harness.
- **Honest gaps:** `\pm`/`\mp` and `\binom` have **no** neutral representation, so they lower to
  a well-formed spanned `FrontendError` rather than being faked. Extending the neutral AST to
  cover them is a future `math-frontend`-crate change, not a hack here.
- **Feature-gated:** the adapter (and the only dependency, `math-frontend`) sit behind the
  default-on **`frontend`** cargo feature. `--no-default-features` builds the zero-dependency
  L0–L5 document/math parser alone (verified: core tests pass under `--no-default-features`).
- Total / panic-free: the lowering walks the tree with an **explicit work stack** (not native
  recursion), so its call-frame usage is O(1) in tree depth. This matters because a LaTeX math
  tree can be arbitrarily deep along a left-associative spine (`a+a+a+…`, juxtaposition `aaa…`,
  a chained relation) that `parse_math`'s nesting `MAX_DEPTH` does **not** bound — a recursive
  lowering would overflow the stack (an uncatchable abort) on such adversarial input. No `unsafe`.
- +16 tests (frac/pow/subscript/root/func/bigop/rel/implicit-mul/normalization/symbol/text/
  group/accent/matrix/exact-number/gap-errors/parse-error-span/registry/conformance, plus a
  deep-chain no-overflow regression at depth 4000). **136 unit + 1 doc test** green with default
  features; core green under `--no-default-features`; clippy `-D warnings` clean both ways. Crate
  0.9.0 → 0.10.0. **This completes the LTX01 ladder (L0–L6).**

## [0.9.0] — 2026-06-27

### Added — LTX01 L5d: document-structure recognition

- **`recognize_structure(Vec<Node>) -> Vec<Node>`** — a new, opt-in recognition pass (a sibling
  of `expand` / `recognize_accents`) that classifies the *generic* `Node::Command`s produced by
  `parse` (L1) into **semantic** structure nodes. `parse` (L1) is unchanged, so its round-trip
  is preserved; run the pass, or don't.
- Four new `Node` variants (+ a `SectionLevel` enum):
  - **`Node::Section { level, starred, short, title }`** — `\part`/`\chapter`/`\section`/
    `\subsection`/`\subsubsection`/`\paragraph`/`\subparagraph`, including the starred form
    `\section*{T}` (the intervening `Text("*")` sibling is folded) and the optional short TOC
    title `\section[Short]{Title}`.
  - **`Node::CrossRef { command, note, target }`** — `\label`/`\ref`/`\eqref`/`\pageref`/
    `\autoref`/`\nameref`/`\cite`/`\citep`/`\citet`, keeping the `\cite[note]{key}` optional.
  - **`Node::Preamble { command, options, name }`** — `\documentclass`/`\usepackage`/
    `\RequirePackage` with their `[options]`.
  - **`Node::Styled { command, content }`** — argument-form text font commands (`\textbf`,
    `\textit`, `\texttt`, `\emph`, `\underline`, …). Font *declarations* (`\bfseries`,
    `\itshape`, …) stay plain `Command`s — their effect is positional, not a wrapped argument.
- **`to_latex`** renders each recognized node back to the exact shape the pass folds, so
  `recognize_structure(parse(&n.to_latex())) == [n]`. The pass recurses into groups, command
  arguments, environment bodies, and the parts of already-recognized nodes, so it is idempotent
  and composes with itself.
- Total / panic-free: a command that does not match its expected shape (a sectioning command
  with no title, a cross-ref with no key, a styled command with the wrong argument count) is
  left as a plain `Command`, never dropped or mis-folded. Recursion is bounded by the L1 tree
  depth (`MAX_DEPTH`). No `unsafe`; no `MAX_DEPTH` change (Node size still dominated by
  `Environment`).
- +14 tests (plain/starred/short sectioning, all seven levels, title-less command left alone,
  cross-refs, citation note, preamble, styled, declaration-stays-command, recursion into
  titles, surrounding text preserved, idempotence, round-trip corpus). **117 unit + 1 doc
  test** green; clippy `-D warnings` clean. Crate 0.8.0 → 0.9.0.

## [0.8.0] — 2026-06-27

### Added — LTX01 L5c: text accents

- **`recognize_accents(Vec<Node>) -> Vec<Node>`** — a new, opt-in recognition pass (a sibling
  of `expand`) that folds an accent control sequence and the character it accents into a new
  `Node::Accent { accent, arg }`. `parse` (L1) is unchanged, so its round-trip is preserved.
- Recognizes both spellings: control-symbol accents `\'  \`  \^  \"  \~  \=  \.` (which take no
  L1 argument, so they pair with the next node) and control-word accents `\u \v \H \c \d \b
  \r \t` (captured as `\c{e}`, or `\c e` where the lexer absorbed the space). The argument is
  a single following character (`\'e` → é over `e`, the rest of the run kept as text) or a
  braced group. Recurses into groups, command arguments, and environment bodies.
- **`Node::Accent::to_latex`** renders the braced form `\'{e}`, so
  `recognize_accents(parse(&n.to_latex())) == [n]` whether the source wrote `\'e` or `\'{e}`.
- Total / panic-free: a dangling accent (nothing accent-able after it) is left as a plain
  command, never dropped or mis-folded. No `unsafe`.
- +9 tests (control-symbol over next char, first-char-only with remainder kept, braced arg,
  control-word braced + bare, accent-in-group, non-accent untouched, dangling, round-trip
  corpus). **103 unit + 1 doc test** green; clippy `-D warnings` clean. Crate 0.7.0 → 0.8.0.

## [0.7.0] — 2026-06-27

### Added — LTX01 L5b: `verbatim` environment

- **`\begin{verbatim}…\end{verbatim}`** (and `verbatim*`) read their whole body **raw** —
  catcodes suspended, newlines included — up to the matching `\end{<env>}`. New
  `TokenKind::VerbatimEnv { env, content }` → `Node::VerbatimEnv { env, content }`, with a
  round-tripping `to_latex`. The lexer peeks after `\begin` (consuming nothing) and only
  diverts to raw scanning for `verbatim`/`verbatim*`; every other `\begin{…}` is still parsed
  structurally. `\end{…}` for a different name inside the body stays literal.
- Total / panic-free / spanned: an unterminated `verbatim` environment (or one closed with the
  wrong name) is a spanned `LexError`; the raw scan advances one char per step and terminates
  at EOF. No `unsafe`.

### Fixed

- Lowered the structural-parser nesting cap `MAX_DEPTH` 512 → 256. The new owned-`String`
  token/AST variants enlarged recursive-descent frames enough that 512-deep pathological input
  could overflow a small (2 MB) test-thread stack; 256 trips the spanned "nesting too deep"
  guard well within it. Real documents never nest this deep.

### Tests

- +9 (lexer: verbatim-env raw body incl. `{}$#`+newline, `verbatim*`, non-verbatim `\begin`
  left to the parser, inner wrong-`\end` stays literal, unterminated/wrong-close errors;
  parser: `VerbatimEnv` node + 2 round-trips). **94 unit + 1 doc test** green; clippy
  `-D warnings` clean; no `unsafe`. Crate 0.6.0 → 0.7.0.

## [0.6.0] — 2026-06-26

### Added — LTX01 L5a: inline `\verb` verbatim

- **Inline verbatim** `\verb<delim>…<delim>` and the `\verb*` visible-space variant. The
  tokenizer now intercepts `\verb` and reads its body **raw** — catcodes are suspended, so
  `{ } $ # \` etc. are literal characters (previously L1 mis-tokenized them). New
  `TokenKind::Verb { star, delim, content }` → `Node::Verb { star, delim, content }`, with a
  round-tripping `to_latex`.
- Total / panic-free / spanned: an unterminated `\verb`, a body that runs past the end of the
  line, `\verb` at end of input, or a `*`/space delimiter each return a spanned `LexError` —
  never a panic or a mis-parse. The body is bounded by the input (single line).
- +7 tests (lexer: raw body with `{}$#\`, `\verb*`, the error family; parser: `Verb` node,
  surrounding text undisturbed; +2 round-trip-corpus entries). **87 unit + 1 doc test** green;
  clippy `-D warnings` clean; no `unsafe`. Crate 0.5.0 → 0.6.0.
- Scope: this is L5a. The `verbatim` environment, text accents (`\'e`…), sectioning/font
  recognition, and cross-refs are later L5 sub-rungs (spec §5).

## [0.5.0] — 2026-06-26

### Added — LTX01 L4a: macro expansion

- **`expand(nodes: Vec<Node>) -> Result<Vec<Node>, ParseError>`** — a new, opt-in pass over
  the structural document tree (`parse` stays purely structural, so its round-trip is
  preserved). It registers user macros and replaces their uses by substituted, recursively
  expanded bodies; definitions vanish from the output (as in LaTeX).
- **Definitions**: `\newcommand`/`\renewcommand`/`\providecommand` with positional arity
  `[n]` and bodies referencing `#1`..`#9`. Handles L1's argument-capture quirk (it stops the
  greedy `{…}` run at the `[n]` arity bracket) by re-scanning the definition's sibling nodes.
- **Substitution** walks the tree (groups, command arguments, environment bodies) so `#n`
  inside `\bar{#1}` works; `##` is a literal `#`; arguments are expanded call-by-value.
- **Bounded & safe**: total, panic-free, spanned errors. Two guards stop runaway expansion —
  a recursion-depth cap (`MAX_EXPANSION_DEPTH`) and a work-budget cap (`MAX_EXPANSION_STEPS`)
  — so a self-recursive macro or an expansion bomb errors instead of hanging/overflowing.
  Bad calls (too few args, parameter out of range, malformed definition, or an unsupported
  `[n][default]` optional-with-default) are spanned errors.
- **Honest scope (L4a)**: positional args only. Deferred: optional arguments with a default,
  TeX-style `\def`, a built-in starter set, and `#n` substitution inside math islands.
- +16 macro tests (zero/one/two-arg, reordering, macro-calls-macro, param-in-group,
  redefinition, unknown-command pass-through, extra-group retention, `##`, recursion &
  too-few-args & out-of-range & default-arg & malformed-definition errors). **80 unit + 1 doc
  test** green; clippy `-D warnings` clean; no `unsafe`. Crate 0.4.0 → 0.5.0.

## [0.4.0] — 2026-06-26

### Added — LTX01 L3: math environments

- **`MathNode::Matrix { env, rows: Vec<Vec<MathNode>> }`** — math environments with
  row/column structure. `parse_math` now handles `\begin{env} … \end{env}` inside a math
  island: `&` separates columns, `\\` separates rows, and each cell is a full math
  expression. Supported environments (case-sensitive — `bmatrix` ≠ `Bmatrix`): `matrix`,
  `pmatrix`, `bmatrix`, `Bmatrix`, `vmatrix`, `Vmatrix`, `smallmatrix`, `cases`, `dcases`,
  `aligned`, `gathered`, `align`, `align*`, `split`.
- Environments **nest** (a cell may itself be an environment — depth-guarded via the
  enclosing atom), and a `Matrix` is an **atom**, so postfix scripts attach
  (`\begin{pmatrix}…\end{pmatrix}^2`).
- **`MathNode::to_latex`** renders the grid back and **round-trips**
  (`parse_math(&m.to_latex()) == m`); a trailing `\\` before `\end` is tolerated and does
  not create an empty final row.
- Total / panic-free / spanned: `\begin`/`\end` name mismatch, unterminated environment,
  unknown environment, a missing `{` after `\begin`, and a stray `\end` each return a
  spanned `ParseError`. Empty cells (`a & & b`) are a documented limitation (spanned error,
  never a silent empty node), as are `array`/`tabular` column-specs and document-mode list
  environments — those arrive in a later layer.
- +9 environment tests + 5 round-trip-corpus entries; **64 unit + 1 doc test** green;
  clippy `-D warnings` clean; no `unsafe`.

## [0.3.0] — 2026-06-26

### Added — LTX01 L2: math grammar

- **`parse_math(&str) -> Result<MathNode, ParseError>`** — a precedence-climbing parser
  over a math island's raw inner source (the string L1 keeps in `Node::Math`). Re-uses the
  L0 `tokenize` and filters space/par/comment tokens, then climbs:
  relations (`= ≠ < ≤ > ≥ \approx \equiv`) < add/sub (`+ - \pm \mp`) < mul/div
  (`\times \cdot \div /` **and implicit multiplication** via adjacency — `2x`, `\pi r`,
  `(a)(b)`) < unary `± ∓` < scripts (`^`/`_`, right-assoc) < atoms.
- **`MathNode` AST** (`math.rs`): `Num`, `Sym`, `Bin`, `Unary`, `Frac`/`\dfrac`/`\tfrac`,
  `Binom`, `Root { degree, radicand }` (`\sqrt[n]{}`), `Script { base, sub, sup }`,
  `Call { func, arg }` (named functions `\sin \log …`), `BigOp { op, lower, upper, body }`
  (`\sum \prod \int \lim` with bound scripts), `Accent` (`\hat \bar \vec …`),
  `Fenced { left, body, right }` (`\left( … \right)`, `\langle`, `|`, …), `Text`, and
  `Rel`. `{…}` groups are **transparent** (grouping only — they do not appear as nodes).
- **`MathNode::to_latex`** — precedence-aware round-trip: `parse_math(&m.to_latex()) == m`.
  Children below the parent's precedence are wrapped in invisible `{…}` so the re-parse
  re-associates identically.
- **`Node::parsed_math`** — parses a `Node::Math` island on demand; the L1 structural tree
  is unchanged (its round-trip stays intact).
- Total / panic-free / spanned; recursion is **depth-guarded** (`MAX_DEPTH`) so adversarial
  nesting (e.g. thousands of `(`) returns a spanned error instead of overflowing the stack.
- +15 math tests incl. the worked corpus (`\frac{12 \times 15}{3}`, `2^{10}`,
  `\sqrt[3]{27}`, `\sum_{i=1}^{n} i`, `\left(\frac{a}{b}\right)^2`), a round-trip corpus,
  relations/functions, and the deep-nesting bound; clippy `-D warnings` clean, no `unsafe`.

## [0.2.0] — 2026-06-26

### Added — LTX01 L1: structural document parser

- **`parse(&str) -> Result<Vec<Node>, ParseError>`** — a recursive-descent parser that
  turns the L0 token stream into a document tree:
  - ordinary characters coalesced into `Text`; `Space`/`Par`;
  - `{ … }` → `Group`;
  - `\cmd[opt]{arg}…` → `Command` (one optional `[…]` if it immediately follows, then a
    greedy run of mandatory `{…}` groups — generic capture; per-command arity is a later
    layer, so `\textbf{a}{b}` captures two args, and a space breaks the run);
  - control symbols (`\,`, `\\`, `\{`) → argless `Command`;
  - `\begin{env}[opt]{arg}… body \end{env}` → `Environment` with a **matched** close
    (a `\begin{a}…\end{b}` mismatch is a spanned error); environments nest;
  - math islands (`$…$`, `$$…$$`, `\(…\)`, `\[…\]`) → `Math { display, content }` keeping
    the **raw inner source** for L2;
  - comments, active `~`; in text mode `& # ^ _` are literal characters.
- **`Node` AST** (`ast.rs`) with **`Node::to_latex`** / `document_to_latex` — round-trips:
  `parse(&render(ast)) == ast` (AST-equality; surface spacing and `$`/`\(` delimiter
  choice are normalized). Reserves an `Unsupported { construct, span }` variant for the
  TeX-programmability asymptote (not produced at L1).
- **`ParseError`** — spanned; structural errors (unbalanced braces, env mismatch,
  unterminated env/math) never panic.
- +19 tests (39 unit + 1 doc total), incl. a round-trip corpus; clippy `-D warnings` clean.

## [0.1.0] — 2026-06-26

### Added — LTX01 L0: crate scaffold + catcode tokenizer

- New standalone, **zero-dependency** crate `latex` (added to the Rust workspace members).
  A full-fidelity LaTeX parser for documents *and* math; first frontend of the
  `math-frontend` framework.
- **`catcode(c)`** — TeX category codes (default plain-LaTeX assignments): Escape,
  BeginGroup, EndGroup, MathShift, AlignTab, EndLine, Parameter, Superscript, Subscript,
  Space, Letter, Other, Active, Comment.
- **`tokenize(&str) -> Result<Vec<Token>, LexError>`** — a catcode-driven, **text-mode-
  primary** state machine:
  - mode stack: Text (primary) ↔ Math (pushed by `$`/`\(`/`\[`, display via `$$`/`\[`,
    popped by the matching close); whitespace is significant in text, ignored in math;
  - control words (`\`+letters, with TeX space-absorption) and control symbols
    (`\`+non-letter, incl. `\\` line break, `\{`, `\,`, …);
  - groups `{ }`, math on/off (`MathOn`/`MathOff` with inline/display flag), `&`, `#`,
    `^`, `_`, active `~`, comments (`%` to end of line, eating the newline);
  - whitespace: a run collapses to one `Space`; a blank line (≥2 newlines) is `Par`;
  - ordinary characters emitted one-per-`Char` (faithful to TeX; the parser coalesces).
- **`Token` / `TokenKind` / `Span`** — every token carries a half-open byte span.
- **`LexError`** — spanned; the scanner **never panics** (trailing `\` → spanned error;
  a stray `\)`/`$` in text mode does not underflow the mode stack).
- 20 unit + 1 doc test; `cargo clippy -- -D warnings` clean; no `unsafe`.

### Notes

- Scope is full LaTeX surface; the Turing-complete TeX tail is the documented asymptote
  (see LTX01). The structural parser (L1), math AST (L2), environments (L3), macros (L4),
  text breadth (L5), and the `MathFrontend` adapter (L6) arrive in subsequent layers.
