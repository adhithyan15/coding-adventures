# Unreleased: `ir.data` lowering, typing and requires; byte parity with `data-forms-v1` (S11b)

Slice S11b of the Warrant corpus-v2 migration (VER04r 13; sections 16.14 [D46] to [D48], R27 to R39, 16.16 [D60], R51 to
R61, 16.17 R67, Appendix A.2). No version bump [D52]: no committed corpus or chunk byte moves (the four adopter fixtures
pass `--check --release` unchanged).

## Lowering, typing, costing

- `types.rs`: `Ty::Named` (a record or variant, by its declared name), `Ty::List` (the reserved `ir.list`, only so that the
  lowering can see one and refuse it), `Ty::Option` is now a real type; `contains_data`, `contains_list` and `type_levels`
  (R9 for `ir.data`). **`Ty::unify` and `Ty::assignable` follow the checker (R51, [D60])**: an `Option` flows and unifies by
  its payload, and a tuple no longer lifts component by component (the oracle's CV2-F18 leniency, R61, which the checker
  refuses).
- `ir.rs`: `Ir::Some`, `Ir::None(Ty)` (JSON `["none","T"]`, the payload type as a string), `Ir::Ctor`, `Ir::Field`, and the
  four data patterns `Pat::Some`, `Pat::None`, `Pat::Ctor`, `Pat::Bool`; `Ir::Match` gains `on_result` (the scrutinee is a
  `Result`: `ir.tuple`, any other: `ir.data`); `ir::levels` counts the tags, the data patterns and the match.
- `lower.rs`: records, variants, constructors, `r.field`, `Some`, `None`, `match` on any scrutinee and the data patterns;
  `requires` by tag, pattern and type, with the intermediate use of `unwrap_or` adding no `ir.data` (R29); a `None` typed
  only by its expected type (R52); patterns checked before any arm body, structure before the bind rules (R35 (3)); the
  corpus limits of 64 constructors and 8 fields, checked first (R34, R35 (1)); a reserved level or feature named in
  preference to a typing error of the same unit, an error yielding to a refusal of an earlier-declared unit, and the
  refusal of the unit on the earliest line named (R35 (2), (5), R53, R54); cycles between records and variants.
- **The width is part of the type (R51)**: a bytes literal is `Bytes<len>`, a `concat` of two literal widths `Bytes<sum>`, an
  `if` or `match` keeps an agreeing width, `Some(e)` and `Ok(e)` wrap the type of `e`. The recorded-width machinery
  (`last_width`, `width_out`, `flows`, `static_width`) is gone. `Ok(concat(a, b"0102"))` into a `Result<Bytes<3>,E>` now
  lowers (R13 (c), the S10t over-refusal): `tests/tuple_forms.rs` asserts it.
- `patterns.rs`: the usefulness check over `Result`, `Option`, `Bool`, tuples, records and variants, lazily per column
  (`check_arms_with`); the oracle's sentence for a bind named like a model, an operation, an error type or a reserved
  built-in (`RESERVED_BIND`).
