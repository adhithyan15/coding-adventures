# aot-core

Ahead-of-time compilation engine for InterpreterIR (IIR).

`aot-core` compiles an entire `IIRModule` to a self-contained `.aot` binary
**before** the program runs.  It is the compile-time counterpart of `jit-core`'s
on-the-fly hot-path specialisation.

## Where it fits

```
Language Frontend
   → lexer / parser / type-checker / compiler → IIRModule
       │
       ├─ jit-core   (runtime: hot path specialisation)
       └─ aot-core   (compile time: all paths, all functions)
              │
              └─ .aot binary  ──→  simulator / native loader
```

## Compilation pipeline

```
IIRModule
    │
    ├── for each IIRFunction:
    │       infer_types(fn)          → HashMap<String, String>
    │       aot_specialise(fn, env)  → Vec<CIRInstr>
    │       CIROptimizer::run()      → Vec<CIRInstr>  (opt levels 1 / 2)
    │       backend.compile(cir)     → Option<Vec<u8>>
    │
    │   compiled  →  fn_binaries
    │   failed    →  untyped_fns  (→ vm-runtime IIR table)
    │
    ├── link(fn_binaries)                       → (native_code, offsets)
    ├── vm_runtime.serialise_iir_table(untyped) → iir_table bytes (if any)
    └── snapshot::write(native_code, iir_table) → Vec<u8>
```

## `.aot` binary format

```
┌─────────────────────────────────────────┐
│ Header (26 bytes, little-endian)        │
│   magic               4 bytes "AOT\0"  │
│   version             2 bytes 0x0100   │
│   flags               4 bytes          │
│   entry_point_offset  4 bytes          │
│   vm_iir_table_offset 4 bytes          │
│   vm_iir_table_size   4 bytes          │
│   native_code_size    4 bytes          │
├─────────────────────────────────────────┤
│ Code section (native_code_size bytes)   │
├─────────────────────────────────────────┤
│ IIR table (optional, when FLAG bit 0)   │
└─────────────────────────────────────────┘
```

## Usage

```rust
use interpreter_ir::module::IIRModule;
use interpreter_ir::function::IIRFunction;
use interpreter_ir::instr::{IIRInstr, Operand};
use jit_core::backend::NullBackend;
use aot_core::core::AOTCore;
use aot_core::snapshot::read;

// Build a tiny module.
let fn_ = IIRFunction::new("main", vec![], "void",
    vec![IIRInstr::new("ret_void", None, vec![], "void")]);
let mut module = IIRModule::new("hello", "tetrad");
module.add_or_replace(fn_);

// Compile to .aot binary.
let mut core = AOTCore::new(Box::new(NullBackend), None, 2);
let bytes = core.compile(&module).unwrap();

// Parse it back.
let snap = read(&bytes).unwrap();
assert_eq!(&bytes[0..4], b"AOT\x00");
```

## Modules

| Module | Purpose |
|--------|---------|
| `core` | `AOTCore` — top-level controller |
| `errors` | `AOTError` — error types |
| `stats` | `AOTStats` — compilation statistics |
| `infer` | `infer_types()` — static type inference |
| `specialise` | `aot_specialise()` — CIR generation |
| `link` | `link()` — binary concatenation |
| `snapshot` | `.aot` binary format reader/writer |
| `vm_runtime` | `VmRuntime` — IIR table serialisation |

## Dependencies

- `interpreter-ir` — `IIRModule`, `IIRFunction`, `IIRInstr`, `Operand`
- `jit-core` — `CIRInstr`, `Backend`, `CIROptimizer`
- `codegen-core` — re-exports from jit-core + `CodegenPipeline`
- `serde_json` — JSON encoding for the IIR table
