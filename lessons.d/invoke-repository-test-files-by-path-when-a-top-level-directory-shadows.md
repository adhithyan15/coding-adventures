---
category: Python
---

# Invoke repository test files by path when a top-level directory shadows a Python standard-library module

Running `python3 -m unittest code.scripts.tests.test_capability_taxonomy`
resolved `code` to Python's standard-library module rather than this
repository's top-level directory, so discovery failed before executing any
test. Run a standalone repository test as a file path (for example,
`python3 code/scripts/tests/test_capability_taxonomy.py`) unless its parent
directories are real import packages known not to shadow the standard library.
