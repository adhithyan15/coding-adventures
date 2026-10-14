---
category: Testing & coverage
---

# Run standalone fixture tests with a Python that has their declared validator

The barcode fixture registry is a standalone unittest file. Running it as
`python -m unittest code/scripts/tests/test_barcode_layout_1d_fixtures.py` tried
to import `code.scripts` from Python's standard-library `code` module. Switching
to discovery fixed that import, but the bundled Python 3.12 runtime lacked the
test's jsonschema dependency. These were test-invocation failures, not fixture
failures.

Verify the interpreter and validator, then invoke the standalone file directly.
The existing Python 3.10 installation had jsonschema, and
`python code/scripts/tests/test_barcode_layout_1d_fixtures.py` passed all nine
tests in 30.458 seconds after the main merge. Do not assume every available
Python runtime has the same development dependencies.
