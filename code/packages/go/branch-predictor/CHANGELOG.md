# Changelog

## 0.1.0 — 2026-03-18

### Added
- `Prediction` struct (Taken, Confidence, Target)
- `PredictionStats` accuracy tracker
- `BranchPredictor` interface (Predict, Update, Stats, Reset)
- `AlwaysTakenPredictor` — static predictor, always predicts taken
- `AlwaysNotTakenPredictor` — static predictor, always predicts not taken
- `BTFNTPredictor` — backward-taken/forward-not-taken direction heuristic
- `OneBitPredictor` — 1-bit dynamic predictor with configurable table size
- `TwoBitPredictor` — 2-bit saturating counter with configurable initial state
- `TwoBitState` type with state transition methods
- `BTBEntry` and `BranchTargetBuffer` — direct-mapped target address cache
- Full test suite with interface compliance checks
