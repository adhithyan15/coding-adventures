# Changelog — coding-adventures-branch-predictor (Lua)

## 0.1.0 — 2026-03-31

Initial release. Lua port of the Elixir branch_predictor package.

### Added

- `Stats` — prediction accuracy tracking (predictions, correct, accuracy%)
- `Prediction` — prediction result type (predicted_taken, confidence, address)
- `AlwaysTaken` — always predicts taken (~60-70% accurate on real workloads)
- `AlwaysNotTaken` — always predicts not-taken (baseline predictor)
- `BTFNT` — Backward Taken, Forward Not Taken (direction-based heuristic)
- `OneBit` — 1-bit predictor with table indexed by PC modulo table_size
- `TwoBit` — 2-bit saturating counter (SNT/WNT/WT/ST states) with hysteresis
- `BTB` — Branch Target Buffer: direct-mapped cache of branch targets
- Comprehensive test suite with 95%+ coverage
