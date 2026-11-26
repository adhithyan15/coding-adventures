# python/warrant-oracle

The **independent oracle** of Warrant corpus v2
([VER04r](../../../specs/VER04r-warrant-corpus-v2.md) section 10, decision [D3]).

The Rust generator (`rust/warrant-grammar`, later `rust/warrant-corpus-gen`) reads
a `.warrant` file and *lowers* it to a language-neutral model IR plus digests of
the inputs and expected values of every claim. A bug in that lowering (`a - b`
emitted as `b - a`, a refinement dropped, a domain in the wrong order) would be
faithfully copied into the corpus and then faithfully *verified* by every lane.
This package is the second, independent reading that catches it:

```text
code/specs/<pkg>.warrant ──► Rust generator ──► corpus.json (IR + digests)
        │                                              │
        │                                              ▼ compared, never trusted
        └────► this package: lex, parse, check, lower, ───► mismatch? the adopter
               enumerate, evaluate, hash                    cannot merge
```

It is **stdlib-only**, shares no code, data file or table with the generator, and
**never reads the generator's IR as truth**: it parses the `.warrant` source and
lowers it itself, then only *compares*.

## What is here (slice S4, parts 1 and 2, S10a, S10t-1, S10c-1, S11a and S11c-1)

| Part | Module | Notes |
|---|---|---|
| reader | `lexer`, `parser`, `ast` | layout tokens, v0 + VER04j v1 + VER04k WX-G syntax, fail-closed, admission limits |
| checker + lowering | `lower`, `levels`, `patterns`, `irtree`, `typenames` | typing, contextual literals, **unoptimised** syntax-directed IR, the 35 `ir.fixed` built-ins, tuples and `match` on a `Result` (`ir.tuple`), records, variants, `Option` and `match` on any scrutinee (`ir.data`), `requires` inference, named level and feature refusals |
| evaluator | `ir_eval` | `ir.core`, `ir.tuple`, the 35 `ir.fixed` built-ins and (S11c-1) the four `ir.data` tags and `match` with the data patterns, the single wrapping rule (VER04 section 3), `MAX_IR_VALUE`, cost bound, call counting (a `match` arm is a guard) |
| domains | `domains` | every descriptor of VER04r 5.1 (`fx_edge` [D21, D25], `nat_shift` [D26]), `size`/`nth`, budget demotion, the shift-count rule, the consumer-side descriptor check |
| streams | `stream` | `warrant-stream-v1/v2` encoders (`40`/`41` `Option`, `64` record and tuple, `65` variant), chunk and suite framing, FNV-1a slim selection |
| suites | `suites` | product enumeration, admission arithmetic, chunking, per-chunk digests |
| gate | `corpus_v1`, `emit`, `cli`, `lowering_dump`, `parity_dumps`, `eval_dumps` | the ct-compare v1 gate, v2 digest emitter, level census, lowering, evaluation, fault, value and domain dumps |
| v2 gate (part 2) | `corpus_v2`, `faults`, `tfv`, `verify_v2` | derives a whole v2 fixture from the `.warrant` and `corpus.profile`; `verify-v2` compares it with the directory |

Every adopter parses: ct-compare, gf256, wasm-leb128, logic-gates, lzw,
arithmetic, x25519 (and the WX-G `inout`/`ensures` syntax of zeroize).

### Feature levels

The evaluator implements **`ir.core`, `ir.tuple`, `ir.fixed` and `ir.data`** (VER04r section 4.1; `ir.tuple` since
slice S10t-1, `ir.fixed` since slice S10c-1 and `ir.data` since slice S11c-1, see below), so `derive`, `emit-v2` and
`verify-v2` accept a document that needs any of the four. **`ir.data`** (records, variants, `Option` as a real type, `match`
on any scrutinee) was *lowered, typed and costed* by slice S11a and is **evaluated** since S11c-1: a unit that needs it is no
longer refused at evaluation. What the oracle refuses at
*lowering*, by name and never as `ir.data`, is what has no level it implements: the reserved `ir.list` (a list type
anywhere, a list literal, `slice to_list to_bytes replicate`) and `ir.seq` (`fold map all any range count`), and the
two feature names `err-value` (an `Err(e)` arm that uses `e`) and `fixed-cast` (`fixed<N>(b)`); none is ever skipped
silently. The built-ins are still refused in a v0 document (VER04j 1.2: the names are version scoped).
Census (the pins of `tests/test_lower.py::CENSUS`: suites whose evaluation is not blocked, `ir.core`, and
the suites blocked at evaluation; `check --spec` also prints `suites_refused_at_lowering`). **Since S11c-1 nothing is blocked
at evaluation but what is refused at lowering**; the `ir.data` column is what S11a counted there:

| Adopter | suites | live (`ir.core`) | `ir.data` | before S10c-1 (core / fixed / data) |
|---|---:|---:|---:|---|
| ct-compare | 9 | 9 | 0 | 9 / 0 / 0 |
| gf256 | 38 | 38 | 0 | 38 / 0 / 0 |
| zeroize | 9 | 9 | 0 | 8 / 1 / 0 |
| x25519 | 8 | 8 | 0 | 6 / 2 / 0 |
| lzw | 21 | 21 | 0 | 4 / 17 / 0 |
| wasm-leb128 | 44 | 44 | 0 | 18 / 24 / 0 (42 claims; the two encoder bounds were split, VER04r D28) |
| logic-gates | 88 | 88 | 0 (was 3) | 28 / 57 / 3 |
| hash-functions | 56 | 54 | 0 (was 19) (+ 2 `err-value`) | 4 / 31 / 21 |
| arithmetic | 63 | 63 (62 in `emit-v2`: one claim has 9 binders and is refused `limit`, CV2-F17) | 0 (was 55) | 7 / 11 / 45 |

(Before S10t-1: wasm-leb128 10 / 8 / 24 tuple, logic-gates 28 / 55 / 2 tuple / 3, arithmetic 5 / 9 / 4 tuple / 45.
S11a: **every suite of logic-gates and arithmetic now lowers whole**; arithmetic's 55 is the 45 that used to be refused at
lowering plus the 10 that counted a record type in their `requires`, and hash-functions' 21 became 19 that lower (the claims that
`match` on a bare tuple) and 2 refused as `err-value`. S11c-1: those 19, logic-gates' 3 and arithmetic's 55 are evaluated.)

### `ir.data`: lowering, typing, `requires` and cost (slice S11a), evaluation (slice S11c-1)

Records, variants (with and without a payload), `Option<T>` as a real type, the four tags of VER04r Appendix A.2 and
`match` on **any** scrutinee are lowered by `lower.py`, typed and costed again by `ir_eval.Program` (`analyze_data=True`, the default since S11c-1) and counted in `requires`:

| Source | IR | Cost |
|---|---|---|
| `Some(e)` | `["some", e]`, typed exactly `Option<type(e)>`, which flows to a wider `Option` by its payload (R39 (a), [D60], R51) | `1 +` the operand |
| `None` | `["none", "T"]`, `T` the canonical type name the context fixes; no context is a `type` error (R39 (b)) | 1 |
| `Name(e1, .., en)`, a bare payload-free `Name` | `["ctor", "Name", [e1, .., en]]` | `1 +` the operands |
| `r.name` | `["field", r, i]` (the index, never the name); `e.i` on a non-tuple and `e.name` on a non-record are `type` errors (R39 (c)) | `1 +` the operand |
| `match s with ..` | `["match", s, [[pat, arm], ..]]` with the patterns `some none ctor bool` beside the six of A.1 | `1 +` the scrutinee `+` the worst arm |

