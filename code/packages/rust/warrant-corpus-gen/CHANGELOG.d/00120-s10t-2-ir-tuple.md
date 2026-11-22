## Unreleased - CV2-S10t-2: `ir.tuple` (tuples and `match` on `Result`), generator half

No version bump, and no emitted byte of an existing fixture changes: the committed ct-compare v2
`--check`, the accept-blind, synth-v2 and omit-example fixtures and the fixed-forms parity all pass
unchanged. The oracle half merged first (S10t-1, #19612); this PR reproduces its `tuple-forms-v1`
dumps byte for byte.

### Added

- **`ir.tuple`**: `Ty::Tuple`, `Value::Tuple`, `Ir::Tuple` / `Ir::Project` / `Ir::Match` (with
  `ir::Pat`), their lowering (`t.i`, `t.0.1`, `f(x).1`, typed literal context), `requires` (R9 type
  half), cost, evaluation (exactly one arm, binders scoped per frame), the `tuple` descriptor, stream
  tag 64, TFV `L<n>` and JSON arrays. `Ir::children` is the one place that knows a tuple's operands
  and a match's arms; no walker visits a pattern.
- **`patterns`** (new module): the usefulness check, the work budget, run-time matching.
- **`match` as a guard**: a subject call in an arm needs a pure scrutinee (`W0701` otherwise) in
  `analysis::check_pure_guards` (lowering time) and `Program::trace`; only the taken arm is traced.
- **Corpus errors** refused with the new generator code `ir` (`code::IR`): fall-through, unreachable
  arm (first, R12), duplicate bind, shadowing bind; the checker's three match refusals map to it.
- **Fault templates**: `ok-as-err`, `drop-consumed`, `swap-components`, tuple-payload `missing-error`,
  bare and `Ok` forms of `flip-result-bit` and `drop-last-result-byte`; `Fault::requires` (sorted
  union, R9); `faults::instantiate_in`, `first_unimplemented`, `SHIPPED_GATE`; `generate_gated`.
- Tests: `tests/tuple_parity.rs` (4 dumps, refusals, negative controls, the CV2-F1 guard rule),
  `tests/tuple_forms.rs`, unit tests in `eval`, `ir`, `patterns`, `types`, `value`, `domains`,
  `analysis`.

### Changed

- `ir.tuple` suites are emitted (`requires` per suite, `levels` the union of every `requires`);
  `ir.fixed` still refuses. A tuple fault's control is `deferred` under the shipped gate.
- `concat`, bytes literals and `if` of sized operands flow to `Bytes<N>` for a model result, an `Ok`
  payload and an argument (the oracle's `_flows`), not only under `from_be`/`from_le`.
- `Fault::level` is replaced by `Fault::requires`; `view` writes empty sides for an omitted example.
- **Census pins moved** (tests/census.rs, README): `ir.tuple` blocks nothing; totals `ir.fixed` 130,
  `ir.data` 100, `inout` 9, `W0701` 8, `16.2` 3, 105 emittable (was 89). logic-gates now equals the
  oracle; remaining differences (wasm-leb128, arithmetic, hash-functions, zeroize) are not tuple.
