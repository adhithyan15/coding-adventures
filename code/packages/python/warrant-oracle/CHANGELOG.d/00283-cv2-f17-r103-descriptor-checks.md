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
- **Not in this change:** the refusal of `Int` outside a `lit` type and a `none` argument (VER04r 2.1) is not made: the committed
  `data-forms-v1` fixture declares ops, binders and a record with `Int` and `Option<Int>`, which would move its bytes, and committed adopters
  write `Int` in model signatures. Reported to the owner of CV2-F17.
