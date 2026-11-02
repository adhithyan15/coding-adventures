---
category: Testing & coverage
---

# Run repository script tests by file path when `code` is not a Python package

Invoking `python -m unittest code.scripts.tests...` fails because this
repository's top-level `code/` tree has no package initializer and can also be
confused with Python's standard-library `code` module. Run standalone script
tests by their checked-in file path, or use their documented discovery root,
instead of inventing a dotted package name.
