# synth-v2: a synthetic warrant-corpus-v2 fixture for `verify-v2`

`synth.warrant` + `corpus.profile` are a small hand-written document; `corpus/` is the v2
corpus the **Rust generator** (`rust/warrant-corpus-gen`, CV2-S3c) produced from them. The
oracle never reads Rust: it derives its own corpus from the two input files and compares.
This directory is the *corpus under test*, committed so `pytest` needs no Rust build.

Why this document (each feature is one thing `verify-v2` must re-derive):

| Feature | Where |
|---|---|
| slim set with a stride denser than automatic (`slim_stride 12`, 236-chunk suites) and one clamped to the automatic stride (64- and 118-chunk suites) | `pick_*`, `add2_*`, `cmp_agrees` |
| a witness chunk outside the stride set (chunk 27 of `add2_model`) and a `slim_chunk` line (chunk 40 of `add2_commutes`) | `add2_model`, `add2_commutes` |
| blind claims (`pick_equal_length` for `missing-error`, `pick_mismatch` for `swap-choice`) | controls |
| an unproven claim (predicate `add2_commutes`) | control `add2 off-by-one` |
| deferred `ir.fixed` controls with opaque fault rows | `xor-lsb` on `bump` and `wide` |
| `truncate-last` (a `Bytes` result) and `off-by-one` on a `Nat` result | `echo`, `size_of` |
| a predicate suite, a v2-header suite (`U16`), a static width, an operation `admit`, a `U8` wrap (v1 document) | `add2_commutes`, `wide_is_identity`, `cmp_fixed_agrees_w2`, `wide`, `bump_is_inc` |

Regenerate (from the repository root; do not hand-edit anything under `corpus/`):

```bash
cargo run -p coding_adventures_warrant_corpus_gen --bin warrant-corpus-gen -- \
  --warrant code/packages/python/warrant-oracle/tests/fixtures/synth-v2/synth.warrant \
  --profile code/packages/python/warrant-oracle/tests/fixtures/synth-v2/corpus.profile \
  --out code/packages/python/warrant-oracle/tests/fixtures/synth-v2/corpus
```

`corpus/` is `* -text` (a CRLF would be a different corpus). The ct-compare v2 fixture is
**not** here: it is `code/specs/fixtures/ct-compare-warrant-v2/` (S3d).

`corpus.json` records `generator` (`warrant-corpus-gen` 0.5.0); the oracle's `BUILD` and
`BUILD_windows` pass `--expect-generator warrant-corpus-gen@0.5.0` for this fixture too, so a
generator version bump fails the build until this directory is regenerated (only
`generator.version` should change) and the pin is updated (VER04r 16.8 [D13]).
