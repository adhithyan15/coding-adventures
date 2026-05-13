# Changelog

All notable changes to this project will be documented in this file.

## [0.3.0] - 2026-05-13 — v0.12 parity (rulebook injection + priming)

### Added

ADJ19 prerequisite: bring contract-demo up to parity with
[adjudication-tsa-demo v0.12](../adjudication-tsa-demo/CHANGELOG.md)
and [adjudication-clinical-demo v0.3](../adjudication-clinical-demo/CHANGELOG.md)
so the cross-domain bench harness can treat all three domains
uniformly.

- `ArmAMode { SingleTurn, Priming }` enum.
- `DemoConfig` gains:
  - `rulebook_text: Option<String>`
  - `max_answer_tokens: usize` (default 2048)
  - `arm_a_mode: ArmAMode` (default SingleTurn)
- `fixture_contract_rulebook()` — hand-authored canonical
  rulebook covering force-majeure / stock-exception / on-time
  delivery / disputed-case reasoning. Cites Restatement (Second)
  of Contracts §261 per ADJ19 §contract-domain.
- Public prompt builders: `build_raw_system_prompt`,
  `build_priming_system_prompt`,
  `build_priming_turn1_user_prompt`,
  `build_priming_turn2_user_prompt`.
- Two-turn priming dispatch in `run_raw_arm`.

### Changed

- `run_raw_arm` dispatches on `cfg.arm_a_mode`.
- Verdict-first prompt format in both single-turn and priming
  variants (`VERDICT: OBLIGATION_HOLDS` / `OBLIGATION_EXCUSED`).

### Tests

8 new tests (14 lib total, all passing). Same shape as
clinical-demo v0.3.

### Verdict set unchanged

v0.3 preserves contract-demo's existing 2-value set
(`OBLIGATION_HOLDS` / `OBLIGATION_EXCUSED`). ADJ19's proposed
3-value set (`IN-BREACH` / `NOT-IN-BREACH` / `DISPUTED`) is
queued for when the cross-domain bench actually needs it.

### Compatibility

- DemoConfig gained three new public fields. Soft break for
  pattern destructuring; no in-tree caller does that.
- `run_raw_arm(cfg)` signature unchanged.
- Default behaviour preserved (modulo verdict line position).

### Follow-ups

Same as clinical-demo v0.3: main.rs env-var wiring + harness
generalization, queued together.

## [0.2.0] - 2026-05-12

### Added

`PipelineArmReport::cache_stats` parity with the TSA demo. Every
LLM client is wrapped in a `CachingClient`, each role's stats
handle is collected, and the side-by-side report prints
`cache: N hits / M misses (X% hit rate), K entries` when any cache
activity happened.

## [0.1.0] - 2026-05-12

### Added

Third-domain A/B demo: contract-clause review. After TSA and
clinical, contracts give us a domain whose IR shape exercises the
**rule + exception** machinery (`NodeKind::Rule` with `Conditional`
modality + `NodeKind::Exception` linked to the rule via `part_of`)
in a way the earlier demos did not.

- `DemoConfig`, `run_raw_arm`, `run_pipeline_arm`,
  `contract_ir_document`, `format_side_by_side` — same shape as
  the TSA + clinical demos.
- Canonical 105-byte fixture `"If the buyer pays within 30 days,
  the seller delivers the goods, unless the goods are out of stock."`
  decomposed as a Conditional Rule + Affirmed Exception + Query
  (rule spans bytes 0..p1 where p1 is the byte index of " unless",
  exception spans p1..len, so they tile).
- Binary at `cargo run -p adjudication-contract-demo`.
- 7 offline unit tests cover canonical IR shape, Conditional
  modality, part_of link from exception to rule, span tiling,
  fallback behavior, and config defaults.
