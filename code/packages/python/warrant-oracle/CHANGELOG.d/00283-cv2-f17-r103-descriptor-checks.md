## Unreleased - consumer-side `tuple` and `variant` descriptor checks (CV2-F17, R103)

- **`domains.descriptor_size`** refuses, as `generation` (the kit's `descriptor`, as R72's `errs` does): a `tuple` whose `of` has no entry
  or more than 32; a `variant` with fewer than 2 or more than 64 constructors, a constructor name that is not a string or is outside the
  name grammar of VER04r 2.2 rule 4, a repeated constructor name, or a constructor with more than 8 field descriptors (VER04r 5.1, 16.20 R103).
  Before, a variant needed only a non-empty list of `[x, [..]]` pairs and a tuple a non-empty list.
- **`conformance._check_errs`** also walks a record's `tuple` and a `variant` descriptor against the `types` table: a record's `of` must have
  exactly its field count, a variant's constructors must equal the declared ones (names, order, field counts); a mismatch is `corpus`, as for
  `errs`. Field descriptors are then checked against the declared field types (so an `errs` below a variant field is compared too).
- **No committed byte moves.** The two consumer-table rows of `data_eval_v1.oracle-domains.json` that this touches (an empty `of`, an empty
  `ctors`) keep their committed messages; the new refusals use new sentences. Every `--check` line of BUILD is identical.
- Tests: `tests/test_descriptor_r103.py` (each bound at the edge and one past it, a non-string name, a duplicate, a name outside the grammar, an
  arity mismatch with and without a `types` table, the kit's `derive_domain` mapping).
- **`Int` never reaches a corpus (VER04r 2.1), refused at generation.** `corpus_v2.refuse_int_in_corpus(spec)` runs in `derive` after
  lowering and before anything is built (and at the start of `emit-v2`): `Int` at any depth (`Option`, `Result`, a tuple, an alias) in a
  record or variant field (the `types` table), an op parameter or result, or a binder is `generation` / `W0701` ("`Int` is model-internal
  and may not be <where> of a corpus"). Before, `record R(i: Int)` derived a corpus whose `types` row carried `"Int"`. **Lowering still
  accepts all of these** (the committed `data-forms-v1` is a lowering-only fixture); a model signature, a `lit` type and a `none` type
  argument are not looked at (`fixed_forms_v1` and `arithmetic` write `Int` in model signatures). No committed byte moves.
  Tests: `tests/test_int_not_in_corpus.py` (each position and depth, the same document still lowers, `emit-v2`, and models, `lit` and `none`
  still accepted); `tests/test_omit_example.py` raises its exact-window overflow inside a model (`ov`) because its ops may no longer take an `Int`.
