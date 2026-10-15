---
category: Testing & coverage
---

# Scan conditional roots when moving an admitted offset boundary

The four-unit repair's first focused run correctly admitted four units, but
one retained refusal fixture still used four in a left/right conditional arm.
My first fixture audit searched only expressions rooted in `pick()`, missing
these two local-root cases. The run remained failed at 17/1, with the complete
library at 862/1; its incoming and owned runtime successes do not erase that.

Inspect every old boundary assertion, including direct conditional arms and
wrappers. Move only the newly admitted boundary to the next refused value,
and add exact formatter positives for the previous shapes on both outcomes.
Keep alias, shared exp/log, rounding, malformed-input and work-limit refusals
unchanged. Preserve the failed receipt and rerun the full affected validation.
