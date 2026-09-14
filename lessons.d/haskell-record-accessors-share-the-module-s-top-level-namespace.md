---
category: Workspace & package metadata
---

# Haskell record accessors share the module's top-level namespace

A field such as `requestBodyKind` creates a function with that exact name, so a private helper with the same spelling fails with `Multiple declarations`. Name decision helpers distinctly (`determineRequestBodyKind`) before compiling.
