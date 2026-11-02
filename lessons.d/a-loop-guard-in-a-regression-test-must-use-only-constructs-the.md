---
category: Testing & coverage
---

# A loop guard in a regression test must use only constructs the old lowering got right, or the old code hangs the harness instead of failing

`sir-conformance` runs each emitted program with no timeout, so a program
that loops forever hangs the whole test binary.

The #18482 regression tests bound each `while` loop with a guard counter so
the old endless-loop lowering would fail with a wrong number instead.  The
first version wrote the guard as `guard += 1`.  Under the old lowering,
which dropped augmented assignment entirely, the guard never moved, and the
"fails before the fix" run hung until the processes were killed.  A one-line
`while x > 3: x -= 1` had the same problem, since the old lowering dropped
one-line bodies too.

What to do: when a regression test must terminate under the *old* code,
write its safety net only with constructs the old code is known to lower
correctly (here a local declared before the loop and `guard = guard + 1`),
and prefer a `for … in range(k)` over a `while` when the loop itself is not
what is being tested.  Run the new tests against the old code before
trusting them.
