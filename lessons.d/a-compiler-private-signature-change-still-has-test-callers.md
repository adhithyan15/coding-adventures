---
category: Rust
---

# A compiler-private signature change still has test callers

CLOC33 added a run-local array ledger argument to transform_source_with_ingestion.
The first focused build at6db75a51 failed E0061: two captured_ingestion_tests
calls still used the old signature. No new test ran and no green claim followed.

Search the entire crate, including cfg(test) companion modules, before changing
a private seam. Give those refusal controls the new explicit ledger while
preserving their original missing-capture/lexical-error stages and exact unchanged
log assertions. A private production call graph is smaller than its test call
graph; building all test targets is what reveals the latter.
