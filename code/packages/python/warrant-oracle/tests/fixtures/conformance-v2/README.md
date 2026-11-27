# conformance-v2: the case tables of the v2 conformance kit (VER04r section 11, slice S5a)

This directory holds the **inputs** of the seven v2 categories of `warrant-corpus-conformance-v1`
(`v2-value`, `v2-arith`, `v2-ir`, `v2-domain`, `v2-chunk`, `v2-fault`, `v2-binding`) and the script that turns them into
fixture files. It holds no fixture JSON: S5b committed the files into
`code/specs/fixtures/warrant-corpus-conformance-v1/`, and `BUILD` runs `--check` against that directory.
Regeneration is two steps: `--write` here, then `python3 code/scripts/warrant_corpus_conformance.py --write` for `index.json`.

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

**`Row.requires` (S10d).** A row of S5 states no `requires` and must compute `["ir.core"]` (R24). A row of a later slice states the `requires`
it expects (`row(..., requires=[...])`; `["ir.core"]` is stated too, for the S10d cases that need no other level), and `finish_case`
fails the build, naming the id, if `requires_of` computes another. S10d's rows are appended after S5's in each table, so no id and no byte
of an earlier case moves.

To add a case: append a row to the table of its category, run `--check` into a scratch directory (it fails naming
your id if your intent is wrong), and, for a new mandatory vector, say which row of the disposition table of section 11
it discharges. A slug matches `[a-z0-9]+(-[a-z0-9]+)*`, ids are `<category>/<slug>` and unique across the family, a
summary is one printable ASCII line of at most 240 characters, and every S5 case has `requires` `["ir.core"]` (R24): a
case that needs another level belongs to S10d or S11 and fails the build here.

