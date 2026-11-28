---
category: Python
---

# A metadata fallback helper still has to meet Ruff line width

The metadata fallback used one conditional expression longer than Ruff's line-width limit. Split the lookup and validation into separate statements, then run Ruff against both changed Python packages before committing the profile adjustment.
