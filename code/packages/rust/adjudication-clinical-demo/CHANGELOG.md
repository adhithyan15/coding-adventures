# Changelog

All notable changes to this project will be documented in this file.

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
