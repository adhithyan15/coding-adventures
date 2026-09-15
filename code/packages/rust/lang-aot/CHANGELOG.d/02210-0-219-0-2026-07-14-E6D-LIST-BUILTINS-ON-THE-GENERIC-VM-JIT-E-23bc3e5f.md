## 0.219.0 - 2026-07-14 (E6d list builtins on the generic VM/JIT — E6d COMPLETE on all 7 engines)

The 12 E6d-1/E6d-3 cons + list matrix cells (`(car (cons 42 0))`,
`(car (list 42 1 2))`, `(+ (length (list 1 2 3)) 39)`, `(null? (list))`,
`(list-ref …)`, `append`, `reverse`, `assoc`, …) regain `Vm` + `Jit`, now that
vm-core 0.19.0 handles `is_null` (list `null?`) with nil-handle reservation. With
records, unions, closures, and lists all running on the interpreter columns, **the
E6d dynamic-value feature set runs on all seven engines** (5 code-gen + VM + JIT).

Verified no regression: all 178 `Vm` and 178 `Jit` matrix cells produce the
expected result. `lang-aot` 0.218.0 → 0.219.0.

