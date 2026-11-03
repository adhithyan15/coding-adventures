---
category: Python
---

# Run script tests directly when the repository code directory is not a Python package

An attempted `python -m unittest code/scripts/tests/test_*.py` invocation
failed before collecting tests: the standard-library `code` module has no
`scripts` package. One guessed CT01 test filename did not exist either. Those
loader errors are not failing validator tests and are not validation evidence.

Find the actual filenames with `rg --files`, then use the existing CI command,
for example `python -m unittest discover -s code/scripts/tests -p
'test_bootstrap_harness_conformance.py'`, or execute its self-running script.
Check the real test count and exit status before continuing to lint and drift
checks. Avoid guessing package imports from repository directory names.
