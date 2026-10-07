---
category: Python
---

# Check the default Python version before relying on tomllib in orchestration helpers

The default `python` here is Python 3.10, so importing `tomllib` failed before any Cargo validation ran. Check the runtime before relying on modules added in Python 3.11. The corrected helper used compatible extraction of the package name from these known simple manifests; the full 19-consumer test/lint run then passed. No compiler behavior changed.
