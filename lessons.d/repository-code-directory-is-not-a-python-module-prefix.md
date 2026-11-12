---
category: Python
---

# Repository code directory is not a Python module prefix

The third-party dependency policy test was invoked as
`python3 -m unittest code.scripts.tests...`. Python resolved `code` to its
standard-library module, which is not a package and has no `scripts` member, so
the loader failed before running any test.

Run repository script tests by file path unless their directory is explicitly
installed or documented as an importable package. Treat `_FailedTest` loader
output as an unexecuted check, then rerun the exact test file directly.
