## Unreleased — Twig comparisons with negative operands (#18453)

Add four Twig `lang_matrix` rows. Each puts a negative operand in front of a
`bool`-typed comparison:

- `(define (f s) (if (string<? s "m") 1 2)) (f "a")` exits 1;
- `(if (string>? "a" "b") 42 7)` exits 7;
- `(if (< -5 0) 42 7)` exits 42;
- `(define (f x) (if (< x 0) 42 7)) (f -5)` exits 42.

The first three are declared on all eight columns. The fourth is on NativeAot,
LLVM, WASM and JVM only:

- **Vm / Jit:** vm-core does not register the `call_builtin "<"` that a
  comparison on a function parameter reaches. That is a capability gap, not a
  sign bug.
- **CLR / BEAM:** not run locally. No other row proves a comparison on a
  function parameter there.

`string<?` lowers to `str_cmp` (-1/0/1) and then `cmp_lt _, 0 [bool]`. `(< a b)`
lowers to `cmp_lt a, b [bool]`. The x86-64 and AArch64 backends treated the
`bool` suffix as unsigned, so -1 < 0 was false natively. Both backends now
compare it as signed. `op_conformance.rs`'s `compare` comment records the fix.
