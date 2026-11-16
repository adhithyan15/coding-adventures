---
category: Testing & coverage
---

# Flipping a checked-in registry row breaks every sibling runner whose tests pin the old shape, so grep all runners before flipping

WD-101 to WD-103 each shipped a Warrant lane runner whose own tests were written
against the checked-in `consumers.json` as it stood: the Python tests asserted the
`python/ct-compare` row was `planned`, took it as `consumers[0]`, and the TypeScript
and Go tests asserted that their own lane was *not* registered. Each pin was
reasonable alone and recorded in the backlog as "flipping the row belongs with
WD-004b1" -- but the pins lived in three other languages' packages, so flipping the
row made all of them fail, and none of them are visible from the Rust crate that
owns the registry.

When a change edits a shared data file, grep every consumer (`git grep -n
consumers.json origin/main`, not just the owning package) for tests that bind to its
current *shape*: a literal status, a row index, "is not registered". Rewrite those
tests to build the case they need from a copy of the file (flip one row in a copy,
find rows by identity rather than position) so the checked-in file can change
without the test changing. A sparse worktree hides these consumers: add their
packages to the sparse set before flipping, and run their suites.
