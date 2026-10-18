## Unreleased - retain original reconciliation corpora

Add eight exact original corpus files and nine verbatim consuming tests so44
authored sources and their existing semantic assertions survive incoming corpus
replacement. No production parser/API/dependency changes. At8531f28d, focused9
and full655 tests/29 harnesses passed with strict all-target lint and independent
source/actual-terminal review; no ignored or filtered cases. Original assertions
are retained without a new full IR or compiler-lineage claim.

Current-main integration, hosted checks/review and actual PR17288 merge remain
open. Miri DEFERRED/PENDING, never passed; frozen inputs/run remain untouched.
