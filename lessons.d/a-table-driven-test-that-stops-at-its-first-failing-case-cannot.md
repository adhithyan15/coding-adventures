---
category: Testing & coverage
---

# A table-driven test that stops at its first failing case cannot show the later cases fail before the fix

Issues #18460 and #18461 were fixed test-first: each `semantic-ir-to-*`
backend got a `twig_pairs_and_closures` test that loops over a table of Twig
programs and asserts on each in turn.  Run before the fix, every backend's
test failed — on the **first row**, `(print (car (cons 1 2)))`, refused for
the missing `Feature::Pairs`.  A loop of `assert!`s panics on the first bad
row, so "the test fails" said nothing about rows 5 onwards: the C lambda-in-
callee-position thunk bug and the Python `_globals` `NameError` were never
actually observed failing.  The first fix (declaring `Pairs`) would have
turned those rows on for the first time, *after* their own fixes were
already in.

Confirming them took a second pass: temporarily restore the pre-fix file
(`git show HEAD:<path> > <path>`), run just the table, watch the specific row
fail with the specific symptom (`'_sir_thunk___lambda_1' undeclared`,
`NameError: name '_globals' is not defined`), then put the fix back.

**When several bugs share one table-driven test, a red run proves only the
first failing row.**  Either confirm each row separately (a filter, a
per-row test, or collecting failures and asserting once at the end), or
reapply fixes one at a time and watch each row turn red-to-green in order.
The cheap structural fix is to collect every row's outcome into a `Vec` and
assert on the whole list once, so one run shows every failure.  The `twig_pairs_and_closures` tests
now do exactly that.
