## 0.275.0 - 2026-08-27 (Twig dynamic parity on VM/JIT)

Twig's two remaining dynamic gaps now run on the generic VM/JIT columns:
quoted symbols are structurally interned before execution and `equal?` is
registered on the generic JIT callback table; forward-referenced globals use
the same lowered module-global slots those engines already execute.

The matrix adds a discriminating forward-global arithmetic case,
`(define (f) (+ g 1)) (define g 41) (f)`, across all eight standard engines.
It also closes the two representation bugs that case exposed: dynamic boxed
returns now retain `ref<any>` through helper return/call signatures so native
AOT and LLVM unbox the final value, and the BEAM path erases generic
`box`/`unbox` to identity moves because Erlang terms are already dynamic.

