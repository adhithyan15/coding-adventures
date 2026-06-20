# vm-core

**Generic register interpreter for InterpreterIR** — the execution engine for the LANG pipeline.

`vm-core` takes an `IIRModule` produced by any language frontend and runs it in a
register VM.  It is the interpreter tier: it warms up while `jit-core` profiles
and specialises hot functions.

---

## Position in the stack

```
IIRModule  (from any frontend)
  → vm-core  ← this crate  (interprets, profiles)
  → jit-core (specialises hot fns → CIRInstr)
  → Backend  (WASM / JVM / CIL / …)
```

---

## Quick start

```rust
use vm_core::core::VMCore;
use vm_core::value::Value;
use interpreter_ir::{IIRModule, IIRFunction, IIRInstr, Operand};

// Build "add(a, b) -> a + b"
let fn_ = IIRFunction::new(
    "add",
    vec![("a".into(), "u8".into()), ("b".into(), "u8".into())],
    "u8",
    vec![
        IIRInstr::new("add", Some("r".into()),
            vec![Operand::Var("a".into()), Operand::Var("b".into())], "u8"),
        IIRInstr::new("ret", None, vec![Operand::Var("r".into())], "u8"),
    ],
);
let mut module = IIRModule::new("calc", "tetrad");
module.add_or_replace(fn_);

let mut vm = VMCore::new();
let result = vm.execute(&mut module, "add", &[Value::Int(10), Value::Int(32)]).unwrap();
assert_eq!(result, Some(Value::Int(42)));
```

---

## Core components

### `Value` — the register type

```rust
pub enum Value { Int(i64), Float(f64), Bool(bool), Str(String), Null }
```

`iir_type_name()` maps a `Value` to its IIR type string, with range-aware
integer classification (`0–255 → "u8"`, `0–65535 → "u16"`, etc.).

The standard dispatch covers the shared IIR: `const`/`mov`, arithmetic
(`add`/`sub`/`mul`/`div`/`mod`/`neg`), bitwise (`and`/`or`/`xor`/`not`/`shl`/`shr`),
all `cmp_*`, control flow (`label`/`jmp`/`jmp_if_*`/`ret`), registers
(`load_reg`/`store_reg`), the flat memory ops (`load_mem`/`store_mem`), the **byte-tape
ops** (`alloc_bytes`/`load_byte`/`store_byte` — a cell is `memory[base+idx]`, `store_byte`
masks to a byte; this is how Brainfuck runs on the VM, v0.4.0), `call`/`call_builtin`, and
`io_*`. Because dispatch is over the shared IIR, **any** frontend that lowers to it runs
here unchanged — the matrix's six scalar languages all share this one interpreter
(LANG-MATRIX Phase V).

Integer arithmetic, bitwise, and shift results are **masked to the width named by
the instruction's `type_hint`** (`u4`→`& 0xF`, `u8`→`& 0xFF`, `u16`, `u32`), so a
`u8`-typed `add` of `200 + 100` wraps to `44` and `~x` on a `u8` flips 8 bits
(LANG-FULL E2). Hints of `i64`/`u64`/`any` keep full machine width. This is the
register-arithmetic analogue of the byte-tape `store_byte` mask, on top of the
whole-module `with_u8_wrap()` flag below.

**Floating-point (`f64`, LANG-FULL E3).** `add`/`sub`/`mul`/`div`/`neg` and the
ordered comparisons take a float track when an op is `f64`/`f32`-typed or has a
`Value::Float` operand — computing in `f64` and yielding a `Value::Float` (never
width-masked). Float division is IEEE-754 (`x / 0.0` → `±inf`, matching the
code-gen backends' `fdiv`, not a trap). This lets the VM execute the `f64` IIR
that the ALGOL 60 `real` frontend now emits; integer programs are untouched.

### `VMFrame` — per-call state

One frame per active function call.  Holds a flat `registers: Vec<Value>` and a
`name_to_reg: HashMap<String, usize>` that maps variable names to register
indices.  `assign()` grows the register file on demand, and `for_function()`
sizes it to at least the parameter count (`max(register_count, params.len())`) so
the dispatcher can place call arguments at indices `0..params.len()` even when a
frontend under-reports `register_count` (e.g. a hoisted McCarthy `LAMBDA` body).

### `VMCore` — the execution API

| Method | Description |
|--------|-------------|
| `VMCore::new()` | Create with sensible defaults |
| `VMCore::with_u8_wrap()` | Tetrad 8-bit mode (results masked `& 0xFF`) |
| `execute(module, fn_name, args)` | Run a function to completion |
| `register_jit_handler(name, fn)` | Short-circuit interpreter with native code |
| `register_opcode(op, handler)` | Add / override an opcode |
| `builtins_mut()` | Access the built-in function registry |
| `metrics_instrs()` | Total instructions dispatched |
| `metrics_jit_hits()` | Total JIT handler invocations |

### `BuiltinRegistry` — named built-ins

Pre-registered: `noop`, `assert_eq`, `print`.  Add your own:

```rust
vm.builtins_mut().register("sqrt", |args| {
    let n = args[0].as_f64().unwrap_or(0.0);
    Ok(Value::Float(n.sqrt()))
});
```

### `VMProfiler` — inline type feedback

When `profiler_enabled = true` (the default), every `"any"`-typed instruction
that produces a value records the runtime type in its `SlotState`.  `jit-core`
reads these slots to guide specialisation.

---

## JIT integration

```rust
vm.register_jit_handler("hot_fn", |args| {
    // Native Rust — bypasses the interpreter entirely.
    Value::Int(args[0].as_i64().unwrap_or(0) * 2)
});
```

The handler is called before the interpreter path whenever `call hot_fn …`
appears.  Unregister with `vm.unregister_jit_handler("hot_fn")`.

---

## Language extensions (custom opcodes)

```rust
use vm_core::dispatch::{DispatchCtx, OpcodeHandler};

let handler: OpcodeHandler = Box::new(|ctx, _jit_handlers, instr| {
    // Access frame, memory, etc. through ctx.
    Ok(None)
});
vm.register_opcode("tetrad.move", handler);
```

Custom opcodes shadow the standard table.

---

## Configuration

| Field | Default | Effect |
|-------|---------|--------|
| `u8_wrap` | `false` | Mask all arithmetic `& 0xFF` (Tetrad mode) |
| `profiler_enabled` | `true` | Collect type-feedback observations |
| `max_frames` | 512 | Call-stack depth limit (raises `FrameOverflow`) |

---

## Crate layout

```
src/
├── lib.rs          — crate root, re-exports
├── value.rs        — Value enum
├── errors.rs       — VMError variants
├── frame.rs        — VMFrame
├── profiler.rs     — VMProfiler
├── builtins.rs     — BuiltinRegistry
├── dispatch.rs     — DispatchCtx + standard opcode handlers + dispatch loop
└── core.rs         — VMCore (public API)
```

---

## Tests

```
cargo test -p vm-core
```

29 unit tests + 6 doctests, all green.
