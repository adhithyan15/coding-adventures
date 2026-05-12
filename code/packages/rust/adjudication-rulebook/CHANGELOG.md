# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-05-12 — Adversarial multi-model elicitation

### Added

- `acquire_rulebook_adversarial(req, model_gateways)` — orchestrate
  rulebook elicitation across multiple LLMs. The recursive use of
  the framework, made **adversarial**: a rule cited at answer time
  comes from one of several independent model elicitations, so
  reviewers can spot rules that only appear in a single model's
  training and treat them as lower-trust than rules multiple models
  agree on.
- `AcquireRulebookAdversarialRequest { document_id_prefix, domain,
  scope, as_of, language_hint }` — input shape.
- `AdversarialRulebook { per_model, merged_source_text,
  successful_count, failed_count }` — output. `merged_source_text`
  is provenance-tagged (one section per model with `=== RULEBOOK
  FROM <model> ===` header), suitable for direct injection into an
  answer-time system prompt. Reviewers reading the eventual
  audit trail trace cited rules back to their model.
- `PerModelOutcome { Acquired { model_label, rulebook } | Failed {
  model_label, error_summary } }` plus `model_label()` /
  `is_acquired()` helpers. A single failing model does NOT fail
  the whole call — the remaining successful rulebooks are still
  returned.
- Document ids generated as `{prefix}-{sanitised_label}` so model
  names containing `:` (like `qwen2.5:1.5b`) become filesystem-safe
  ids.

### Tests

6 new unit tests, total 10 pass (was 4).

### Not yet shipping

- Semantic-equivalence comparison across rulebooks (`Confirms` /
  `ConflictsWith` edges per ADJ09).
- Parallel elicitation (v0.2 runs the per-model `acquire_rulebook`
  calls serially).

## [0.1.0] - 2026-05-12 — ADJ14 Stage 0 implementation

### Added

First implementation of [ADJ14](../../specs/ADJ14-rule-elicitation.md).
Composes the new `elicit_rules` primitive with `decompose_text` and
`adjudication_ir::validate` to produce a typed `Rulebook` from a
domain hint.

- `Rulebook { document_id, domain, scope, ir_document, source_text,
  trust, elicit_prompt_version, decompose_prompt_version,
  model_identity, as_of, audit_trail, validation_passed,
  validation_error }` — the audited container shape from
  ADJ14 §"Stage 0 — Orchestrator".
- `RulebookTrust { Tentative, Reviewed, Authoritative }` — provenance
  tier from ADJ14 §"Trust Tiers". Every `acquire_rulebook` output
  starts at `Tentative`.
- `AcquireRulebookRequest { document_id, domain, scope, as_of,
  language_hint }` — input shape.
- `AcquireRulebookError { ElicitFailed, DecomposeFailed }` —
  error shape distinguishing the two primitive call sites.
- `acquire_rulebook(req, gateway)` — the headline orchestrator. Flow:
    1. `elicit_rules` against `Role::RuleExtractor` (fallback to
       `Role::Extractor`).
    2. `decompose_text` against `Role::Extractor` with `domain_hint
       = "<domain>/rulebook"`.
    3. `adjudication_ir::validate` on the resulting JSON, parsed via
       a hand-rolled JSON → `IRDocument` decoder (the IR crate
       doesn't ship serde derives).
    4. Package everything plus the full audit trail into a
       `Rulebook { trust: Tentative }`.
- Internal `ir_from_json` decoder covers every v3 node kind, every
  closed-set `EdgeRelation`, and the `DomainSpecific(name)` escape
  hatch. Falls through to safe defaults on missing fields so a
  partially-populated response still parses for validation.

### Tests

5 unit tests:

- Happy path produces a `Tentative` rulebook with a 2-record audit
  trail and the right prompt-version constants.
- Empty nodes+edges yields a vacuously-valid IR (current behaviour;
  follow-up may add a non-empty-rulebook check).
- `RulebookTrust::as_str` is stable.
- `elicit_rules` failure propagates as `AcquireRulebookError::ElicitFailed`.

### What v0.1 does NOT ship yet

- ADJ02–05 checker passes against the rulebook IR. Today the crate
  runs only `adjudication_ir::validate` (the well-formedness gate).
  Full checker integration + ADJ06 retry loop is a follow-up PR.
- Disk persistence / caching of acquired rulebooks. Until that
  lands, every `acquire_rulebook` call re-hits the LLM (the
  underlying `llm-cache` still memoises individual calls, so
  repeated invocations are fast).
- Tentative → Reviewed promotion CLI per ADJ09's review workflow.
