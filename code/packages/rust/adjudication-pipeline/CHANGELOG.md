# Changelog

All notable changes to this project will be documented in this file.

## [0.13.0] - 2026-05-14 — ADJ28: orchestrator reads boolean kind schema + discard_justification

### Changed

`parse_child_node` now extracts the node's `kind` from two
supported shapes:

1. **Legacy single-`kind` string field** (levels 1 & 2 keep this).
2. **ADJ28 per-kind `is_X` boolean schema** (levels 3 & 4). The
   new helper `extract_kind` walks the boolean field set,
   collects every `is_X: true`, and accepts the node only when
   exactly one boolean is true. Zero true or multiple true →
   node rejected.

Discard nodes additionally have:

- `discard_reason` parsed via the new helper `parse_discard_reason`
  into the `adjudication_ir::DiscardReason` enum (unknown strings
  fall back to `NonDomainContent`).
- `discard_justification` (free-form string) stored on the node's
  `metadata` map under the reserved key
  `DISCARD_JUSTIFICATION_METADATA_KEY = "adj.discard_justification"`.
  Lets the audit trail keep the model's own rationale for
  discarding.

### Tests

4 new test cases:

- `adj28_boolean_kind_schema_derives_kind` — end-to-end through
  the orchestrator: a Claim node emitted via `is_fact: true` flows
  through `parse_child_node` and lands in the IR as a Fact.
- `adj28_zero_or_multiple_true_booleans_rejected` — both
  failure paths (no true booleans, multiple true booleans) cause
  the parser to skip the child.
- `adj28_discard_justification_lands_in_metadata` — when the
  LLM emits a `discard_justification`, the orchestrator copies it
  into `node.metadata` under the reserved key.
- `adj28_discard_reason_string_parsed_into_enum` — round-trip
  every documented reason string + unknown / empty fallbacks.

Total `adjudication-pipeline` hierarchical-module tests: 9 → 13,
all passing. Workspace pipeline tests: 47 → 51.

### Notes

- Version: 0.12.0 → 0.13.0 (additive parsing change).

## [0.12.0] - 2026-05-13 — orchestrator: content-shaped span derivation

### Changed

The hierarchical orchestrator now derives child `source_spans` by
matching LLM-emitted `text` fields against the parent text — the
model never computes byte offsets. Per
`feedback_no_byte_arithmetic_for_llm`.

Specifically:

- `parse_child_node` reads each child's `text` field instead of
  `source_spans` and returns `(IRNode, Option<String>)` so the
  caller can do the content-matching.
- `splice_children` walks LLM-emitted children left-to-right against
  the parent text with a cursor. The leftmost match at-or-after the
  cursor anchors the child's absolute spans; the cursor advances
  past the match. Content fabrications (text not found in the
  remaining parent) are skipped — the coverage check surfaces the
  resulting gap.
- `snapshot_children_as_json` (the prior-attempt rendering for
  retries) emits `text` fields instead of `source_spans`, so the
  model never sees its own byte offsets fed back.
- `render_gap_description` extracts the LITERAL missing substrings
  from the source and shows them to the model. Byte ranges are
  banished from the retry prompt.
- The dead `parse_spans` function and the `MAX_SPANS_PER_NODE`
  constant were removed.

### Tests

Two prior parse_spans unit tests were replaced with content-shaped
tests: `adj27_text_matching_derives_document_absolute_spans` (clean
3-byte + 7-byte tiling with absolute-span verification) and
`adj27_text_not_in_parent_is_skipped` (fabrication handling). Total
hierarchical-module tests: 9, all passing. Workspace pipeline lib
tests: 47/47.

### Smoke benchmark

Gemma4 against `"1 carry-on bag, matches."` cell wallclock dropped
from 505s (ADJ27 byte-shaped contract) to 176s with the new content
matching. Coverage still doesn't close fully — the model emits some
text that doesn't match parent bytes verbatim — but more children
are accepted into the IR per parent than before, and retries now
ride on top of literal missing substrings rather than byte
arithmetic.

