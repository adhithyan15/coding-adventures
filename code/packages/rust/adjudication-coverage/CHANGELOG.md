# Changelog

All notable changes to this project will be documented in this file.

## [0.3.0] - 2026-05-13 — ADJ22 typed-quantity coverage

### Added

`check_typed_quantity_coverage(doc, ir_doc)` — a sibling checker to
the existing `check_coverage`, implementing
[ADJ22](../../../specs/ADJ22-typed-quantity-coverage.md).

For every numerical literal in the source text (`\d+(\.\d+)?`), the
checker verifies that at least one IR node has source_spans
overlapping the literal AND a `quantity(value, unit)` compound term
somewhere in its term tree (recursively, so deeply-nested
quantities are also detected).

Violations come back as
`TypedQuantityViolation::MissingQuantity { literal, location,
nearby_nodes }` which an ADJ06 clarification prompt can use to
re-prompt the model: *"You produced node N1 over this span but did
not include the quantity 4."*

### Why this exists

ADJ18's empirical bench (v0.13 prompts, PR #3066) showed that
LLMs reliably mishandle numerical thresholds when asked to apply
rules inside their own forward pass. The structural fix is to
preserve typed quantities in the source IR so the engine can do
the arithmetic deterministically.

[ADJ21](../../../specs/ADJ21-typed-quantity-decomposition.md)
(PR #3071) updates `decompose_text`'s prompt to teach the LLM to
emit `quantity(value, unit)` compounds. ADJ22 is the validator
that catches when the LLM drops the quantity anyway — either by
omitting it, flattening it into the predicate name
(`blade_4_inches` instead of `blade_length(knife, quantity(4,
inches))`), or losing the unit.

### Scope

The check focuses on `NodeKind::Fact`, `NodeKind::Rule`, and
`NodeKind::Uncertainty` nodes — the kinds that are expected to
carry source-level quantities. Section, Entity, Query, Discarded,
and Exception nodes are exempt; their terms carry structure,
identity, or queries rather than measurements.

The checker matches the literal's value (post-normalisation —
`"4"`, `"4.0"`, `"04"` all canonicalise to `"4"`) against atoms
OR numeric `Term::Num` values in the IR. Units are not validated
in this iteration — the checker only verifies that *some*
`quantity(<lit>, _)` compound exists; subsequent passes (a future
ADJ22.x) can enforce specific unit vocabularies per domain.

### Edge cases

- Numbers without units (`"1 carry-on bag"`): still flagged if no
  `quantity(1, _)` exists. The ADJ21 prompt teaches the model to
  emit `quantity(1, count)` for these.
- Decimals (`"3.4 oz"`): matched via canonical-decimal
  normalisation so `quantity(3.4, oz)` and `quantity(Float(3.4), oz)`
  both pass.
- Nested compounds: the term-tree walk is recursive, so deeply-
  nested quantities inside other compounds (e.g.,
  `meets_threshold(blade_length(knife, quantity(4, inches)))`) are
  detected.

### Tests

13 new tests added (20 lib total, all passing):

- Literal scan: integers, decimals, multiples, none.
- Pass: top-level quantity compound, decimal, numeric-atom,
  no-numbers-in-source, deeply-nested quantity.
- Fail: missing quantity, flattened-into-predicate
  (`blade_4_inches`), multiple missing literals reported
  separately.
- Normalisation: leading zeros, trailing decimal zeros, 0.5
  edge case.

### Compatibility

- Pre-existing `check_coverage` API unchanged.
- `Document` struct unchanged.
- New types (`TypedQuantityViolation`, `TypedQuantityResult`,
  `check_typed_quantity_coverage`) are additive.
- `logic-core` moved from dev-dep to dep so the checker can
  inspect `Term` shapes.

### What's NOT in this PR

- **Pipeline wiring**: ADJ22 is a pure check today. Wiring it
  into `adjudication-pipeline` so failures route to ADJ06
  clarification is a follow-up. The check is callable standalone
  in the meantime.
- **Unit-vocabulary enforcement**: the check verifies a
  `quantity(<lit>, _)` exists but doesn't validate the unit
  atom. A future iteration could enforce per-domain unit
  vocabularies if needed.

## [0.2.0] - 2026-05-11 — ADJ02 v2 structural rewrite

### Replaced

The entire v0.1.0 rule-based coverage check is replaced with a
**structural tree-tiling check** over the v2 IR (per
[ADJ02 v2](../../../specs/ADJ02-coverage-checker.md)).

What's gone:

