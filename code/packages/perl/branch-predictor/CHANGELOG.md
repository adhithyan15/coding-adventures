# Changelog

## [0.01] - 2026-03-31

### Added

- `CodingAdventures::BranchPredictor::Stats` — immutable accuracy tracker with
  `record()`, `accuracy()`, `misprediction_rate()`, and `reset()`.
- `CodingAdventures::BranchPredictor::Prediction` — immutable result type holding
  `predicted_taken`, `confidence`, and optional `address`.
- `CodingAdventures::BranchPredictor::Static` — three static predictors in one file:
  - `AlwaysTaken` — predicts every branch taken (~60% accuracy).
  - `AlwaysNotTaken` — predicts every branch not taken (~40% accuracy).
  - `BTFNT` — Backward Taken, Forward Not Taken (~70% accuracy); caches last-seen
    target addresses to determine branch direction.
- `CodingAdventures::BranchPredictor::OneBit` — 1-bit dynamic predictor indexed by
  `pc % table_size`; predicts whatever happened last time (~80% accuracy).
- `CodingAdventures::BranchPredictor::TwoBit` — 2-bit saturating counter predictor
  with SNT/WNT/WT/ST states and hysteresis (~90% accuracy); matches the design used
  in the Intel Pentium and Alpha 21064.
- `CodingAdventures::BranchPredictor::BTB` — direct-mapped Branch Target Buffer;
  tracks branch targets by `pc % size`; reports hit rate.
- `CodingAdventures::BranchPredictor` — convenience loader that `use`s all sub-modules.
- Comprehensive Test2::V0 test suite in `t/test_branch_predictor.t` covering cold
  starts, learning, aliasing, loop patterns, 2-bit hysteresis, BTB eviction/aliasing,
  and integration benchmarks comparing all predictors on a synthetic loop workload.