### Notes

- Version: 0.11.0 → 0.12.0.

## [0.11.0] - 2026-05-13 — ADJ25 PR-6: foundation bench harness

### Added

- `src/bin/adj_pr6_bench.rs` — small CLI binary that wraps
  `decompose_hierarchical` and reports per-level coverage as
  structured JSON. Reads source / model / endpoint / timeout /
  retries from env vars (`ADJ_PR6_*`) and emits one JSON record to
  stdout per invocation. Errors always exit 0 so the harness can
  capture diagnostics.
- 3 unit tests for the binary's helpers: epoch conversion (1970-01-01
  / one-day round-trip) and `summarise_coverage` shape.

### Spec

- `code/specs/ADJ26-foundation-bench.md` — methodology for the 8 ×
  5 matrix bench, reproduction instructions, hypotheses, gating
  condition for unblocking the other workstreams (ADJ14 / 15 / 16
  / 17 / 18 / 19 / 20).

### Harness

- `scripts/adj_pr6_foundation_bench.py` — Python driver that iterates
  the 8 ADJ18 declarations × 5 ADJ12 models matrix, shells out to
  the bench binary per cell, captures JSON output, writes to
  `code/specs/data/adj25-pr6-foundation-bench-YYYY-MM-DD.json`.
  Persists after every cell so a crash loses at most the in-flight
  cell.

### Dependency change

- `llm-gateway` promoted from `[dev-dependencies]` to a regular
  dependency — needed by the new bench binary.
- `llm-provider-ollama` added as a regular dependency — same.

### Scope

This PR is **methodology + harness only**. The empirical-results
section of `ADJ26` is marked `TBD`; a follow-up data PR runs the
bench against a live Ollama instance and adds the empirical
results + a proposed threshold for unblocking the paused
workstreams.

### Notes

- Version: 0.10.0 → 0.11.0 (new bin + new deps).

## [0.10.0] - 2026-05-13 — ADJ25 PR-5: orchestrator emits correlation IDs

### Added

The hierarchical orchestrator (`decompose_hierarchical`, introduced
in PR-4) now assigns a `CorrelationId` to every IR node and every
`Contains` edge it produces. IDs are deterministic from the
`NodeId` / `EdgeId` so re-runs against the same source produce the
same correlation tree (which is exactly the property the audit-trail
replay discipline depends on).

Specifically:

- Every node emitted by the orchestrator carries
  `adj.correlation_id = "corr.<node_id>"` in its metadata.
- Every Contains-edge emitted by the orchestrator carries
  `adj.correlation_id = "corr.e.<edge_id>"` in its metadata.

The output of `decompose_hierarchical` satisfies
`adjudication_ir::check_correlation_completeness` by construction.

### Tests

2 new test cases: `adj25_orchestrator_output_is_correlation_complete`
(end-to-end completeness check on the orchestrator's output, with
spot checks on the Document root and an LLM-supplied id) and
`adj25_orchestrator_emits_correlation_ids_on_contains_edges`
(every Contains edge has a non-empty `corr.e.*` id). Total tests:
45 → 47, all passing.

### Notes

- Version: 0.9.0 → 0.10.0 (additive behaviour — the orchestrator's
  surface is unchanged; only the metadata it embeds is richer).
- Connector + audit-trail integration in this same release cycle
  (adjudication-connector v0.3.0 — see its changelog).

## [0.9.0] - 2026-05-13 — ADJ25 PR-4: hierarchical decomposition orchestrator

### Added

New `pub mod hierarchical` containing `decompose_hierarchical`, the
orchestrator that drives the ADJ25 level-by-level decomposition flow
end-to-end. Per the
[ADJ25 spec](../../specs/ADJ25-hierarchical-decomposition.md), the
orchestrator:

1. Builds the synthetic `Document` root node spanning the full source.
2. For each level transition in order
   (`Document → Sentence`, `Sentence → Phrase`, `Phrase → Claim`,
   `Fact → TypedComponent`), iterates every parent at that level and
   dispatches one `adjudication_clarification::retry_decompose_level`
   call. Parses the response, splices children into the IR, connects
   parent → child via `Contains` edges, and translates LLM-reported
   parent-relative spans to document-absolute byte offsets.
