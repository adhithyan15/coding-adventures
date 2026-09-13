---
category: Workspace & package metadata
---

# Do not label a forward-only runtime as a training compiler

Use the real
runtime for saved forward evidence, then name a new language-neutral
backward/optimizer contract explicitly. Keep backward, gradient reduction,
optimizer update, and zeroing as separate observable operations until the
production runtime implements those contracts. Prove persistence with a
nonzero incoming gradient buffer: reduce the current batch separately, add it
to the prior buffer, and finite-difference only the current batch loss.
