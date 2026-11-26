---
category: Testing & coverage
---

# A metamethod-trap test that includes __gc fails whenever the collector happens to run, so stop the collector or leave __gc out

A helper that installs a metatable whose every metamethod records and raises (to prove
the engine never triggers a user metamethod) must not list `__gc` or `__close`: a
finalizer runs when the garbage collector decides to, not when the engine touches the
value, so the "log must be empty" assertion passed in a plain run and failed under
luacov, where allocation timing differs. Test the finalizers separately with
`collectgarbage("stop")` around the call and `collectgarbage("restart")` after.

Found building the VER05o hostile-value tests (`lua/test_bootstrap`).