3. Runs `adjudication_coverage::check_hierarchical_coverage` against
   the assembled IR. For every reported gap, dispatches a retry
   targeting the specific failing parent (with the gap description
   in the retry prompt body).
4. Loops up to `max_retries_per_parent` per parent.

### New public surface

- `HierarchicalDecomposeRequest` — `{ document_id, source_text,
  max_retries_per_parent }`.
- `HierarchicalDecomposeOutcome` — `{ ir_document, total_llm_calls,
  retry_calls }`.
- `HierarchicalDecomposeError::{ Primitive, UnparseableResponse,
  CoverageUnresolved }`.
- `decompose_hierarchical(req, gateway, now)`.
- `DEFAULT_MAX_RETRIES_PER_PARENT = 3`, `PER_LEVEL_DISPATCH_CAP = 1024`.

### Crate-placement deviation from the ADJ25 spec

ADJ25's PR-4 entry suggested the orchestrator live in
`llm-primitives`. That cannot work without a dependency cycle: the
orchestrator needs `retry_decompose_level` (in
`adjudication-clarification`) and `check_hierarchical_coverage` (in
`adjudication-coverage`), and both crates depend transitively on
`llm-primitives`. Placing the orchestrator in `adjudication-pipeline`
(which already depends on both) lets it use both without inverting
the dependency graph. The intent of the spec — a single orchestrator
that drives the level-by-level flow — is unchanged.

### Dependency change

- `adjudication-clarification` is now a direct dependency of
  `adjudication-pipeline`. No version change required (path dep).

### Scope and what PR-4 deliberately does not do

- **Correlation vector propagation** — PR-5 territory. The
  orchestrator assigns deterministic `NodeId`s; PR-5 adds a parallel
  `CorrelationId` space that flows through engine clauses and the
  audit trail.
- **New `decompose-text-vN` prompt** that teaches the LLM the
  hierarchy. The orchestrator currently relies on whatever prompt
  `decompose_text` is shipping with (`v5`, which teaches the flat
  IR shape). Real-LLM behaviour against a hierarchy-aware prompt is
  measured in PR-6 (the foundation bench). The orchestrator is
  designed to work end-to-end with scripted clients today and benefit
  from a richer prompt later without changing its surface.

### Tests

7 new test cases covering: clean-hierarchy assembly for a single-word
source, span translation (parent-relative → document-absolute),
span clamping past parent end, unparseable response rejection,
wrong-kind filtering at level, retry-budget exhaustion, and a
`parse_kind` round-trip locking the v3 + ADJ25 enum surface. Pipeline
test count: 38 → 45, all passing.

### Notes

- Adding variants to a non-`#[non_exhaustive]` enum is a SemVer
  breaking change. None of the existing pipeline enums got new
  variants here (the new orchestrator types are entirely new); this
  is a pure-additive minor bump.
- Version: 0.8.0 → 0.9.0.

## [0.8.0] - 2026-05-13 — ADJ24 typed-quantity wiring

### Added

ADJ22 is now a first-class pipeline pass. Per
[ADJ24](../../../specs/ADJ24-typed-quantity-pipeline-wiring.md):

- Both `run_with_gateway` and `run_with_rulebooks` now call
  `adjudication_coverage::check_typed_quantity_coverage` between
  ADJ02 (coverage) and ADJ03 (polarity/modality). The result is
  recorded as a `CheckerResult` with
  `pass_name: PassName::Adj22TypedQuantity` and
  `pass_version: "v0.1"`.
- ADJ22 joins the engine-gating set: the engine runs only when
  ADJ02 passes AND ADJ22 passes AND ADJ03 passes. ADJ04 and ADJ05
  are also skipped if ADJ22 fails (no point paying for the LLM
  calls when the IR's missing typed quantities the downstream
  engine needs).
