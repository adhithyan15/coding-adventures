## Unreleased — gate model evaluator and LG02 guard (VER33 ADL-002)

- New `tests/logic_gates_model.rs`: a small AST evaluator (a test oracle over the guarded
  parser's AST, not a second parser) that runs `code/specs/logic-gates-model.warrant`. It pins
  the claim set (the 26 LG02 names plus eight additions), evaluates all 34 claims (`Bool`
  binders exhaustively, `Int` binders over a sample with negatives, 2, 255, 256 and 2^40),
  replays every one of LG02's 39 `example` rows against the model function of the same name,
  restates the truth tables from literals, checks rejection of -1, 2, 255 and 2^40 in every
  argument position, and runs two negative controls (a `> 1` guard, a broken `nand_or`).
- `tests/v0_compat.rs` already parses and checks the new document (it scans `code/specs`).
