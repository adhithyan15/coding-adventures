---
category: Python
---

# Write UTF-8 explicitly and stop publishing when document generation fails

A Windows Path.write_text used cp1252 and truncated a spec before rejecting an
arrow character. The shell then continued to commit and push. Restore the spec
from the prior commit with its actual encoding and write UTF-8 explicitly.
Inspect generation exit status and expected nonempty files before committing;
a semicolon or newline does not make PowerShell stop on a Python failure.