- New helper `typed_quantity_to_checker_result` maps the
  `TypedQuantityResult` (Pass / Fail) into the audit-trail shape;
  each `TypedQuantityViolation::MissingQuantity` becomes one
  `Violation` with `kind: MissingQuantity` and a `detail` JSON of
  `{ literal, location: [start,end], nearby_nodes: [ids…] }` —
  the exact shape ADJ06's typed-quantity retry consumes.

### Why this ordering

Per the ADJ24 spec rationale:

- **ADJ02 first** — if the IR doesn't tile the source, ADJ22's
  `nearby_nodes` computation (which uses overlapping source spans)
  reports misleading information. Fix coverage first.
- **ADJ22 before ADJ03** — typed quantities are a structural
  property of the IR shape, the same way coverage is. ADJ03's
  polarity / modality concerns are layered on top of a
  structurally-correct IR.

### Compatibility

- The pre-existing API surface (`run`, `run_with_gateway`,
  `run_with_rulebooks`) is unchanged. Existing callers continue
  to work; the only behaviour change is one additional
  `checker_result` in the trail and the gate-tightening on the
  engine path.
- `adjudication-coverage` dep is unchanged at `path = "../..."`
  (v0.3.0 ships the `check_typed_quantity_coverage` entry).

### Tests

Existing pipeline tests stay green. Behaviour is exercised
end-to-end through `adjudication-tsa-demo` (v0.x → next, which
adds an ADJ22 branch to its clarification loop).

## [0.7.0] - 2026-05-12

### Added

ADJ16 step 4: agreement-weighted rulebook merging.

- `compute_agreement_weighted_rulebook(rulebooks, output_document_id)` —
  takes a slice of `&IRDocument` (one per source rulebook) and
  returns a single merged `IRDocument` where each rule's weight
  reflects multi-model agreement. The output is a probabilistic
  rulebook IR suitable to feed back into [`run_with_rulebooks`].

### Algorithm

1. For each input rulebook, walk every Rule node.
2. For `definitional(head, [body...])` rules, group by exact Term
   equality of `(head, body)` across all rulebooks. The weight for
   each group is `count / total_rulebooks`. The output emits one
   `probabilistic(weight, head, [body...])` rule per group.
3. `probabilistic(p, ...)` rules pass through unchanged (their
   existing probability is preserved).
4. `constraint(...)` and `default(...)` rules pass through
   unchanged. The agreement-weight idiom is naturally expressed
   over `definitional` rules; extending to `default` is a future
   iteration.

### Edge cases

- Empty `rulebooks` slice returns an empty IRDocument.
- One rulebook: every rule gets weight 1.0 (1/1).
- Within-rulebook duplicates are deduplicated: a single rulebook
  listing the same rule twice contributes at most 1 to the count.

### Rationale (ADJ16 step 4)

[ADJ17](../../../specs/ADJ17-adversarial-rulebook-empirical-results.md)
showed that adversarial multi-model elicitation produces rulebooks
where rules vary in quality. Some rules appear in every model's
output (high confidence); others are model-specific (low
confidence). Step 4 quantifies that signal: feeding the merged
rulebook into the engine via `SearchMode::EnumerateAll` produces
weighted-model-counting probabilities that propagate rule-level
uncertainty into the verdict's marginal probability.

### Tests

8 new tests added (35 lib + 4 integration total, all passing):
- empty rulebook slice returns empty doc
- single rulebook assigns weight 1.0 (1/1)
- two rulebooks in full agreement collapse to one rule with weight 1.0
- partial overlap yields proportional weights (1.0 and 0.5)
- three rulebooks with no overlap yields three rules each at 1/3
- within-rulebook duplicates don't inflate the count
- non-definitional rules pass through unchanged
- end-to-end smoke: merged rulebook feeds into `run_with_rulebooks`
  cleanly and produces the expected verdict + provenance

### Compatibility

- All existing API is unchanged.
- The new function is purely additive.

## [0.6.0] - 2026-05-12

### Added

ADJ16 step 3: `DisputedAnswer` detection.

