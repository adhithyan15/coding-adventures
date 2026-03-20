# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-20

### Added

- `CodingAdventures.BranchPredictor.Prediction` struct with `predicted_taken`, `confidence`, and `address` fields
- `CodingAdventures.BranchPredictor.Stats` module for tracking prediction accuracy (accuracy, misprediction rate, reset)
- `CodingAdventures.BranchPredictor.Static.AlwaysTaken` — always predicts taken (~60% accuracy on typical code)
- `CodingAdventures.BranchPredictor.Static.AlwaysNotTaken` — always predicts not taken (baseline predictor)
- `CodingAdventures.BranchPredictor.Static.BTFNT` — Backward Taken Forward Not Taken direction-based heuristic
- `CodingAdventures.BranchPredictor.OneBit` — one-bit flip-flop predictor using ONE_BIT_DFA from state_machine
- `CodingAdventures.BranchPredictor.TwoBit` — two-bit saturating counter predictor using TWO_BIT_DFA from state_machine
- `CodingAdventures.BranchPredictor.BTB` — Branch Target Buffer (direct-mapped cache for branch targets)
- DFA integration: one-bit and two-bit predictors formally defined as DFAs using `coding_adventures_state_machine`
- Comprehensive test suite with 80+ tests covering all predictor types, DFA equivalence, and edge cases
- Literate programming style with extensive inline documentation explaining CPU branch prediction concepts