- `Tagger` trait
- `RuleBasedTagger` default implementation
- English stopword / filler lists
- `TokenAnnotation`, `TokenLabel`
- `NonMeaningfulReason` enum
- `StrictnessMode` enum (Strict / Permissive / AuditOnly)

What replaces them:

- `check_coverage(doc, ir_doc) -> CoverageResult` — runs
  `adjudication_ir::validate` (conditions 1, 3, 4 from ADJ02 v2),
  then adds the root-tiling check (condition 2) and the
  `Unparseable` discarded check (condition 5).
- `CoverageViolation` enum with 9 precise variants:
  `SpanWrongDocument`, `InvalidSpan`, `UnparseableDiscarded`,
  `RootsDoNotTileDocument` (with `missing_ranges`),
  `DanglingPartOf`, `ChildSpansExceedParent`,
  `ChildrenDoNotTileParent` (with `missing_ranges`),
  `NonTextRunHasChildren`, `PartOfCycle`.
- `Document { id, normalized_text }` — the check reads only
  `normalized_text.len()`; it never inspects bytes.

### Algorithm

Pure structural. Runs in `O(N log N)` over IR node count + total
span count. No LLM call at check time. Language-agnostic by
construction.

### Why this rewrite

The v0.1.0 path baked English-language assumptions into the
framework core (stopwords list, default-meaningful direction). The
v2 IR (ADJ01 v2) introduced the hierarchical decomposition so the
LLM can encode "what counts as content" in the tree itself; the
framework's job is to verify the tree's structural completeness,
not to second-guess what the LLM saw.

### Tests

11 tests cover: empty document, nonempty document with empty IR,
single-root TextRun tiling, root gap with `missing_ranges`, child
gap within TextRun with `missing_ranges`, Unparseable always fails,
Pleasantry-Discarded passes, nested TextRuns, merge_ranges /
subtract_intervals helpers, dangling part_of surfaces as a coverage
violation. `cargo build / test / clippy --no-deps` clean.

## [0.1.0] - 2026-05-11

### Added

- `Document { id, normalized_text }` — the unit of coverage analysis,
  carrying the bytes that `IRNode::source_spans` reference into.
- `Tagger` trait with one method `classify_tokens(doc) ->
  Vec<TokenAnnotation>`. A `TokenAnnotation` says whether a byte
  range of the document is meaningful (must be covered by some IR
  node) or non-meaningful (allowed to be ignored).
- `RuleBasedTagger` — the default implementation. Splits the document
  into word-shaped tokens plus punctuation, then classifies each as
  meaningful or non-meaningful using a configurable stopword list,
  punctuation predicate, and an "always-meaningful" override list.
  Sufficient for narrow domains; ADJ02 also envisions classifier-
  model and LLM taggers as drop-in alternatives.
- `NonMeaningfulReason` enum mirroring the controlled vocabulary
  from the spec: `Whitespace`, `Punctuation`, `Stopword`,
  `SocialPleasantry`, `DocumentChrome`, `Boilerplate`, `Determiner`,
  `Filler`.
- `CoverageResult` enum — `Pass` or `Fail { uncovered }`. The
  `uncovered` list reports each meaningful byte range that no IR
  node covers, suitable for surfacing as clarification questions
  (per ADJ06).
- `check_coverage(doc, ir_doc, tagger)` — the main entry point.
  Implements the interval-cover algorithm from ADJ02 in linear time
  after sorting and merging the IR's source spans.
- `StrictnessMode` enum (`Strict`, `Permissive`, `AuditOnly`). Per
  ADJ02:
  - `Strict` — any uncovered meaningful byte fails coverage.
  - `Permissive` — uncovered `Filler` / `Determiner` tokens are
    tolerated; everything else fails.
  - `AuditOnly` — never fails; returns `Pass` regardless but the
    result still reports the uncovered ranges for telemetry.
- Enforcement of ADJ01's rule that a `Discarded` node with reason
  `Unparseable` always fails coverage (the spec explicitly forbids
  shipping such spans).
- 14 tests covering: empty document, fully-covered single-fact
  trace, an uncovered meaningful span flagged with location, the
  three strictness modes, the `Unparseable` rejection, multi-IR-
  node overlap, the canonical TSA "I am not bringing matches"
  example with both correct and incorrect span citation.

### Notes

This is the Rust reference implementation of [`ADJ02`](../../../specs/ADJ02-coverage-checker.md).
The tagger is pluggable: drop-in classifier-model or LLM taggers
are supported by implementing the `Tagger` trait. The default
`RuleBasedTagger` is sufficient for narrow domains and for the
TSA worked example from ADJ10.
