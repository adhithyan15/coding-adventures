## Unreleased - S10t-1: `ir.tuple` (types, literal, `project`, `match` on `Result`) and its faults (oracle half)

No version bump, and no emitted byte of an existing fixture changes: `synth-v2`, `accept-blind-v2`,
`omit-example-v2`, the ct-compare v1 and v2 fixtures and `fixed-forms-v1` verify unchanged. The
generator follows in its own PR (VER04r section 13, S10t-2), against the committed dumps below.

### Added

- **`ir.tuple`** (VER04r 4.1, A.1, [D20], [D35], R2): the tuple type `(T1,..,Tn)`, the literal
  `["tuple", [..]]`, `["project", e, i]` (also `t.i`, `t.0.1`, `f(x).1`), tuple values (Python tuples,
  JSON arrays), TFV `L<n> v1 .. vn`, structural `==`/`!=`, and **`match` on a `Result`** with the patterns
  `ok err err_any tuple bind wild`, in claim, model, op `admit` and example bodies. Lowering
  (`lower.py`), typing and cost (`ir_eval.py`: `1 +` the operands; a match is `1 +` the scrutinee `+` the
  maximum over the arms) and evaluation (exactly one arm). A `match` on any other scrutinee is `ir.data`.
- **Corpus errors of A.1** (`patterns.py`, code `corpus`): a match that falls through, an arm no value can
  reach (the usual usefulness check, with a work budget), the same `bind` twice in a pattern, a `bind` that
  shadows a binder, a parameter, `result` or an outer `bind`; checked before any arm body is lowered.
- **A `match` arm is a guard for call counting** (VER04r 4.4): `compile_calls`, `compile_trace`,
  `compile_admit_check`, `admit_check_cost` and the v0 rule-3 walk follow the arm the scrutinee selects with
  the binders typed (`ir_eval.walk_scoped`); a subject call in an arm under an impure scrutinee is `W0701`.
- **`irtree.py`**: `children`, `free_vars`, `pattern_binds`, the one place that knows a `tuple` node's
  operands are one list and that a `match` pattern is not an expression (every walker uses it).
- **Fault templates** (`faults.py`, R3, R11, D27): `ok-as-err`, `drop-consumed`, `swap-components`, the
  tuple-payload `missing-error`, and `flip-result-bit` / `drop-last-result-byte` (bare and `Ok` forms,
  exact JSON of 8.2). A fault row's `requires` is the sorted union of the template's levels and the levels
  of the op's parameter and result types (R9).
- **The fault-evaluation gate**: `IMPLEMENTED = ("ir.core",)` is unchanged, so a control whose fault needs
  `ir.tuple` or `ir.fixed` records `deferred` witnesses (naming the first unimplemented level of its
  `requires`); `derive(..., implemented=...)` / `derive_controls(..., implemented=...)` is the library
  parameter the tests use. The control row's `requires` is the fault's whole list.
- **The committed dumps** `tests/fixtures/tuple-forms-v1/` (`make_tuple_forms.py`, `BUILD` and
  `BUILD_windows` run `--check` and `dump-lowering --check`): `tuple_forms_v1.warrant`, `.profile`,
  `.oracle-lowering.json`, `.oracle-evaluation.json` (`parity_dumps.evaluation_view`),
  `.oracle-faults.json` (`parity_dumps.faults_view`, normative for the composite fault JSON, R11) and
  `.oracle-refusals.json` (every corpus-error document and its refusal).
- Tests: `test_tuple_lowering.py`, `test_tuple_eval.py`, `test_tuple_faults.py`, `test_tuple_dump.py`; the
  lzw acceptance (the two `match decompress(..) with Ok/Err` bomb examples lower and are `not-op-eq-pure`; the
  four eq-shaped ones still refuse at `ir.fixed`).

### Changed

- `ir.tuple` suites and examples are **evaluated**, so `derive`, `emit-v2` and `verify-v2` accept them
  (`emit`'s `levels_implemented` is `["ir.core", "ir.tuple"]`); `ir.fixed` documents still refuse.
- **Census pins moved** (tuple-blocked suites are live, or `ir.fixed` when they also use a built-in):
  wasm-leb128 `ir.core`/`ir.tuple`/`ir.fixed` 10/24/8 -> 18/0/24; logic-gates core/data/tuple/fixed 28/3/2/55
  -> 28/3/0/57; arithmetic 5/45/4/9 -> 7/45/0/11.
- The source typer lets a `concat` of statically sized operands, a bytes literal and an `if` of equal widths
  flow to `Bytes<N>` (VER04r A.1's static-width rule; it was only applied to `from_be`/`from_le`), which
  hash-functions needed once its tuple models stopped being refused.
- `lowering_dump` shows `ir.tuple` documents (`LOWERED_LEVELS`).
