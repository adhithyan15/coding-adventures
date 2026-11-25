---
category: Testing & coverage
---

# Coverage repair should test the exact resource boundary in the isolated package

A 95% analyzer gate failed at 94.99% because several source points in the input
limits and Dune scanner were not reached. Adding a normal-path test alone did
not establish that the advertised source-count limit was inclusive or that the
next input was rejected. The test now accepts exactly 10,000 empty source files,
then adds one and checks the specific limit error. A reviewed-profile depth case
and Dune token-boundary case add distinct behavior coverage without changing the
analyzer's production rules.

Use the pinned formatter alias and run the instrumented package tests from the
package directory, including the copied package context used by CI. Describe the
result as Bisect-PPX source-point coverage; the number is not line or branch
coverage. Keep tests bounded and assert behavior, not just a target percentage.
