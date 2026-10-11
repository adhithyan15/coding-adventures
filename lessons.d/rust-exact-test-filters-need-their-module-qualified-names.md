---
category: Testing & coverage
---

# Rust exact test filters need their module-qualified names

The first focused Perl72 command supplied only the function name with
--exact. The executable reported zero tests and 22 filtered out: it had built
the test but had not run the new assertion. Rerunning with tests:: followed by
the function name selected exactly one test, which failed at the still-pending
inventory row as expected. A zero-test success is not regression evidence.
Use the test listing or module-qualified name and inspect the executed count
before claiming a red or green focused result.
