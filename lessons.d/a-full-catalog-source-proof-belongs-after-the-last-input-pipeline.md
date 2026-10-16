---
category: Compiler / VM / language pipeline
---

# A full-catalog source proof belongs after the last input pipeline

CLOC33's first contract asked for replay-backed node checks before pipeline
execution. The compiler actually captures and transforms one input at a time;
SourceCatalog::validate_core requires every expected input/root/path to exist.
Moving that full proof before the first pipeline would reject multi-input runs.
Independent contract review caught the scheduling contradiction before tests/code.

Keep callback boundary checks and construction receipts provisional. After all
input pipelines finish, run the complete existing source replay, borrow its
admitted premise, then validate every node receipt and the final namespace census
before publication. Check each recorded construction Merge event and its original
context/order there. A borrowed proof must expire before graph mutation. Test
multiple and repeated inputs and tampering after construction; single-input
success does not establish the schedule.
