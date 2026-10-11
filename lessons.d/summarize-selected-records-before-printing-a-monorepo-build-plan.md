---
category: CI & GitHub Actions
---

# Summarize selected records before printing a monorepo build plan

A CI plan inspection printed the entire platform_overrides object while looking for one compiler consumer. Those objects include monorepo dependency graphs, so the response exceeded 700,000 tokens before output truncation; truncation did not make the query appropriately bounded.

Inspect schema keys first, then select the exact compiler package, per-platform affected membership, toolchain requirement and gate booleans. Preserve the raw plan outside the checkout and emit a small verified receipt with its hash, count and source-head binding. Do not print unrelated graph edges or rely on a truncation limit as the primary data-selection boundary.
