# LTXDOC02 — precise per-token byte spans (retiring the region-coarse caveat)

## 1. Motivation

LTXDOC01 shipped the LaTeX → `Document` AST end-to-end (D1–D6), culminating in the D6 provenance API
`Document::walk()` / `Document::node_at(byte)`. But every D6 doc-comment carries an honest caveat: the
spans are **region-coarse**. When the D2 fold lowered a source region it handed *every* block and inline in
that region the same enclosing-region `Span` (the `region` parameter threaded through
`lower_block`/`lower_inlines`). So a `Section`'s title inlines, its child paragraphs, and their `Text` runs
frequently **share one span**, and `node_at(byte)` can only resolve to the innermost node *at region
granularity* — the tightest-covering node among many that share a region, not the single leaf a
precise-span parser would pinpoint.

**The information to fix this already exists.** The L1 lexer records a precise half-open byte `Span` on
**every** `Token` (`token.rs`: "Every token records its half-open byte Span"). The parser reads it —
`parser.rs`'s `parse_one`/`parse_seq_inner`/`parse_seq` each bind `let sp = tok.span` at the exact point
they build a `Node` — and then **discards** it: every `Node` variant is span-less *except*
`Node::Unsupported { span }`. The precise byte ranges are computed and thrown away at the parse→Node step.

LTXDOC02 threads those real token spans from the lexer, through the parser and the opt-in recognition
passes (`recognize_structure`/`recognize_accents`/`recognize_tables`), into `build_document`, so each
`Document` `Block`/`Inline` carries its **tightest true byte range** and `node_at` resolves to a genuine
per-token leaf. On completion the region-coarse caveat is **retired**.

## 2. Scope

- **In:** byte-accurate `Span` on every `Node` produced by L1 + the recognition passes; propagation into
  every `Document` `Block`/`Inline`; a `node_at` that resolves to the tightest true leaf; a precise
  byte-coverage capstone; deletion of the region-coarse doc-caveats once earned.
- **Out (unchanged):** the parsing grammar itself (no new LaTeX constructs — LTXDOC02 only *retains
  information the parser already has*); math-island internals (a `Math`/`DisplayMath` island keeps its
  source + one span for the whole island, as today — sub-tokenizing math source is a separate future
  effort); `to_latex` output text (round-trip must stay a fixed point modulo spans).

## 3. Invariants (hard gates, every rung)

1. **Totality / no panic.** Span threading is infallible; no `unwrap`/`expect`/unchecked indexing on any
   parse or fold path; span arithmetic guarded (`saturating_sub`, `Span::new` guards `end < start`).
2. **Containment.** For every node, `child.span ⊆ parent.span ⊆ Document.span`; asserted in tests.
3. **Tightness (the new guarantee).** A leaf node's span is exactly the byte range of its own source
   token(s) — not an enclosing region. A byte inside `widgets` resolves via `node_at` to the `Text`
   node owning `widgets`, not to the enclosing `Paragraph`/`Section`.
4. **Round-trip fixed point.** `parse_document(d.to_latex()).strip_spans() == d.strip_spans()` still holds;
   `to_latex` text is unchanged (spans move on re-emit, structure does not).
5. **No `unsafe`; recursion `MAX_DEPTH`-bounded** (unchanged from LTXDOC01).

## 4. Mechanism (the design decision)

