---
category: Security boundaries
---

# A conformance test is not a boundary, and saying "enforces" does not make it one

Same arc, three overclaims: "enforcing" for a check nothing called, "closes" for a check that narrows, and a refactor described as done when one of three call sites had moved. The failure mode is writing the claim while holding the intent, before the code has finished disagreeing. Write the CHANGELOG line *after* re-reading the diff, and prefer "narrows"/"checks" unless a caller actually refuses.