* **`requires`** (R9 as extended): by tag, by pattern and by **type**. A `match` is `ir.tuple` when its scrutinee is a `Result`
  and `ir.data` for any other type, and `ir.data` as well when a data pattern appears (so `Ok(true) -> ..` needs both). The type rule adds
  `ir.data` for a record, a variant or an `Option` anywhere in a type, **except the intermediate use** of R29: a producer node
  (`try checked_add checked_sub checked_mul div rem div_trunc rem_trunc at`) directly the first operand of `unwrap_or`
  adds none, so `unwrap_or(checked_add(a, b), 0u8)` still requires `ir.fixed` only and **no `requires` of the seven committed corpora
  moved** (their `verify-v2` lines pass unchanged).
* **Patterns** are checked before any arm body, outside in, with the structure (unknown constructor, a type that does not fit, a wrong field
  count) before the duplicate-bind and shadowing rules (R35 (3)); then the usefulness check (`patterns.py`, now over `Bool`, `Option`,
  `Result`, variants, records and tuples), unreachable arms before fall-through. A name that is a declared constructor is a constructor
  pattern, never a binder.
* **Refusal order** (R35): the declaration limits of section 2.2 first (`limit`: 64 constructors, 8 fields per constructor, 32 per
  record, 64 error cases), then a reserved level or feature before a type error of the same unit, then units in declaration order.
* **Evaluation** (slice S11c-1; S11a refused it, R37): see the next section.
* **The committed dumps** (`tests/fixtures/data-forms-v1/`, generated by `make_data_forms.py`, a new directory on purpose):
  `data_forms_v1.oracle-lowering.json` (86 models, 94 suites, 11 examples, the corpus `types` rows) and
  `data_forms_v1.oracle-refusals.json` (109 documents: every reserved level, the corpus limits, the typing rules and corpus errors of
  Appendix A.2, and the refusal order). **They make Appendix A.2 stop being provisional**; slice S11b must reproduce them byte for byte.
  The one row this slice moved in an existing dump is `tuple-forms-v1`'s `a-projection-of-a-non-tuple` (now a `type` error).

### `ir.data` evaluation, values and faults (slice S11c-1)

