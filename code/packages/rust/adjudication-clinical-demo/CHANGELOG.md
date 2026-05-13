# Changelog

All notable changes to this project will be documented in this file.

## [0.3.0] - 2026-05-13 — v0.12 parity (rulebook injection + priming)

### Added

ADJ19 prerequisite: bring clinical-demo up to parity with
[adjudication-tsa-demo v0.12](../adjudication-tsa-demo/CHANGELOG.md)
so the cross-domain bench harness can treat both domains uniformly.

- `ArmAMode { SingleTurn, Priming }` enum.
- `DemoConfig` gains:
  - `rulebook_text: Option<String>` — optional rulebook injected
    into Arm A's system prompt.
  - `max_answer_tokens: usize` (default 2048) — output-token cap.
  - `arm_a_mode: ArmAMode` (default `SingleTurn`).
- `fixture_clinical_rulebook()` — hand-authored canonical
  rulebook covering ACS / meningitis / asthma / URI / dehydration
  / denied-allergy reasoning. Cites AHA, IDSA, GINA, WHO, and ACR
  reference material per ADJ19 §clinical-domain.
- New public prompt builders mirroring tsa-demo:
  `build_raw_system_prompt`, `build_priming_system_prompt`,
  `build_priming_turn1_user_prompt`,
  `build_priming_turn2_user_prompt`.
- Two-turn priming dispatch in `run_raw_arm`. Turn 1 hands the
  model the rulebook with an ACK-only instruction; turn 2 sends
  the assessment and asks for a verdict-first answer. Falls back
  to single-turn when no rulebook is configured.

### Changed

- `run_raw_arm` now dispatches on `cfg.arm_a_mode`.
- Both single-turn variants of the system prompt require the
  verdict line as the FIRST line of the response. Same discipline
  as tsa-demo v0.12: the verdict survives reasoning truncation.

### Tests

8 new tests added (15 lib total, all passing):
- default config: SingleTurn + 2048 token cap
- fixture rulebook covers the canonical-fixture symptoms and
  denied-allergy reasoning
- verdict-first phrasing in both raw and priming system prompts
- priming protocol: Turn 1 / Turn 2 / ACK / no analysis until
  turn 2
- priming turn 1 embeds rulebook + demands ACK
- priming turn 2 embeds assessment + restates verdict-first
- ArmAMode round-trips through Debug/Clone/Eq
- DemoConfig field plumbing

### Verdict set unchanged

v0.3 preserves clinical-demo's existing 2-value verdict set
(`SAFE_TO_DISCHARGE` / `KEEP_FOR_OBSERVATION`). ADJ19 proposes
moving to a 3-value set
(`URGENT-EVAL` / `OUTPATIENT` / `PROCEED-WITH-MONITORING`) once
the cross-domain bench actually needs the finer-grained verdict.
Keeping the change minimal here.

### Compatibility

- `DemoConfig` gained three new public fields. Soft break for
  pattern destructuring; no in-tree caller does that.
- `run_raw_arm(cfg)` signature unchanged.
- Default behaviour preserved (modulo the verdict line moving to
  the top of the response).
- The main binary doesn't yet wire `ADJ_DEMO_RULEBOOK_MODE` or
  `ADJ_DEMO_ARM_A_MODE` through `config_from_env`. That's a
  follow-up alongside the harness generalization PR.

## [0.2.0] - 2026-05-12

### Added

`PipelineArmReport::cache_stats` parity with the TSA demo. Every
LLM client is wrapped in a `CachingClient`, each role's stats
handle is collected, and the side-by-side report prints
`cache: N hits / M misses (X% hit rate), K entries` when any cache
activity happened.

## [0.1.0] - 2026-05-12

### Added

Second-domain A/B demo: clinical-note triage. Mirrors
`adjudication-tsa-demo`'s shape with a different source text and a
different hand-built IR fixture. Proves the framework's IR grammar +
checkers generalize across domains.

- `DemoConfig` with `model`, `adversary_model`, `endpoint`,
  `source_text`, `cache_dir`, `timeout`.
- `run_raw_arm(cfg)` — triage-officer prompt; raw model verdict.
- `run_pipeline_arm(cfg)` — hand-built clinical IR + full
  ADJ02+ADJ03+ADJ04+ADJ05+engine pipeline.
- `clinical_ir_document(source)` — canonical 4-node IR
  (3 Facts tiling the document + 1 Query). The allergy fact is
  intentionally `Polarity::Denied` to capture the "no known drug
  allergy" phrasing — a real test of ADJ03's polarity tracking
  that small models often miss.
- 7 offline unit tests cover: canonical IR shape, denied-allergy
  polarity, span tiling (every byte of the 64-byte canonical
  source is covered), non-canonical fallback, empty-source fallback,
  default config sanity, and the outcome formatter.
