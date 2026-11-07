## Unreleased — Twig floored modulo lowers to `i64_mod_floor` (#18486)

`dynamic_arith` now lowers `modulo` to `i64_mod_floor` rather than truncating `mod`.
`Wasm` is now declared on the Twig `modulo` row in `lang_matrix.rs` (`(+ (* 16 (f -17 5)) (f 17 -5))` ⇒ exit 45), which previously truncated on WASM.
`tests/e6d2b_dynamic_arith.rs` adds `modulo_floors_and_remainder_truncates`, asserting that `(modulo -7 2)` → 1, `(modulo 7 -2)` → -1, `(remainder -7 2)` → -1, and `(modulo 6 3)` → 0 match across VM and native.
`feature_coverage_doc_counts_match_programs_source` and `LANG-VM-FEATURE-COVERAGE.md` update Twig declared cells from 507 to 508.
