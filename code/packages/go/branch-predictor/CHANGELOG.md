# Changelog

## 0.2.0 — 2026-03-20

### Added
- `NewTwoBitDFA()` — constructs a state-machine DFA equivalent to the two-bit saturating counter
- `NewOneBitDFA()` — constructs a state-machine DFA equivalent to the one-bit predictor
- `TwoBitStateName()` / `TwoBitStateFromName()` — bidirectional mapping between `TwoBitState` integers and DFA state name strings
- `twoBitDFAStateNames` / `twoBitDFAStateFromName` package-level maps for state name lookups
- `dfa_equivalence_test.go` — exhaustive equivalence tests proving the DFA representations produce identical transitions to hand-coded logic
- Dependency on `github.com/adhithyan15/coding-adventures/code/packages/go/state-machine` (local replace directive)

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