| File | Contents |
|---|---|
| `cases_value.py` | `v2-value`: `type`, `types`, `input_hex`; the strict reader, section 2.1 type text, section 2.3 decoder, section 2.4 stream |
| `cases_arith.py` | `v2-arith`: one node over `lit` operands; the ten slugs of section 3.4 |
| `cases_ir.py` | `v2-ir`: levels, IR, env, models, ops, `subjects` (the pure stand-in per op), `max_ir_cost`, `role`; every Appendix A and A.1 tag and the nine error codes |
| `cases_domain.py` | `v2-domain`: descriptor JSON, binder type, `index` or `range`; every `ir.core` leaf |
| `cases_chunk.py` | `v2-chunk`: `corpus`, `suite` row, `chunk_file_hex`, `chunk`; valid cases and one tamper per error |
| `cases_fault.py` | `v2-fault`: a fault row instantiated by the oracle's templates (or written by hand), the op row, args and result |
| `cases_binding.py` | `v2-binding` (S5c): `binding_hex` (the file's bytes), `corpus` (`types` and `ops`), `host` (four closed lists); derived by `bindings_v2.py`, the lane-neutral reference binding validator; every refused case changes one line of a valid base binding and triggers exactly one `reason` of the closed set listed in the package README |
| `kit.warrant`, `kit.profile` | the `v2-chunk` suites at the default `chunk_size` (a `2**24` suite among them); the profile declares no fault |
| `kit-small.warrant`, `kit-small.profile` | the suites at `chunk_size 1024` (a lane uses the recorded value); the profile declares no fault |
| `kit-width.warrant`, `kit-width.profile` | S5e: an op with `width_params` (`eq_fixed<N>`) and a ground claim, at `chunk_size 1024`. A third document, so that no byte of an S5b case moves |
| `kit-tuple.warrant`, `kit-tuple.profile` | S10d: six suites with tuple binders at `chunk_size 1024`: `pairs` `(U8,Bool)`, `triples` `(U8,U8,Bool)` (128 chunks), `nested` `((Bool,Bool),(U8,Bool))`, `mixed` a `U8` and a `(Bool,Bool)`, `result_of_tuple` `Result<(Bool,Bool),E>` and `two_pairs`; every suite `requires` `["ir.core", "ir.tuple"]` and runs under the `warrant-stream-v2` header. A fourth document, so that no byte of an earlier case moves |

`kit.warrant` is the expensive part (about a minute, 256 chunks of the `big` suite); it is derived once per run.

## `v2-binding` (S5c derives it, S5d commits it)

`--write` writes `v2-binding.json` (176 cases) beside the six files, and S5d committed it. `--check` holds it to the
derivation like the others: a row without the file, a file that differs, a file without a row, and a directory with
neither the file nor its row (the "not committed yet" branch is removed, VER04r section 16.19 R93) are all exit 1. Regeneration stays two steps: `--write`, then `code/scripts/warrant_corpus_conformance.py --write`.

## Choices the spec leaves open (see section 16.15 of VER04r)

* **Type depth and descriptor depth (R40).** A type expression counts the constructors that enclose a name: a bare
  name is depth 0, so 16 nested `Result` wrappers are accepted and 17 refused (`type-depth-16-wrappers`,
  `type-depth-17-wrappers`). A descriptor tree counts nodes on a path, a leaf alone being one level: 16 levels
  (`nested(15)`) are accepted, and `nested(16)` (17 levels, although its type text is within the type depth) and
  `nested(17)` are refused as `descriptor`.
* **`Int` literals ([D58]).** No `lit` carries more than 20 decimal digits, `Int` included. The edge of the `Int`
  window, `2^127-1`, is reached by arithmetic in `v2-ir` from three literals (`2^63 * (2^64-1) + (2^63-1)`), so the
  window-edge vectors and the `Nat` product overflow case (`nat-bound-generation-error`) stay.
* **Static widths.** A `Bytes` literal has the static type `Bytes<len>` in the oracle (Appendix A.1 note). No
  `v2-ir` result type depends on it: byte results come from variables, and a `Result` result comes from an `if`
  whose other branch has a declared type, because a bare `ok` has an unknown error type.
* **Nat arithmetic.** `Nat + Nat`, `Nat * Nat` and `Nat - Nat` have type `Int` (sections 3.2 and 8.2).
* **`v2-fault` `requires`.** A fault row's `requires` is its own levels united with `ir.core` (`["ir.core",
  "ir.fixed"]` for `xor-lsb`); the derivation checks that a row names every level its tags and types use.
* **Faults.** Every derived fault output is read back through the section 2.3 decoder: an out-of-range one fails the
  build, and the kit has no `off-by-one` on a `Nat` result of `2^64-1` (R47).
* **`ill-typed` by name (R41, R45).** An `err` whose case no error type in `types` declares, and a `subject_op` that
  names an op missing from `ops`, are `ill-typed`; every other `err` case passes `types` with the case declared.
* **Schema violations are kit errors (R45 f).** A negative or non-integer `index`, `range` bound, `chunk`,
  `chunk_size` or `slim_chunks` entry fails the build (exit 2) instead of becoming a vector; "an invalid
  `chunk_size`" is a non-negative integer that is not a power of two in `2^10..=2^16`.
* **Undetermined types (R41).** A derived `expect.type` with an undetermined component (the `?` of a bare `ok`'s error
  type or a bare `err`'s payload) fails the build.
* **`v2-chunk` validation and the slim digests (R46).** A lane checks neither slim digest nor the base set; the
  tampers below re-seal them anyway so that a lane that does compare them is not what the case tests.
* **`v2-chunk` tampers.** Each tamper re-seals what it does not mean to break (the file digest and, for a changed
  `slim_chunks`, the two slim digests), so a case triggers its one intended code.

## Vectors deliberately left out because the spec does not decide them

The tables avoid a case whose expected code the text of section 11 leaves open, rather than pin a behaviour a lane
might reasonably read otherwise: a negative `index` or `chunk`, a `chunk`, `chunk_size` or `slim_chunks` entry that is
not an integer, a `chunks` count that disagrees with `ceil(product / chunk_size)` (`corpus` or `chunk-file`?), a
`discarded` that disagrees with the chunk file, a `max_ir_cost` below 2,000 in the *corpus* part of a `v2-chunk` case,
a `Bytes<07>` width, and a `level-missing` caused by a *type* (S5-0 says
the S5b vectors use tags). A later PR that settles one of them may add the case.

## S5e: the vectors of section 16.18 (the *S6a-0 additions* bullet of section 11)

Each is a case of this directory's tables, derived by the oracle (every intent is checked against the derivation); the
committed files are the fixture PR's. "Oracle change" is what the derivation or the oracle had to learn first.

| Vector | Case id (all `requires` `["ir.core"]`) | Outcome | Oracle change |
|---|---|---|---|
| R70 an op with `width_params`, `v2-ir` | `width-params-one-width`, `-width-zero` | a value, `calls` 1 | the kit's op reader reads a named width (`parse_type(widths=..)`); a stand-in has plain `Bytes` |
| R70 `v2-chunk` | `width-params-first-chunk`, `-last-chunk` | accepted (1024 cases) | third kit document `kit-width.warrant` |
| R70 `v2-fault` | `width-params-ignore-last-byte`, `-invert`, `-result-length-changes`, `-truncate-last` | args or result | a fault row is typed with each named width unstated; args and result decode as `Bytes` |
| R70 refused widths, `v2-ir` | `width-params-missing-width`, `-extra-width`, `-width-over-4096`, `-widths-on-an-op-without`, `-empty-widths-on-an-op-without` (and `-empty-widths-for-an-op-with-one`, `-width-mismatch`) | `ill-typed` | an empty widths list on an op without `width_params` is refused (`ir_eval._subject`) |
| R71 impure guard, `v2-ir` | `overflow-if-impure-guard-runs-case-class`, `overflow-and-impure-guard-runs-case-class` | `ir-overflow`, `case` | none (the derivation already said `case`) |
| R71 stand-in that skips the node | `overflow-if-impure-guard-skips-the-node` (and `overflow-and-impure-guard-skips-the-node`) | a value, `calls` 1 | none |
| R71 pure-guard twin | `overflow-if-pure-guard-corpus-class` | `ir-overflow`, `corpus` | none |
| R71 model body under a subject argument | `overflow-in-model-under-a-subject-argument-case-class` (twin: `overflow-in-model-with-a-pure-argument-corpus-class`) | `ir-overflow`, `case` / `corpus` | none |
| R72 `errs`, `v2-domain` | `descriptor-errs-empty`, `-65-names`, `-duplicate`, `-bad-name` (and `-name-starts-with-a-digit`, `-empty-name`, `-64-names-accepted`, `-underscore-name-accepted`) | `descriptor` | CV2-F17: `descriptor_size` checks 1 to 64 names, the name grammar, no duplicates |
| R72 `errs` against the declared cases, `v2-chunk` | `corpus-errs-in-another-order`, `-name-not-declared`, `-fewer-than-declared` | `corpus` | `check_corpus_and_row` compares `errs` with the error type's `cases` |
| R73 bytes `lit`, `v2-ir` | `lit-bytes-into-a-fixed-width-parameter` (accepted); `lit-written-type-bytes-n`, `lit-object-form-bool` (`ill-typed`) | | none |
| R74 count, `v2-ir` | `ill-typed-take-count-fixed-width`, `ill-typed-drop-count-fixed-width` | `ill-typed` | none |
| R75 binders, `v2-chunk` | `ground-suite-no-binder` (accepted, `cases` 1); `corpus-nine-binders`, `corpus-product-over-2-28` | `corpus` | `check_corpus_and_row` checks 0 to 8 binders and the product bound |
| R69 `level-missing` by a type, `v2-ir` | `level-missing-env-tuple-type`, `level-missing-env-list-type` | `level-missing` | static pass and `requires_of` decided over `reach()` |
| R69 a model `ir` does not call | `level-missing-model-not-reached-is-a-value` | a value | same |
| R77 uncalled chain, `v2-ir` | `depth-model-chain-65-uncalled` (and `-64-uncalled-is-a-value`) | `depth` | every model is compiled as a model, so the depth of `m65` is 65 |
| R94 common type, not flow, `v2-fault` | `common-type-not-flow-take-of-a-fixed-result` | accepted, result unchanged | the kit checks a common type (`unify`); the oracle's own fault path still compares nothing (CV2-F17) |

Left out on purpose: whether a stand-in model that holds a tag of a missing level counts as reached (R69 a names the
call closure of `ir`, which does not follow `subjects`).

## S10d: the vectors at `ir.fixed` and `ir.tuple` (VER04r section 13, 16.20 to 16.23)

Each group is a block at the end of its `cases_*.py`; every row states its `requires` (see above).  Counts are cases; "core" is a case
whose computed `requires` is `["ir.core"]` (a lane that implements `ir.core` runs it; there are 57).

| Vector | Where | Notes |
|---|---|---|
| Tuple values, stream tag `64`, arity 2 to 8, nesting, Bytes and Result components, depth 16 and 17 (R17) | `v2-value`, 49 (10 core: the `type` refusals) | a stream is the bare section 2.4 value |
| The `tuple` descriptor (last component fastest) and `nat_shift` (73 values, [D26]) (R17) | `v2-domain`, 57 (13 core: 12 `nat_shift` and one `nat_0_64`, `Nat` leaves over a `Nat` binder) | `descriptor_levels` is what levels a descriptor |
| A v2-stream chunk with a tuple binder (R17); the ops and models of a chunk corpus (R125) | `v2-chunk`, 23 (8 core: R125) | `kit-tuple.warrant` |
| Every `ir.fixed` tag: edges, the count compared with `w` before any shift, `unwrap_or` of the Option producers, `div`/`rem` floor and `div_trunc`/`rem_trunc`, `from_be`/`from_le` static width | `v2-ir` | accepted and refused rows for every tag; every width of 16, 32 and 64 bits for the bit operators, shifts, rotations and counts; `div_trunc`/`rem_trunc` and `sar` at `I16`, `I32`, `I64`; the cost of a match (the maximum over the arms) and the class of an overflow in an arm |
| `ir.tuple`: literal, `project`, equality, tuples in models and at the op boundary, `match` on a `Result` with every pattern, only the taken arm counted | `v2-ir` | |
| R76: `match` on a `Result` runs; on a bare tuple or a `Bool` it is `level-missing` (`ir.data`); at `ir.core` it is `level-missing` | `v2-ir` | requires is the case's `levels` |
| R100: a stand-in with an `ir.fixed` tag under an `ir.core` tree; an unreached stand-in counts for nothing | `v2-ir` | the oracle's kit build already counts a stand-in for `requires` and refuses what R100 and R101 rule out (CV2-F17, 16.23) |
| R111 (d), [D75]: a model with an `Int` parameter; an op with one is `ill-typed` (R122) | `v2-ir` | core |
| R120 (d): an `env` type the reader refuses (`Int`, an undeclared name, `Result<U8,Q>`, 17 nested tuples) | `v2-ir` | core |
| The `ir.fixed` and `ir.tuple` templates: `xor-lsb`, `ignore-high-bit`, `flip-result-bit` (also under `Ok`), `drop-last-result-byte` under `Ok`, `ok-as-err`, `drop-consumed`, `swap-components` | `v2-fault`, 72 in all | `level-missing` at each missing level |
| R96: `off-by-one-consumed` on a `Result<(Bytes,Nat),E>`, accepted | `v2-fault` | all four combinations of bare or `Ok`, `Nat` or fixed-width |
| R67: `missing-error` with a tuple payload and no parameter of the plain `Bytes` type | `v2-fault` | the zero literal is a plain `Bytes` |
| R96's second vector, R98's `if`, `match` arm and tuple-component refusals, R94 refusals; R121: an op row naming `Int`, an error type or an undeclared type | `v2-fault` | `ill-typed` ([D74]) |
| R119 (e): `unknown-tag`, `depth` (65 deep, with the 64-deep boundary) and `model-order` | `v2-fault` | core |

**Left out on purpose.** `v2-arith`: its closed `op` set is all `ir.core`, so it has no `ir.fixed` vector.  A match that falls through, an arm
no value can reach, the same bind twice in a pattern and a bind that shadows a name are corpus errors of Appendix A.1 (the oracle's
`corpus`, the generator's `ir`) and the closed set of `v2-ir` has no code for them: they stay oracle tests until the owner gives them one.
The `ir.data` and `ir.list` vectors are S11d's.
