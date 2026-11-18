## Unreleased - CV2-F1 (extended), oracle half: lowering-time `W0701` refusals and the emission cap

No version bump. Every committed corpus (`synth-v2`, `accept-blind-v2`, `omit-example-v2`, the
ct-compare v1 and v2 fixtures, `tuple-forms-v1`) verifies byte for byte unchanged. Only the
`fixed-forms-v1` document and its lowering dump change (the eight `c_not_*` claims). The generator
mirror PR follows: it removes the dump-only `skip_pure_guards` path and enforces the emission cap.

### Changed

- **An impure guard is refused at lowering** (VER04r 4.4). `lower.check_pure_guards` mirrors the
  generator's `check_pure_guards` (and adds `match`, whose arms are guarded by the scrutinee): a
  subject call under the condition of an `if`, the left operand of `and`/`or`/`implies` or the
  scrutinee of a `match` is `W0701` (code `generation`). It used to be refused only when a suite was
  evaluated. The refusal is recorded per claim in the new `LoweredSuite.refusal` (a level refusal in
  `unsupported` wins in evaluation, as the generator types before it guards), so `lower_document`
  of an adopter with such claims (`wasm-leb128`) still lowers for a census; `derive`, `SuitePlan`
  (`emit-v2`) and `dump-lowering` raise it. The evaluation-time refusals stay as a backstop.
- **A claim whose body calls no op** (`forall a: U8. a == a`) is `W0701` at lowering (VER04r 4.3).
  The oracle used to derive it, with `expected_calls` 0.
- **The 4,096-byte `Bytes` limit is enforced at emission** (VER04r 2.3, 14). A non-omitted example
  whose argument or pure value (itself, under an `ok`, or in a tuple) is a `Bytes` over 4,096 bytes
  is a generation error, code `limit`, instead of an emitted row. With an `omit_example` line the
  decision is unchanged (`value-over-limit`).
- **A size error while evaluating a callee's `admit`** (the D19 check) now refuses (`ir-overflow`,
  a plain `OracleError`) and never qualifies as `value-over-limit`; only a size error in the
  arguments or the pure side does (VER04r 8.4 step 3).
- **`fixed-forms-v1`**: the eight `c_not_*` claims were `op_not(a) == bit_xor(a, max) or op_not(a) ==
  bit_not(a)`, an impure guard. They are `bit_not(a) == bit_xor(a, max_value<T>()) and op_not(a) ==
  bit_not(a)`, the same built-ins under a pure guard. `fixed_forms_v1.warrant` and
  `fixed_forms_v1.oracle-lowering.json` are regenerated.

### Added

- `tests/test_lowering_refusals.py` (the generator's `analysis.rs` cases, `match`, the depth bound,
  the no-op claim, level precedence); tests in `tests/test_omit_example.py` for the cap and the
  admit size error.
