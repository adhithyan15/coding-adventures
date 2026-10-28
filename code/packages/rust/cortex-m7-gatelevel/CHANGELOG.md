# Changelog

## Unreleased

- Move GE and NZCVQ on APSR `MSR` by mask, matching the functional Cortex-M7
  core after the Cortex-M APSR mask repair (#18343). Gate equality networks
  decode the `nzcvq` (bit 11) and `g` (bit 10) mask wires; NZCVQ moves only
  through a mux enabled by `execute AND nzcvq` and GE only through a mux
  enabled by `execute AND g`. `MSR APSR_nzcvq` now preserves GE instead of
  overwriting it.
- Execute `APSR_g` (mask `01`) and `APSR_nzcvqg` (mask `11`) in the companion,
  since the inherited non-DSP Cortex-M3 gate engine now refuses them, with the
  oracle's fault order (SP/LR/PC sources fault before the IT condition), IT
  skip and advance, and atomic commit. `classify` reports these forms as the
  inherited special-register class, as the oracle does.
- Share the settle/commit/trace tail between the DSP and masked-`MSR` paths.
- Add an oracle differential over all three accepted masks and every APSR
  `SYSm` alias with distinguishable NZCVQ and GE images, plus fault, IT, fetch,
  and classification parity cases and hostile-backend dependency proofs for
  the new helpers. Line coverage was not re-measured for this change.

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
