---
category: CI & GitHub Actions
---

# CodeQL flags `int64 → int` downcasts of CLI input

as `go/incorrect-integer-conversion`. Add explicit platform-sized bounds checks first; for `float64`, reject NaN/Inf/non-integral before the cast.
