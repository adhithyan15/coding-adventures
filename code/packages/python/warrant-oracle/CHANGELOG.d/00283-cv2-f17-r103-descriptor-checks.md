## Unreleased - consumer-side `tuple` and `variant` descriptor checks (CV2-F17, R103)

- **`domains.descriptor_size`** refuses, as `generation` (the kit's `descriptor`, as R72's `errs` does): a `tuple` whose `of` has no entry
  or more than 32; a `variant` with fewer than 2 or more than 64 constructors, a constructor name that is not a string or is outside the
  name grammar of VER04r 2.2 rule 4, a repeated constructor name, or a constructor with more than 8 field descriptors (VER04r 5.1, 16.20 R103).
  Before, a variant needed only a non-empty list of `[x, [..]]` pairs and a tuple a non-empty list.
- **`conformance._check_errs`** (the corpus reader's `types` walk) is now a strict kind-and-shape check of every binder descriptor against its
  type: a `result` for a `Result`, an `option`/`list` for `Option`/`List`, a `tuple` for a tuple type (exactly its component count, also under
  `Option<..>`) or a record (exactly its field count), a `variant` whose constructors equal the declared ones (names, order, field counts), a leaf
  for its own type; a descriptor that is not one for the binder's type (a variant descriptor for a record, a tuple for a variant, `bool` for a
  record) is `corpus`, and a malformed `types` row (`fields: [["x"]]`) is `corpus` too, never a traceback. Field descriptors are walked against the
  declared field types (so an `errs` below a variant field is compared too).
- **No committed byte moves.** The two consumer-table rows of `data_eval_v1.oracle-domains.json` that this touches (an empty `of`, an empty
  `ctors`) keep their committed messages; the new refusals use new sentences. Every `--check` line of BUILD is identical.
- Tests: `tests/test_descriptor_r103.py` (each bound at the edge and one past it, a non-string name, a duplicate, a name outside the grammar, an
  arity mismatch with and without a `types` table, wrong-kind descriptors, malformed rows, and the `desc_from_json` mapping to `descriptor`).
