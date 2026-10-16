---
category: Rust
---

# Return a test fixture constructor directly when strict Clippy rejects let and return

The first pipeline delivery strict check exited 101 on a new derived-geometry
fixture helper that bound a constructor and immediately returned it. Return the
same literal directly, with no lint suppression and no assertion, scalar, source
reader or Scene-oracle changes. Preserve the actual failed log and rerun strict
checks plus the affected suite on the final source.
