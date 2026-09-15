## 0.280.0 - 2026-08-28 (Macsyma universal-JIT parity)

`run_macsyma_on_jit` drives Macsyma's v0 integer subset through the shared
dynamic-arithmetic lowering and `GenericCirJit`. The existing 21-program
Macsyma conformance corpus now agrees across VM, JIT, WASM, CLR, JVM, LLVM,
and native AOT.

The implementation disproves the earlier callback-glue assumption: Macsyma's
scalar integer slice needs no McCarthy tagged-runtime callbacks because
`lower_dynamic_arith` produces typed arithmetic plus generic `box`/`unbox`,
which are identity operations for VM/JIT values.

