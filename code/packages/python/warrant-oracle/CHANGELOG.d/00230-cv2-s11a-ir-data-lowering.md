## Unreleased - CV2-S11a: `ir.data` lowering, typing, `requires` and cost; the `data-forms-v1` dumps

VER04r section 16.14 ([D46] to [D48], R27 to R39) and Appendix A.2. No version bump ([D52]); the generator is not
changed (slice S11b mirrors this one). **Evaluation of `ir.data` still refuses** (`level ir.data`, R37): slice
S11c-1 opens it.

### Added

- **`ir.data` lowering and typing** (`lower.py`): records, variants (with and without a payload), `Option<T>` as a
  real type, the four tags `["some", e]`, `["none", "T"]` (typed, from the context), `["ctor", "Name", [..]]` and
  `["field", e, i]`, structural `==`/`!=` on them, and `match` on **any** scrutinee with the data patterns `some none
  ctor bool` beside the six of A.1. A source name that is a declared constructor is always a constructor pattern.
  Typing follows R39: `Some(e)` is exactly `Option<type(e)>` (no lifting), a `None` with no context is a `type` error,
  `e.i` on a non-tuple and `e.name` on a non-record are `type` errors, constructor arguments are assignable per argument
  only. `unwrap_or` takes any `Option` first operand.
- **`requires` by tag, pattern and type** (`levels.py`, R9 as extended): `ir.data` for `some none ctor field`, for a
  `match` whose scrutinee is not a `Result` and for any data pattern (so `Ok(true) -> ..` needs `ir.tuple` and `ir.data`),
  and for a record, variant or `Option` type anywhere; **except the intermediate use** (a producer directly the first
  operand of `unwrap_or`), which adds none, so no `requires` of the seven committed corpora moves (R29 (f)). `ir_levels`
  takes the levels of each `match` from the caller (`match_levels`), because an IR tree does not carry its scrutinee's type.
- **Cost** (`ir_eval.py`): `Program(analyze_data=True)` types and costs the four tags and the data patterns (`some`, `ctor`,
  `field` `1 +` operands, `none` 1, `match` `1 +` scrutinee `+` the worst arm); without it every data tag and every `Option`
  outside the intermediate use is the level `ir.data`, as before. `typenames.py` reads a canonical type name back for `none`.
- **Usefulness over the larger pattern set** (`patterns.py`): `Bool`, `Option`, records and variants join `Result` and tuples;
  `Types` carries the declared records and variants; IR patterns are type-checked (`some none ctor bool`).
- **Refusals by name** (R27, R28): `ir.list` (a list type anywhere, a list literal, `slice to_list to_bytes replicate`),
  `ir.seq` (`fold map all any range count`, `range` was `ir.data`), `err-value` (an `Err(e)` arm that uses `e`) and
  `fixed-cast` (`fixed<N>(b)`); the name `ir.data` is no longer a lowering refusal. A list type in a record, variant or
  alias declaration or an op signature refuses the document whole; elsewhere it is recorded on the unit.