- `error.rs`: `Beyond::IrList`, `FixedCast`, `ErrValue`. **R28's renames**: a list type or literal and `slice to_list
  to_bytes replicate` are `ir.list`; `range` is `ir.seq`; `fixed<N>(b)` is `fixed-cast`; an arm that uses a bound error
  value is `err-value`. The name `ir.data` is never a *lowering* refusal again.
- `view.rs`: the conditional `types` object of the lowering dump (R59): records and variants only, present only for a
  document that declares one, so every older dump keeps its bytes.
- `eval.rs`, `lib.rs`: `ir.data` is **not evaluated**: the four tags, a `match` on a non-`Result` and a data pattern are
  refused at evaluation with `level ir.data` (R37), and `analyse`, and so `generate`, refuses a document that declares a
  record or variant or in which any unit needs the level (`ir.data is lowered, typed and costed but not generated yet`),
  until S11c-2.

## Tests, census, fixtures

- New `tests/data_parity.rs`: byte parity with the oracle's `data-forms-v1` lowering dump (97 ops, 95 models, 103 suites, 11
  examples, 7 types) and the 121 refused documents of its refusals dump with the generator code of each row (`level` 39,
  `ir` 28 with the oracle's sentence, `limit` 4, 50 typing errors), `generate` fail-closed, negative controls. New
  `tests/data_lowering.rs`: each rule once, small.
- **Removed lowering carve-outs**: `tests/off_by_one_consumed_parity.rs` (an `Option` signature was refused at lowering with
  `ir.data`: the `an-option-result` row now reaches `instantiate` and is refused with `fault`, section 8.2) and
  `tests/scalar_faults_parity.rs` (the exact five-row list `saturate-add/an-option-result`, `saturate-sub/an-option-result`,
  `missing-error/an-option-payload`, `missing-error/a-variant-payload`, `missing-error/an-option-result`: all 53 rows reach
  `instantiate` and match the oracle's reason text).
- **Census pins** (`tests/census.rs`): arithmetic `ir.data` 55 + `ok` 8 becomes `ok` 63 (R60); logic-gates `ir.data` 3 + `ok`
  85 becomes `ok` 88; hash-functions `ir.data` 21 becomes `err-value` 2 + 19 more `ok` (48, R28); vos-sched-any `ir.data` 15
  becomes `ok` 5 + `W0701` 10 and vos-sched-fp-bitmap `ir.data` 13 becomes `ok` 11 + `W0701` 2 (the rules `ir.data` hid). The
  oracle table is now the oracle's lowering census and covers all eleven documents; ten equal the generator's (zeroize's
  `inout` is the one generator-side feature gap). 364 claims: `ok` 335, `W0701` 18, `inout` 9, `err-value` 2.
- `tests/fail_closed.rs`, `fixed_lowering.rs`, `omit_example.rs`, `check_cli.rs`, `tuple_forms.rs`: the constructs that were
  `ir.data` refusals lower (or are `ir.list`, `ir.seq`, `fixed-cast`, `err-value`); the binary-refusal tests use
  `arithmetic.warrant` (vos-sched-any is now refused by the `W0701` rule of an example, which `ir.data` hid).
- `BUILD` is unchanged (the new tests run under `cargo test`); the build tool selects this package for an edit to
  `python/warrant-oracle/tests/fixtures/data-forms-v1/` (a row in `repository_inputs.go`).

## Review fixes

- **B1, no panic on a list-refused declaration.** A variant refused for a list field (`One(xs: List<U8>) | Two`,
  `Option<List<U8>>`, `(U8, List<U8>)`) was never registered, though pass 1 had registered its constructors, so a
  constructor expression, a constructor pattern, a comparison or a claim over it indexed a missing entry and panicked
  (`no entry found for key`, exit 101); a refused record whose fields do not resolve (`record Q(xs: List<Zed>)`) did the same
  through the `unchecked()` attempt. The refused variant is now registered with whatever resolves (as the record already
  was), and both indexes (`constructor`, `ctor_pattern`) go through `ctor_fields`, which returns the refusal the earliest
  declared unit holds (the oracle's `ir.list`) or a typing error. New row `a_use_of_a_list_refused_type_names_...` in
  `tests/data_lowering.rs`.
- **N1.** The 50 typing rows of `tests/data_parity.rs` are pinned to `check` (they were only "not ir, level or limit").
- **N2, R54.** `check_or_refuse` discarded an `ir` error of the lowering attempt unless the checker's own error was one of the
  three match kinds, so `match v with Circle(r) -> range(1) == 1 | Rect(r, r) -> ...` was `check` (W0301 never reached: the
  checker's arm-body typing error came first) where the oracle, which checks patterns before bodies, names the duplicate
  bind (`corpus`). The attempt's `ir` error is now returned. No fixture or dump changed.
- **N3.** The scan site of `fixed<N>(b)` carries the oracle's sentence `fixed<N>(b) has no IR form`; the level rows of
  `tests/data_parity.rs` now compare the message too (all 39 equal).
- **N4, predates S11b (S10t-2).** A `match` of more than 256 arms was `check` (the front-end checker's W0602) where the oracle
  and Appendix A.2 say `limit`; the attempt's `limit` is now returned too. Test `a_match_of_more_than_256_arms_is_a_limit`.
- **N5.** The comment in `tests/check_cli.rs` named vos-sched-any where the test uses `arithmetic.warrant`.
