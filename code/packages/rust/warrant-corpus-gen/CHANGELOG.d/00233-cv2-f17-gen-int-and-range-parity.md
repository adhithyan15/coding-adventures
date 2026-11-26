# Unreleased: `Int` never reaches a corpus, and an out-of-range value says the oracle's sentence (CV2-F17, VER04r 2.1 and 2.3)

The generator half of the oracle's PR #20144 (`corpus_v2.refuse_int_in_corpus`, `corpus_v2._check_value`, the range checks of
`stream.make_encoder`; oracle shard `00284`). No version bump, so `generator.version` and every committed byte stay: the five
adopter fixtures (ct-compare, x25519, lzw, wasm-leb128, logic-gates-basic) pass `--check` unchanged.

## Added

- `corpus_check.rs`: `refuse_int_in_corpus(&Lowered)`, a **document check** that runs after lowering and before costing and
  emission (the position of the oracle's `derive`; `generate_gated` calls it, `analyse`, `enumerate_document` and the census do
  not, as the oracle's `emit-v2` does not). `Int` at any depth (`Option`, `Result`, a tuple, a list; an alias is resolved by the
  lowering) in a record field, a variant field, an op parameter or result, or a claim binder is `W0701`, message
  ``` `Int` is model-internal and may not be <where> of a corpus (VER04r 2.1) ``` with `<where>` one of "a record field",
  "a variant field", "an op signature", "a binder"; when several kinds are present the first in that order names it (the
  oracle's precedence). The lowering still accepts `Int`, so the lowering-only `data-forms-v1` parity tests stay green.
- Not checked, as in the oracle: a model's result and parameters (an open owner question: the parameters ARE written, contrary
  to [D23], but `arithmetic` and `fixed_forms_v1` use them), a `lit` type, a `none` argument.
- `tests/int_not_in_corpus.rs` (21 tests), a case-by-case mirror of the oracle's `test_int_not_in_corpus.py`.

## Changed

- `stream::encode_in`: a `Nat`, unsigned or signed value outside its type (past `0..=2^64-1`, past `i64`, or outside a
  fixed-width type's own range) is `W0701` with the oracle's sentence ``a value is outside the range of <type> (VER04r 2.3)``
  (before: `a stream value is outside 0..=2^64-1` and two more). An `Int` value is `an `Int` value has no stream encoding
  (VER04r 2.1)`, and `tfv::value_json` / `tfv::value_tfv` refuse it as `an `Int` value is never written to a corpus (VER04r 2.1)`;
  both are the oracle's words. The generator already refused these values (no panic, no wrap: the evaluator is `i128` and the
  2^127 window is a `W0701`); only the text changed. The refusal keeps its `at` (the claim or example), which the oracle's
  message does not carry.
- `tests/data_parity.rs`: `generate` on the `data-forms-v1` lowering fixture is now refused by the `Int` check
  (``op signature``, no `at`), as the oracle's `derive` refuses it, instead of by the resolver at `c_opt_widen_eq`.
- `tests/omit_example.rs` follows the oracle's `test_omit_example.py`: ops `nn` and `nb` take `Bytes` and the exact-window overflow is raised inside a
  model (`ov`), since an op may no longer name `Int`; `tests/enumeration.rs` expects the new range sentence.

## Parity evidence (oracle at `2d1c8bc82c5`, same documents, the generator's `W0701` after the `W0701: ` of the oracle's text)

| document | oracle (`derive`) and generator |
|---|---|
| `record R(i: Int)`, `Option<Int>`, `(Int,Bool)`, `Result<Int,E>`, `Option<Option<Int>>`, `(Bool,(Nat,Option<Int>))`, an alias `Z = Int` | both: `W0701: `Int` is model-internal and may not be a record field of a corpus (VER04r 2.1)` |
| `variant V = A(i: Int)`, `A(i: Option<Int>)`, `A(i: (Int,Bool))` | both: `... a variant field ...` |
| op parameter `Int` / `Option<Int>`, result `Int` / `Option<Int>` / `Result<Int,E>`, an unused op `f(n: Int)` | both: `... an op signature ...` |
| claim binder `n: Int` | both: `... a binder ...` |
| record and op and binder all `Int` | both: `... a record field ...` |
| `data-forms-v1.warrant` | both: `... an op signature ...` |
| models with `Int` parameters, result, `lit` and `none` | both derive |
| `example f(3) == 2 - 5` on a `Nat` result; `== 18446744073709551615 + 1`; `Some(2 - 5)` of `Option<Nat>` | both: `W0701: a value is outside the range of Nat (VER04r 2.3)` |
| claim `f(a) == a - 1`, `a * 18446744073709551615`, `a + 18446744073709551615` on a `Nat` result | both: the same; the generator does not panic |
| claim `a * 2^64-1 * 2^64-1 * 2^64-1` (past the 2^127 window) | both refuse (oracle `ir-overflow`, generator `W0701`: unchanged) |

## Known divergence, not changed (parity first)

A `Bytes<N>` value of another length is **derived by the oracle and refused by the generator**: `op f(a: U8) -> Bytes<2>` with
`example f(1u8) == b"aabbcc"`, or the claim `f(a) == b"aabbcc"`. The oracle's `_check_value` and encoder check no `Bytes<N>`
length, and the generator's `encode_in` has refused it since before this change (`a stream value does not have the declared
Bytes<N> length`). The spec decides it: the value table (VER04r 2.3) says "a `Bytes<N>` of another length, cannot be encoded
and is `W0701` at generation". So the generator is right and the oracle is the side that moves (a CV2-F17 follow-up);
`int_not_in_corpus.rs` pins the generator's side until then.
