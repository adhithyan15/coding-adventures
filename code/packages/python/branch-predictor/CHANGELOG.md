# Changelog

All notable changes to the `branch-predictor` package will be documented here.

## [0.2.0] - 2026-03-20

### Added
- `TWO_BIT_DFA` — formal DFA definition of the 2-bit saturating counter state machine, using the `state-machine` package's `DFA` class
- `ONE_BIT_DFA` — formal DFA definition of the 1-bit predictor state machine
- Both DFAs exported from `branch_predictor.__init__`
- `coding-adventures-state-machine` added as a package dependency
- `test_dfa_equivalence.py` — 20 new tests verifying DFA definitions match predictor behavior, including transition equivalence, acceptance semantics, mapping bijectivity, and Graphviz visualization output

### Changed
- `TwoBitState.taken_outcome()` and `not_taken_outcome()` now delegate to `TWO_BIT_DFA.transitions` instead of inline arithmetic, making the DFA the single source of truth
- `OneBitPredictor.update()` now uses `ONE_BIT_DFA.transitions` for state transitions instead of directly setting the bit
- BUILD file updated to install `state-machine` dependency before `branch-predictor`

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
