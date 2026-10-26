---
category: Testing & coverage
---

# Widening a subset turns the narrower subset's refusal cases into accepted programs for every release that gains the wider one

**What went wrong.** LANG84 phase 7c slice 3 added `py-float-print` on six
of the seven releases that claim `py-int-print`. A corpus negative case is
checked as "the compiler for release R refuses this", for every release that
claims the subset, and the compiler is one per release, not one per subset.
`py-int-print` had `reject_float` (`print(1.5)`) and `reject_true_division`
(`print(7 / 2)`), which pinned the integer subset's boundary. Once the six
releases compile floats, both negative cases compile on them, so stage I for
`py-int-print` would have failed on six releases while the code was right.
Changing the expected category to "Unsupported on 3.0a1 only" is not
possible either: the loaders accept `.reject@<release>` overrides only on
positive cases.

**Fix.** The two cases left `py-int-print`. Their positive versions live in
the new `py-float-print` corpus, and 3.0a1's refusal of both (its profile,
`py30`, has no floats) is pinned by a dedicated test,
`release_3_0a1_refuses_floats_and_true_division`.

**Do differently.** Before widening a subset on some of its releases, list
the narrower subset's negative cases that the wider one accepts, and move
them (to the wider corpus as positives, plus an explicit test for the
releases that still refuse them) in the same change. A negative corpus case
describes a release's compiler, not an abstract subset boundary.
