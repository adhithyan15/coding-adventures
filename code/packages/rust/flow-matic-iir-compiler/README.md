# flow-matic-iir-compiler

The **code-generation** layer of the FLOW-MATIC stack: it lowers a parsed
FLOW-MATIC program (B-0, 1955–1959) into [`interpreter_ir`](../interpreter-ir)'s
`IIRModule`, so FLOW-MATIC **runs on every execution backend** the LANG VM AOT
chain targets — NativeAOT, LLVM, WASM, JVM, CLR, the VM, and the JIT. It is the
sibling of the tree-walk semantics; the IR path is what makes FLOW-MATIC a
*compiled* language. Implements [PL09](../../../specs/PL09-codegen.md).

## API

```rust
use flow_matic_iir_compiler::compile_source;
let module = compile_source(source, "prog")?; // interpreter_ir::IIRModule
```

`main` returns an `i64` (the process exit code). Drive it end-to-end through
[`lang-aot`](../lang-aot) as `Language::FlowMatic`.

## Scope

FLOW-MATIC is a file/record data-flow language, but its **control flow** and
**scalar-field moves** lower with no file runtime: operations become labels,
`COMPARE`/`IF`/`OTHERWISE`/`GO TO`/`JUMP` become the IIR's comparison and branch
ops, `MOVE` a register copy, `WRITE-ITEM` prints the file's record (its fields via a
synthesized digit-print helper over `putchar`), `STOP` a `ret 0`, and the
`INPUT`/`OUTPUT`/`HSP` file declarations are no-ops. Each file-qualified field is
an `i64` register.

Record input (`READ-ITEM`, which needs the EOF-aware read — PL09 D4), `TRANSFER`, tape control, and the `END OF DATA` loop are a later rung and return a descriptive error until then —
never wrong output. See PL09 for the roadmap toward the full inventory-pricing
program and the COBOL frontends.
