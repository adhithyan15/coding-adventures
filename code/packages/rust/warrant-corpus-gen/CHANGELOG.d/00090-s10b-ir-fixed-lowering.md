## Unreleased - S10b: lowering, typing, `requires` and cost of the `ir.fixed` built-ins (VER04r A.1)

No version bump and no emitted byte changes: the committed ct-compare v2 fixture `--check`, the
oracle's `accept-blind-v2` and `synth-v2` fixtures are untouched, and `generate` stays a
fail-closed stub for `ir.fixed` documents (refused as a whole naming `ir.fixed`, nothing written;
the binary exits 1 as for every refusal).

### Added

- **Lowering and typing of the 35 built-ins** of Appendix A.1 (`Lowerer::fixed_call`, `Ir::Fixed`):
  one node per call, the operand types inferred, a static type argument written as the canonical
  type name in position 1, every mismatch a `type` refusal (mixed widths, `sar` of an unsigned, a
  count that is not a `Nat`, `widen` past the target, a signed `div`, ...). `div`/`rem` on two
  `Nat` operands are `Option<Int>` (R7). The shapes, types and costs are those of the oracle's
  S10a dump.
- **`Option<T>` is an IR-only intermediate** (`Ty::Option`, D23, R8): nine built-ins produce one,
  `unwrap_or` consumes it as its first operand and directly; any other use, and an `Option` in a
  model, op or binder type, is refused with the level `ir.data`.
- **The static width of a `from_be`/`from_le` operand** is read from the single pass that lowers
  it (a bytes literal, a `concat` of sized operands, an `if` of equal widths: the checker's
  `Literal::Bytes` / `concat` / `common_type` rule, A.1). The operand is never lowered a second
  time; `tests/fixed_lowering.rs` has a 40-level regression (the oracle's first S10a version was
  exponential in the depth).
- **`requires`** on `LoweredOp`, `LoweredModel`, `LoweredSuite` and `LoweredExample` (R9, the tag
  part): `ir::levels` is `ir.core` plus `ir.fixed` per built-in plus each callee's own `requires`
  (no walk of the call closure). `LoweredOp::to_json` writes it.
- **Cost**: a built-in costs `1 +` its operands (`max_value`/`min_value` cost 1); `cost.rs` needed
  no change.
- **`view::oracle_view_v1`** (schema `warrant-lowering-parity-v1`: the v0 view plus `requires` on
  every op row and suite) and **`lower::lower_document_with_fixed`**, the only way to get the IR of
  an `ir.fixed` document (for the dump; it also skips VER04r 4.4's impure-guard refusal, which the
  oracle applies when evaluating and not when lowering).
- **`tests/fixed_parity.rs`**: lowers the oracle's `fixed_forms_v1.warrant` and reproduces its
  committed `fixed_forms_v1.oracle-lowering.json` byte for byte (the files are read from
  `python/warrant-oracle/tests/fixtures/fixed-forms-v1/`; two negative controls). It runs under the
  `cargo test` line of `BUILD` and `BUILD_windows`.
- `tests/fixed_lowering.rs` (13 tests), `tests/census.rs` pins, and unit tests for every
  defensive refusal of `fixed_call` in `lower.rs`.

### Changed

- **Census naming**: a refused unit is named by the first construct, in source order, that cannot
  be lowered, and is `ir.fixed` only when nothing else blocks it (an item that lowers with built-ins
  is refused with `ir.fixed` by `lower_document`, `census` and `Partial::first_block`; a later tuple,
  constructor or `W0701` in the same unit names the unit instead). Adopter totals (11 documents,
  355 claims): before `ir.data` 154, `ir.fixed` 101, `inout` 9, `W0701` 2; now `ir.fixed` 99,
  `ir.data` 79, `ir.tuple` 71, `inout` 9, `W0701` 8, 89 emittable. Per adopter, the histogram is
  pinned in `tests/census.rs` and agrees with the oracle's `test_lower.py` pins for ct-compare,
  gf256, lzw and x25519; logic-gates, arithmetic and wasm-leb128 differ only where the generator
  still refuses a tuple, record or variant *type* in an op signature or the impure guard.
- New level name `ir.tuple` (`Beyond::IrTuple`): tuple types, the tuple literal, projection and
  `match` on a `Result` (D20, D35) were `ir.data`; `match` on anything else, lists, records,
  variants and `Option` types stay `ir.data`.
- A claim that lowers within `ir.core` is no longer blocked by a callee op whose `admit` needs
  `ir.fixed` (the oracle's `requires` does not read it either); the document is still refused, at
  the op.
- Six `hash-functions` claims, hidden before by an `ir.fixed` built-in, are `W0701` (a subject
  call under an impure guard, VER04r 4.4).
