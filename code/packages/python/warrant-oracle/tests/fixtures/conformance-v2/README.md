# conformance-v2: the case tables of the v2 conformance kit (VER04r section 11, slice S5a)

This directory holds the **inputs** of the six v2 categories of `warrant-corpus-conformance-v1`
(`v2-value`, `v2-arith`, `v2-ir`, `v2-domain`, `v2-chunk`, `v2-fault`) and the script that turns them into
fixture files. It holds no fixture JSON: S5a writes the files into a scratch directory only, and S5b commits
them into `code/specs/fixtures/warrant-corpus-conformance-v1/`.

```text
make_conformance.py --write [--fixture DIR]   # DIR defaults to the family directory
make_conformance.py --check [--fixture DIR]   # rebuild in memory; the bytes on disk must be identical
```

Exit 0: identical (or written). Exit 1: a file differs, or the index rows of an existing `index.json` are stale.
Exit 2: a table and its derivation disagree (the message names the case id), or any other error. The script never
writes `index.json` (the family's v1 script does, section 11 R26) and skips the index check in a directory that
has none.

## The rule: inputs and intent in the table, the outcome from the oracle

A table row is `Row(slug, summary, fields, intent)`. `fields` are the case's inputs (the category's schema of
section 11), `intent` is the outcome the author expects, written as a (sub-)object of `expect` from the spec and by
hand. `coding_adventures_warrant_oracle.conformance` derives the real outcome from the inputs, independently, and the
build fails if the intent is not contained in it. Keys an author cannot write by hand (a SHA-256) are left out of
the intent. **Nothing here is ever edited in a generated file.**

To add a case: append a row to the table of its category, run `--check` into a scratch directory (it fails naming
your id if your intent is wrong), and, for a new mandatory vector, say which row of the disposition table of section 11
it discharges. A slug matches `[a-z0-9]+(-[a-z0-9]+)*`, ids are `<category>/<slug>` and unique across the family, a
summary is one printable ASCII line of at most 240 characters, and every S5 case has `requires` `["ir.core"]` (R24): a
case that needs another level belongs to S10d or S11 and fails the build here.

| File | Contents |
|---|---|
| `cases_value.py` | `v2-value`: `type`, `types`, `input_hex`; the strict reader, section 2.1 type text, section 2.3 decoder, section 2.4 stream |
| `cases_arith.py` | `v2-arith`: one node over `lit` operands; the ten slugs of section 3.4 |
| `cases_ir.py` | `v2-ir`: levels, IR, env, models, ops, `subjects` (the pure stand-in per op), `max_ir_cost`, `role`; every Appendix A tag and the nine error codes |
| `cases_domain.py` | `v2-domain`: descriptor JSON, binder type, `index` or `range`; every `ir.core` leaf |
| `cases_chunk.py` | `v2-chunk`: `corpus`, `suite` row, `chunk_file_hex`, `chunk`; valid cases and one tamper per error |
| `cases_fault.py` | `v2-fault`: a fault row instantiated by the oracle's templates (or written by hand), the op row, args and result |
| `kit.warrant`, `kit.profile` | the `v2-chunk` suites at the default `chunk_size` (a `2**24` suite among them); the profile declares no fault |
| `kit-small.warrant`, `kit-small.profile` | the suites at `chunk_size 1024` (a lane uses the recorded value); the profile declares no fault |

`kit.warrant` is the expensive part (about a minute, 256 chunks of the `big` suite); it is derived once per run.

## Choices the spec leaves open (all conservative; see the SPEC-QUESTIONS of the S5a hand-off)

* **Type depth.** Section 2.2 says a type expression is at most 16 deep without saying whether a leaf is depth 0 or
  1. The cases stay clear of the one ambiguous boundary: 15 nested `Result` wrappers are accepted and 17 refused,
  whichever convention a lane uses. Descriptor trees are treated the same way (14 accepted, 17 refused).
* **Static widths.** A `Bytes` literal has the static type `Bytes<len>` in the oracle (Appendix A.1 note). No
  `v2-ir` result type depends on it: byte results come from variables, and a `Result` result comes from an `if`
  whose other branch has a declared type, because a bare `ok` has an unknown error type.
* **Nat arithmetic.** `Nat + Nat`, `Nat * Nat` and `Nat - Nat` have type `Int` (sections 3.2 and 8.2).
* **`v2-fault` `requires`.** A fault row lists the levels beyond `ir.core` when it has any (`["ir.fixed"]` for
  `xor-lsb`); the derivation checks that a row names every level its tags and types use.

## Vectors deliberately left out because the spec does not decide them

The tables avoid a case whose expected code the text of section 11 leaves open, rather than pin a behaviour a lane
might reasonably read otherwise: a negative `index` or `chunk`, a `chunk`, `chunk_size` or `slim_chunks` entry that is
not an integer, a `chunks` count that disagrees with `ceil(product / chunk_size)` (`corpus` or `chunk-file`?), a
`discarded` that disagrees with the chunk file, a `max_ir_cost` below 2,000 in the *corpus* part of a `v2-chunk` case,
a `Bytes<07>` width, a `subject_op` of an op missing from `ops`, and a `level-missing` caused by a *type* (S5-0 says
the S5b vectors use tags). A later PR that settles one of them may add the case.
