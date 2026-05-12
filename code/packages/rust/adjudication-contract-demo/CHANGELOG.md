# Changelog

All notable changes to this project will be documented in this file.

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
