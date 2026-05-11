# Changelog

All notable changes to this project will be documented in this file.

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
