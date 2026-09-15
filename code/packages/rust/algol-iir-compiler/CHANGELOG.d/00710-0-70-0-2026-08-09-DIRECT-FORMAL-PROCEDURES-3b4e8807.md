## 0.70.0 — 2026-08-09 — direct formal procedures

`procedure` formals now accept a direct declared procedure actual. The compiler
specialises the enclosing call, retains the target in a formal-procedure binding,
and invokes it with its existing checked signature; no function-pointer or
closure ABI crosses IIR. A formal may be forwarded through a nested direct
wrapper, while expressions, scalar variables, standard procedures, and `value`
procedure formals remain rejected because they need a runtime descriptor ABI.

