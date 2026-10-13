---
category: Python
---

# Run repository script tests with unittest discovery because code collides with Python's standard-library module

The first focused run used `python -m unittest
code.scripts.tests.test_human_language_hot_files`. Python resolved `code` to
its standard-library module, which is not a package and has no `scripts`
attribute, so no test was collected. This repository's script tests add their
own script directory to `sys.path`; invoke them with `unittest discover -s
code/scripts/tests -p <file>` so discovery loads the file rather than treating
the repository's `code/` directory as an import package.
