---
category: Testing & coverage
---

# A generated harness that changes output under CI=true needs its own tests run with CI=true before pushing

WX-H/E (#19121) failed `every_fault_fails_rows_of_the_shipped_tables` on
the Linux and macOS CI builds but never locally: not alone, not in the full
suite, not on the PR merge ref, not under 1-4 test threads or concurrent load.
The cause was the environment. The Warrant evidence prelude reads `CI` and,
when it is set, keeps no failed row ids, a deliberate log-size and data
choice that also prints `repro-fnv` hashes instead of counterexamples. The
test asserted 16 ids. GitHub Actions sets `CI=true`; a local shell does not.
The build tool showed only the test's stdout, so the panic message, which
goes to stderr, never reached the log.

The fix: the test reads `CI` and checks the behaviour that applies (counts and
verdicts always; ids only outside CI). It does not call `set_var`, which
races with the binary's other tests on parallel threads.

Do this: when a test fails only in CI, rerun it locally with `CI=true` (and
`GITHUB_ACTIONS=true`) first. Then `grep -rn 'env::var("CI")'` the code under
test for behaviour that changes in CI. For crates whose code reads `CI`, run
the suite both ways before pushing.
