# iir-to-llvm

IIR → textual LLVM IR backend.  Emits a `.ll` source string for an LLVM
target triple, without depending on `llvm-sys` or a native LLVM install.

**Status: v0.1.0 — skeleton (LLVM01).**  This release emits a valid empty
module (a `; ModuleID` comment + a `target triple` directive) but does
**not** lower IIR instructions.  Instruction lowering arrives in v0.2.0+
(LLVM02–04).

## Where it fits

| Backend                       | Target                                |
|-------------------------------|---------------------------------------|
| `iir-to-wasm`                 | WebAssembly 1.0 bytecode              |
| `iir-to-jvm-class-file`       | JVM class file                        |
| `iir-to-cil-bytecode`         | CLR CIL bytecode                      |
| `iir-to-beam`                 | Erlang BEAM bytecode                  |
| **`iir-to-llvm` (this crate)**| LLVM textual IR (`.ll`)               |

The first four target *managed* runtimes (each runtime owns register
allocation, GC, etc.).  This crate is the first **AOT-native** IIR backend
that doesn't hand-roll its own machine encoder — instead it hands a `.ll`
string to LLVM (`opt` + `llc`) and lets LLVM produce native code for any
CPU LLVM ships a backend for.

The hand-rolled `aarch64-backend` / `x86_64-backend` crates remain the
right call when we want full encoding control (e.g. for the AOT debugger
story).  This crate is the right call when we want world-class O2
optimization for free.

## Why textual `.ll`, not `llvm-sys`?

- **Zero build-time dep.**  CI doesn't need LLVM installed; emit a string.
- **Debuggability.**  The output is the human-readable form.
- **Forward-compat.**  A `llvm-sys` emitter can be added later as a sibling
  without breaking callers.

The cost is that `.ll` is slower to ingest than bitcode.  For a hobby
codebase compiling small modules, that's irrelevant.

## Quick start

```rust
use interpreter_ir::IIRModule;
use iir_to_llvm::{validate_for_llvm, lower_iir_to_llvm, IIRLlvmConfig};

let module = IIRModule {
    name: "demo".into(),
    functions: vec![],
    entry_point: None,
    language: "demo".into(),
    exports: vec![],
    imports: vec![],
};

assert!(validate_for_llvm(&module).is_empty());

let ll = lower_iir_to_llvm(&module, &IIRLlvmConfig::default())
    .expect("lowering should succeed");
println!("{ll}");
// ; ModuleID = 'iir_module'
// target triple = "x86_64-unknown-linux-gnu"
```

## Configuration

`IIRLlvmConfig` has two knobs:

- `module_name` — emitted in the `; ModuleID = '<name>'` comment.
- `target_triple` — emitted in `target triple = "<triple>"`.

The default triple is a **fixed** string (`"x86_64-unknown-linux-gnu"`)
rather than a host-derived value.  This keeps test output byte-identical
across CI runners.  Override via `.with_target("riscv32-unknown-elf")` when
you actually intend to run `llc` for a non-default architecture.

## Roadmap

| Version | Scope                                                       |
|---------|-------------------------------------------------------------|
| v0.1.0  | Crate skeleton: empty module header. *(this release)*       |
| v0.2.0  | Function signatures + `ret`/`ret_void`/`const`/`mov`.       |
| v0.3.0  | Typed arithmetic (`add`/`sub`/`mul`/...) + cmp + branches.  |
| v0.4.0  | `call` + `call_builtin print_i64` extern declarations.      |
| v0.5.0  | Tagged-word lisp `cons`/`car`/`cdr` → `call @__twig_lispy_*` (McCarthy W12b-1). |
| v0.6.0  | `COND` via stack-slot (`alloca`) SSA-merge + `jmp_if` void-cond + empty-block `br` (McCarthy W12b-3). |
| v0.7.0  | Lisp symbols — `symbol` type → `i64` tagged immediate (McCarthy W13a). |
| v0.8.0  | Lisp lambda (F7) — declare `lispy_to_exit_code` runtime switch; **LLVM McCarthy-complete F1–F7** (McCarthy W13b). |
| (later) | GC, debug info via `!dbg`. |

See [`code/specs/iir-to-llvm.md`](../../../specs/iir-to-llvm.md) for the
full spec and [`code/specs/MULTILANG-BACKEND-PLAN.md`](../../../specs/MULTILANG-BACKEND-PLAN.md)
§LLVM for how this fits the broader plan.

## Tests

```sh
cargo test -p iir-to-llvm
```

7 tests at v0.1.0 covering validator stub, output shape, config defaults,
and error display.
