## Unreleased - CV2-S11c-1: `ir.data` evaluation, values, the data faults and the opened gate; the `data-eval-v1` dumps

VER04r section 13 (row S11c-1), 16.14 ([D46] to [D57], R27 to R39), 16.16 ([D60], R51 to R61), 16.17 (R67), 16.19 (R94), 8.2 and
Appendix A.2. No version bump ([D52]): the generator is not changed (slice S11c-2 mirrors this one) and no committed fixture, kit
file or dump byte moves.

### Added

- **Evaluation of the four `ir.data` tags** (`ir_eval.py`): `["some", e]`, `["none", "T"]`, `["ctor", "Name", [..]]` and `["field", e, i]`
  build and read the oracle's values (a record is the tuple of its fields, a variant `(constructor, fields)`, an `Option` `("none",)` or
  `("some", v)`, the values the domains already enumerate); costs and typing are S11a's. `match` on any scrutinee evaluates exactly one arm
  with the data patterns `some none ctor bool` (`patterns.compile_pattern(pattern, types)`: it needs the declared types to tell a record from
  a variant). Equality of records, variants and `Option` values is structural; ordering stays undefined. `Program(analyze_data=...)` now
  defaults to **True**; `False` is the program of a lane that does not declare `ir.data` (the conformance kit passes `ir.data in levels`), which
  refuses every data tag, an `Option` outside the intermediate use and a `match` on a non-`Result` as the level `ir.data`, exactly as before.
- **No evaluation refusal** (`lower.py`): `seal` no longer marks a unit that needs `ir.data` `unsupported` (`level ir.data`); `EVAL_REFUSAL_MESSAGE`
  and `eval_refusal` are removed. `unsupported` is set by a lowering refusal only (`ir.list`, `ir.seq`, `err-value`, `fixed-cast`).
  `emit-v2` reports `ir.data` among `levels_implemented`; `census` counts every lowered suite under `ir.core`.
- **Values** (`corpus_v2.value_json`, `tfv.value`, both with an optional `spec`; `_over_limit`): JSON `{"none":true}` / `{"some":v}`, an array for a
  record, `{"ctor":"Name","fields":[..]}` for a variant; TFV `L0` / `L1 x` for an `Option`, `L<n> ..` for a record and `L2 S<hex ctor> L<k> ..` for a
  variant (VER04j 4.9, adopted by section 2.5); the 4,096-byte limit of an example sees through them. The stream encoders (`40`/`41`, `64`, `65`)
  already existed. A value of a list type is refused as the reserved level `ir.list` (it was named `ir.data`).
- **`field-zero`** [D49] and the **record, `Option` and variant payloads of `missing-error`** [D50] (`faults.py`): the exact JSON of section 8.2 for
  `RippleCarryResult` (both adders), `AluResult`, `tri_state`'s `["none","U8"]` and every row of the adopter table; `build(X)` (a tuple or a record is
  always built, a leaf is the first parameter of exactly its type else its zero, a variant has no zero); `requires` computed from the built tree
  (`ir.tuple` for a `tuple` node, `ir.data` for `ctor` or `none`, `ir.core` for a bare `var` or `lit`). The registry holds eighteen names
  (`wrong-ctor` is not one, [D49]). `fault_row(..., records=...)` takes the document's record table.
- **The oracle's `ir.data` fault gate opens** (`faults.IMPLEMENTED` gains `ir.data`): a `{"deferred": "ir.data"}` witness is no longer derivable and
  `verify-v2` rejects one (negative tests, the S10c pattern; `emit-v2` and `verify-v2` on a synthetic data document, in `tests/test_data_eval.py`).
- **R67 and R94, the first comparison of a fault row's type with a declared type** (`ir_eval.Program.compile(expected=...)`, `faults._typed`,
  `_fault_unify`): the expected type is handed down as R52 hands one down (both branches of an `if`, the payload of `ok` and `some`, component `i`
  of a `tuple`, argument `i` of a `ctor`, every `match` arm); a `Bytes` literal in a plain-`Bytes` position is a `Bytes`; a fault expression
  must have a **common type** with the op's result (or the parameter a `pre` rewrite rewrites), never "flows to". `derive_controls` types every row
  once, even when no claim can stand in for the subject. The three tests R67 asks for are in `tests/test_data_eval.py`.
