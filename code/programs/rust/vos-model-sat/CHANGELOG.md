# Changelog

All notable changes to this package are documented here.

## [0.1.0] - Unreleased

### Added (VOS-040a, VOS31 sections 5 and 6)

- **The encoder** (`src/model.rs`): `SchedModel`, one method per model function of
  `warrant/contract.part`, the honest transcription as default methods, so a mutant that overrides
  one method is seen by every caller exactly as a spliced Warrant model is. `Encoder { n }` is
  N-parametric for N = 1 to 64 (`Encoder::new` bounds it). Types in `src/types.rs`: `Ready`
  (width N), `PickEnc` (8-bit id at every width; settles VOS31 section 16 item 2), `UpdEnc`.
- **The ten lemmas** (`src/lemmas.rs`): `lemma_circuit` / `lemma` build each claim as one edge
  `holds`, plus `premise` for the six implication lemmas (for VOS-040b's premise witness).
- **The ten mutants** (`src/mutants.rs`): `Mutant`, generalised to any N; plus four deliberately
  wrong encoders (`Fault`: `top` off by one, `<=` for `<` in `pre_insert`, `mem` ignoring thread 0,
  a `put` that cannot clear) for the negative controls.
- **The three cross-checks** (`src/crosscheck.rs`, `src/oracle.rs`, `src/probes.rs`,
  `src/reference.rs`): (a) the lemma check against the Warrant evaluator at N = 8 on the base and
  on each mutant document, comparing the failing **sets** and the counts of `lemmas.table`; (b) the
  function check, twenty probes read through the evaluator's ground-call interface; (c) the width
  check against a plain Rust reference at N = 1 to 10, and the reference against the evaluator at
  N = 8. Settles VOS31 section 16 item 3 (the ground-call interface is usable).
- **Labels** (`src/labels.rs`): the exact strings of VOS31 section 11 and the forbidden words.
- `vos-model-sat cross-check <contract-crate-dir>`: runs the three checks (exit 0, 1 or 2).
- Tests: 48; every check has a negative control (`tests/cross_checks.rs`), the oracle's refusals and
  the CLI are in `tests/oracle_and_cli.rs`, the encoder alone in `tests/encoder.rs`.

### Changed from the spec (recorded in VOS31 by a separate commit)

- The two relations `safe_dispatch` and `policy_dispatch` are probed on every `Ready`, every
  `pick_of(k)` and four candidates for the second `Ready`, not on their whole 16.8M-case domain.
- `vos-link` and `vos-manifest` are dependencies (the evaluation document and loader), and
  `cdcl-sat`, `drat-check` and `sha256` are not yet (no solving in this PR).

### Fixed (VOS-040a security review, LOW items)

- A refused or short sweep is now a disagreement. Check (c) and check (b) discarded the `TooWide`
  error of `sweep` (`let _ =`), so a failed sweep added zero comparisons and hid the shortfall.
  A new `counted_sweep` records a refusal, and any comparison count other than `2^inputs` of the
  subject's declared domain, as a disagreement; `encoder_failed` now returns a `Result`. Tests: an
  N = 64 encoder (every wide sweep refused) and an encoder with an extra input (double count).
- A mutant must break its lemma: `check_mutant_breaks` requires a nonzero failing count on both the
  evaluator and the encoder for every lemma's mutant (`closure_at_m` included: its mutant fails on
  every case). Test: the honest model used as a mutant fails this check directly, not only through
  the table.
- `label_collapsed` takes a closed `CollapseReason` enum (`constant-fold`, `propagation-only`,
  VOS31 7.5) instead of free text.