* **Values** (the domains' own): a record is the tuple of its fields in declaration order, a variant `(constructor, (field, ..))`,
  an `Option` `("none",)` or `("some", v)`. `["some", e]`, `["none", "T"]`, `["ctor", "Name", [..]]` and `["field", e, i]` build and
  read them; `match` evaluates exactly one arm with the patterns `some none ctor bool` (`patterns.compile_pattern`, which needs the
  declared types to tell a record from a variant); equality of records, variants and options is structural and ordering is
  undefined. `analyze_data=False` is the program of a lane that does **not** declare `ir.data` (the conformance kit's `levels`): every
  data tag, an `Option` outside the intermediate use and a `match` on a non-`Result` are the level `ir.data`, as before S11a.
* **Encodings**: JSON `{"none":true}` / `{"some":v}`, `[..]` for a record, `{"ctor":"Name","fields":[..]}` for a variant
  (`corpus_v2.value_json`); stream `40`/`41`, `64`, `65` (`stream.make_encoder`, unchanged) and the **v1** header whenever every type
  is in the v1 set (`Option<U8>` is, a record is not); TFV `L0`/`L1 x`, `L<n> ..`, `L2 S<ctor> L<k> ..` (`tfv.value`; VER04j 4.9).
  The 4,096-byte limit of an example sees through all of them.
* **Faults** (`faults.py`, section 8.2 [D49] [D50]): `field-zero` (the first numeric field of a record, bare or in an `Ok`; `ir.data`
  and `ir.tuple`) and the record, `Option` and variant payloads of `missing-error` (`build`: a record is a `ctor` of its fields, a leaf is
  the first parameter of exactly its type else its zero, `["none","T"]` for an `Option`, a variant has none). The registry holds
  eighteen names. `wrong-ctor` is not in it ([D49]).
* **The gate opens** (no flip, no version bump, [D52]): `faults.IMPLEMENTED` gains `ir.data`; a `{"deferred": "ir.data"}` witness is no
  longer derivable and `verify-v2` rejects one (a negative test, the S10c pattern).
* **R67 and R94, the first comparison of a fault row's type with a declared type**: a fault expression is compiled with the op's
  declared result (or the parameter) as its *expected* type, so the zero literal of a component declared plain `Bytes` is a `Bytes`,
  and must have a **common type** with it (not "flows to"; a tuple is invariant but for the `Nat`/`Int` widening of
  `off-by-one-consumed`), **in every branch of an `if` and every arm of a `match`**, not only the first. Every fault row is typed once in
  `derive_controls`, even when no claim can stand in for the subject. The common type of two `Result`s takes the error type from whichever arm
  knows it (`Result<(Bytes,Bool),E>`, never the placeholder `?` of a bare `ok`).
* **`fault_row(..., data_names, records)`** has **one rule** (owner decision D71): it is always given the document's records and variants
  (`derive`, `make_scalar_faults.py` and the conformance kit's `instantiate_fault` all pass them), there is no data-blind call, and a variant
  leaf with no parameter of its type is refused everywhere with `missing-error has no template for this payload type`. The committed
  `scalar-faults-v1` refusals dump lost its `an-option-payload` row (53 rows became 52; `Result<Option<U8>,E>` has a template now) and was
  regenerated; `a-variant-payload` keeps its bytes. A nested tuple component is always built (8.2, R31 (a)).
* **TFV depth**: `tfv.value` counts every container it writes (`L`, `M`, `R`) and refuses over 64 with the code `limit` (VER04j 4.9, W0709).
* **The committed dumps** (`tests/fixtures/data-eval-v1/`, `make_data_eval.py`, a new directory on purpose): the lowering, evaluation,
  fault, value, domain and refusal dumps of a synthetic data document with a suite on each stream header, which the generator's slice
  S11c-2 must reproduce byte for byte (every file: the typing rows of the refusals dump too). Its README records the oracle's derive time for both adopters (logic-gates about 60 s (60.1 to 61.6 s measured),
  3,770,123 cases; arithmetic about 36 s, 2,654,925 cases since CV2-F17 refuses its 9-binder claim, was 2,655,437) and arithmetic's largest `ir_cost` (1,524, under the default 2,000).

### Domain and binder fixes (CV2-F17)

Three defects found when the generator's S11c-2 slice was reviewed; in each the generator followed VER04r and the oracle did not.

* **Demotion refreshes cached sizes bottom-up.** A variant caches one size per constructor and a list its `d16`/`d4`, from the sizes of
  its children. `_refresh` recomputed a variant before the tuple or record below it had dropped its own cache, so after a 16-bit leaf
  was demoted the variant kept the stale size, the budget loop saw no progress and refused `W0706` where the generator demotes. Five
  shapes were affected (variant over tuple, variant over record, nested variant, record over variant, `Option` over variant) and each
  has a test pinning the demotion path and size (`variant M = X | Y(t: (U16, Bool))` at budget 1000 demotes `0.Y.0.0` to 563).
* **Composite labels follow the demotion.** The same walk recomputes every composite's `label` from its children: a demoted leaf is
  `bounded-set`, so a tuple, record, variant, `Option`, `Result` or list holding one is no longer `full` (VER04r 9.3).
* **A descriptor path has at most 16 nodes** (section 14, a leaf alone counting as one). `descriptor_for` started at depth 0 and tested
  `> 16`, accepting 17 nodes; it now agrees with the consumer check `descriptor_size`: 16 nodes are accepted, 17 are refused `limit`
  (tested for nested records and nested `Option`).
* **A claim has at most 8 binders** (sections 5.3 and 14, R75). `SuitePlan` refuses a ninth with `limit` before any descriptor is
  built, so `emit-v2` reports the suite `refused: limit` (`derive` and `verify-v2` refuse the whole document). arithmetic's `ripple_carry_adder_with_carry_4_bit_exhaustive` has nine, so
  `emit-v2` lists 62 arithmetic suites and 2,654,925 cases (was 63 and 2,655,437); the adopter splits the claim, not this package.

* **`tuple` and `variant` descriptors are checked as a consumer must (VER04r 16.20 R103).** `descriptor_size` refuses (`generation`,
  kit code `descriptor`, as `errs` of R72 does) a `tuple` whose `of` has none or more than 32 entries, and a `variant` with fewer than 2
  or more than 64 constructors, a constructor name that is not a string or is outside the name grammar (section 2.2 rule 4), a repeated
  name, or a constructor with more than 8 field descriptors. Where a `types` table is present (`conformance._check_errs`, the corpus
  reader), the constructor list must equal the declared one (names, order, field counts) and a record's `tuple` must have exactly its
  field count; a mismatch is a `corpus` refusal as for `errs`. The wording of the two refusals the committed
  `data_eval_v1.oracle-domains.json` pins (an empty `of`, an empty `ctors`) is unchanged, so no committed byte moves.
  Tests: `tests/test_descriptor_r103.py`.

* **`Int` never reaches a corpus (VER04r 2.1).** Lowering accepts `Int` anywhere (the committed `data-forms-v1` is a lowering-only
  fixture), but `derive` refuses, as `generation` / `W0701` and before anything is built, `Int` at any depth in a record or variant field
  (the `types` table), an op parameter or result, or a binder (`corpus_v2.refuse_int_in_corpus`); `emit-v2` still reports per suite. A model
  *result* is never written, so it is not checked; a model's *parameters* are written (`corpus["models"][..]["params"]`), contrary to [D23], and
  are left for an owner ruling because `arithmetic` and `fixed_forms_v1` use them. A `lit` type and a `none` type argument are left alone.
  An expected value outside its type (`f(3) == 2 - 5` for a `Nat` result, `f(a) == a - 1`) is `W0701` too, and the stream encoder range-checks every
  scalar. Tests: `tests/test_int_not_in_corpus.py`.

* **The oracle's share of VER04r 16.22 (CV2-F17).** (1) **[D74], R110**: `v2-fault`'s closed set gains `ill-typed`. `derive_fault` reads the
  case's `types` first (`check_type_table(types, "ill-typed")`), then `level-missing`, then `subject-forbidden`, then the typing of the row; a row
  with no common type (R94, R96 to R98), a fault expression that is ill-typed as a tree and an op row that names `Int` or an undeclared type are
  `ill-typed`, where they were a `KitError`. `instantiate_fault` no longer ends in an uncaught `Refused` on an undeclared field. (2) **[D75], R111**:
  a model parameter may have the type `Int` (and `Int` under `Option`, `Result`, a tuple or `List`): `_models` and `reached_levels` read it with
  `allow_int`; an op row (now `ill-typed`, was `type`), a field, a binder, an `env` entry and every value still refuse it. The oracle's `v2-chunk`
  reader never reads a corpus model row, so nothing changes there. (3) **R113**: `bindings_v2.read_corpus` applies `check_type_table` to
  `corpus.types` (`BindingKitError`: `v2-binding` has no table code, so a bad table is a kit error, never a vector). (4) **R115 (b)**:
  `compile_ir_case` checks the bound on `max_ir_cost` before the table. (5) **R117**: an error type is not a value type: `check_type_table`,
  `parse_type` and `lower._resolve` refuse an error name anywhere but the second argument of `Result`; the table is read whole (every member text is
  parsed, rules 2 to 4 of section 2.2 are enforced). No committed byte moves. Tests: `tests/test_cv2_16_22_checks.py`.
* **The oracle's share of VER04r 16.23 (CV2-F17).** (1) **[D77], R119**: `v2-fault`'s closed set gains `unknown-tag`, `depth` and `model-order`.
  `derive_fault` reads the fault row's schema (R126), then the table and the op row (`ill-typed`), then `level-missing` (levelling only the tags it
  knows), then the three new codes (`fault_static_refusal`: a tag of no level, a nesting over 64, a `model_call`), then `subject-forbidden`, then the
  typing of the row. `requires_of` gives a `unknown-tag` case its `levels`. (2) **R120**: any refusal of an `env` type text is `ill-typed` in `v2-ir`;
  an entry that is not `[name, {"type": text, "value": V}]` is a kit error. (3) **R122**: `compile_ir_case` reads every op signature right after the
  table (so an unused bad op row precedes a tree defect); model signatures are left open. (4) **R123**: the consumer-side `check_type_table` reserves
  `TABLE_RESERVED` (`V0_BUILTINS` plus `U16 U32 I8 I16 I32 I64`) for row and constructor names; lowering keeps its version-scoped set. (5) **R124**: an
  unreached model that needs a level outside `levels` is not compiled (`_models`, `_uncompiled_models`); the tag-name checks, the expression depth and
  the chain depth of R77 still apply to it. (6) **R125**: `check_corpus_and_row` reads `corpus.ops`, `corpus.models` and `model_order`
  (`check_corpus_ops_models`) and refuses their defects as `corpus` (`Int` model parameters stay accepted, [D75]). (7) **R126**: a malformed fault row
  or an undefined level in `fault.requires` is a `KitError` (`check_fault_row`), read before the table. No committed byte moves. Tests:
  `tests/test_cv2_16_23_checks.py`.
* **Three checks of VER04r 16.21 (CV2-F17).** (1) **[D73]**: `conformance.check_chunk_file` refuses `chunk-file` a chunk file whose
  `calls` do not sum to the row's `expected_calls` (so every `v2-chunk` and `verify-v2`-adjacent reader path refuses it); `verify-v2` already
  compared the file byte for byte. (2) **R106**: a value written for a `Bytes<N>` position whose length is not `N` (`N` after static-width
  substitution) is `generation` / `W0701: a stream value does not have the declared Bytes<N> length`: in `stream.make_encoder` (a claim's
  expected stream at any depth: `Option`, `Result`, tuple, record, variant, list) and in `corpus_v2._check_value` (an example's arguments and
  expected value). Plain `Bytes`, a still-named width and an omitted example are not checked. (3) **R108**: `conformance.check_type_table`
  refuses a `types` table that holds a recursive or mutually recursive record or variant (also through `Option`, `Result`, a tuple or a list), a
  repeated field or constructor name, or a field naming an undeclared type: `type` in `v2-value` (after `malformed-json`, before the type or
  any value is read), `corpus` in `v2-chunk`. `v2-ir` runs it first in `compile_ir_case` with code `ill-typed` (whether or not the case reaches the table, so a used bad record is `ill-typed`, no longer `type`, which is outside the `v2-ir` code set). A malformed row (a non-object row a field names, an unhashable name) is a refusal with the context's code, never a crash. The `RecursionError` of a record
  containing itself and the uncaught `Refused` of an undeclared field type in an unused record are now outcomes. **Not table-checked yet:**
  `v2-fault`'s `types` table (its closed code set has no code for it, so it needs a spec ruling), a future `v2-domain` `types` (the category has
  none today) and `v2-binding`'s `corpus.types` (S5d's reader). No committed byte moves. Tests: `tests/test_cv2_16_21_checks.py`.

