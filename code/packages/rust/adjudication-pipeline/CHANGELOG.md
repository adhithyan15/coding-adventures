# Changelog

All notable changes to this project will be documented in this file.

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
