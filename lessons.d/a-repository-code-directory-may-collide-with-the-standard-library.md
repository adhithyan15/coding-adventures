---
category: Python
---

# A repository code directory may collide with the standard library

Running unittest with a dotted path beginning in `code` imported Python's
standard-library `code` module instead of the repository directory. Use
unittest discovery with an explicit filesystem start directory when a top-level
repository directory is not an import package or collides with a standard
library module.
