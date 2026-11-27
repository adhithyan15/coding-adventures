---
category: Lua
---

# A Lua object whose instance field has the same name as a method shadows the method, and the call fails with attempt to call a boolean value

`setmetatable({ refused = false }, Budget)` with `function Budget:refused()` looks
fine until `scope:refused()` runs: `scope.refused` is found in the instance table first
(it is `false`), so the metatable's `__index` is never consulted. Same for `depth`,
`checks`, `attempts` and `work`. Name stored state with a suffix or prefix
(`refused_`, `depth_`) when the same word is also a query method.

Found building the VER05o budget tree (`lua/test_bootstrap`); five methods were dead
until the first test that called them.
