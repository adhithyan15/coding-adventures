# codegen-core (Rust)

Universal IR-to-native compilation layer — LANG19 / LANG20.

`codegen-core` is the single shared layer that defines what *code generation*
means across every compilation path in this repository.  It provides three
composable abstractions:

```text
IR
  │
  ▼
[Optimizer<IR>]        ← optional IR → IR transformation
  │
  ▼
[Compile<IR>]          ← translate IR → Option<Vec<u8>>
  │  (CodegenPipeline wraps both)
  ▼
Option<Vec<u8>>        ← opaque native binary
```

---

## Compilation paths

```text
JIT / AOT path
  Vec<CIRInstr>
    → CIROptimizer::optimize()      (constant fold + DCE)
    → any Compile<Vec<CIRInstr>>    (Intel 4004 backend, …)
    → Option<Vec<u8>>

Compiled-language path  (Nib, Brainfuck, Algol-60)
  IrProgram
    → IrProgramOptimizer::optimize()  (NOP stripping)
    → any Compile<IrProgram>          (WASM, JVM, …)
    → Option<Vec<u8>>
```

---

## Module overview

| Module | Key items |
|--------|-----------|
| [`optimizer`] | `Optimizer<IR>` trait, `CIROptimizer` impl, `IrProgramOptimizer` |
| [`codegen`]   | `CodeGenerator<IR, Assembly>` trait (LANG20), `CodeGeneratorRegistry` |
| [`pipeline`]  | `Compile<IR>` trait, `CodegenPipeline<IR>`, `CodegenResult<IR>` |
| [`registry`]  | `BackendRegistry` |

---

## Quick start

### JIT pipeline

```rust
use codegen_core::{CIRInstr, CodegenPipeline, CIROptimizer};
use codegen_core::pipeline::Compile;
use jit_core::backend::NullBackend;

// NullBackend auto-implements Compile<Vec<CIRInstr>> via the blanket impl.
let pipeline: CodegenPipeline<Vec<CIRInstr>> = CodegenPipeline::new(
    Box::new(NullBackend),
    Some(Box::new(CIROptimizer)),
);

let cir: Vec<CIRInstr> = vec![ /* ... */ ];
let result = pipeline.compile_with_stats(cir);
println!("backend: {}, time: {}ns", result.backend_name, result.compilation_time_ns);
```

### LANG20 CodeGenerator

```rust
use codegen_core::codegen::{CodeGenerator, CodeGeneratorRegistry};

struct EchoGenerator;

impl CodeGenerator<String, String> for EchoGenerator {
    fn name(&self) -> &str { "echo" }
    fn validate(&self, _ir: &String) -> Vec<String> { vec![] }
    fn generate(&self, ir: &String) -> String { ir.clone() }
}

let mut registry = CodeGeneratorRegistry::new();
registry.register("echo", Box::new(EchoGenerator));
assert_eq!(registry.len(), 1);
```

---

## Re-exports from `jit-core`

`CIRInstr`, `CIROperand`, `CIROptimizer`, and `Backend` are already defined in
`jit-core` and re-exported here so callers can import everything from one place:

```rust
use codegen_core::{CIRInstr, CIROperand, CIROptimizer, Backend};
```

---

## Dependencies

| Crate | Role |
|-------|------|
| `jit-core` | `CIRInstr`, `Backend`, `CIROptimizer` |
| `compiler-ir` | `IrProgram`, `IrInstruction`, `IrOp` |
| `ir-optimizer` | `IrOptimizer` (NOP stripping) |

---

## Tests

```sh
cargo test -p codegen-core
```

41 unit tests + 17 doc tests = **58 total**, all green.
