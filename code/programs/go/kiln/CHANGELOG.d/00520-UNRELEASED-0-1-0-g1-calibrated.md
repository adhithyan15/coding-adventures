## [0.1.0] - UNRELEASED

### Changed (gate G1: host-normalised capability gate plus absolute floor)

- GitHub's runner fleet varies about 1.75x in CPU speed (the synthetic slice read
  12.3 to 21.6 MB/s over ten legs), so the raw 14 MB/s floor gave false failures
  on slow hosts. G1 (KILN04 4.9) now has two parts: an absolute safety floor of
  **8 MB/s raw** on every gated line (synthetic slice and real traces of 32 to
  128 KB), and a **host-normalised capability gate** of 14 MB/s on the
  synthetic slice.
- `internal/trace/measure.go` gains `refKernel` (FROZEN: a byte-class table, an
  8x8 transition table, a data-dependent accept branch, FNV-1a mixing into a
  package-level sink), `RefBytes`, and `BestPass1Calibrated`, which times one
  kernel sample (8 passes) and one grammar pass per iteration of the usual
  best-of loop and returns both minima. `TestMeasureIsNotUsedByProductionCode`
  covers the new names.
- The `synthetic-slice` PROBE line gains `ref_mb_s`, `norm_mb_s` (`best_mb_s *
  refNominalMBs / ref_mb_s`) and `calibrated=0|1`, before `gated=` and before
  any refusal field. `refNominalMBs` is a PLACEHOLDER and
  `refNominalCalibrated = false`: the normalised check is report-only until a
  follow-up PR records the value measured on CI (procedure in KILN04 4.9).
  Non-finite or non-positive values fail everywhere.
- `ci/assert_sandbox_tests.py`: `GATE_FLOOR_MB_S` is 8 for every gated line; the
  synthetic line must carry a finite, positive `ref_mb_s`; `NORM_FLOOR_MB_S = 14`
  is enforced on `norm_mb_s` only when the line says `calibrated=1`. New pytest
  cases: a missing and a non-finite ref, norm below the floor when calibrated,
  ignored when not, and a raw rate between 8 and 14 now passing.
- The kiln-sandbox job also prints the CPU model and `nproc`.
- KILN04 4.9 rewritten for the two-part G1 (fleet table, calibration procedure)
  and gains an explicit soak rule: a run counts toward the 200 if its
  cross-checked tests ran under `KILN_TRACE_DIFF=1` with no mismatch or
  divergence whatever G1 did; a job dead before its tests counts neither way;
  only a mismatch or divergence resets the count.
