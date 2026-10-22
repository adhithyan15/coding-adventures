---
category: Python
---

# Use unittest discovery when repository paths are not Python packages

The capability taxonomy test was invoked as
`python -m unittest code.scripts.tests.test_capability_taxonomy`, but this
repository path is not a Python package and `code` resolved to the standard
library module, producing `module 'code' has no attribute 'scripts'`. Mirror
CI for script tests with `unittest discover -s code/scripts/tests -p ...`
instead of translating filesystem paths into dotted module names.
