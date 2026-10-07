---
category: Testing & coverage
---

# Refresh generated help and source-pinned CLI audits when changing a flag

Adding a Rust CLI extension changed both generated help and the source-pinned surface audit. The full compiler suite correctly rejected the stale help fixture. Regenerate help from the newly built binary, classify the extension in the closed local-extension set, and regenerate the audit with the exact upstream bytes whose size and SHA-256 match the pin. Run both gates again; a focused new-flag test alone does not check generated artifacts.
