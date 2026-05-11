# iir-codegen-adapters

Unified IIR backend registry: compile an `IIRModule` to **BEAM, WASM, JVM, or CLR** bytecode by name.

Part of the LANG pipeline.  Spec: [`LANG30-iir-codegen-adapters.md`](../../../specs/LANG30-iir-codegen-adapters.md).

## What it does

LANG29 delivered four new Rust crates that each lower an `IIRModule` to a target VM bytecode.
This crate wires them together into one registry so callers pick a backend by name string instead
of importing four crates and matching on strings themselves.

```
┌──────────────────────────────────────────────────────────────┐
│              iir-codegen-adapters                            │
│                                                              │
│  compile_iir(module, "iir-wasm")  ──▶  IIRBackendArtifact   │
│  compile_iir(module, "iir-beam")  ──▶       ::Wasm(...)      │
│  compile_iir(module, "iir-jvm")   ──▶       ::Beam(...)      │
│  compile_iir(module, "iir-clr")   ──▶       ::Jvm(...)       │
│                                        ::Clr(...)             │
└──────────────────────────────────────────────────────────────┘
```

## Quick start

```rust
use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};
use iir_codegen_adapters::{compile_iir, list_iir_backends, IIRBackendArtifact};

let fn_ = IIRFunction::new(
    "add",
    vec![("a".into(), "i32".into()), ("b".into(), "i32".into())],
    "i32",
    vec![
        IIRInstr::new("add", Some("v0".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "i32"),
        IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "i32"),
    ],
);
let module = IIRModule {
    name: "calc".into(), functions: vec![fn_],
    entry_point: Some("add".into()), language: "test".into(),
};

// Compile to WASM
let artifact = compile_iir(&module, "iir-wasm").unwrap();
let wasm_module = artifact.as_wasm().unwrap();

// Or compile to all four backends
for backend in list_iir_backends() {
    let art = compile_iir(&module, backend).unwrap();
    println!("{}", art);  // e.g. "Wasm(types=1, functions=1)"
}
```

## Available backends

| Name | Output type | Crate |
|------|-------------|-------|
| `"iir-beam"` | `BEAMModule` | `iir-to-beam` |
| `"iir-wasm"` | `WasmModule` | `iir-to-wasm` |
| `"iir-jvm"`  | `JvmClassFile` | `iir-to-jvm-class-file` |
| `"iir-clr"`  | `CILProgramArtifact` | `iir-to-cil-bytecode` |

## API

- `compile_iir(module, backend) -> Result<IIRBackendArtifact, IIRAdapterError>` — one-shot dispatch
- `build_iir_codegen_registry() -> CodeGeneratorRegistry` — type-erased registry for pipeline drivers
- `list_iir_backends() -> Vec<&'static str>` — enumerate backend names
- `IIRBackendArtifact` — closed enum wrapping all four artifact types
- `IIRAdapterError` — `UnknownBackend` | `ValidationFailed` | `LoweringFailed`

## Where this fits

```
Language Frontend
        │
        ▼
   IIRModule  (interpreter-ir)
        │
        ▼
iir-codegen-adapters  ◄─── this crate
        │
        ├──▶ iir-to-beam   → BEAMModule
        ├──▶ iir-to-wasm   → WasmModule
        ├──▶ iir-to-jvm-class-file → JvmClassFile
        └──▶ iir-to-cil-bytecode  → CILProgramArtifact
```
