## 0.2.2

- Every engine module captures the engine functions and `Limits` constants it uses at load and
  calls only those, so a target that rewrites `Handle`, `Executor`, `Limits` or other module tables
  through `require` changes nothing; the corpus runner snapshots `Hooks` per run; `Handle.intact`
  compares the full handle-metatable key set.
- Coroutines made through the swapped `create`/`wrap` are held strongly for one execution (cap 4,096)
  and checked after it; a wrapped coroutine is checked after each resume.
- Spec section 5.3: the require-rewrite rules, the create/wrap-captured-in-another-execution note
  and the stated residual (debug library, tamper-and-restore; out-of-process runner PT-F14).
- Tests: the reviewer's probes as regressions.
