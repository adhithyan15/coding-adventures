# iir-linker

Static linker for `IIRModule`s — implements the LANG33 module system at the
InterpreterIR level.

## What it does

`iir-linker` takes a set of `IIRModule`s (each produced by a separate
compilation unit) and merges them into one self-contained module that can be
executed by `vm-core` or lowered to BEAM / WASM / JVM / CLR by the
`iir-to-*` backends.

## Quick start

```rust
use interpreter_ir::{IIRModule, IIRFunction, IIRInstr};
use interpreter_ir::module_exports::{IIRExport, IIRImport};
use iir_linker::link;

// Module "math" exports "add".
let mut math = IIRModule::new("math", "twig");
math.entry_point = None;
math.add_or_replace(IIRFunction::new(
    "add",
    vec![("a".into(), "i64".into()), ("b".into(), "i64".into())],
    "i64",
    vec![IIRInstr::new("ret_void", None, vec![], "void")],
));
math.exports.push(IIRExport::new("add"));

// Module "app" imports "add".
let mut app = IIRModule::new("app", "twig");
app.add_or_replace(IIRFunction::new(
    "main", vec![], "void",
    vec![IIRInstr::new("ret_void", None, vec![], "void")],
));
app.imports.push(IIRImport::new("math", "add", "any"));

let merged = link(&[math, app]).unwrap();
assert!(merged.get_function("add").is_some());
assert!(merged.get_function("main").is_some());
assert!(merged.imports.is_empty()); // self-contained
```

## API

| Function | Purpose |
|----------|---------|
| `link(modules)` | Full link: resolve + type-check + merge. Returns all errors. |
| `link_strict(modules)` | Fail-fast variant: returns the first error. |
| `verify_imports(module, providers)` | Pre-flight: check imports without merging. |
| `IIRLinker::new().link(modules)` | Struct API (same as `link`). |

## Error types

```text
LinkError::Unresolved         — import has no matching export
LinkError::TypeMismatch       — import type annotations don't match export
LinkError::DuplicateExport    — two modules export the same (module, name)
LinkError::UndeclaredCall     — call instruction targets neither local nor import
```

## Where it fits

```
Language Frontend
    ↓  IIRModule (per compilation unit)
iir-linker::link
    ↓  merged IIRModule
vm-core / iir-to-beam / iir-to-wasm / iir-to-jvm-class-file / iir-to-cil-bytecode
```

## Spec

`code/specs/LANG33-module-system.md`
