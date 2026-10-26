# Changelog

## 0.1.0 — 2026-10-26

- Add the DFF-backed Cortex-M7 architectural gate companion with exactly
  524,956 production-derived persistent bits.
- Reuse the non-committing Cortex-M3 gate composition boundary and add
  independently decoded QADD, QSUB, SADD8, and UADD8 networks.
- Add sticky-Q, per-lane GE, APSR transport, IT control, exact traces, typed
  atomic lifecycle, deterministic allocation failure, and whole-run rollback.
- Make the inherited candidate commit boundary fallible and revalidate after
  any public post-settle mutation before the first DFF clock.
- Add exhaustive operand-field and byte-lane truth tables, functional-oracle
  differentials, and hostile compiled-backend dependency proofs.
- Reach 93.94% package-source line coverage (697 of 742 lines) after the final
  candidate-boundary repair.
