---
category: Security boundaries
---

# Validate loaded model dimensions before passing fixed C batch buffers

While constructing NN48's --resume command, local source review caught a trust
boundary error before publication: the V1 loader correctly accepts models for
many shapes, but the command owns a fixed XOR buffer (four samples, two inputs,
one output). Passing that buffer to an arbitrary valid loaded model violates
the public C readable-length precondition and can cause an out-of-bounds read.
Expose read-only input/output widths and require 2/1 before calling any batch
operation in the fixed command. Library schema validity is not dataset-shape
compatibility. Add adversarial wrong-width resume command tests, including
sanitizer runs; do not restrict the reusable library to XOR to repair a CLI.
