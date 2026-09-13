---
category: Python
---

# Compiler-generated data segments need source-stage byte caps

AST-depth and source-size limits don't bound semantic frame plans or generated runtime images. Cap at the earliest stage that computes the size.
