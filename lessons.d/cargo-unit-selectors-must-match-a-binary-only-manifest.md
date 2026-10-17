---
category: Rust
---

# Cargo unit selectors must match a binary-only manifest

The first CLOC35 focused helper copied --lib from a library workflow, but
closurec declares only [[bin]] closurec. Cargo returned101 with no library target,
no build-finished event and no private tests; only the earlier CLI3 tests passed.

Read the manifest before selecting private units. Use --bin closurec plus the
intended filter, preserve the setup-failure receipt, and rerun under a new helper
version. A selector failure is neither a behavioral red nor implementation evidence.
