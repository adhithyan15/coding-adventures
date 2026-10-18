---
category: Testing & coverage
---

# Read the independent oracle before inserting its floating-point literals

A test proposal inserted an anticipated LR total height before consuming the
independent arithmetic tool result. The result was310.105, while the proposal
used310.1050000000001. This was corrected before any native execution or claimed
acceptance. Run and read independent calculations first; only then edit the
expected literals. Tool orchestration must keep oracle calculation and the
dependent source edit sequential rather than batching them under one call.
