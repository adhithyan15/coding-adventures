## Unreleased - CV2-S10c-2: `ir.fixed` evaluation, the domains of D21/D25/D26 and `generate` for `ir.fixed`, generator half

No version bump (the flip, S10c, does that) and no emitted byte of an existing fixture changes: the
committed ct-compare v2 `--check` passes unchanged (the fault gate is still `ir.core`, so
`eq_u64/ignore-high-bit` stays `deferred ir.fixed`), and the accept-blind, synth-v2, omit-example,
fixed-forms and tuple-forms fixtures reproduce byte for byte. The oracle half merged first (S10c-1,
#19624); this PR reproduces its `fixed-eval-v1` dumps.

### Added

- **The `ir.fixed` evaluator**: all 35 built-ins of Appendix A.1 (`eval::fixed_builtin`), delegating
  the bit work to `warrant-grammar`'s `fixed` module (which agrees with the oracle's independent
  evaluator on every row of the values table: `sar`, `div_trunc`/`rem_trunc` with `MIN / -1` and
  zero, shift counts of the width or more, rotation modulo the width, the `from_be` length check,
  `unwrap_or`). `Value::Opt` is the IR-only Option (its one consumer is `unwrap_or`, which evaluates
  both operands); `wraps` counts only `add sub mul neg` [D31]; cost is unchanged. The lowering
  records the width of a built-in with no static type argument in `Ir::Fixed::at` (not in the JSON).
- **Domains**: `nat_shift` `{"d":"nat_shift","size":73}` [D26]; `fx_edge` for the signed 32- and
  64-bit leaves [D21] and for a demoted 16-bit leaf [D25] (an 8-bit one is refused); the demotions
  are recorded in each suite's `demoted` in the oracle's path text (`"0"`, `"1"`, `"0.1"`, `"0.ok"`);
  `domains::descriptor_size`, the consumer-side check of VER04r 5.1 with the oracle's messages.
  `census` no longer reports 16.2 items 3, 4 and 5.
- **`generate` takes `ir.fixed` documents**; `generate_gated` with `ir.fixed` in the gate searches the
  `ir.fixed` faults (the oracle's `GATE_OPEN`), a library switch only.
- **Tests**: `tests/fixed_eval_parity.rs` reproduces the five dumps of the oracle's `fixed-eval-v1`
  byte for byte (lowering, evaluation, faults under the shipped and the open gate, the 9,800-row values
  table, the domain table with the consumer's verdicts), with negative controls; `tests/support/parity.rs`
  holds the evaluation view shared with `tests/tuple_parity.rs`; unit tests for `fixed_builtin` and the
  domain descriptors.

### Changed

- **`Expr::Data` is gone**: the `xor-lsb`, `ignore-high-bit` and `flip-result-bit` templates are
  ordinary IR (`Ir::Fixed`), so the gate alone decides whether a fault is searched. The fault JSON is
  byte for byte what it was.
- **`lower_document_with_fixed` is removed** (`lower_document` is the one path, and no unit is refused
  for needing `ir.fixed`); a v0 document still refuses the built-in names.
- **Census** (`tests/census.rs`): 238 of 355 claims emittable (was 105); `ir.fixed` 130 and `16.2` 3 are
  gone, `ir.data` 100, `inout` 9, `W0701` 8 remain. `ORACLE_PINS` is the oracle's S10c-1 census (wasm-leb128
  40 `ok` + 2 `W0701`, hash-functions 29 + 6 + 21 `ir.data`, logic-gates 85 + 3, lzw 21, arithmetic
  18 + 45, x25519 8, zeroize 9) and the generator's equals it for seven adopters; arithmetic (records and
  variants in op signatures, until S11) and zeroize (`inout`) differ. VER04r R13(d) says so.
- The tests that asserted "`ir.fixed` is refused" (`fail_closed`, `fixed_lowering`, `check_cli`,
  `omit_example`, `enumeration`, `limits`) assert lowering and evaluation; a refusal is now shown with
  `ir.data` (`vos-sched-any`).

### Fixed

- A parenthesised shift count selects `nat_shift` (`warrant-grammar`'s `count_binders` looks through
  `Expression::Grouped`), matching the oracle's `c_sel_parenthesised` row (VER04r 5.1, R10).
