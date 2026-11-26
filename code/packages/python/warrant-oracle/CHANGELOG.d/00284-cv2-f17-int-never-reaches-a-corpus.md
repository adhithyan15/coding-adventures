## Unreleased - `Int` never reaches a corpus (CV2-F17, VER04r 2.1)

- **`derive` refuses it at generation, lowering still accepts it.** `corpus_v2.refuse_int_in_corpus(spec)` runs after lowering and before anything is
  built: `Int` at any depth (`Option`, `Result`, a tuple, an alias) in a record or variant field (the `types` table), an op parameter or result, or a
  binder is `generation` / `W0701` ("`Int` is model-internal and may not be <where> of a corpus"). Before, `record R(i: Int)` derived a corpus whose
  `types` row carried `"Int"`. The committed `data-forms-v1` is a lowering-only fixture and is untouched. It is a document check of `derive` only:
  `emit-v2` still reports per suite exactly as at base (the 8-binder precedent); a test pins it, and `emit` of the 76 `code/specs/*.warrant`
  documents is byte-identical to base.
- **Not checked, and why.** A model's *result* is never written to a corpus, so nothing is wrong. A model's *parameters* ARE written
  (`corpus["models"][name]["params"]`), contrary to [D23] ("`Int` never appears in an op or model signature"); but `arithmetic` (`fits`, `fits16`)
  and `fixed_forms_v1` (six models) use them, so that case is left for an owner ruling and a backlog item. A `lit` type and a `none` argument are IR
  nodes and are never looked at.
- **An expected value must be a value of its type (W0701).** `stream.make_encoder` range-checks every scalar (`Nat`, `U8..U64`, `I8..I64`); before,
  `claim f(a) == a - 1` or any value over 2^64 crashed with a raw `OverflowError`, and a signed value was silently masked. An example's arguments and
  expected value are checked the same way, so `example e: f(3) == 2 - 5` with `f: Nat -> Nat` is refused instead of writing `"expected": "-3"` and the TFV
  `Kexpect I-3`. A value of type `Int` in `value_json`, `tfv.value` or the encoder is `generation` with the `W0701` prefix. No committed byte moves.
- Tests: `tests/test_int_not_in_corpus.py`; `tests/test_omit_example.py` raises its exact-window overflow inside a model (`ov`) because its ops may no
  longer take an `Int`.
