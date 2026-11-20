---
category: Python
---

# Run Ruff on every fixture Python file before committing the shared corpus.

The first required Ruff run found import ordering, one unused import, and a lambda loop-variable capture in the new fixture validator. I applied Ruff's import fixes, removed the unused import, bound the callback result as a lambda default, and narrowed the command-line validation exception handler to expected input/contract failures. Run `ruff check` over every new fixture Python file before committing so the independent oracle and generator stay executable under repository lint rules.
