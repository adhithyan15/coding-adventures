---
category: Mosaic compiler pipeline
---

# `_grammar.rs` hand-edit caveat:

the general rule is "never hand-edit"; however for embedded grammar reordering (alternation order fix) it is acceptable when `grammar-tools` is not available in the current environment. Always note the edit prominently and regenerate properly before the next full CI run.
