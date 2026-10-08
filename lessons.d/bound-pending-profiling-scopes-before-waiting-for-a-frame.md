---
category: C#
---

# Bound pending profiling scopes before waiting for a frame

Capping only completed profiling samples leaves pending rendering callbacks
unbounded when a minimized window continues to receive events. Reserve capacity
before creating a scope or subscribing to the rendering event; use a capped
atomic compare/exchange rather than a counter that keeps incrementing forever.
Test thousands of scopes held open at once, not only immediately disposed scopes.
Dispose must be idempotent. Keep export off the input path and test an invalid
export destination without changing application success.

When adding timing around snapshot persistence, place the scope inside the
method so conditional-save guards remain intact. Existing source-contract tests
caught the initial call-site wrapper. For JSON schema checks in a dependency-free
Rust wrapper crate, use the emitted C# driver's System.Text.Json rather than
assuming serde_json is a declared test dependency.