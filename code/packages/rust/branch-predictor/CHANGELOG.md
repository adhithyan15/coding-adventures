# Changelog

## 0.2.0 (2026-03-20)

### Added
- Dependency on `state-machine` crate for formal DFA representations
- `two_bit_dfa()` function that constructs a DFA modeling the two-bit saturating counter (4 states, 2 inputs, 8 transitions)
- `one_bit_dfa()` function that constructs a DFA modeling the one-bit predictor (2 states, 2 inputs, 4 transitions)
- `TwoBitState::to_dfa_name()` and `TwoBitState::from_dfa_name()` for converting between enum variants and DFA state name strings
- DFA equivalence tests verifying that the DFA transition tables match the hand-coded enum transition methods
- Lock-step tests walking the DFA and predictor simultaneously to confirm identical behavior on arbitrary input sequences
- DFA completeness, validation, and accepts() tests for both one-bit and two-bit DFAs

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
