---
category: Security boundaries
---

# Check integer bounds before narrowing instead of casting to prove the bound

PR #17905's CodeQL check reported three integer-conversion findings in the
BF03 Go test fixture reader. The helper tested `int64(int(n)) != n`, which
detects a changed value only after the narrowing conversion has occurred.
The sequence-count call site also converted an `int64` directly to `int`.

Compare the original wide value with `math.MinInt` and `math.MaxInt` before
converting, then route every fixture conversion through that checked helper.
Test native endpoints, wrong dynamic types and values outside 32-bit bounds;
run the tests as a real 32-bit executable as well as on the default host.
Passing a 64-bit test run alone does not validate a narrower target's domain.
The focused 32-bit admission tests passed; the supplementary full corpus run
could not construct semantic-negative values beyond the legacy 32-bit `int`.
Keep that corpus qualification limit explicit instead of counting a transport
rejection as a production semantic rejection.

The CodeQL workflow itself completed successfully while its separate code
scanning check failed on annotations. Inspect the check's annotations as well
as workflow logs before treating a green analysis job as a security verdict.