- `DisputedAnswer { query, candidates, resolution_required }` — new
  public type. Captures a query whose engine proof DAG contains
  multiple proofs that come from different rulebooks AND produce
  different variable bindings.
- `DisputeCandidate { bindings, via_facts, via_rules, source_rulebooks }` —
  one entry per distinct (binding, rulebook-attribution) pair in a
  disputed answer's proof DAG. `source_rulebooks` is sorted
  lexicographically for stable display.
- `ResolutionRequirement::HumanReview` — the default emitted by
  `detect_disputes` in v0.6. A future variant
  `TrustTierDominates { winner_rulebook_id }` is sketched in the
  spec for deployments where a higher-trust rulebook should win
  automatically; not emitted yet.
- `detect_disputes(answers, provenance) -> Vec<DisputedAnswer>` —
  the dispute-detection function. Walks every `EnumerateAllResult`
  in `answers`, attributes each proof's `via_facts` / `via_rules`
  to source rulebooks via the `ClauseProvenanceTable`, and surfaces
  a dispute whenever distinct rulebook attributions produce
  distinct bindings. Returns an empty vec when no dispute is
  detected.
- `PipelineOutput.disputed_answers: Vec<DisputedAnswer>` — new
  public field. Empty for legacy entry points (run,
  run_with_gateway), populated by `run_with_rulebooks` based on
  `detect_disputes`'s output.

### Changed

- `run_with_rulebooks` now uses `SearchMode::EnumerateAll` when the
  `rulebooks` slice is non-empty (was `AutoDetect`). Rationale:
  dispute detection requires every successful proof to be returned
  — `FindFirst` would stop at the first success and hide
  disagreements between rulebooks. The audit trail's `search_mode`
  reflects what actually ran. With no rulebooks attached the search
  mode is still `AutoDetect`.

### Dispute semantics

A dispute requires a **pair of proofs** `(p_i, p_j)` satisfying:

1. `p_i.source_rulebooks != p_j.source_rulebooks` — different
   rulebooks contributed to the two proofs, AND
2. `p_i.bindings != p_j.bindings` — those rulebooks produced
   different bindings for the query variables.

The joint per-pair check (rather than two global existence
checks) avoids a subtle false-positive where one rulebook's
within-rulebook ambiguity gets paired with an unrelated second
rulebook's *corroborating* proof. The standard formulation is "is
there a pair of proofs that disagree across rulebook boundaries",
and that's exactly what the implementation checks.

Same bindings from different rulebooks = **corroboration**, not a
dispute. Same rulebook with different bindings (alone) =
**within-rulebook ambiguity**, also not a dispute. Tests in
`no_dispute_when_two_rulebooks_corroborate_with_same_bindings` and
`no_dispute_from_corroborating_pair_even_with_within_rulebook_ambiguity`
document the edge cases.

### Tests

6 new tests added (27 lib + 4 integration total, all passing):
- `no_dispute_when_single_proof_returned` — single proof = no
  ambiguity to dispute
- `no_dispute_when_two_rulebooks_corroborate_with_same_bindings` —
  same bindings across rulebooks = corroboration
- `dispute_detected_when_rulebooks_produce_different_bindings` —
  canonical conflict case (strict vs lenient classification)
- `no_dispute_from_corroborating_pair_even_with_within_rulebook_ambiguity` —
  joint per-pair check correctly flags only genuine
  cross-rulebook disagreements
- `detect_disputes_with_empty_attribution_returns_empty` — sanity
- `run_with_rulebooks_uses_enumerate_all_when_rulebooks_attached` —
  documents the search-mode change

### Rationale (ADJ16 step 3)

[ADJ16](../../../specs/ADJ16-engine-programmatic-adjudication.md)
§"Open questions §2" names the data shape: when two rulebooks
disagree, the engine should return BOTH proof paths attributed to
their sources rather than silently picking one. v0.5 added the
provenance plumbing; v0.6 adds the detector that surfaces the
disagreement through `disputed_answers`. Downstream consumers
(ADJ06 clarification dialogue, ADJ09 expert review, or a
deployment-policy resolver) decide what to do with the dispute.

