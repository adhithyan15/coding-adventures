# Unreleased: `ir.data` evaluation, values, descriptors, faults and fault typing; byte parity with `data-eval-v1` (S11c-2)

Slice S11c-2 of the Warrant corpus-v2 migration (VER04r 13; sections 16.14 [D47] to [D54], R27 to R39, 16.17 R67, 16.19
R94, 16.20 [D71] and R95 to R99, Appendix A.2, sections 2.3 to 2.5, 5.1, 5.3, 8.2). No version bump [D52]: the generator's
`ir.data` gate opens with no flip, and no committed corpus or chunk byte moves (the five adopter fixtures, ct-compare,
x25519, lzw, wasm-leb128 and logic-gates-basic, pass `--check --release` unchanged).

## Added

- `value.rs`: `Value::Con(constructor, fields)` for a record (its constructor is named like it) and a variant; an `Option`
  is the existing `Value::Opt`, now a value of any node and a binder, no longer an `ir.fixed` intermediate only.
- `types.rs`: `Decls`, the declared records and variants (`ctors`, `fields`, `owner`, `is_record`), which the stream, JSON
  and TFV writers and the fault templates read; `Program::with_decls`.
- `eval.rs`, `patterns.rs`: `some`, `none`, `ctor`, `field` and `match` with the four data patterns are evaluated; the
  S11b refusal (`data_not_evaluated`) is gone.
- `stream.rs`: tags `40` (none), `41` (some), `64` (a record, like a tuple), `65` (a variant: name, count, fields);
  `encode_in` takes the declarations (`encode` is the empty table); an `Option` of a v1 type keeps `warrant-stream-v1`,
  a record or a variant is v2.
- `tfv.rs`: JSON `{"none":true}` / `{"some":v}` / array / `{"ctor","fields"}`, TFV `L0` / `L1 x` / `L<n> ..` /
  `L2 S<ctor> L<k> ..`; **the TFV depth of R99** (a tuple, record, `Option` or `Result` is one container, a variant two,
  the 65th is `limit`); `value_json` and `value_tfv` are public.
- `domains.rs`: `option`, `tuple` (a record) and `variant` descriptors with their labels; the demotion path of a variant
  and a record (`0.Done.1.some`, [D54]); `resolve_claim_in` and `BinderDomain::new_in` carry the declarations.
- `faults.rs`: `field-zero` ([D49], R32) and the record, `Option` and variant payloads of `missing-error` as `build(X)`
  ([D50], R31): a composite is always built, **a nested tuple or record included** (R95 (e): the old refusal of a nested
  component is gone), a leaf is the first parameter of exactly its type else its zero (`["none","T"]`), a variant with no
  such parameter is refused with the one wording of every position; `Level::Data`; `instantiate_with` (the declarations),
  `instantiate` and `instantiate_in` keep the empty tables (R95 (d)); the registry has 18 names; **`SHIPPED_GATE` holds
  `ir.data`**.
- `fault_types.rs` (new): the typing of every fault row, R67 (the expected type is handed down; a bytes literal in a
  plain-`Bytes` position is `Bytes`), R94 (a common type, not "flows to", in every branch of an `if` and arm of a `match`),
  R96 (tuples component by component under the narrower relation), R97 (an undetermined component takes the determined
  side) and R98 (the `type` of an accepted row is the common type); `generate` types each row once, an op with a width
  parameter at the widths of each stand-in.
- `emit.rs`, `view.rs`: the corpus `types` object holds the records and variants after the error types (the oracle's
  `_types`).
- `tests/data_eval_parity.rs`: the six files of `data-eval-v1` byte for byte (lowering with its `types` rows, evaluation,
  faults under the gate before `ir.data` and with it, values, domains, refusals with the `typing` rows), negative
  controls, and the oracle's `verify-v2` run on a generator-written corpus (1,839,544 cases, 68 suites);
  `tests/data_eval_rules.rs`: R95, R96 to R98 and R99 (64 and 65 containers for a chain of records and of variants) one at a
  time.

## Changed

- `lib.rs`: `analyse`, and so `generate`, no longer refuses a document for needing `ir.data` (the guard S11b kept: R37).
- The message of an unknown fault name is `the fault cannot be instantiated on this operation: an unknown fault name`, the
  oracle's reason (it was `the fault name is not in the registry of VER04r 8.2`); the code is still `fault`.
- Tests that pinned the S11b fail-closed behaviour now pin its removal (`census`, `data_lowering`, `data_parity`,
  `fail_closed`, `faults`, `fixed_lowering`, `check_cli`, `emission`); `scalar_faults_parity` is unchanged (52 rows).

## Findings

- `arithmetic.warrant` still does not generate, for a reason outside this slice: its claim
  `ripple_carry_adder_with_carry_4_bit_exhaustive` has nine binders and a suite has at most eight (VER04r 5.3, the
  generator's `limit` since S3b). The oracle's derive does not enforce the bound for a document; the adopter PR decides.
