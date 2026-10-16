---
category: C#
---

# Cast wheel deltas before negating to avoid signed integer overflow

An executed generated-code test using int.MinValue found that -delta overflowed before conversion to double. Cast first, then negate. Keep extreme delta tests alongside fractional accumulation and boundary tests.
