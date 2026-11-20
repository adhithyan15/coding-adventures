---
category: Python
---

# A package module named like a standard-library module shadows it for every interpreter started in that directory

A new sub-package module was named `types.py`. Inside the package that is harmless
(imports are absolute), but the first throw-away script run with the module's
directory as the working directory (`python3 - <<EOF` from there) died inside
`import re` with `cannot import name 'GenericAlias' from 'types'`: `sys.path[0]`
is the current directory for `-c` and stdin scripts, so the module replaced the
standard library for the whole interpreter. The same happens to a test runner
started from that directory and to any tool a developer launches there.

Do not give a module the name of a standard-library module (`types`, `string`,
`json`, `queue`, `code`, `random`, `logging`, ...), not even as a "namespaced"
file inside a package. Rename it (`wtypes.py`), and when diagnosing an
`ImportError` from the standard library itself, look for a same-named file in the
directory the interpreter was started from before suspecting the installation.
