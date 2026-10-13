---
category: Python
---

# Run ruff immediately after adding a Python module so unused design-stage imports do not survive into review

The first lint run found an unused `Iterable` import left over from the report
tool's design sketch even though the final signatures used only `Sequence`.
The unit tests could not expose a dead import, but the repository requires all
Python to pass ruff. Run focused ruff alongside the first focused test run, and
remove imports as soon as the corresponding design path is dropped.
