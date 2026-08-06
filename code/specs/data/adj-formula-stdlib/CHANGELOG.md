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
