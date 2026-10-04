---
category: Python
---

# Run path-selected unittest files as scripts when code shadows the standard-library module

`python -m unittest path/to/test_file.py` converts the path into a dotted
module name. In this repository that begins with `code`, which can resolve to
Python's standard-library `code` module instead of the repository directory and
fail before discovery. For focused repository checks that are executable
`unittest` files, invoke each file directly with `python3 path/to/test_file.py`;
reserve dotted `-m unittest` names for directories that are actual importable
packages.
