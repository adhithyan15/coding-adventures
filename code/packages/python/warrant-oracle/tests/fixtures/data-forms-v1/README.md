# data-forms-v1

The oracle's committed proof of **`ir.data` lowering** (VER04r 16.14 [D46] to [D48], R27 to R39, Appendix A.2, slice S11a).

`make_data_forms.py` generates one synthetic document, `data_forms_v1.warrant`, that uses every form of Appendix A.2, and two dumps of
what the oracle computes from it. **Appendix A.2 stops being provisional with these files**: they are its first reader, and where they
differ from the appendix the appendix is corrected by erratum before the generator's slice S11b reproduces them.

## Files

| File | What it is |
|---|---|
| `make_data_forms.py` | writes (or `--check`s) the three files below; `BUILD` runs `--check` and `dump-lowering --check` |
| `data_forms_v1.warrant` | the generated document: 7 records and variants (nested records, a record with an `Option` and a tuple field, a payload-free variant, a variant with an `Option` payload), 86 models, 88 operations, 94 claims, 11 examples |
| `data_forms_v1.oracle-lowering.json` | `lowering_dump`, schema `warrant-lowering-parity-v1`: lowered IR, computed `requires` and `ir_cost` of every model, op `admit`, claim and example, plus a `types` object (the corpus `types` rows of the records and variants) **that appears only for a document declaring one**, so every older dump keeps its bytes |
| `data_forms_v1.oracle-refusals.json` | schema `warrant-refusals-parity-v1`: 109 documents the oracle refuses, each with its code, level, message and source |

`.gitattributes` (`* -text`) keeps the bytes exact on every platform. There is no evaluation, values, stream, TFV or faults dump here: the oracle lowers, types and
costs `ir.data` in this slice and still refuses to evaluate it (`level ir.data`, R37); slice S11c-1 adds those in `tests/fixtures/data-eval-v1/`.

## What the lowering dump pins

* **The four tags and the typed `none`**: `["some", e]`, `["none", "Bytes<2>"]` (the type the context fixes), `["ctor", "Name", [..]]`, `["field", e, i]`.
* **`requires` by tag, pattern and type**: every suite of the document is `ir.core` + `ir.data` unless noted; a `match` on a `Result` is `ir.tuple`, with a data
  pattern inside it `ir.data` too; a `match` on a bare tuple is `ir.data` by its scrutinee and `ir.tuple` by its type.
* **The R29 invariant**: `c_inter_*` claims (`unwrap_or` directly over a producer) require `ir.core` and `ir.fixed` only, exactly as before the level existed;
  `c_unwrap_binder`, `c_prod_*` (a producer not directly under `unwrap_or`, or `unwrap_or` over a binder) add `ir.data`.
* **Costs**: `some`, `ctor`, `field` `1 +` operands, `none` 1, `match` `1 +` scrutinee `+` the worst arm.

## What the refusals dump pins

The reserved levels and feature names (`ir.list` 18 documents, `ir.seq` 10, `fixed-cast` 4, `err-value` 3), the corpus limits of section 2.2 (R34, 5), the typing rules of R39
and Appendix A.2 (41 `type`), the corpus errors of A.2 (28 `corpus`: fall-through, unreachable arm, duplicate and shadowing binds, over `Bool`, `Option`, records, variants
and tuples), and the **order** of R35 (the `order-*` rows: limits first; a level or feature over a `type` error of the same unit and the first in source order; patterns before arm
bodies and structure before the bind rules; unreachable before fall-through; units in declaration order).

## The contract with the generator

Slice S11b (`rust/warrant-corpus-gen`) must reproduce `data_forms_v1.oracle-lowering.json` byte for byte (`oracle_view_v1`, including the conditional `types` object), and
refuse each document of `data_forms_v1.oracle-refusals.json` with the generator code of its `generator_code` (`ir` for a corpus error, `level`, `limit`; an empty one means
the generator's own typing error, R13 (a)) in the order R35 states. The dumps carry no generator name or version. Regenerate with
`python3 make_data_forms.py`; never edit a file by hand.