### `ir.tuple`: tuples and `match` on a `Result` (slice S10t-1)

* **Forms** (Appendix A.1, R2): the type `(T1,..,Tn)` (2 to 8 components), the literal
  `["tuple", [e1, .., en]]`, `["project", e, i]` (`t.i`, a chain `t.0.1`, or `f(x).1`), values as
  Python tuples and JSON arrays, stream tag `64` with a `u32` count, TFV `L<n> v1 .. vn`, the
  descriptor `tuple` (Cartesian product, last component fastest) and component-wise `==`/`!=`.
* **`match` on a `Result`** with the patterns `ok err err_any tuple bind wild`
  (`["match", s, [[pat, arm], ..]]`) in claims, models, op `admit`s and examples. It is `ir.tuple`
  exactly when the scrutinee is a `Result` (D35); any other scrutinee is `ir.data`. Cost is
  `1 +` the scrutinee `+` the maximum over the arms; exactly one arm is evaluated. A literal pattern
  (`Ok(true)`) is not one of the six and is `ir.data`; `Err(e)` is `err_any`, and `e` may not be used
  (an error value has no IR form).
* **Corpus errors** (code `corpus`, checked before any arm body is lowered, `patterns.py`): a match
  that can fall through, an arm no value can reach (the usual usefulness check), the same `bind`
  twice in one pattern, and a `bind` that shadows a binder, a parameter, `result` or an outer `bind`.
* **Lowering-time `W0701`** (CV2-F1): a subject call under an impure guard (VER04r 4.4) and a claim that
  calls no op (4.3) are refused when the claim is lowered, as the generator refuses them (recorded in
  `LoweredSuite.refusal`, raised by `derive`, `emit-v2` and `dump-lowering`). A non-omitted example with a
  `Bytes` over 4,096 bytes is a `limit` error (VER04r 2.3).
* **A `match` arm is a guard** for call counting (VER04r 4.4): a subject call in an arm needs a pure
  scrutinee, and then only the arm the scrutinee selects counts; the wrap trace, the D19 admit check
  and the v0 rule-3 walk follow the same arm (`irtree.children` and `ir_eval.walk_scoped`).
* **Emission**: `derive`, `emit-v2` and `verify-v2` evaluate `ir.tuple` suites and examples
  (`levels_implemented` lists `ir.tuple`; since S10c-1 also `ir.fixed`).
* **`requires`** (R9) was already type-directed since S10a; S10t-1 pins every place a tuple type
  can hide (binder, op or model parameter or result, an example value, a pattern binder) and a
  fault row's `requires` now also counts the types of its op.
* **Faults** (`faults.py`): `ok-as-err`, `drop-consumed`, `swap-components`, the tuple-payload
  `missing-error` (R3) and the `Ok` forms of `flip-result-bit` and `drop-last-result-byte` are
  instantiated as data. **Their evaluation is gated** exactly like `ir.fixed`'s: since the atomic flip (S10c, 0.5.0)
  `IMPLEMENTED` is `("ir.core", "ir.fixed", "ir.tuple")`, so their controls carry witness chunks; under the
  closed gate `derive(..., implemented=("ir.core",))` (tests and the committed dumps) a control that needs
  `ir.tuple` has `{"claim": C, "deferred": "ir.tuple"}` witnesses.
* **`off-by-one-consumed`** (S10o-1, VER04r 8.2 and 16.10 [D36]): the second component `n` of a tuple result,
  or of the tuple inside an `Ok`, becomes `n + 1` when `n` is even and `n - 1` when odd (a `Nat` component:
  `if unwrap_or(rem(C, 2), 0) == 0 ...`; a fixed-width one: `bit_xor` with `1`). `requires` is
  `["ir.fixed", "ir.tuple"]`; it refuses what `drop-consumed` refuses. Section 8.2 fixes its JSON, and the
  committed dump in `tests/fixtures/off-by-one-consumed-v1/` (a directory of its own, generated by
  `make_off_by_one_consumed.py`, which `BUILD` runs with `--check`) must be reproduced byte for byte by the
  generator's slice S10o-2. The registry now holds fourteen names.