- **Corpus limits and refusal order** (R34, R35): `limit` for an error type over 64 cases, a record over 32 fields, a variant
  over 64 constructors or a constructor over 8 fields, checked on the declarations first; a reserved level or feature is
  named over a `type` error of the same unit (a pre-order scan of the unit's source), patterns before arm bodies and structure
  before the duplicate-bind rules, units in declaration order; declarations also check a variant has two constructors, no
  duplicate field names, a constructor namespace shared with records, models and ops, and no recursive type.
- **The evaluation refusal is separate from the lowering refusal**: a unit that lowered and needs `ir.data` is marked
  `unsupported` with `level ir.data` and `lowered` is true (`LoweredModel/OpSig/LoweredSuite/LoweredExample.lowered`);
  `lowering_refusal(spec)` is the first refusal a unit recorded while lowering. `census` gains `suites_lowered`,
  `suites_refused_at_lowering`, `suites_w0701`, `models_refused_at_lowering` and `examples_refused_at_lowering`.
- **`tests/fixtures/data-forms-v1/`** (a new directory, S11a's acceptance): `make_data_forms.py` writes and `--check`s
  `data_forms_v1.warrant` (generated: 95 models, 103 claims, 11 examples, 2 more ops with a data `admit`), the lowering dump
  `data_forms_v1.oracle-lowering.json` (schema `warrant-lowering-parity-v1`, plus the corpus `types` rows, which appear only
  for a document that declares a record or variant) and the refusals dump `data_forms_v1.oracle-refusals.json` (121 documents:
  every reserved level, the limits, the typing rules, the corpus errors of A.2 and the refusal order). **Appendix A.2 stops
  being provisional.** Two BUILD lines (and the Windows ones).
- `tests/test_data_lowering.py`, `tests/test_data_forms_dump.py`.

### Changed

- **Census**: logic-gates (88 suites) and arithmetic (63) lower whole; arithmetic's blocked suites are 55 `ir.data` (was 45,
  plus 10 that only counted a record type), hash-functions' 21 `ir.data` are 19 that lower (a `match` on a bare tuple) and 2
  `err-value`. `ir.core`-only documents are unchanged.
- **`tests/fixtures/tuple-forms-v1/tuple_forms_v1.oracle-refusals.json`: one row**, `a-projection-of-a-non-tuple`, is now a
  `type` error (R39 (c), `oracle_code` `type`, empty `oracle_level`, the new message); it was `level ir.data`. The other five
  files of that directory are byte-identical; the generator's `tuple_parity.rs` reads only the row's code class
  (`generator_code` is empty either way) and its `source`, which did not change.
- `tests/fixtures/off-by-one-consumed-v1/README.md`: the sentence that said a generator refuses an `Option` signature at
  lowering with `ir.data` is corrected to the rule of section 8.2 (true of the generator from S11b).
- `levels.type_levels` counts `Option` and no longer counts `List`; `levels.type_refusal` refuses a list (`ir.list`) and no
  longer `Option`; the tag `list` is the reserved `ir.list` (`DATA_TAGS` is `some none ctor field`).
- Tests that pinned the old behaviour were updated: an `Option` anywhere is `ir.data` (now lowered), a list is `ir.list`, a
  record binder is refused at evaluation, `e.i` on a non-tuple is a type error, the census pins.

### Review fixes (S11a, against VER04r 16.16 [D60], R51 to R58)

- **`Option` follows the checker (B1, R51).** `lower._assignable` and `_unify` are the grammar checker's `is_assignable_to` and
  `common_type`: `Option<S>` flows to `Option<T>` when `S` flows to `T` and two `Option` types unify by payload, as `Result`
  does; a tuple, record, variant or list never lifts (`(Nat,Nat)` is not `(Int,Int)`, also as a `ctor` argument: M3). The
  same in `ir_eval` for the IR reader (tuples compare component by component without lifting, the unknown error type of a bare
  `ok`/`err` still tolerated).
- **The bytes width lives in the type (R51)**: a bytes literal is `Bytes<len>`, `concat` of two literal widths `Bytes<sum>`, an
  `if`/`match` of equal widths keeps it, `take`/`drop`/a plain or named-static operand give `Bytes`; `Some`/`Ok` wrap it. The side
  table (`_widths`, `_note_width`, `_static_width`, `_flows`) is gone; the emitted IR is unchanged (a literal's node keeps the
  written type `Bytes`). `Some(b"6162")` flows to `Option<Bytes<2>>` and `Option<Bytes>`, not to `Option<Bytes<3>>`.
- **`None` is typed by its expected type only (M2, R52)**: `o == None` is typed, `None == o` is a `type` error, and a branch or arm
  never takes its type from a sibling (`unwrap_or(if c then at(d, i) else None, 0u8)` is refused). The fixture's one such model
  (`prod_in_if`) now reads `... else at(d, 0)`, which the Rust checker accepts: **the Rust checker accepts the whole fixture
  document** (it did not accept line 39 before).
- **`ir.list` is the refusal (M1, R57)**: a `List` operand of `len concat take drop at` and a call to an op whose signature is
  refused for `ir.list` are `level ir.list`, whether the unit is declared before or after the declaration.
- **Reserved built-ins as declared names (L1, R58)**: a record, variant or constructor named `Some`, `Int`, `take` ... is a `type`
  error, version scoped (`_RESERVED_V0` / `_RESERVED_V1`).
- **Op signatures are units (R53)**: a typing error in a signature yields to an earlier-declared level refusal; the name pass and
  the recursion check yield to nothing.
- **Refusals dump** (109 -> 121 rows): `a-some-is-not-lifted-to-a-wider-option` and `an-option-nat-is-not-an-option-int` are now
  lowering models (`opt_widen_*`, 9 more models and claims); `a-record-of-33-fields` and `a-variant-of-one-constructor` are withdrawn
  (the grammar's parser refuses both, R56; the oracle still refuses the second as a `type` error at lowering, its parser admits it),
  `order-limit-before-a-level` is rebuilt on a 9-field constructor; 16 rows added (None context, no lifting, list operands and calls,
  reserved names, signature order).
- `lowering_dump.py`: the unreachable fallback branches are removed, the `types` comment is corrected (keys sorted, not declaration
  order); `ir_eval` caches the record names once.
- `tests/test_conformance.py::test_type_levels` pinned `List` as `ir.data`, which S11a no longer counts (a list is refused whole).
