---
category: CI & GitHub Actions
---

# Adding scoped gate paths can exceed real registry matching budgets

Adding C's dependency and validator paths to the neural conformance registry
made the production Go evaluator's real 10,751-path language-shard regression
fail with `CI_GATE_MATCH_LIMIT_EXCEEDED`. The package tests and small-input gate
tests had passed: the failure depended on total unique pattern cost in the
complete registry, not on a bad C glob or numerical execution.

Consolidate this gate's fixed neural-validator filename family into
`code/scripts/validate_neural_framework_*.py` and its test family into
`code/scripts/tests/test_neural_framework_*.py`. Preserve explicit native
package/dependency directories and exercise every old and new selecting path
through the actual Go evaluator, including `.py.bak` and unrelated negative
controls. Keep the existing work cap, literal filtering and 10,751-path test
unchanged; do not raise a resource limit merely to make a registry edit pass.
