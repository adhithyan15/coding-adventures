---
category: Rust
---

# Rust mutation tests must retain owned diagnostics before moving fixtures

When a negative-fixture loop moves an owned JSON value into the mutated report,
formatting that value afterward in the assertion message borrows a moved value
and fails compilation. Clone small fixture values when assigning them, or build
the diagnostic string before the move; keep the test failure text available
without weakening the production check.