### Compatibility

- `run` and `run_with_gateway` signatures unchanged. Both still
  return `PipelineOutput` with `disputed_answers: Vec::new()`.
- `PipelineOutput` gained one new field
  (`disputed_answers: Vec<DisputedAnswer>`). Soft-break for
  pattern destructuring (no in-tree caller does that). All three
  downstream demos build unchanged.
- `run_with_rulebooks`'s search-mode change is observable in the
  audit trail (`engine_artifacts.search_mode == EnumerateAll`
  instead of `AutoDetect`). Callers that asserted `AutoDetect` in
  the rulebook path need to update — the new mode is the correct
  one for dispute-aware adjudication. The existing
  `run_with_rulebooks_merges_external_rule_into_kb` test was
  updated to reflect this.

### Dependency change

- `logic-core` moved from `[dev-dependencies]` to `[dependencies]`.
  `DisputeCandidate.bindings: logic_core::Substitution` requires
  the type at compile time of consumers. Logic-engine already
  depends on logic-core, so no transitive dependency is added to
  the workspace.

## [0.5.0] - 2026-05-12

### Added

ADJ16 step 2: the pipeline gains a rulebook-merging entry point.

- `run_with_rulebooks(input, id, now, gateway, rulebooks)` —
  new public entry point that accepts a slice of
  `(IRDocument, ClauseProvenance)` pairs alongside the input
  document. The input document's IR is lowered with a default
  `Authoritative` provenance keyed to the document id; each
  rulebook is lowered with its caller-supplied provenance. The
  resulting `LoweredKb`s are combined via
  `LoweredKb::extend` before the engine queries run. The returned
  `PipelineOutput.clause_provenance` carries per-FactId /
  per-RuleId attribution so the audit trail (and ADJ16 step 3's
  future `DisputedAnswer` resolution) can trace each cited clause
  back to its origin.
- `ClauseProvenanceTable { fact_provenance, rule_provenance }` —
  new public type mirroring
  `adjudication_connector::LoweredKb`'s attribution maps, lifted
  to the pipeline layer so downstream consumers don't have to
  reach into the connector.
- New optional field `PipelineOutput.clause_provenance`. `None`
  for legacy entry points (`run`, `run_with_gateway`); `Some(table)`
  for `run_with_rulebooks`.
- Re-exports `ClauseProvenance` (as `RulebookProvenance`) and
  `TrustTier` (as `RulebookTrustTier`) so callers can construct
  rulebook inputs without depending on `adjudication-connector`
  directly.
- 7 new tests covering: empty-slice no-op, multi-rulebook merge,
  per-rulebook attribution preservation, coverage-blocked early
  exit, malformed-rulebook error path (names the offending
  rulebook id), Query nodes in rulebook IRs ignored, backward-compat
  of `clause_provenance: None` on legacy entry points.

### Rationale (ADJ16 step 2)

[ADJ16](../../../specs/ADJ16-engine-programmatic-adjudication.md)
proposes replacing the answer-time LLM call with the
deterministic logic engine. The pipeline's engine step already
runs deterministically — what step 2 adds is the *plumbing for
rulebook-merged KBs*: today the engine sees only facts/rules from
the source document IR, so a Tentative rulebook elicited via
`acquire_rulebook_adversarial` has no way to contribute clauses
to the answer-time KB. After step 2, callers can pass a list of
rulebook IRs with their provenance, the engine reasons over the
combined KB, and the returned attribution table tells you which
rulebook contributed which clause.

This is the bedrock for step 3 (`EngineVerdict::DisputedAnswer`
with attributed proof paths) and for the ADJ17 follow-up bench
that runs the engine on the adversarially-elicited rulebook
instead of injecting the rulebook text into an LLM system prompt.

### Compatibility

- `run` and `run_with_gateway` are unchanged in signature and
  semantics. They construct `PipelineOutput` with
  `clause_provenance: None`.
