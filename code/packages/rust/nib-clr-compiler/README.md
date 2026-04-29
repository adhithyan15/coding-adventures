# nib-clr-compiler

End-to-end Nib → CLR CIL bytecode compiler for the coding-adventures monorepo.

## What it does

Compiles Nib source text through a five-stage pipeline into CLR CIL (Common Intermediate
Language) method body bytes, ready for the CLR runtime or a PE/CLI wrapper.

```
Nib source
  → nib_parser::parse_nib()                 — lex + parse Nib tokens
  → nib_type_checker::check()               — type inference + constraint checking
  → nib_ir_compiler::compile_nib()          — emit target-independent IrProgram
  → ir_optimizer::optimize_program()        — constant fold + dead-code elimination
  → ir_to_cil_bytecode::lower_ir_to_cil_bytecode()
                                            — emit CIL method body bytes
  → CILProgramArtifact                      — structured CIL output
```

## Where it fits

`nib-clr-compiler` is the CLR/CIL leg of the Nib backend family:

| Package              | Target          |
|----------------------|-----------------|
| `nib-wasm-compiler`  | WASM binary     |
| `nib-jvm-compiler`   | JVM class file  |
| **`nib-clr-compiler`** | **CLR CIL**  |

All three share the same frontend (parser + type checker + IR compiler + IR optimiser) and
differ only in the final lowering step.

## Quick start

```rust
use nib_clr_compiler::compile_source;

// Compile a Nib program
let result = compile_source("fn main() { let x: u4 = 7; }").unwrap();
println!("CIL bytes: {} bytes", result.cil_bytes.len());
println!("Entry label: {}", result.cil_artifact.entry_label);

// Write raw CIL bytes to disk
use nib_clr_compiler::write_cil_file;
write_cil_file("fn main() { let x: u4 = 7; }", "/tmp/program.cil").unwrap();
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

### NibClrCompiler

```rust
use nib_clr_compiler::NibClrCompiler;

let result = NibClrCompiler {
    assembly_name: "MyAssembly".to_string(),
    type_name: "MyAssembly.Program".to_string(),
    optimize_ir: true,
    ..Default::default()
}
.compile_source("fn main() { let x: u4 = 7; }")
.unwrap();

assert_eq!(result.assembly_name, "MyAssembly");
```

### PackageResult fields

| Field            | Type                  | Description                                    |
|------------------|-----------------------|------------------------------------------------|
| `source`         | `String`              | Original Nib source                            |
| `assembly_name`  | `String`              | CLR assembly name in metadata                  |
| `type_name`      | `String`              | CLR type name in metadata                      |
| `ast`            | `GrammarASTNode`      | Parsed grammar AST                             |
| `typed_ast`      | `TypedAst`            | Type-annotated AST from type checker           |
| `raw_ir`         | `IrProgram`           | IR before optimisation                         |
| `optimized_ir`   | `IrProgram`           | IR after constant fold + DCE                   |
| `cil_artifact`   | `CILProgramArtifact`  | Structured CIL output (methods + locals)       |
| `cil_bytes`      | `Vec<u8>`             | Convenience copy of entry method body          |
| `assembly_path`  | `Option<PathBuf>`     | Path if written to disk                        |

### Error stages

| `stage`        | Cause                                    |
|----------------|------------------------------------------|
| `"parse"`      | Malformed Nib syntax                     |
| `"type-check"` | Type errors in the Nib source            |
| `"ir-compile"` | IR emission error (should be rare)       |
| `"lower-cil"`  | CIL lowering error                       |
| `"write"`      | File I/O error                           |

## CLR simulator note

The `clr-simulator` package implements a limited subset of CIL opcodes (arithmetic, branches,
local variable access).  Nib programs may emit `call` instructions for some operations, which
the simulator does not support.  Use `validate_for_clr` from `ir-to-cil-bytecode` to verify
that the IR is valid for CIL lowering; use a real CLR runtime (Mono, .NET) to execute the
output after wrapping in a PE/CLI assembly.

## Tests

24 tests (19 unit + 5 doc-tests).  Run with:

```bash
cargo test -p nib-clr-compiler
```

## Dependencies

| Crate                            | Role                                      |
|----------------------------------|-------------------------------------------|
| `coding-adventures-nib-parser`   | Nib lexer and parser                      |
| `nib-type-checker`               | Type inference + constraint checking      |
| `nib-ir-compiler`                | Nib typed AST → IrProgram                 |
| `compiler-ir`                    | `IrProgram`, `IrInstruction`, `IrOp`      |
| `ir-optimizer`                   | Constant fold + dead-code elimination     |
| `ir-to-cil-bytecode`             | IrProgram → CIL method body bytes         |
| `parser`                         | `GrammarASTNode` type                     |
| `type-checker-protocol`          | `TypeCheckResult`, `TypeErrorDiagnostic`  |
| `clr-simulator` (dev)            | Integration test for CIL bytecode         |
