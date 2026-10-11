---
category: CI & GitHub Actions
---

# Keep PowerShell Select-Object count arguments numeric

A Windows CI-log inspection accidentally passed the word `seventy` to
`Select-Object -First`, which requires an integer. The tool failed before reading
the evidence; it did not show that the requested log section was absent.

Use numeric literals such as `-First 70` and inspect command errors before
interpreting missing output. The corrected read exposed the actual Clippy
diagnostic and enabled a focused repair.
