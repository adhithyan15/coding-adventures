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
- `mathematics/operation-properties.adj` (new) — `addition_is_commutative(a, b)`,
  `subtraction_is_commutative(a, b)`, and `addition_is_associative(a, b, c)`. A property like
  commutativity is a LAW about every pair of numbers, not a fact about one named pair, so instead
  of a `relate` fact this uses ADJ-FORMULA-LIBRARIES FL-8's comparison-formula shape: compute both
  sides of the property's defining equation from a given instance and confirm they agree with
  `==`, citing the general law the agreement demonstrates (MathWorld's `Commutative.html`/
  `Associative.html`). `subtraction_is_commutative` is included deliberately for CONTRAST with
  `addition_is_commutative` — it computes false whenever the operands differ, teaching by
  counterexample that subtraction, unlike addition, is not commutative in general. Composes
  `arithmetic.adj`'s `sum`/`difference` only; no new language capability. See
  `code/packages/rust/adj-lang-cli/tests/formula_operation_properties_e2e.rs`.
- `mathematics/comparison.adj` and `mathematics/data-displays.adj` — closed ADJ-STDLIB-COVERAGE.md
  5.1's "number lines" gap as a `surface` VOCABULARY extension of two already-shipped, already-
  tested formulas, not a new one: ordering two quantities and ordering two number-line positions
  are the same comparison (`greater_than`), and the distance between two points on a number line
  is the same computation as the statistical range of a two-element sample (`range_two`). The same
  write-once-use-many discipline `comparison.adj`'s own header already documents for `less_than`
  (definitionally `greater_than(b, a)`, not a duplicated formula). `arithmetic.adj`'s `sum`
  dictionary was deliberately left untouched for the "number line as jumps" framing — it is one of
  the repo's few byte-pinned libraries, outside this loop's delivery scope (owned by the separate
  CAS-provenance track). No new formula, no new citation, no engine change: `surface` synonyms are
  a decomposer-facing hint only, not engine-parsed, so the change is covered by the existing
  formulas' own test suites.
- `mathematics/geometry-formulas.adj` — four rung-0 CAS-wiring companions (ADJ-FORMULA-LIBRARIES
  FL-10, §3D — the Wave 3 opener): `width_from_rectangle_area`, `height_from_triangle_area`,
  `side_from_square_perimeter`, `width_from_rectangle_perimeter`. Each solves the SAME cited
  equation as its forward `formula` sibling for a different unknown, through `cas-solve`'s real
  linear-equation solver (the exact discipline `electricity.adj`'s `resistance_from_ohms_law`
  already established) — no new citations, each reuses its forward formula's own verified
  MathWorld source. `square_area`'s inverse (side from area) is deliberately absent: it is
  quadratic in the target, and rung-0's solver is linear-only, so attempting it is a clean
  `SymbolicNonLinear` compile error (added as a regression test, not worked around). New query
  companions `geometry-formulas-solve.query.adj` and `geometry-formulas-solve-perimeter.query.adj`
  (split across two programs: a `symbolic`'s target may not already be observed anywhere in the
  same compiled program, and two of the four solves share a target name — `width` — so they cannot
  coexist in one program even in the right order). New manifest objective
  `adj.math.algebra.rearrange_geometry_formulas`. New e2e test
  `formula_geometry_algebra_e2e.rs` (5 tests, including the nonlinear-boundary regression).
