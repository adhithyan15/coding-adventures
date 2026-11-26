---
category: Testing & coverage
---

# Even-sample medians must round after summing subnormals

The overflow-avoidance midpoint `low + (high - low) / 2` can underflow the
difference for adjacent subnormal values before the final division. That
changes IEEE ties-to-even results (for example, the median of the first and
second positive subnormal is the second subnormal). Sum the values first and
halve; if that sum overflows, halve each operand before adding. Test positive
and negative subnormal ties, the normal/subnormal boundary, and large finite
values.