- `PipelineOutput` gained a new public field
  (`clause_provenance: Option<ClauseProvenanceTable>`). This is a
  soft-break for callers that pattern-destructure
  `PipelineOutput`; no in-tree caller does. All in-tree callers
  (TSA demo, clinical demo, contract demo) access fields by name
  and continue to build unchanged.

## [0.4.0] - 2026-05-12

### Added

ADJ05 adversarial checker wired in. The pipeline now runs
`adjudication-adversarial::check_adversarial` when:

1. A `GatewayConfig` is supplied, AND
2. `Role::Adversary` is registered, AND
3. The Extractor and Adversary clients come from different
   `(vendor, model_family)` pairs (LM00b independence requirement,
   enforced by `GatewayConfig::check_independence`).

If any of these conditions fails, ADJ05 records as `Skipped` with a
human-readable `skipped_reason` in `telemetry`. If the checker itself
errors (gateway transport failure, primitive validation exhausted,
…), the pipeline records `Failed` with the error string in
`telemetry.check_error` — same pattern ADJ04 uses.

- `Adj05Decision` enum with `Skipped { reason }` / `Ran(result)` /
  `CheckErrored(detail)` variants.
- `run_adj05` / `adj05_to_checker_result` / `adversarial_violation_to_audit`
  helpers next to the ADJ04 equivalents.
- ADJ05 violations carry `ClarificationKind::AdversarialReading`
  with `ir_rendered`, `adversary_reading`, `adversary_explanation`,
  and `judge_reason` in the detail JSON.
- ADJ05 is **advisory** at v0.4 — drift records as Failed but the
  engine still runs. A future ADJ06 wiring can gate on it.
- Two new tests cover ADJ05: skip-when-no-gateway and
  skip-with-reason-when-Adversary-role-missing. The Ran/CheckErrored
  paths are exercised end-to-end by the demo's live integration
  against Ollama.

## [0.3.0] - 2026-05-11

### Added

ADJ04 round-trip checker wired in via a new `GatewayConfig` argument.
When a caller supplies `Renderer` + `Nli` clients, the pipeline now
runs `adjudication-round-trip::check_round_trip` and records the
result in the audit trail with `pass_version = "v1.0"`. When the
gateway is omitted (or roles are missing), the v0.2 behaviour is
preserved — ADJ04 records as `Skipped`.

- New entry point `run_with_gateway(input, id, now, gateway)` —
  the v0.3 preferred surface.
- Existing `run(input, id, now)` is unchanged on the wire and now
  delegates to `run_with_gateway(_, _, _, None)`, so v0.2 callers
  recompile without source changes.
- ADJ04 is **advisory** at v0.3 — a failing round-trip records as
  `PassOutcome::Failed` with structured violations
  (`ClarificationKind::RoundTripDrift`) but does NOT block the
  engine. ADJ06 clarification (a future PR) will gate on drift.
- Round-trip is **not run** when ADJ02 or ADJ03 already failed —
  no point burning tokens to re-discover what the deterministic
  checkers already proved.
- Round-trip checker errors (missing role, validation exhaustion,
  transport failure) surface as `Failed` with the error string in
  `telemetry["check_error"]` rather than panicking.
- 5 new unit tests cover: high-score pass, drift-fails-but-engine-
  still-runs, no-gateway-records-Skipped, missing-role-records-
  Failed-with-detail, prior-fail-skips-ADJ04.

### Notes

ADJ05 still records as `Skipped`. It needs a second `Adversary`
client from a different `(vendor, model_family)` than the `Extractor`
to satisfy the LM00b independence requirement; that arrives once a
second model family is wired into the deployment.

This is also the first piece that lets the framework be driven by a
**local Ollama instance** end-to-end — a deployment with two locally
served models (e.g. `gemma:7b` for `Renderer`, a separate family
like `llama3.1:8b` for `Nli`) can now exercise ADJ04 without any
cloud LLM access.

## [0.2.0] - 2026-05-11

### Added

ADJ10 TSA worked-example integration test (the third E2E goal of
the session: Prolog ✅, ProbLog ✅, semantic source map ✅).

