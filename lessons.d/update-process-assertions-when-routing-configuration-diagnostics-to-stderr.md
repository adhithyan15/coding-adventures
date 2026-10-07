---
category: Testing & coverage
---

# Update process assertions when routing configuration diagnostics to stderr

A full compiler run found an old typed-AST alias conflict test reading the configuration diagnostic from stdout. The intended failure-stream contract had moved errors to stderr. Preserve the test's exact exit status and disagreement assertion, read stderr, and add an empty stdout assertion. Search other failure fixtures for the same assumption before rerunning the full suite; do not relax the diagnostic check or move errors back to the success stream.
