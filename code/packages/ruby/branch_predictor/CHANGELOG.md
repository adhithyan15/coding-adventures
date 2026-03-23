# Changelog

## 0.2.0 — 2026-03-20

### Added
- `TWO_BIT_DFA` module constant — formal DFA definition of the 2-bit saturating counter using `coding_adventures_state_machine`
- `ONE_BIT_DFA` module constant — formal DFA definition of the 1-bit predictor using `coding_adventures_state_machine`
- `TwoBitState::STATE_TO_NAME` and `TwoBitState::NAME_TO_STATE` mappings between integer states and DFA state names
- DFA equivalence test suite (`test/test_dfa_equivalence.rb`) verifying structural correctness and transition-by-transition equivalence between imperative and DFA representations
- Runtime dependency on `coding_adventures_state_machine` gem

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
