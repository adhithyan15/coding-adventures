---
category: Testing & coverage
---

# A golden geometry contract must include the existing clamp before declaring a coordinate

The first Timeline direction specification declared a TD event width of 140
from raw canvas subtraction. The owned layout actually applies a minimum width
of 160. Direct source inspection and an independent review caught the omission
before test or production edits. The corrected specification retains the clamp;
no production arithmetic changed to fit the incorrect draft.

Read the whole existing expression, including clamps and floating-point
association, before declaring a literal oracle. Independently calculate the
fixture from those declared rules, then compare the complete output. A plausible
raw subtraction is not an independent model of the existing product contract.
