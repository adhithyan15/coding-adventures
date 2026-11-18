## [0.1.0] - UNRELEASED

### Changed (gate G1: host-normalised capability gate plus absolute floor)

- GitHub's runner fleet varies about 1.75x in CPU speed (the synthetic slice read
  12.3 to 21.6 MB/s over ten legs), so the raw 14 MB/s floor gave false failures
  on slow hosts. G1 (KILN04 4.9) now has two parts: an absolute safety floor of
  **8 MB/s raw** on every gated line (synthetic slice and real traces of 32 to
  128 KB), and a **host-normalised capability gate** of 14 MB/s on the
  synthetic slice.
- `internal/trace/measure.go` gains two CANDIDATE reference kernels, both FROZEN:
  `refKernelA` (a byte-at-a-time state machine with a serial FNV multiply chain:
  a clock-frequency meter) and `refKernelB` (parser-shaped: line split, token
  slices that grow, a `switch` on token class, string copies, a map lookup, a
  ring of live lines, one hash per token). Calibration keeps whichever tracks
  the reader better across hosts and deletes the other. `BestPass1Calibrated`
  returns a `Calibrated{Best, RefA, RefB, Refusal}`: each iteration times one
  sample of A, one of B and one grammar pass, keeping the minimum of each; the
  refusal-check pass is also the grammar warm-up. `RefBytes` gives each sample's
  byte count. `TestMeasureIsNotUsedByProductionCode` covers the new names.
- The `synthetic-slice` PROBE line gains `ref_mb_s` (A), `refb_mb_s` (B), `k_a`,
  `k_b` (`ref * 19.5 / best_mb_s`), the per-round `k_a_rounds`/`k_b_rounds` (the
  test makes three calibrated rounds), and `calibrated=0|1`, all before `gated=`
  and before any refusal field. `norm_mb_s` is logged only when calibrated.
  `refNominalMBs` is a PLACEHOLDER and `refNominalCalibrated = false`: the
  normalised check never fails until a follow-up PR records the value measured
  on CI (procedure and kernel selection rule in KILN04 4.9).
  Non-finite or non-positive values fail everywhere.
- `ci/assert_sandbox_tests.py`: `GATE_FLOOR_MB_S` is 8 for every gated line; the
  synthetic line must carry finite, positive `ref_mb_s` and `refb_mb_s` and a
  `calibrated` of exactly `0` or `1`; `NORM_FLOOR_MB_S = 14` is enforced on
  `norm_mb_s` only when the line says `calibrated=1`; the new module constant
  `REQUIRE_CALIBRATED = False` makes `calibrated=0` fail once the calibrating
  commit sets it, so the Go side alone cannot switch the check off. New pytest
  cases cover each of these, plus a raw rate between 8 and 14 now passing.
- The kiln-sandbox job also prints the CPU model and `nproc`.
- KILN04 4.9 rewritten for the two-part G1 (fleet table, both kernels, selection
  rule, calibration procedure, margins, the hand-check note) and gains an
  explicit soak rule: a run counts toward the 200 if its cross-checked tests
  ran under `KILN_TRACE_DIFF=1` with no mismatch or divergence whatever G1 did;
  a job dead before its tests counts neither way; only a mismatch or divergence
  resets the count.