- New integration test crate `tests/integration_adj10_tsa.rs`. Builds
  a TSA-style IR document programmatically (two `Fact` nodes that
  tile a `"1 carry-on bag, matches."` 24-byte document plus one
  `Query` node) and feeds it through `pipeline::run`.
- 4 tests cover: the happy path (Resolved verdict with one engine
  answer, all four checker results recorded — ADJ02/ADJ03 Passed,
  ADJ04/ADJ05 Skipped); audit trail round-trips through
  `serde_json`; the trail mirrors the input document and every IR
  node id; the Blocked path (out-of-bounds span surfaces as a
  coverage violation, engine never runs, outcome is
  `ClarificationExhausted`).

### Notes

This is the **third of three E2E test goals** the user asked for at
the start of the session, alongside [#2752 Prolog](https://github.com/adhithyan15/coding-adventures/pull/2752)
and [#2756 ProbLog](https://github.com/adhithyan15/coding-adventures/pull/2756).
The fixture is programmatic at v0.2 because `adjudication-ir` does
not yet derive `serde::Deserialize`; a future version will load the
ADJ10 fixture from a JSON file under
`code/specs/fixtures/adj10-tsa/`.

A follow-up will pass an LLM `GatewayConfig` into `run` so ADJ04
(round-trip) and ADJ05 (adversarial) can flip from Skipped to
Passed/Failed using the merged `adjudication-round-trip` and
`adjudication-adversarial` checker crates.

## [0.1.0] - 2026-05-11

### Added

End-to-end orchestrator for the adjudication framework. Composes the
merged checker passes + the engine connector + the audit-trail
schema into a single function.

- `run(input, adjudication_id, now)` — one-call orchestrator. Runs
  ADJ02 (coverage) + ADJ03 (polarity-modality), records both into
  the audit trail, then runs the engine connector if (and only if)
  every gating check passed.
- `PipelineInput { document: PipelineDocument, ir_document: IRDocument }`
  — minimal input. `PipelineDocument` carries id / name / received_at /
  normalized_text / normalization metadata so callers don't have to
  import `adjudication-coverage` just to build an input.
- `PipelineOutput { verdict, audit_trail }`.
- `Verdict::Resolved { answers }` / `Verdict::Blocked { violation_count }` /
  `Verdict::EngineError(String)`.
- ADJ04 (round-trip) and ADJ05 (adversarial) are recorded as
  `PassOutcome::Skipped` with `pass_version = "not-yet-wired"` so
  the trail shape is complete and the slots are ready for those
  checkers to fill in.

7 unit tests cover: empty IR + empty text resolves cleanly with all
four checker results recorded; an out-of-bounds span surfaces as a
coverage violation, blocks the engine, and populates the audit
trail's `ClarificationExhausted` outcome; input document
normalization metadata is mirrored into `AuditTrail.documents`; schema
version stamp is recorded; IR nodes are mirrored into
`AuditTrail.ir_nodes`; every checker result carries a non-empty
`pass_version`; the full `AuditTrail` round-trips through serde_json.

### Notes

This is the **semantic source map running end-to-end** for the slices
of the framework that exist today. ADJ04 (round-trip) and ADJ05
(adversarial) need their own checker crates plus the
`find_contradicting_reading` primitive before they can slot in;
those land in follow-ups. The pipeline's public surface
(`PipelineInput`, `PipelineOutput`, `Verdict`) is designed to stay
stable across those additions — only the `Skipped` entries flip to
`Passed`/`Failed` as each checker comes online.

Extraction (LLM source-text → IR) lives a layer below: v0.2 will
wire `llm_primitives::decompose_text` in front so the input shrinks
to `(source_text: String, doc_id: DocumentId)`.

Reference: [ADJ00](../../../specs/ADJ00-adjudication-framework.md),
[ADJ07](../../../specs/ADJ07-audit-trail-schema.md), and the
[ADJ10 TSA worked example](../../../specs/ADJ10-tsa-worked-example.md).
