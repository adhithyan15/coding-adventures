---
category: Testing & coverage
---

# Coverage source expects a module or directory rather than a Python file path

Passing a script filename to `coverage run --source` ran the tests but collected
no coverage: coverage interpreted it as a module name. Use a module/directory
source or a precise `--include='*/code/scripts/compiler_ir_conformance.py'`
pattern for a dynamically imported standalone script. Check for nonzero measured
statements and the exact file denominator before reporting any percentage.