* **`saturate-add`, `saturate-sub`, `shift-mask` and the scalar `missing-error`** (S11f-1, VER04r 8.2 and
  16.14): the scalar faults of the Rust harness (`V1_FAULTS`). The saturating pair is kind `replace` over the
  first two parameters (one fixed-width type `T`; result `T` or `Result<T,E>`, wrapped in `["ok", V]`
  unconditionally, because the harness never calls the export); `shift-mask` is kind `pre` on the last parameter
  (`rem`, or `rem_trunc` when it is signed, by the width of the FIRST parameter's type). All three are
  `["ir.fixed"]` and live. `missing-error` now also takes a scalar payload (`Bool`, `Nat`, a fixed-width type:
  the first parameter of exactly that type, else the zero) and a `Bytes` payload with no `Bytes` parameter
  (the zero literal); the record, `Option` and variant payloads need `ir.data` (S11c-1, above). `Nat` parameters, mixed
  widths and other result types have no template (the code `fault`). The committed dump is in
  `tests/fixtures/scalar-faults-v1/` (`make_scalar_faults.py`, which `BUILD` runs with `--check`); the
  generator's slice S11f-2 must reproduce it byte for byte. The registry now holds seventeen names.
* **The committed dumps** (`tests/fixtures/tuple-forms-v1/`, generated by `make_tuple_forms.py`,
  which `BUILD` runs with `--check`): the document, the lowering dump, the evaluation dump
  (`parity_dumps.evaluation_view`), the fault dump (`parity_dumps.faults_view`, **normative for the
  composite fault JSON**, R11) and every corpus-error document with its refusal. Slice S10t-2 (the
  generator) must reproduce them byte for byte. They carry no generator name or version.

### `ir.fixed` lowering, `requires` and the lowering dump (slice S10a)

* **Lowering and typing** (`lower.fixed_call`, `ir_eval.Program._fixed_type`): one IR node per
  built-in, same spelling, positional operands, a static type argument as the canonical type
  name in position 1 (`["widen", "U32", e]`, `["max_value", "I64"]`). Two independent typers
  (the source typer and the IR typer) implement Appendix A.1; `div`/`rem` on two `Nat`
  operands are `Option<Int>` (R7). `Option<T>` is an IR-only intermediate whose only consumer is
  `unwrap_or` as its first operand, directly (D23, R8); any other use, and an `Option` in a
  signature or binder, is the level `ir.data`.
* **`requires`** (`levels.ir_levels`, R9): the sorted union of the levels of every tag of a
  suite's, op's `admit`'s expressions and of the call closure of every `model_call`; a tuple
  *type* adds `ir.tuple` and a list, record or variant type adds `ir.data`; `ir.core` is always
  included. `derive` writes it into suite and op rows. Every committed fixture verifies unchanged;
  a tuple-binder document gains `ir.tuple` in `requires`/`levels`, and an `Option` binder is
  refused at `ir.data`.
* **Cost**: every new tag is `1 +` its operands (a static type argument is not an operand).
* **The dump** (`dump-lowering`, `lowering_dump`): the canonical JSON of the lowered IR, the
  `requires` and the cost of a document. `tests/fixtures/fixed-forms-v1/` holds the synthetic
  `ir.fixed` document (every one of the 35 built-ins at every width the spec allows; generated
  by `make_fixed_forms.py`) and its committed dump, which slice S10b must reproduce byte for
  byte. `BUILD` runs both `--check`s.

### `ir.fixed` evaluation, the domains of D21/D25/D26 and the `fixed-eval-v1` dumps (slice S10c-1)

* **The evaluator** (`ir_eval._fixed_fn`): all 35 built-ins of Appendix A.1 on Python integers with
  explicit masking. A fixed-width value is its mathematical integer; bit operations work on the
  `w`-bit pattern and re-read the result in the type; a shift count is an exact `Nat` compared with `w`
  **before** any shift (`shl`/`shr` give 0, `sar` gives `-1` or 0, `bit` gives false), `rotl`/`rotr` reduce
  `n mod w` first; `div`/`rem` on `Int`/`Nat` are floor division and the divisor-signed remainder,
  `div_trunc`/`rem_trunc` truncate and are `None` for `b == 0` and `MIN / -1`; `from_be`/`from_le` of a
  runtime width other than the target's is an `ir` error, never a truncation; `unwrap_or` evaluates both
  operands. **`wraps` counts only `add sub mul neg`** [D31]. Cost is as S10a defined it (`1 +` the
  operands). `Program(analyze=...)` is kept and changes nothing. `tests/test_fixed_eval.py` holds every
  built-in at every width against an independent bit-list reference.
* **Domains** (`domains.py`): the signed `fx_edge` is `wrap_S(E_U(w)) | {-255..-1}`, de-duplicated and
  ascending (I16 527, I32 559, I64 623: the generator's set) [D21]; `fx_edge` at 16 bits exists by
  budget demotion only and an 8-bit one is refused [D25]; the new leaf `{"d":"nat_shift","size":73}`
  (`0..=64` then `65 127 128 129 255 256 65535 65536`) is chosen for a `Nat` binder that **is** the count
  operand of `shl shr sar rotl rotr bit` in the claim body, and for nothing else (R10: not
  `shl(x, f(n))`, not a `model_call` argument, not a count only in an `admit`) [D26]. `descriptor_size` is
  the consumer-side check of VER04r 5.1 (sizes recomputed, depth, the legality of every leaf).
* **Emission**: `derive`/`emit-v2`/`verify-v2` accept `ir.fixed` documents. The synthetic documents
  and `lzw.warrant` evaluate (`tests/test_fixed_lzw.py`: with `omit_example` lines for the six bomb
  examples, four `value-over-limit` and two `not-op-eq-pure`, and `max_ir_cost 16384`; a full derive
  took about 21 seconds).
* **The fault gate is open since the flip (S10c, D29, 0.5.0)**: `faults.IMPLEMENTED` is
  `("ir.core", "ir.fixed", "ir.tuple")`; the `ir.fixed` templates (`xor-lsb`, `ignore-high-bit`,
  `flip-result-bit`, `drop-last-result-byte`, `truncate-last`) are searched, so ct-compare's
  `eq_u64/ignore-high-bit` is live with its witness at chunk 0 (`a = 0`, `b = 2**63`, ordinal 311).
  `verify-v2` therefore refuses a corpus that still says `deferred` for `ir.fixed` or `ir.tuple`
  (`tests/fixtures/ct-compare-v2-preflip/` is the negative test). The closed gate stays reachable through
  `derive(..., implemented=("ir.core",))`; the committed dumps pin their gate explicitly.
* **The committed dumps** (`tests/fixtures/fixed-eval-v1/`, generated by `make_fixed_eval.py`, run by
  `BUILD` with `--check`; slice S10c-2 must reproduce them byte for byte; none carries a generator
  name or version): the document (every built-in at every width, as tuples of up to 8 built-ins per
  suite, the shift-count cases, the faults), its profile (`budget 2^20`, so the 16-bit binders are
  demoted), the lowering dump, the evaluation dump (`parity_dumps.evaluation_view`), the fault dump (the
  controls under the closed gate **and** the open gate `ir.core + ir.fixed + ir.tuple`), the value table
  (`eval_dumps.values_view`, about 9,800 rows `shl U8 255 7 = 128`) and the domain table
  (`eval_dumps.domain_view`: the full `fx_edge` and `nat_shift` value lists and the descriptors a consumer
  must reject). Where VER04r 8.2 leaves a form loose, the committed dump is normative (R11).

## Usage

```bash
# The ct-compare gate: all three checks against the committed v1 corpus.
python -m coding_adventures_warrant_oracle verify-v1 \
    --spec code/specs/ct-compare.warrant \
    --corpus code/specs/fixtures/ct-compare-warrant-v1/corpus.json \
    --emit-v2 scratch/ct-compare-v2.json

# The oracle's own v2 enumeration digests for any document (slim or full depth),
# ready to diff against S2/S3 generator output.
python -m coding_adventures_warrant_oracle emit-v2 \
    --spec code/specs/gf256.warrant --profile corpus.profile \
    --out scratch/gf256-v2.json --depth slim

# Exit 3 means some suites were not-live or refused; --allow-partial accepts that.
# Parse and lower a document; print what level each suite needs.
python -m coding_adventures_warrant_oracle check --spec code/specs/lzw.warrant

# The canonical lowering dump of a document (S10a): write it, or check a committed one.
python -m coding_adventures_warrant_oracle dump-lowering \
    --spec tests/fixtures/fixed-forms-v1/fixed_forms_v1.warrant --check \
    tests/fixtures/fixed-forms-v1/fixed_forms_v1.oracle-lowering.json
```

Exit status: `0` the gate passed, `1` a mismatch, `2` an input was unreadable or
refused (size cap, symbolic link, malformed document, an output path that is an
input or lies in `code/specs`), `3` (`emit-v2`) some suites were `not-live` or
`refused`. Failure messages name a
suite and a check, never input bytes.

### The ct-compare gate

`verify-v1` reproduces, from `ct-compare.warrant` alone, everything the generator
committed in `ct-compare-warrant-v1/corpus.json`:

1. **Lowering**: operations, types, models, examples and every suite's
   `admit`/`lhs`/`rhs`/binders equal the corpus IR as canonical JSON.
2. **Enumeration and evaluation**: `cases`, `discarded`, `expected_calls`,
   `inputs_sha256` and `expected_sha256` of all nine suites (572,109 cases, about
   1.3 s), computed through the *chunked* enumeration path, so it also proves that
   chunking reproduces the v1 order (VER04r section 1.2).
3. **Faults and controls**: the fault registry and the eight control rows.

## Findings recorded while building it

These are facts the S3/S8 owners need; none is hidden by a flag.

* **`MAX_IR_COST = 2,000` cannot hold gf256 as first written (VER04r 16.1 amended).**
  Charging both branches of an `if` (the VER04i 6.2 reading) `xor8` costs 294,900.
  The amended rule is the **worst path** (one branch of an `if`): `gmul` costs
  2,011, `ginv` 26,204, the largest gf256 expression 26,210, and four of its 38
  suites exceed the default 2,000, so `emit-v2` records them `refused: limit`.
  A corpus raises the bound only with a `max_ir_cost N` line in `corpus.profile`
  (`--profile`; 2,000 to 65,536); there is no command-line or environment
  override. Each suite row carries `ir_cost` (admit + body + expected) and the
  total work (cases and IR work) is metered against `MAX_TOTAL_WORK_*` and
  `MAX_IR_WORK_*`.
* **gf256 never wraps** (`wraps == 0` on every suite it enumerates), so the VER04r
  3.3 guard for a v0 document passes rule 2.
* **Composite demotion differs from VER04j.** VER04j 14.1 item 20 demotes both
  components of `(U16, U16)` at `2^26` because the Rust generator stores a
  composite domain as one vector under a 64 MiB byte budget. VER04r enumerates
  chunk-wise and never materialises a domain, so this oracle demotes by product
  size only. If the generator keeps the VER04j rule the descriptor comparison will
  say so.
* **Reserved words.** VER04 section 2 reserves `op`, but `arithmetic.warrant` has a
  parameter named `op`; the reader treats declaration keywords as contextual.
* **Assumptions where the spec is silent** (each is a place the first v2 corpus may
  disagree, which is exactly what the gate is for): a bare decimal literal is typed
  `Nat` when its context is `Nat` and `Int` otherwise; several refinements fold
  left into binary `and`; an unsigned `neg` is allowed at the IR level and refused
  by the source checker; `wraps` counts subject-independent nodes the oracle
  evaluated (`admit`, `expected`, call-count guards); the two slim digests are
  `slim_inputs_sha256` and `slim_expected_sha256`.

## `verify-v2` (slice S4 part 2)

```bash
python -m coding_adventures_warrant_oracle verify-v2 code/specs/fixtures/ct-compare-warrant-v2 \
    --warrant code/specs/ct-compare.warrant \
    --profile code/specs/fixtures/ct-compare-warrant-v2/corpus.profile \
    [--bindings code/packages/rust/ct-compare/evidence.bindings] [--repo-root .] \
    [--expect-generator warrant-corpus-gen@0.6.0]
```

`spec` in the corpus must equal the repo-relative path of `--warrant` (root: nearest ancestor with a
`code/` directory, or `--repo-root`). The spec's effective size cap is the lexer's 1 MiB.

The oracle **derives** the corpus (`corpus.json`, `chunks/*.txt`, `cases.tfv`) from the
`.warrant` source and `corpus.profile` with its own reader, lowering, evaluator,
enumeration, stream writer, fault registry and witness search, then compares the result
with the directory. It reads no generator code and no generator output beyond the
directory it is asked to check, so a bug the generator wrote into a corpus is a mismatch
here, not a shared assumption. The comparison groups what it finds by the three checks of
VER04r section 10.3:

| Check | Group | What is re-derived and compared |
|---|---|---|
| 1. Lowering | `lowering` | `types ops models model_order examples`, each suite's `id kind claim ops requires admit body oracle result_type`, binder types, and `cases.tfv` |
| 2. Enumeration and evaluation | `enumeration` | `budget chunk_size max_ir_cost`, each binder's resolved descriptor, `cases discarded expected_calls wraps ir_cost product chunks stream domain_label demoted`, every chunk line (`first_admitted`, `calls`, both chunk digests), `chunks_sha256`, the suite digests, the v1 whole-suite digests, `slim_chunks` and both slim digests |
| 3. Faults and controls | `faults`, `controls` | every fault row (incl. `rewrites` and the opaque `ir.fixed` expressions), every control row, `required_claims`, `blind_claims`, `slim_witness`, `accepted_blind_faults` (the control ids of the profile's `accept_blind` lines whose fault is all-blind, sorted; `[]` for ct-compare) |

Also: `header` (identity and digest binding: `profile module document spec spec_sha256
profile_sha256 generator levels assumes`), `files` (exactly the derived chunk files, regular
files only, size caps, no symbolic links), `bytes` (the canonical writer: key order,
indent, LF, final LF) and `drift` (`corpus.profile` against the bindings' `faults=`).
Exit status `0` pass, `1` mismatch, `2` an input was unreadable or the oracle refuses the
document (a level above `ir.core`, a resource bound, a generation error).

Bytes are compared exactly where VER04r fixes them (chunk files, `cases.tfv`, the canonical
form of `corpus.json`) and structurally, type-strict and closed, where the `corpus.json`
key set is provisional (section 1.4). The `generator` key (name and version) cannot be
derived and is checked for shape only, unless `--expect-generator name@version` is given
(owner decision [D13], VER04r 16.8): any other recorded pair is a `header` difference at
`generator`. The ct-compare BUILD passes it, so a generator version bump fails until the fixture
is regenerated and the pin reviewed with it.

A fault whose level the **fault-evaluation gate** (`faults.IMPLEMENTED`, `ir.core` only until the flip)
lacks (`ir.fixed` or `ir.tuple`, e.g. ct-compare's `ignore-high-bit`) is
**instantiated and compared as opaque data, never evaluated**; its candidate claims are
`deferred` (VER04r 10.4). The witness search is the one of section 5.6: the lowest admitted
**product ordinal** at which the faulted expected value differs, as a chunk; a claim is
*witnessed*, *blind* or *unproven*, and a fault every candidate is blind to fails generation.

Gaps closed by part 2 (all were listed in VER04r 16.3 to 16.5 as "S4 part 2"): `corpus.profile`
strictness (repeated `fault`/`slim_chunk` lines, an empty token, `budget 0`, `slim_stride 0`
are errors); `concat` is evaluated on the pure trace (a result past `MAX_IR_VALUE` in a
subject argument fails generation); v0 rules 2 and 3 of section 3.3 (also through a
`model_call` whose callee's call closure wraps); the IR depth is 64 levels, not 65; the
`slim_stride` clamp per suite; witness chunks and `slim_chunk` lines in the slim set.

`accept_blind <op> <fault>` ([D15], VER04r 16.8) is a profile line: a fault every candidate claim is
blind to passes when named (its control row has no `required_claims`; its control id goes in
`accepted_blind_faults`) and fails generation otherwise; naming a fault a claim can see, one with
no `fault` line, or repeating the line is a profile error (`corpus`).

`omit_example <id>` ([D22], [D34], VER04r 8.4, S10-1) removes one example from `examples` when the
tool can decide that it is inexpressible. The line carries only the id; the **reason is computed**:
`not-op-eq-pure` (the shape is not `op(args) == pure`; tested first) or `value-over-limit` (the
normative order of 8.4: arguments left to right then the pure side; a non-size error refuses; a
size error, `errors.SizeError`, qualifies only if it is the first error; on a completed evaluation
the D18/D19 checks run and refuse first; only then does an argument or pure value over 4,096 bytes
qualify). An unknown id, a repeated line, a malformed line and a line for an emittable example (a stale
profile) are errors (`corpus`); a D18/D19 failure or any other error still refuses generation
(`generation` or the error's own code). `corpus.json` gains the **conditional** key
`omitted_examples` (`[{"id", "reason"}]`, sorted by id) only when an example is omitted, so no
committed fixture moves a byte; `verify-v2` recomputes it, rejects `[]`, unsorted or duplicate rows,
extra row keys and unknown reasons, and ties it to `profile_sha256` through the derived comparison. An
omitted example is neither evaluated for `cases.tfv`, encoded nor admit-checked. `verify-v2` prints the
omitted count after the suite count when it is non-zero (this oracle has no `not_live` report; the lane
reports of S9 show the count beside it). `tests/fixtures/omit-example-v2` is the agreement fixture the
generator's parity PR is compared with.

Tests use `tests/fixtures/synth-v2` and `tests/fixtures/accept-blind-v2`, small synthetic
documents and the corpora the Rust generator produced from them (the first's README says how; the
second is regenerated byte for byte by the generator's tests, so the two implementations agree on
`accept_blind`). The committed ct-compare fixture, `code/specs/fixtures/ct-compare-warrant-v2/`
(CV2-S3d), is verified by a test and by this package's `BUILD` with `--profile
<fixture>/corpus.profile` (an edit to the hand-written profile fails through `profile_sha256`, [D14]).
The second adopter's fixture, `code/specs/fixtures/x25519-warrant-v2/`, is verified by a second line of the
same shape (with `--bindings code/packages/rust/x25519/evidence.bindings`).
The third adopter's fixture, `code/specs/fixtures/lzw-warrant-v2/` (21 suites, 17,563,819 cases, six
`omit_example` lines, `max_ir_cost 16384`), is verified by a third line of the same shape (with `--bindings
code/packages/rust/lzw/evidence.bindings`); its full derive takes about 20 s, so no sharding is needed
(VER04r 16.10 [D33]).
The fourth adopter's fixture, `code/specs/fixtures/wasm-leb128-warrant-v2/` (44 suites, 46,566,930 cases, 740
chunks, 20 faults including the composite ones and `off-by-one-consumed`), is verified by a fourth line of the
same shape (with `--bindings code/packages/rust/wasm-leb128/evidence.bindings`). Its full derive takes **4 min
53 s** on one core (15 times lzw's), by far the most expensive line of this package; it is one line, not
sharded (VER04r 16.11 [D38]), and sharding by chunk stays the documented remedy if it ever stops fitting.

## Two refusals (S3e, owner decisions D18 and D19)

The same two rules as the generator, derived independently: an example whose evaluation wraps is
`W0701` in a v0 document (`f(255u8 + 2u8) == 1u8`) and accepted in v1; and a claim or example that
calls an op outside that op's `admit` is `W0701` naming the claim, the op and the first
admitted-case ordinal (`SuitePlan.check_admits`, `ir_eval.compile_admit_check`), with a
subject-dependent argument the op's `admit` reads refused statically (VER04r 4.5 and 16.9).
`tests/test_admit_calls.py` holds the same documents as the generator's `tests/admit_calls.rs`.

## The v2 conformance kit (S5a, S5b, S5e)

VER04r section 11 adds six categories to the shared fixture family `warrant-corpus-conformance-v1`:
`v2-value`, `v2-arith`, `v2-ir`, `v2-domain`, `v2-chunk` and `v2-fault`, and a seventh, `v2-binding` (S5c derives it,
S5d commits it: see "The reference binding validator" below). All seven files are committed: 958 cases, 1,419 in the
family with the four v1 categories.
The oracle **derives every expectation**: `src/coding_adventures_warrant_oracle/conformance.py` is the
derivation module, and the hand-written case tables, the three small kit documents the chunk cases come from
and `make_conformance.py` live in `tests/fixtures/conformance-v2/` (its README says how to add a case).

| Category | The module derives it from |
|---|---|
| `v2-value` | the strict JSON reader of VER04i 9.1, the section 2.3 decoder (`decode_int_text` and the typed forms), the section 2.4 stream encoder |
| `v2-arith` | one node of the typed IR evaluator over two `lit` operands |
| `v2-ir` | the IR evaluator with each `subject_op` rewritten to a pure stand-in model for the value, and the pure call trace of section 4.4 on the unrewritten tree for `calls` |
| `v2-domain` | a descriptor-JSON reader over `descriptor_size` and the leaf constructors, then `size()` and `nth()` |
| `v2-chunk` | the chunk-file check of sections 5.5 and 14, then `SuitePlan.run_chunk` (one chunk, by unranking) |
| `v2-fault` | the existing fault templates and the three wrappers of section 8.1 |

A table row states a case's inputs and its **intent**; the oracle derives the outcome independently, and a row
whose intent disagrees fails the build naming the case id. The oracle's own codes are mapped to the kit's closed
vocabulary (section 16.13 R25) in `classify_ir_refusal`; each kit case triggers exactly one error.

```bash
python3 tests/fixtures/conformance-v2/make_conformance.py --write --fixture DIR   # the six v2-*.json files
python3 tests/fixtures/conformance-v2/make_conformance.py --check --fixture DIR   # rebuild and compare bytes
```

`--fixture` defaults to the family directory `code/specs/fixtures/warrant-corpus-conformance-v1`; `--check` also
requires an `index.json` that is present to hold one row per derived file and skips that check when there is none.
Exit codes: 0 identical, 1 the bytes differ, 2 a table and its derivation disagree or any other error. The script
never writes `index.json` (the family's v1 script does).

S5b commits the six `v2-*.json` files (734 cases) and their rows of `index.json`. S5e's oracle PR (this package)
adds 48 cases to the tables, 782 in all (`v2-ir` 248, `v2-domain` 162, `v2-chunk` 74, `v2-fault` 41, the other two
unchanged), and the derivation changes they need (VER04r section 16.18 R69 to R77: a `width_params` op in the kit,
`level-missing` and `requires` by what `ir` reaches, the `errs` rules, the chain depth of an uncalled model); the
committed `v2-*.json` files and `index.json` are the fixture PRs': S5e's regenerated the six (782 cases) and S5d
(`v2-binding.json`, 176 cases) brought the family to 1,419. **Regeneration is two steps**:
`make_conformance.py --write` (the seven files), then `python3 code/scripts/warrant_corpus_conformance.py --write`
(`index.json`: the four v1 rows, then one row per `v2-*.json` on disk in the fixed order). `BUILD` runs
`make_conformance.py --check --fixture ../../../specs/fixtures/warrant-corpus-conformance-v1` (a second derivation of
the `2**24` suite, about a minute, that the pytest session fixture does not share), the v1 checker
`code/scripts/warrant_corpus_conformance.py` (which indexes and structurally checks the `v2-*` files without importing
the oracle or a lane) and its tests, so either tool alone catches a stale index. An edit to the fixture directory,
the checker or its test selects this package (build-tool `isWarrantOracleInput`).

`tests/test_conformance_mutants.py` is the mutant test of [D42]: sixteen named wrong evaluators (no wrap,
saturation, a signed compare on the bit pattern, eager `and`, first-binder-fastest enumeration, a missing chunk
index in the stream prefix, sequential `pre` rewrites ...), each of which must change the derived expectation of at
least one case of each category it is named for. `tests/test_conformance_cross.py` re-derives slices of the kit by
different algorithms (Python integers, `struct.pack`, a naive `itertools.product` enumerator, the cost model written
out) and runs the v1 checker's strict reader and canonical writer over the v2 files.

## The reference binding validator and `v2-binding` (S5c)

`src/coding_adventures_warrant_oracle/bindings_v2.py` is the **reference binding validator** of VER04r section 7
([D43]): it reads the bytes of an `evidence.bindings` file, the `types` and `ops` of the corpus it is validated
against and a `host` object, and returns the **normalized** binding or one refusal, `BindingInvalid` with a `reason`.
It is **lane-neutral** (it names no host language: what a lane alone knows arrives as four closed lists in `host`:
`values`, the host representations allowed per Warrant value name; `exports`; `discriminators`; `immutable`), pure
(no file, no lane import) and **closed**: the only refusals are the `reason` values below. It is used only to derive
the kit; `verify-v2` does not call it.

```python
from coding_adventures_warrant_oracle import bindings_v2 as BV

BV.validate_binding(binding_bytes, {"types": {...}, "ops": {...}}, host)   # the normalized dict, or BV.BindingRefused
BV.violations(binding_bytes, corpus, host)                                  # every violation, in the order of the checks
```

`tests/fixtures/conformance-v2/cases_binding.py` is the hand-written `v2-binding` table (176 cases: 30 accepted, 146
refused; every one `requires` `["ir.core"]`; the normalized form of an accepted case is written by hand and compared by
equality, never by sub-object). A refused case is the valid base binding of its corpus with **one** line
changed, and the build demands that the validator finds exactly one violation in it, so no case depends on which of two
problems an implementation meets first. `make_conformance.py --write` writes `v2-binding.json` beside the six other files;
`--check` holds it to the derivation like every other category: S5d commits the file and removes the "not committed
yet" branch S5c had (VER04r section 16.19 R93), so a missing file, a missing index row, or both together, is exit 1.
The v1 checker `code/scripts/warrant_corpus_conformance.py` indexes the file, requires every case id of every category
to be printable ASCII (U+0021 to U+007E) and checks each `v2-binding` `expect.reason` against the 39 names above (a set
it carries as a literal, held equal to VER04r R85 (d) by its tests); the closed set of the four v1 categories applies to
them only.

### The `v2-binding` reason set

The set is closed (VER04r section 16.13 R20, fixed by S5c). The checks run in this order, and the first violation is
the one `validate_binding` raises: the bytes (`too-large`, `invalid-utf8`, `bom`, `cr`, `control-character`; one
violation, then stop); the lines (`syntax`, `unknown-directive`, `duplicate-directive`, `bad-value`, and `op-key` or
`type-shape` for a repeated or unknown key; every line is read and, if any is wrong, the model is not checked); then
the model (required directives, levels, `values`, `type` lines, what the signatures need, the `op` lines in corpus
order).

| Reason | The refusal |
|---|---|
| `too-large` | more than 65,536 bytes (a binding is size-capped before it is parsed, section 14) |
| `invalid-utf8` | the bytes are not UTF-8 |
| `bom` | the text starts with a byte order mark |
| `cr` | a carriage return anywhere: the file is LF-only |
| `control-character` | any other C0 control character, or DEL (tabs included: the separator is one space) |
| `syntax` | a line that is not `directive` plus single-space-separated printable tokens, or a token that is not of the shape its directive needs (`key=value`, a list, a name) |
| `unknown-directive` | the first token is not a directive of section 7.1 |
| `duplicate-directive` | a one-line directive twice, or two `op` lines for one op, or two `type` lines for one type |
| `bad-value` | a directive argument that is not of its form (`depth`, `seed`, `source`, `lane`, `package`, `module`, a repeated level) |
| `missing-directive` | one of `lane package module import source seed depth levels` is absent |
| `level-unknown` | a `levels` entry that is no level of section 4.1 (reserved ones are known) |
| `level-implication` | a level declared without a level it needs (the lattice of section 4.1) |
| `values-name` | a `values` key that is not a Warrant value name, or is given twice |
| `values-representation` | a `values` host name outside that Warrant name's closed list |
| `values-missing` | a Warrant value name the corpus signatures need has no `values` entry |
| `type-unknown` | a `type` line for a name that is no record or variant of the corpus |
| `type-shape` | a `type` line whose `host`, `fields` or `ctors` is missing, extra, repeated or does not match the corpus declaration |
| `type-missing` | a record or variant an op signature uses has no `type` line |
| `op-unknown` | an `op` line for an op the corpus does not declare |
| `op-missing` | a corpus op with no `op` line (the binding is **total**, VER04i section 5 rule 1) |
| `op-key` | an `op` key that is unknown or given twice |
| `export-missing` | an `op` line with no `export=` |
| `export-unknown` | an `export=` or `type host=` name that is not in the host's export list |
| `args` | `args=` of the wrong arity, or a representation not allowed for that parameter |
| `ret` | `ret=` that is not a representation allowed for the result |
| `width` | `width=` absent on an op with `width_params`, present on one without, or not `implicit` or `runtime` |
| `errors-form` | more than one of `errors=`, `error=`, `err_any=` on one op |
| `errors-missing` | an op whose result is a `Result` and which maps no error |
| `errors-unexpected` | an op whose result is not a `Result` and which maps an error |
| `errors-case` | a case that the op's error type does not declare, a case mapped twice, a malformed `CASE:discriminator` pair |
| `errors-incomplete` | an `errors=` that leaves a declared case unmapped (VER04i section 5 rule 1) |
| `discriminator-shared` | two cases of one op with the same discriminator (section 7.5) |
| `discriminator-bare` | `Error`, `Exception` or `BaseException` as a class: a binding cannot swallow arbitrary failures (section 7.5) |
| `discriminator-unknown` | a discriminator not in the host's closed list (a class that is not found) |
| `err-any-multi` | `err_any=` on an error type with two or more cases (section 7.5) |
| `panics-form` | `panics` as an `errors=` discriminator, or `error=` on an error type with two or more cases (`panics` is only for a single-case `error=`) |
| `inout-param` | `inout=` that names a parameter that is not `inout` in the corpus, names one twice, is malformed, or leaves an `inout` parameter without a form |
| `inout-form` | a form that is not one of `mutable growable cell returned`, a reserved record form, or a form that does not fit the parameter's type |
| `fresh` | `fresh=` on an op that is not `fresh` in the corpus, a value other than `probe` or `immutable`, or `immutable` for a mutable representation |

The level names and the lattice are those of VER04r section 4.1, reserved levels included: every level other than
`ir.core` needs `ir.core`, `ir.data` needs `ir.tuple` and `ir.list` needs `ir.data`, each checked directly. The
normalized shape (`lane`, `package`, `module`, `import`, `source`, `seed` as decimal text, `depth`, sorted `levels`,
`values`, `types`, `ops`) is in the module docstring. A lane's own validator must agree with this one on every case of
`v2-binding.json` (S6b and S7b).

## Not yet done

* `inout`/`ensures` monitoring (S12): a document that needs more than `ir.fixed`, `ir.tuple` and `ir.data` is refused
  at evaluation, naming the level, and the `ir.list`, `ir.seq`, `err-value` and `fixed-cast` constructs are refused at
  lowering. The fault-evaluation gates for `ir.fixed` and `ir.tuple` opened at the atomic flip (S10c) and the one for
  `ir.data` in S11c-1 (no flip, [D52]).
* The oracle's `domain_label`/`depth` rows are emitted but not yet compared with a
  lane manifest.
* The shape of `assumes` (VER04r 16.4 item 4); `verify-v2` expects `[]`.

## Security

The oracle parses a `.warrant` file from a pull request, so it treats it as
hostile: size cap (1 MiB), token cap (100,000), identifier (128 bytes) and integer
(4,096 digits) caps, nesting cap 64 (types, expressions, patterns, IR), no `eval`,
no `exec`, no regular expression on attacker text, no subprocess, no network, and
file access limited to the two paths on the command line (regular files only,
symbolic links refused). `MAX_IR_VALUE` is checked before any allocation, so a
`concat` chain cannot exhaust memory.

## Tests

```bash
uv venv --quiet --clear && uv pip install -e ".[dev]" --quiet
.venv/bin/python -m ruff check src tests
.venv/bin/python -m pytest tests/ -v --tb=short
```

Parser goldens over every adopter, evaluator vectors for the section 3.4 wrapping
cases, enumeration-order tests against a naive reference enumerator, chunk
independence and boundary tests, resource-bound and hostile-input tests, and
seeded-wrong corpora that the gate must reject. Coverage is held at 95 percent.