- **`tests/fixtures/data-eval-v1/`** (a new directory): `make_data_eval.py` writes and `--check`s the synthetic document (9 records, 4 variants, 60
  models, 68 claims, 23 examples), its profile (12 data faults) and six dumps: lowering, evaluation (68 suites, 1,839,544 cases, a suite on each stream
  header: 29 v1, 39 v2; the demotion paths `1`, `0.0` and `0.Done.1.some`), faults under the gate before and after this slice, values, domains and
  refusals; README. Two BUILD lines (and the Windows ones). `tests/test_data_eval.py`, `tests/test_data_eval_dump.py`.
- **Recorded for the two adopters' PRs** (README of `data-eval-v1`): the oracle derives logic-gates whole in about 60 s (60.1 to 61.6 s measured) (88 suites, 3,770,123 cases, the
  largest suite 1,048,576, largest `ir_cost` 2,372, so `max_ir_cost` above 2,000; one `omit_example`, `decoder_at_the_cap_has_65536_lines`) and
  arithmetic whole in about 36 s (63 suites, 2,655,437 cases, largest suite 284,258, **largest `ir_cost` 1,524** (`alu_add_4_bit_exhaustive`), so no
  `max_ir_cost` line). Neither adopter needs anything outside S11c-1; no control is all-blind.

### Changed

- **Census**: logic-gates (88) and arithmetic (63) have no suite blocked at evaluation (were 3 and 55 `ir.data`); `models_not_ir_core` of arithmetic is 0
  (was 19). The tests that pinned the S11a evaluation refusal were updated (`test_data_lowering`, `test_lower`, `test_cli_emit`, `test_fixed_lowering`,
  `test_fixed_requires_cost`, `test_tuple_lowering`, `test_suites`, `test_ir_eval`), those that pin the gate (`test_fixed_faults`, `test_tuple_faults`,
  `test_controls_v2`) now list `ir.data` and `field-zero`, and those that pin what a lane without `ir.data` gets build their program with
  `analyze_data=False`.
- `conformance.py` builds its programs with `analyze_data = "ir.data" in the case's levels`, so a kit case that does not declare the level is refused
  by it as before and no kit byte moves.
- `emit.py`: `levels_implemented` is `["ir.core", "ir.data", "ir.fixed", "ir.tuple"]`.

### Open questions for the owner (the spec is silent or in conflict; the most conservative reading was taken)

1. **`scalar-faults-v1` pins an `Option` payload of `missing-error` as refused.** Its committed refusals dump (and S11f-1's unit test) has the row
   `an-option-payload`, `op f(a: U8) -> Result<Option<U8>,E>`, refused with `fault: missing-error has no template for this payload type`, while [D50] and
   R31 (c) give that payload a template (`["none","U8"]`, the `tri_state` row) and say no committed dump pins a refusal of it. No committed byte may move, so
   `fault_row` keeps the rule before `ir.data` when it is **not told the document's records** (`records=None`, which is how that script and the kit call
   it); `derive` always passes them. The follow-up is a decision: regenerate that dump without the row (and the generator's S11f-2 parity with it), or keep
   the data-blind call as a documented mode.
2. **Nested tuple components.** Section 8.2 says a tuple or a record is "always built, never taken whole from a parameter of its type", and `build` recurses; the
   older rule took a *nested* tuple component from a parameter of exactly its type and refused one with none (`test_a_component_with_no_zero...`). The data-aware
   rule follows the table (a nested tuple is built); the data-blind rule keeps the old behaviour. No committed row has a nested tuple.
3. **R94 and tuples.** R94 lists the unify cases (`Nat`/`Int`, `Bytes`, `Option`, `Result`) and no tuple rule, and R51 makes a tuple invariant; section 8.2's
   `off-by-one-consumed` paragraph says `(T1,Int,..)` and `(T1,Nat,..)` have a common type, and the committed wasm-leb128 fixture has that row. The comparison
   therefore allows exactly one tuple-component widening (`Nat`/`Int`) and nothing else (R67's second test needs `(Bytes<1>,Bool)` against `(Bytes<2>,Bool)`
   refused).
4. **The consumer-side descriptor check** (`domains.descriptor_size`) does not validate a variant's constructor names (grammar, duplicates, the 64-constructor limit)
   or a result's `errs` (R72). Nothing was added: the generator's check is not specified to do more. The domains dump pins what the oracle does today.
5. **TFV nesting depth** (VER04j 4.9, W0709: 64 containers) is not checked for a data value; a 16-deep type cannot exceed it (at most 3 containers per level).
