## 0.279.0 - 2026-08-28 (FLOW-MATIC unified matrix baseline)

The unified language matrix now executes a FLOW-MATIC record-output program
on NativeAOT, LLVM, WASM, JVM, CLR, VM, and JIT. The program exercises a
file-qualified field, `MOVE`, `WRITE-ITEM`, and the shared integer output
helpers, closing FLOW-MATIC's zero-coverage gap in the unified matrix.

Macsyma's missing universal-JIT lane remains independently tracked because it
requires dynamic runtime callback glue rather than a matrix-only proof.

