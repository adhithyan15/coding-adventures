# `vm-runtime`

**LANG15** — Linkable, embeddable runtime library for AOT-compiled
InterpreterIR binaries.

## Overview

`vm-runtime` is the linkable, embeddable form of `vm-core` (LANG02). It
exists so that AOT-compiled binaries (LANG04) can fall back to interpretation
for the parts of a program that could not be fully specialised, without
depending on a host interpreter.

## Runtime levels

| Level | Name | What's linked | Typical program |
|-------|------|--------------|-----------------|
| 0 | `None` | Nothing (pure native code) | Fully-typed Tetrad on 4004 |
| 1 | `Minimal` | Dispatch loop + arithmetic/control | Mostly-typed program |
| 2 | `Standard` | Level 1 + builtins + I/O | Programs calling `print` |
| 3 | `Full` | Level 2 + profiler + GC hooks | Hybrid AOT+JIT or GC |

```rust
use vm_runtime::level::{required_level, RuntimeLevel};
let level = required_level(&module);
assert!(level >= RuntimeLevel::Minimal || level == RuntimeLevel::None);
```

## IIR table format (IIRT)

Unspecialised functions are embedded in the `.aot` binary as a flat binary
table with magic `"IIRT"`:

```rust
use vm_runtime::iir_table::{IIRTableWriter, IIRTableReader};

let mut writer = IIRTableWriter::new();
writer.add_function(fn_);
let blob = writer.serialise();  // starts with b"IIRT"

let reader = IIRTableReader::new(blob)?;
let idx = reader.lookup("main");  // binary-search by name
let fn_ = reader.get(idx.unwrap())?;
```

## In-process runtime (development / tests)

```rust
use vm_runtime::inprocess::InProcessVMRuntime;

let mut rt = InProcessVMRuntime::from_module(module);
let result = rt.vm_execute(fn_index, &[]);
assert!(!result.is_trap());
```

## VmResult

All vm-runtime entry points return a `VmResult` — a tagged union that covers
void, integers (u8–u64), bool, string, heap references, and traps:

```rust
use vm_runtime::result::{VmResult, VmResultTag};

let r = VmResult::from_bool(true);
assert_eq!(r.tag, VmResultTag::Bool);
assert_eq!(r.as_bool(), Some(true));
```

## Relocation entries

AOT binaries emit relocation entries for unresolved function/builtin
references that the linker patches after layout:

```rust
use vm_runtime::reloc::{RelocationKind, RelocationEntry};

let e = RelocationEntry {
    site_offset: 0x100,
    symbol: "helper".into(),
    kind: RelocationKind::IirFnIndex,
    addend: 0,
};
let bytes = e.serialise();  // 16 bytes
```

## Stack position

```
vm-runtime
    ├── interpreter-ir  (IIRModule, IIRFunction, IIRInstr)
    └── vm-core         (VMCore, Value)

Consumers:
    aot-core            (vm_runtime::VmRuntime is extended here)
    gc-core (LANG16)    (adds level-3 GC hook support)
```
