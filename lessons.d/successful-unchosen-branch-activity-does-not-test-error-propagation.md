---
category: Testing & coverage
---

# Successful unchosen branch activity does not test error propagation

CLOC38's first independent review withheld acceptance because successful folds
in both conditional branches and outer-selection limit failures did not exercise
an error originating inside the unchosen branch. There was no production defect
in the reviewed change, but the claimed error-order coverage lacked evidence.

For both truthy choices, inject an actual checked child merge refusal and assert
the exact latched error and unchanged graph/journal after that refusal. Include
an ample-capacity missing-parent case so absence of outer events is not explained
only by exhausted capacity. Target the unchosen arithmetic child's own merge and
derive in CLI budget controls and preserve every publication sentinel byte.
