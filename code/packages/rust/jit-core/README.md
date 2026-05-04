# jit-core (Rust)

JIT compilation engine for InterpreterIR.  This is the Rust port of the
Python `jit-core` package (LANG03).

## What it does

`jit-core` sits between the `vm-core` interpreter and the native backend.
It monitors function call counts and type-feedback slots, specialises hot
`IIRFunction`s into typed `CIRInstr` sequences, optimises them with constant
folding + dead-code elimination, and registers compiled handlers with `vm-core`
so subsequent calls bypass the interpreter.

```text
IIRModule (interpreter-ir)
    │
    ▼  vm-core interprets + profiles
VMProfiler fills IIRInstr.observed_type / .observation_count
    │
    ▼  jit-core detects hot functions
specialise()         IIRFunction → Vec<CIRInstr>  (typed + guarded)
CIROptimizer::run()  Vec<CIRInstr> → Vec<CIRInstr>  (folded + DCE'd)
Backend::compile()   Vec<CIRInstr> → Vec<u8>  (native binary)
    │
    ▼  vm-core JIT handler fires on next call
Backend::run()       Vec<u8> + args → Value
```

## Tiered compilation

| Tier | Default threshold | Trigger |
|---|---|---|
| `FullyTyped` | 0 | Before the very first interpreted call |
| `PartiallyTyped` | 10 | After 10 interpreted calls |
| `Untyped` | 100 | After 100 interpreted calls |

A threshold of `0` compiles the function before any interpreted execution —
useful for ahead-of-time compilation of statically-typed code.

## Deoptimisation

If a compiled function's type guards fire too often the JIT permanently
invalidates it.  The deopt threshold is `deopt_count / exec_count > 0.10`.
Invalidated functions run interpreted forever — recompiling a chronically
deopting function wastes CPU without improving throughput.

## Module structure

| Module | Contents |
|---|---|
| `errors` | `JITError`, `DeoptimizerError`, `UnspecializableError` |
| `cir` | `CIRInstr`, `CIROperand` — typed compiler IR |
| `backend` | `Backend` trait, `NullBackend`, `EchoBackend` |
| `optimizer` | `CIROptimizer` — constant folding + dead-code elimination |
| `specialise` | `specialise()` — IIRFunction → Vec\<CIRInstr\> |
| `cache` | `JITCache`, `JITCacheEntry` |
| `core` | `JITCore` — the top-level API |

## Quick start

```rust
use jit_core::core::JITCore;
use jit_core::backend::NullBackend;
use vm_core::core::VMCore;
use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};

// 1. Build an IIRModule.
let fn_ = IIRFunction::new(
    "add",
    vec![("a".into(), "u8".into()), ("b".into(), "u8".into())],
    "u8",
    vec![
        IIRInstr::new("add", Some("v0".into()), vec![
            Operand::Var("a".into()), Operand::Var("b".into()),
        ], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("v0".into())], "u8"),
    ],
);
let mut module = IIRModule::new("hello", "tetrad");
module.add_or_replace(fn_);

// 2. Attach JITCore to a VMCore.
let mut vm = VMCore::new();
let mut jit = JITCore::new(&mut vm, Box::new(NullBackend));

// 3. Execute with JIT — Phase 1 compiles eagerly, Phase 2 interprets,
//    Phase 3 promotes hot functions.
let result = jit.execute_with_jit(&mut vm, &mut module, "add", &[]);
assert!(jit.is_compiled("add"));
```

## Implementing a backend

```rust
use jit_core::backend::Backend;
use jit_core::cir::CIRInstr;
use vm_core::value::Value;

pub struct MyBackend;

impl Backend for MyBackend {
    fn name(&self) -> &str { "my-backend" }

    fn compile(&self, ir: &[CIRInstr]) -> Option<Vec<u8>> {
        // Translate CIRInstr list to your binary format.
        // Return None if compilation is not possible.
        Some(vec![0u8])  // stub
    }

    fn run(&self, binary: &[u8], args: &[Value]) -> Value {
        // Execute the binary with the given arguments.
        Value::Null  // stub
    }
}
```

## CIRInstr mnemonic conventions

| Category | Pattern | Examples |
|---|---|---|
| Integer arithmetic | `{op}_{type}` | `add_u8`, `sub_i32`, `mul_u64` |
| Float arithmetic | `{op}_f64` | `add_f64`, `div_f64` |
| Comparisons | `cmp_{rel}_{type}` | `cmp_eq_u8`, `cmp_lt_f64` |
| Constants | `const_{type}` | `const_u8`, `const_bool`, `const_f64` |
| Control flow | unchanged | `label`, `jmp`, `jmp_if_true` |
| Returns | `ret_{type}` / `ret_void` | `ret_u8`, `ret_void` |
| Runtime calls | `call_runtime` | `srcs[0]` = runtime name |
| Type guards | `type_assert` | `deopt_to` set to IIR index |

## Dependencies

- `interpreter-ir` — `IIRFunction`, `IIRInstr`, `Operand`, `SlotState`
- `vm-core` — `VMCore`, `Value`, `VMError`

## Tests

```bash
cargo test -p jit-core
# 91 unit tests + 8 doc-tests
```

## Position in the LANG pipeline

```
LANG01  interpreter-ir  — IIR bytecode format
LANG02  vm-core         — register interpreter
LANG03  jit-core        — THIS PACKAGE: JIT specialisation + compilation
LANG04  (future)        — codegen-core bridge
```
