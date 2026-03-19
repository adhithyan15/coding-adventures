# Changelog

## 0.1.0 — 2026-03-18

### Added
- `Prediction` value object (taken, confidence, target)
- `PredictionStats` accuracy tracker (predictions, correct, incorrect, accuracy, misprediction_rate)
- `AlwaysTakenPredictor` — static predictor, always predicts taken
- `AlwaysNotTakenPredictor` — static predictor, always predicts not taken
- `BackwardTakenForwardNotTaken` — static direction-based heuristic (BTFNT)
- `OneBitPredictor` — 1-bit dynamic predictor with configurable table size
- `TwoBitPredictor` — 2-bit saturating counter with configurable initial state
- `TwoBitState` module with state transition logic
- `BTBEntry` and `BranchTargetBuffer` — direct-mapped target address cache
- Full test suite with minitest
