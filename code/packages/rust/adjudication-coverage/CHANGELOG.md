# Changelog

All notable changes to this project will be documented in this file.

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
