# Changelog

All notable changes to the branch-predictor package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- Initial TypeScript port from the Python implementation
- `PredictionStats` class for tracking prediction accuracy metrics
- `Prediction` interface and `BranchPredictor` interface (Strategy pattern)
- Static predictors: `AlwaysTakenPredictor`, `AlwaysNotTakenPredictor`, `BackwardTakenForwardNotTaken`
- Dynamic predictors: `OneBitPredictor` (1-bit flip-flop), `TwoBitPredictor` (2-bit saturating counter)
- `TwoBitState` enum with `takenOutcome`, `notTakenOutcome`, and `predictsTaken` helper functions
- `BranchTargetBuffer` with direct-mapped cache, hit/miss tracking, and branch type metadata
- `BTBEntry` interface and `createBTBEntry` factory function
- Comprehensive test suite covering all predictors, state machines, aliasing, loop patterns, and reset behavior
- Full Knuth-style literate programming comments explaining CPU pipeline context, hardware implementation details, and historical usage
