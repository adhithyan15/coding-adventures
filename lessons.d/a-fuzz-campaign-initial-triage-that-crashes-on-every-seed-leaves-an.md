---
category: Rust
---

# A fuzz campaign initial triage that crashes on every seed leaves an empty corpus for iteration

In coverage-guided fuzz testing (VER06), seeds submitted in the initial corpus
undergo baseline triage before the iterative mutation loop begins. If every initial
seed triggers an unhandled crash or panic in the target during triage, none of the
seeds are admitted into the active mutating corpus. As a result, the subsequent
mutation loop exits immediately (`corpus.is_empty()`) despite a non-zero iteration
budget. When writing unit tests or conformance tests for fuzz target runners and crash
deduplication, ensure at least one baseline seed passes initial triage so the engine
can successfully populate the corpus and proceed into iterative fuzzing mutations.