Every `Node`-building site in `parser.rs` already holds the start token's span; the construct's end span is
the last token consumed before returning. The recommended mechanism (implementation may refine, per the
repo's autonomous-decisions principle):

- **Carry a `Span` on `Node`.** Give each `Node` variant a byte `Span` (the `Unsupported { span }` variant
  already sets the precedent). The parser fills it from `[first_token.span.start, last_token.span.end)` as
  it builds each node; `Group`/`Command`/`Environment` spans span their delimiters (`{`…`}`,
  `\name`…last-arg-`}`, `\begin`…`\end`). Composite spans are the union of the covered tokens, computed
  from the already-tracked start/end — **not** re-derived by substring search.
- **Recognition passes** (`recognize_structure`/`accents`/`tables`) regroup existing nodes; each synthesised
  node's span is the union of its constituents' spans (`Section` = heading-command start … last-child end;
  `Tabular`/`List` = `\begin` … `\end`). No new byte scanning.
- **`build_document`** stops assigning the enclosing `region` span and instead reads each node's carried
  span directly, so `Block`/`Inline` spans become precise. The `region` plumbing is deleted once every
  consumer reads real spans.

Alternative considered and rejected: a parallel `Spanned<Node>` wrapper or an out-of-band node→span side
table. Both double the surface and de-sync from `Node` under edits; carrying the span on the node (as
`Unsupported` already does) is the faithful, single-source-of-truth choice. Compat may break freely
(nothing is released) — prefer the clean restructure over a shim.

## 5. Ladder (small, layered PRs; each fully tested; final state = precise spans)

- **S1 — spanned L1 nodes.** Add the `Span` to `Node`; fill it in `parser.rs` from the tracked token
  spans. `to_latex` unchanged; add tests asserting each top-level node's span slices back to its exact
  source substring (`&src[span] == "\\textbf{x}"`, etc.). The one parser-level rung.
- **S2 — spanned recognition passes.** `recognize_structure`/`recognize_accents`/`recognize_tables` compute
  each synthesised node's span as the union of its parts; tests assert `Section`/`Accent`/`Tabular`/`List`
  spans cover exactly their source extent.
- **S3 — precise Document fold.** `build_document` reads carried node spans instead of the `region`;
  `Block`/`Inline` spans become tight. Delete the region-coarse span assignment; containment tests tighten
  from "⊆ region" to "== node source range".
- **S4 — precise `node_at` + retire the caveat.** ✅ *(shipped, latex 0.35.0)* With tight spans,
  `node_at(byte)` returns the true per-token leaf — the narrowest node whose *precise* span contains the byte
  (ties → deepest in pre-order). Retired the "region-coarse" hedging on `node_at`/`Provenance`/`walk` and the
  module span note for **body** nodes; kept the honest region-coarse notes on `Preamble`/`DocumentClass`/`Package`
  (classified out of directives, not walked) and updated the LTXDOC01 §4/§5 caveats to say body resolution is now
  precise. Added leaf-resolution tests (`node_at_resolves_to_text_leaf_not_paragraph`,
  `node_at_in_section_title_resolves_to_heading_inline`, `node_at_in_textbf_resolves_to_inner_leaf`) — a byte inside a
  `Text`/title/inner-run resolves to that leaf and its span slices back to exactly the word — plus an honest body
  byte-coverage test (`body_bytes_resolve_to_containing_node`) asserting every non-whitespace body byte both resolves
  and resolves to a node whose precise span contains it (representative input only; the whole-corpus
  tightest-covering-leaf capstone stays S5). No `node_at`/`walk` logic changed.
- **S5 — precise byte-coverage capstone.** Over the LTXDOC01 capstone corpus, assert every non-whitespace
  body byte resolves (`node_at(byte).is_some()`) AND the resolved node is a **leaf** whose span is the
  tightest covering range (no strictly-narrower node also contains the byte). The precise counterpart of
  D6's region-scoped coverage test.

## 6. Verification (every rung)

`cargo test -p latex` green; `cargo clippy -p latex -- -D warnings` clean; downstream
`cargo test -p adj-lang -p adj-lang-cli` green; `cargo build -p latex --no-default-features` builds. No
`cargo fmt`, no grammar regen. Round-trip corpus fixed point preserved at every rung.

## 7. Payoff

Precise spans make the LaTeX → `Document` provenance surface exact: the ADJ byte-provenance pipeline (the
north-star consumer) can point at the *specific* source token a fact was read from, not merely the region.
This is the foundation for byte-faithful justification over LaTeX documents — the same guarantee the ADJ
adjudication stack enforces for prose, now for structured LaTeX.
