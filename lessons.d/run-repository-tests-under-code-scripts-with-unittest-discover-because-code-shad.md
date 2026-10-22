---
category: Python
---

# Run repository tests under code scripts with unittest discover because code shadows the standard-library module

`python3 -m unittest code.scripts.tests...` imports Python's standard-library
`code` module instead of treating the repository directory as a package, then
fails before loading the requested test. Invoke these suites with `python3 -m
unittest discover -s code/scripts/tests -p '<test file>'`, matching the CI
workflow and avoiding the module-name collision.
