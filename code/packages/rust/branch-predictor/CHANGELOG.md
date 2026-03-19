# Changelog

## 0.1.0 (2026-03-18)

### Added
- `Prediction` struct with taken, confidence, and optional target address
- `BranchPredictor` trait defining the predict/update/stats/reset interface
- `AlwaysTakenPredictor` -- static predictor that always guesses taken
- `AlwaysNotTakenPredictor` -- static predictor that always guesses not-taken
- `BackwardTakenForwardNotTaken` -- direction-based heuristic (MIPS R4000 style)
- `OneBitPredictor` -- dynamic predictor with one flip-flop per branch
- `TwoBitPredictor` -- saturating counter predictor with configurable initial state
- `TwoBitState` enum with taken_outcome/not_taken_outcome transitions and predicts_taken threshold
- `BranchTargetBuffer` -- direct-mapped cache for branch target addresses with hit/miss tracking
- `BTBEntry` struct storing valid bit, tag, target, and branch type
- `PredictionStats` with accuracy and misprediction rate computation
- Comprehensive test suite covering cold starts, learning, aliasing, loop patterns, hysteresis, BTB eviction, and reset behavior
- Knuth-style doc comments explaining branch prediction concepts, pipeline implications, and hardware tradeoffs
