## Unreleased - CV2-S11c-1: `ir.data` evaluation, values, the data faults and the opened gate; the `data-eval-v1` dumps

VER04r section 13 (row S11c-1), 16.14 ([D46] to [D57], R27 to R39), 16.16 ([D60], R51 to R61), 16.17 (R67), 16.19 (R94), 8.2 and
Appendix A.2. No version bump ([D52]): the generator is not changed (slice S11c-2 mirrors this one) and no committed fixture, kit
file or corpus dump byte moves. **Review fixes (owner decision D71 and the findings of the S11c-1 review) are listed at the end.**

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
  (`wrong-ctor` is not one, [D49]). `fault_row(..., data_names, records)` takes the document's tables (see D71 below).
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

### Review fixes (after the first review; owner decision D71)

1. **D71: `scalar-faults-v1` regenerated, the data-blind bridge removed.** The row `missing-error/an-option-payload` (`Result<Option<U8>,E>`) leaves
   `make_scalar_faults.py`, and `scalar_faults_v1.oracle-refusals.json` is regenerated: **52 rows, not 53**; the other three files and every other byte of
   it are unchanged (`a-variant-payload` keeps its bytes). A variant leaf with no parameter of its type is refused everywhere with exactly
   `missing-error has no template for this payload type`; the `data-eval-v1` refusals dump uses that wording (it read `a variant has no zero value`).
   `_missing_error_core` and the `records=None` mode are gone: `fault_row` has one rule and is always given the document's records and variants
   (`make_scalar_faults.py` passes them; the kit's `instantiate_fault` now passes the case's record fields and variant names, so `field-zero` and the
   record and `Option` payloads of `missing-error` work there, which S11d needs). A nested tuple component is always built (8.2, R31 (a)).
   `code/packages/rust/warrant-corpus-gen/tests/scalar_faults_parity.rs`: the literal `53` is `52` and its comment says why (the generator still
   refuses the shape until S11c-2; nothing else in the file changed). The `scalar-faults-v1` README states the count. This answers open questions 1 and 2.
2. **The typing rows no longer pin a placeholder.** `ir_eval._merge_result` takes payload and error type each from whichever arm knows it, so the
   committed `typing` rows read `Result<(Bytes,Bool),E>`, not `Result<(Bytes,Bool),?>` (8.2: "the common type takes the other arm's error type").
   The `data-eval-v1` README no longer calls these rows "oracle-only": every committed file is part of the byte parity S11c-2 owes.
3. **R94 in every branch (a leniency closed).** The comparison saw only the first `Result` branch, so `["if", ["is_err", R], R, ["ok", ["lit", true]]]` on
   `Result<U8,E>` and a `match` whose second arm is `ok(true)` were accepted. A fault compile now compares every branch of an `if` and every arm of a
   `match` on the value spine with the declared type (R98a; an `if` in a condition or an operand is typed by R51 alone, tested) (`Program.branch_check`, set by `faults._typed`; a document's own models keep the branch-to-branch rule), then the whole.
   Two probes are tests and two rows of the typing dump (`r94/a-bool-ok-in-the-else-branch...`, `r94/a-bool-ok-in-the-second-match-arm...`: the dump has 11
   typing rows, 5 accepted and 6 refused). No row a template writes changes.
4. **TFV depth (R99).** `tfv.value` counts the `L` and `M` containers open inside one field value (a variant is two: `L2` and `L<k>`; an `Option`,
   a tuple, a record and a `Result` one each), starting at depth 0, and refuses the 65th with the code `limit` (VER04j 4.9, W0709, TF01 2.2). **The line's `R<n>`
   does not count** (it is the record header, not a list or a map; every TFV reader starts a field value at depth 0). Tests: 64 accepted and 65 refused for a
   chain of records and for one of variants, and the review's 40-link variant chain (80 containers, once unchecked). **Open question 5's reasoning was wrong**:
   named types chain without adding depth to a *type expression* (a name is depth 0, so the 16-level limit says nothing), yet each link nests one container, or
   two for a variant, in the line.
5. **Nits.** The key `controls_shipped` is **kept**: the older committed dumps (`tuple-forms-v1`, `fixed-eval-v1`, `scalar-faults-v1`) use it, so the data
   README says what it means here (the gate before this slice). The fallback refusal of `tfv.value` and `value_json` names `ir.data` for a record or variant
   whose spec was not given (`ir.list` only for a `List`). `test_the_script_runs_as_a_program` no longer derives the fixture a third time (the BUILD line runs
   it): it checks the entry point and the plumbing on an empty build. The kit has one common-type rule: S5e's `_require_common_types` (built on
   `lower._unify`, no tuple widening) is deleted in favour of `faults._fault_unify` after merging origin/main, and `analyze_data=` is kept.

### Questions this slice raised, and where they are answered

1, 2 and 5 are answered above by D71 and the TFV fix. The other two are answered by the S11c-1-0 erratum (VER04r section 16.20):

3. **R94 and tuples (answered by R96).** R94 lists the unify cases (`Nat`/`Int`, `Bytes`, `Option`, `Result`) and no tuple rule, and R51 makes a tuple
   invariant; section 8.2's `off-by-one-consumed` paragraph says `(T1,Int,..)` and `(T1,Nat,..)` have a common type, and the committed wasm-leb128 fixture
   has that row. R96 settles it for fault rows: components unify pairwise under a narrow relation (equal types, `Nat`/`Int`, `Option`/`Result` of related
   components, nested tuples) with no `Bytes` width lifting (R67's second test keeps `(Bytes<1>,Bool)` against `(Bytes<2>,Bool)` refused). The code here
   already does exactly that.
4. **The consumer-side descriptor check (answered by R103, not done here).** `domains.descriptor_size` does not validate a variant descriptor's constructor
   names, count, duplicates or fields, nor a `tuple` descriptor's arity. (A result's `errs` is validated since S5e, R72.) R103 records the rule under
   CV2-F17; it lands before S11d derives the `v2-domain` variant vectors. The domains dump pins what the oracle does today, and every row is correct under
   the spec.
