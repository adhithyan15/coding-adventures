# Changelog

This directory is spec/content data, not a compiled package — entries record what content
landed and why, not a semver-tracked API.

## Unreleased

- Added `README.md` documenting the directory's file-pair convention, domain layout, and
  verification commands (a standing gap flagged during the adj-curriculum loop's Wave 0
  backfill work).
- `mathematics/number-sequence.adj` — `next`/`previous` for the K-2 counting sequence,
  composing `arithmetic.adj`'s `sum`/`difference` (CCSS K.CC.A.1/A.2).
- `mathematics/comparison.adj` — `greater_than(a, b)`, the first shipped library to use
  ADJ-FORMULA-LIBRARIES FL-8 (a `formula` body ending in a comparison instead of arithmetic)
  (CCSS K.CC.C.6/C.7).
- `mathematics/cardinality.adj` — `total_cardinality`, composing `arithmetic.adj`'s `sum` to
  find the count of a group made by combining two counted groups (CCSS K.CC.B.4/B.5).
- `mathematics/place-value.adj` — `tens_and_ones_to_number`, composing `arithmetic.adj`'s
  `product`/`sum` to compose a two-digit number from its tens and ones digits (CCSS 1.NBT.B.2).
- `mathematics/place-value.adj` — `tens_digit`/`ones_digit`, the DECOMPOSE direction (a number
  back into its tens and ones), using ADJ-FORMULA-LIBRARIES FL-9's new `floor`/`mod` built-ins.
  `tens_and_ones_to_number(tens_digit(n), ones_digit(n)) = n` for any two-digit `n` — the two
  directions are verified algebraic inverses of each other (see
  `code/packages/rust/adj-lang-cli/tests/formula_place_value_e2e.rs`).
