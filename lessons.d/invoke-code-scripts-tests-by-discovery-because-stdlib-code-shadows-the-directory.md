---
category: Testing & coverage
---

# Invoke code/scripts tests by discovery because stdlib code shadows the directory

`python -m unittest code/scripts/tests/test_ct_compare_conformance.py` failed
before loading the test: Python resolved `code` to its standard-library module,
which has no `scripts` child. For repository script tests, run
`python -m unittest discover -s code/scripts/tests -p 'test_*.py'` (or an exact
test-file pattern) so the test directory is the discovery root.
