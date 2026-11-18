# ct-compare-v2-preflip: the ct-compare v2 `corpus.json` as `warrant-corpus-gen` 0.5.0 wrote it

A frozen copy of `code/specs/fixtures/ct-compare-warrant-v2/corpus.json` from before the S10c flip (D29): its `eq_u64/ignore-high-bit` control says `{"deferred": "ir.fixed"}`, which the shipped gate no longer derives.
It is a **negative test fixture** (`test_a_preflip_ct_compare_corpus_with_a_stale_deferred_witness_fails_verify_v2`): `verify-v2` must refuse it. Never regenerate it, and never point a BUILD line at it.
Only `corpus.json` is kept; the test borrows the chunk files and `cases.tfv` of the live fixture, which did not move in the flip.
