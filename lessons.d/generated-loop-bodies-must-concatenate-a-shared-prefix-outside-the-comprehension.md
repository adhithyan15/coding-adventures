---
category: Python
---

# Generated loop bodies must concatenate a shared prefix outside the comprehension

While extending the benchmark fixture registry, a Python generator placed a
shared Go loop opener next to an expression inside a comprehension. Python
included the opener in every generated row, producing repeated loop declarations.
A broad string replacement also changed the registry declaration instead of
the intended switch clause. Formatting and compilation rejected both mistakes.

Build row strings separately, then concatenate the single prefix outside the
comprehension. Match each edit exactly once within its intended function, inspect
the resulting diff, and compile before committing. When adding a language, replace
any existing empty-filter expectation instead of adding a duplicate map key.
