---
category: TypeScript / JavaScript
---

# Python `math.isfinite(huge_int)` can raise `OverflowError` before a validator rejects the value

Check an integer's absolute magnitude before asking a float-oriented finiteness helper to convert it. Keep the `isfinite` call for bounded floats. Caught by the NN29 hostile thousand-digit input regression.
