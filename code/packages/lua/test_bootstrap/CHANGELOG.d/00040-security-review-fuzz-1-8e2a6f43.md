## 0.2.1

- Hook tampering is detected beyond the function: the count and mask are compared too
  (`debug.sethook(f, "", 1 << 30)` was a `pass`), for the main coroutine and every coroutine the
  target made through `create`/`wrap`, suspended ones included; the hook also re-checks its own
  count and mask each poll.
- The handle metatable is locked and its methods are unreachable from a handle; after every
  execution the engine verifies and repairs the `Handle` module, the metatable and the method
  table (crash kind `engine-tampered`, leaked).
- The `coroutine.create`/`wrap` replacements are restored in a `<close>` guard and fall through to
  the real functions when called after the execution.
- Memory polling backs off after a full collection that finds live memory under budget.
- Spec: residual in-process limits, the `fuzz-timeout`/`executions-leaked` convention for leaked
  executions, and that `report.seed`/`corpus` are raw fields.
- Tests: an explicit assertion that retired, forged and retained handles record no edge.
