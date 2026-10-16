---
category: Testing & coverage
---

# Publish a validation manifest only after its serialization succeeds

The second pending-main static auditor failed at final manifest serialization
because it referenced an undefined receipt variable. Preserve that external
exit1 and incomplete outcome. Correct the reference, reverify every already
materialized blob and all original path/resource checks, then publish a new
complete v3 receipt after actual exit0. Do not relabel the failed producer or
let an absent manifest act as accepted evidence; neither failure ran tests or
mutated source/targets.

A later publication checkpoint helper failed before execution because a generated
dict call repeated the keyword full for a receipt path and a count vector.
Preserve the parse-time exit1/source hash and unchanged checkpoint/issues outcome.
Rename only the receipt key, compile the complete corrected helper before running
it, and retain the failed v1 alongside actual v2. A successful push is separate
from success of its later external checkpoint writer; never relabel either.
