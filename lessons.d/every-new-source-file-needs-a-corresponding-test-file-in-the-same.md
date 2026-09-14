---
category: Testing & coverage
---

# Every new source file needs a corresponding test file in the same commit

Pytest-cov `fail_under=80` and similar gates trip on uncovered new code. Plan tests alongside implementation.
