---
category: Python
---

# Use the selected Python version when a harness requires tomllib

The native affected-suite runner used tomllib, but plain python selected3.10
on Windows and failed before any Cargo validation ran. Inspect python --version
and py -0p, then use py -3.13 for this harness. Do not install a dependency or
count the failed harness as test acceptance. Preserve its error separately
from the actual Cargo terminal receipts.
