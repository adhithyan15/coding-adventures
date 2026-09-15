## 0.220.2 - 2026-07-30 (native closures — broaden capture coverage)

Test-only. `closures_run_on_native` (in `e6d7a_wasm_closures`) gains a **two-capture** case,
`(((lambda (a b) (lambda (c) (+ (+ a b) c))) 10 20) 12) = 42`: the inner lambda captures both `a`
and `b`, so the synthesized `__dyn_call_closure` dispatcher must `car`/`cdr`-walk a two-entry
captured-environment cons chain before the call-time argument — the multi-capture path the prior
capture-free and one-capture cases could not reach. Compiled to a real native binary and run. No
production-source change.

