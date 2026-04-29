# brainfuck-clr-compiler

End-to-end Brainfuck → CLR CIL bytecode compiler for the coding-adventures monorepo.

## What it does

Compiles Brainfuck source text through a four-stage pipeline into CLR CIL (Common Intermediate
Language) method body bytes, ready for the CLR runtime or the `clr-simulator` package.

```
Brainfuck source
  → brainfuck::parse_brainfuck()            — lex + parse BF tokens
  → brainfuck_ir_compiler::compile()        — emit target-independent IrProgram
  → ir_optimizer::optimize_program()        — constant fold + dead-code elimination
  → ir_to_cil_bytecode::lower_ir_to_cil_bytecode()
                                            — emit CIL method body bytes
  → CILProgramArtifact                      — structured CIL output
```

## Where it fits

`brainfuck-clr-compiler` is the CLR/CIL leg of the Brainfuck backend family:

| Package                   | Target          |
|---------------------------|-----------------|
| `brainfuck-wasm-compiler` | WASM binary     |
| `brainfuck-jvm-compiler`  | JVM class file  |
| **`brainfuck-clr-compiler`** | **CLR CIL**  |

All three share the same frontend (parser + IR compiler + IR optimiser) and differ only in the
final lowering step.

## Quick start

```rust
use brainfuck_clr_compiler::compile_source;

// Compile a BF program
let result = compile_source("++++.").unwrap();
println!("CIL bytes: {} bytes", result.cil_bytes.len());
println!("Entry label: {}", result.cil_artifact.entry_label);

// Write raw CIL bytes to disk
use brainfuck_clr_compiler::write_cil_file;
write_cil_file("++++.", "/tmp/program.cil").unwrap();
```

## API

### Free functions

```rust
// Compile to CIL artifact (default settings)
compile_source(source: &str) -> Result<PackageResult, PackageError>

// Alias for compile_source (mirrors Python pack_source API)
pack_source(source: &str) -> Result<PackageResult, PackageError>

// Compile + write raw CIL bytes to disk
write_cil_file(source: &str, path: impl AsRef<Path>) -> Result<PackageResult, PackageError>
```

### BrainfuckClrCompiler

```rust
use brainfuck_clr_compiler::BrainfuckClrCompiler;

let result = BrainfuckClrCompiler {
    filename: "hello.bf".to_string(),
    assembly_name: "HelloBrainfuck".to_string(),
    optimize_ir: true,
    ..Default::default()
}
.compile_source("+++.")
.unwrap();

assert_eq!(result.assembly_name, "HelloBrainfuck");
```

### PackageResult fields

| Field            | Type                  | Description                               |
|------------------|-----------------------|-------------------------------------------|
| `source`         | `String`              | Original BF source                        |
| `filename`       | `String`              | Logical filename for error messages       |
| `assembly_name`  | `String`              | CLR assembly name in metadata             |
| `ast`            | `GrammarASTNode`      | Parsed grammar AST                        |
| `raw_ir`         | `IrProgram`           | IR before optimisation                    |
| `optimized_ir`   | `IrProgram`           | IR after constant fold + DCE              |
| `cil_artifact`   | `CILProgramArtifact`  | Structured CIL output (methods + locals)  |
| `cil_bytes`      | `Vec<u8>`             | Convenience copy of entry method body     |
| `assembly_path`  | `Option<PathBuf>`     | Path if written to disk                   |

### Error stages

| `stage`        | Cause                                  |
|----------------|----------------------------------------|
| `"parse"`      | Malformed BF syntax (unmatched `[`/`]`)|
| `"ir-compile"` | IR emission error (should be rare)     |
| `"lower-cil"`  | CIL lowering error                     |
| `"write"`      | File I/O error                         |

## CLR simulator note

The `clr-simulator` package implements a limited subset of CIL opcodes (arithmetic, branches, local
variable access).  Full Brainfuck programs emit `call` instructions for tape memory operations
(`MemLoadByte`, `MemStoreByte`), which the simulator does not support.  Use `validate_for_clr` from
`ir-to-cil-bytecode` to verify that the IR is valid for CIL lowering; use a real CLR runtime
(Mono, .NET) to execute the output.

## Tests

25 tests (20 unit + 5 doc-tests).  Run with:

```bash
cargo test -p brainfuck-clr-compiler
```

## Dependencies

| Crate                    | Role                                 |
|--------------------------|--------------------------------------|
| `brainfuck`              | BF lexer and parser                  |
| `brainfuck-ir-compiler`  | BF AST → IrProgram                   |
| `compiler-ir`            | `IrProgram`, `IrInstruction`, `IrOp` |
| `ir-optimizer`           | Constant fold + dead-code elimination|
| `ir-to-cil-bytecode`     | IrProgram → CIL method body bytes    |
| `parser`                 | `GrammarASTNode` type                |
| `clr-simulator` (dev)    | Integration test for CIL bytecode    |
