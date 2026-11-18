## [0.1.0] - UNRELEASED

### Changed (gate G1: host-normalised capability gate plus absolute floor)

- GitHub's runner fleet varies about 1.75x in CPU speed (the synthetic slice read
  12.3 to 21.6 MB/s over ten legs), so the raw 14 MB/s floor gave false failures
  on slow hosts. G1 (KILN04 4.9) now has two parts: an absolute safety floor of
  **8 MB/s raw** on every gated line (synthetic slice and real traces of 32 to
  128 KB), and a **host-normalised capability gate** of 14 MB/s on the
  synthetic slice.
- `internal/trace/measure.go` gains `refKernel`, a FROZEN parser-shaped
  reference workload (line split, token slices that grow, a `switch` on token
  class, string copies, a map lookup, a ring of live lines, one hash per token;
  it allocates, about 28 bytes per input byte), `RefBytes`, and
  `BestPass1Calibrated`, which times one reference sample (2 passes) and one
  grammar pass per iteration of the usual best-of loop and returns both minima.
  A byte-at-a-time serial-multiply candidate was rejected during calibration (a
  clock meter, cross-leg spread 1.39). `TestMeasureIsNotUsedByProductionCode`
  covers the new names. The `bestPass1` collector-off comment now states the
  real garbage bound.
- The `synthetic-slice` PROBE line gains `ref_mb_s`, `norm_mb_s`
  (`best_mb_s * 72.0 / ref_mb_s`) and `calibrated=1`, before `gated=` and before
  any refusal field. Calibrated over 8 CI legs: median k = 71.85, so
  `refNominalMBs = 72.0`; the synthetic test now FAILS when norm is below 14 (the
  worst observed leg is 17.6). Non-finite or non-positive values fail everywhere.
- `ci/assert_sandbox_tests.py`: `GATE_FLOOR_MB_S` is 8 for every gated line; the
  synthetic line must carry a finite, positive `ref_mb_s`, `calibrated=1`
  (`REQUIRE_CALIBRATED = True`: Go cannot switch the check off by logging
  `calibrated=0`) and a `norm_mb_s` of at least `NORM_FLOOR_MB_S = 14`. New pytest
  cases cover each of these, plus a raw rate between 8 and 14 now passing.
- The kiln-sandbox job also prints the CPU model and `nproc`.
- KILN04 4.9 rewritten for the two-part G1 (fleet table, the kernel, the 8-leg
  calibration table, validity rule, recalibration triggers, margins) and gains an
  explicit soak rule: a run counts toward the 200 if its cross-checked tests
  ran under `KILN_TRACE_DIFF=1` with no mismatch or divergence whatever G1 did;
  a job dead before its tests counts neither way; only a mismatch or divergence
  resets the count.
