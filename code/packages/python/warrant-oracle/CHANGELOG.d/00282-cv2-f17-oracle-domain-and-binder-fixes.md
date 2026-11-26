## Unreleased - oracle domain and binder fixes (CV2-F17)

- **O1, stale size cache in demotion.** `domains._refresh` now recomputes strictly bottom-up (post-order). It used to recompute a
  variant before the tuple or record beneath it dropped its cache, so a 16-bit leaf in a tuple or record under a variant, over
  budget, was refused `W0706` instead of demoted; `verify-v2` would have rejected a valid corpus. Tests for all five shapes
  (variant over tuple, variant over record, nested variant, record over variant, `Option` over variant) pin path and size.
- **O2, descriptor path depth.** `descriptor_for` counts nodes from 1 like `descriptor_size` (VER04r 14: at most 16, a leaf alone is
  one): 16 nested records or 16 `Option` levels over `Bool` are accepted, 17 are refused `limit`. Before, 17 were accepted.
- **O3, the 8-binder cap on derive.** `SuitePlan` raises `limit` ("a claim has more than 8 binders (VER04r 5.3)") before building
  any descriptor (VER04r 5.3, 14, R75); until now only the kit reader refused it. `emit-v2` reports such a suite `refused: limit`.
- **Pins moved.** arithmetic's `ripple_carry_adder_with_carry_4_bit_exhaustive` has 9 binders: it is now refused `limit`, so
  `emit-v2` reports it `refused: limit` and lists 62 suites and 2,654,925 cases (was 63 and 2,655,437; 512 cases fewer); `derive` and
  `verify-v2`, which refuse the whole document on a refused suite, now refuse arithmetic until the adopter splits the claim. README and
  `tests/fixtures/data-eval-v1/README.md` updated. `arithmetic.warrant` is untouched and no committed fixture byte moved (every
  `--check` line of BUILD is identical).
- **Composite labels after demotion.** `_refresh` also recomputes `label` for tuple, record, variant, `Option`, `Result` and list
  nodes from their children, post-order. A demoted leaf is `fx_edge` (`bounded-set`) but its ancestors kept the `full` they were
  built with, so `verify-v2` rejected the generator's `domain_label: bounded-set` for any demoted composite with no other
  `bounded-set` leaf (VER04r 9.3). Tests pin the label for the five shapes, for a bare tuple, `Option`, and two `Result` shapes, and
  one test derives a document of demoted composites and requires `verify-v2` ok.
