# Changelog

All notable changes to the branch-predictor package will be documented in this file.

## [0.2.0] - 2026-03-20

### Added

- Integrated state-machine DFA into the branch predictor package
- `TWO_BIT_DFA`: formal DFA definition of the 2-bit saturating counter (4 states, 2 inputs, 8 transitions)
- `ONE_BIT_DFA`: formal DFA definition of the 1-bit predictor (2 states, 2 inputs, 4 transitions)
- `transitionViaDFA()`: compute 2-bit state transitions using the DFA engine
- `oneBitTransitionViaDFA()`: compute 1-bit state transitions using the DFA engine
- `DFA_STATE_TO_ENUM` / `ENUM_TO_DFA_STATE`: bidirectional mappings between DFA state names and TwoBitState enum values
- `ONE_BIT_DFA_STATE_TO_BOOL` / `ONE_BIT_BOOL_TO_DFA_STATE`: bidirectional mappings for 1-bit predictor
- `dfa-equivalence.test.ts`: 47 tests proving equivalence between manual implementations and DFA definitions
- Added `@coding-adventures/state-machine` as a dependency

### Changed

- Updated BUILD file to install state-machine dependency before building
- Updated index.ts to export all new DFA-related symbols

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
