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
- `mathematics/geometry-formulas.adj` — `square_perimeter(side)`, closing the one gap in the
  library's own perimeter/area set (MathWorld's perimeter table's `square | 4a` row was already
  cited by the library's header comment but never had a formula clause).
- `mathematics/volume-formulas.adj` (new) — `cube_volume(side)` and
  `rectangular_prism_volume(box_length, box_width, box_height)`, the measurement track's third
  dimension: volume formulas for the first two solids a grade-schooler meets, the direct sibling
  of `geometry-formulas.adj`'s area/perimeter formulas one dimension up (CCSS 5.MD.C). Distinct
  from `reference/volume-conversions.adj`, which converts an already-known volume between units
  rather than computing one from edge lengths. See
  `code/packages/rust/adj-lang-cli/tests/formula_measurement_e2e.rs`.
- `mathematics/word-problems.adj` (new) — `separate_result` (TAKE FROM) and `compare_difference`
  (COMPARE), the two of CCSS 1.OA.A.1's four word-problem situation types (ADD TO, TAKE FROM, PUT
  TOGETHER/TAKE APART, COMPARE) not already covered under a different name (ADD TO/PUT TOGETHER is
  `cardinality.adj`'s `total_cardinality`). Each composes `arithmetic.adj`'s `difference`, but
  names the SITUATION so a consumer can pick the right formula from how a word problem is worded,
  not just from seeing a minus sign. See
  `code/packages/rust/adj-lang-cli/tests/formula_word_problems_e2e.rs`.
- `mathematics/data-displays.adj` (new) — `range_two(a, b)` and `range_three(a, b, c)`, the
  statistical range (largest minus smallest, MathWorld's `R = Y_N - Y_1`) of two or three counted
  or measured values. The first content library built on ADJ-FORMULA-LIBRARIES FL-11, which wires
  the two-argument `min(a, b)`/`max(a, b)` form onto the plain-arithmetic surface grammar (they
  were previously reachable only via the `latex "…"` escape). `range_three` composes `max`/`min`
  twice each (associatively), the same fold `average.adj`'s `mean_three` uses for `sum`. See
  `code/packages/rust/adj-lang-cli/tests/formula_data_displays_e2e.rs`.
