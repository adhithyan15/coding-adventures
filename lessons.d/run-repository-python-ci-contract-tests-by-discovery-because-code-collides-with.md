---
category: Testing & coverage
---

# Run repository Python CI contract tests by discovery because code collides with the standard-library module

Running `python -m unittest code.scripts.tests.test_ci_gate_registry` resolved the standard-library `code` module instead of this repository directory and failed before executing tests. Use unittest discovery against the known `code/scripts/tests` directory and an exact file pattern. The repaired command executed all 18 contracts successfully. Discover referenced CI registry/script paths from the test rather than guessing filenames. Read the lessons CLI category inventory: `Testing & coverage` is valid, `Testing` is not.
