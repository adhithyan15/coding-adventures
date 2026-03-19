# Changelog

All notable changes to the `branch-predictor` package will be documented here.

## [0.1.0] - 2026-03-18

### Added
- `PredictionStats` dataclass for tracking prediction accuracy metrics
- `Prediction` frozen dataclass for prediction results (taken, confidence, target)
- `BranchPredictor` protocol defining the pluggable predictor interface
- `AlwaysTakenPredictor` — static predictor, always predicts taken (~60% accuracy)
- `AlwaysNotTakenPredictor` — static predictor, always predicts not-taken
- `BackwardTakenForwardNotTaken` — direction-based heuristic (MIPS/SPARC style)
- `OneBitPredictor` — 1-bit dynamic predictor with configurable table size
- `TwoBitPredictor` — 2-bit saturating counter with configurable table size and initial state
- `TwoBitState` enum for the 4 states of a 2-bit counter, with transition methods
- `BranchTargetBuffer` — direct-mapped BTB with tag-based hit/miss detection
- `BTBEntry` dataclass for BTB entries with branch type tracking
- Comprehensive test suite (90%+ coverage) covering:
  - Static predictor accuracy on taken/not-taken/mixed sequences
  - One-bit double-misprediction problem on loops
  - Two-bit state transitions, loop behavior, comparison vs one-bit
  - BTB lookup/update, eviction, aliasing, branch type tracking
  - Stats edge cases (zero predictions, reset behavior)
