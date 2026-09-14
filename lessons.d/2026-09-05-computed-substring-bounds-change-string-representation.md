# 2026-09-05 — Computed substring bounds change string representation

A literal source does not imply a literal substring: runtime indices produce
a runtime handle. Propagate that fact to receiver copies and consumers before
building a function-wide literal table, or stale initializers can silently
replace live output. Keep actual cross-backend substring/MOVE programs with
padding markers and invalid-bound traps; frontend oracle tests and validator
acceptance did not expose native/LLVM missing routing or WASM stale facts.
