# Changelog — adj57 byte-provenance pipeline

## [0.1.0] — 2026-06-04

### Added

- **ADJ57 — the four-layer byte-provenance pipeline**, proven end-to-end on a fresh
  case (pheochromocytoma, PMC11521393).
  - **L0 — CAS** (`pipeline/cas.py`): content-addressed source store. Interns each
    source by `sha256(content)` (free dedup), records byte-anchored citation spans,
    and rejects any quote not literally present. The reusable indexed-source corpus.
  - **L1 — coverage** (`pipeline/coverage.py`): the case→IR partition must
    reconstruct the input byte-for-byte. Demonstrated the enforce→correct loop —
    caught a collapsed `\n\n` at byte 1296, localized to segment 42, corrected,
    re-verified clean (22 typed facts + 24 reasoned discards = 100% of 1499 bytes).
  - **L1–L3 workflow** (`pipeline/slice.workflow.js`): lossless typed partition →
    fact-driven link derivation → recursive-to-root grounding spider.
  - **driver** (`pipeline/assemble.py`): ties the layers together and writes the
    byte-addressable `rulebook.json`.
  - **Proof:** `blood_pressure(160/100) → pheochromocytoma  LR = 0.762 [grounded,
    root]` traced to a JAMA Rational Clinical Examination span byte-anchored in the
    CAS (`content[165:297] == quote`). `weight_loss` returned `direction_only` (no
    root data → no number).
  - Spec: [ADJ57](../../ADJ57-byte-provenance-pipeline.md).
