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
pub enum Value { Int(i64), Float(f64), Bool(bool), Str(String), HeapRef(gc_core::HeapRef), Null }
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

## GC-managed heap objects

`alloc`/`field_store`/`field_load`/`is_null` — what Twig's compiler actually
emits for every cons cell, record, union, and closure — allocate on
`gc-core`'s `FlatHeap`, the exact same collector engine the native-AOT
backends share via `gc-core-capi`, linked here directly as a Rust
dependency. They are direct dispatch-table aliases for `gc_alloc`/
`gc_field_store`/`gc_field_load` below (also reachable under those opcode
strings directly, e.g. for hand-built test IIR):

| Op | Effect |
|----|--------|
| `alloc` / `gc_alloc [<size_bytes>] -> dest` | Allocate on `FlatHeap` (kind 0), returning a `Value::HeapRef`. No operand defaults to 16 bytes (a 2-word cons cell). Capped by `max_memory_entries` live objects (an O(1)-tracked count) — bounds `gc_field_load`/`gc_field_store`'s O(live-count) size check against DoS |
| `field_load` / `gc_field_load dest <- obj, idx` | Read the `idx`-th 8-byte word. Decoded from a 3-bit tag in the word itself (low bits `111` → `HeapRef`, else → `Int`) — **not** from `type_hint`, which can't disambiguate a dynamically-typed field (a cons cell's car/cdr can hold either a nested pair or a plain integer at the same position) |
| `field_store` / `gc_field_store obj, idx, val` | Write the `idx`-th 8-byte word, tagging it (`HeapRef` → low bits `111`, address masked; `Int` → low bits `000`, value shifted left 3 — rejected if outside `[i64::MIN >> 3, i64::MAX >> 3]`, not truncated). Runs the write barrier for a `HeapRef` value. `Float`/`Bool`/`Str` are rejected — none fits the tag scheme |
| `is_null` | `true` for the top-level nil sentinel (`Int(0)`) *or* a `HeapRef` whose address is null — a field that stored nil and was read back through a `ref<...>`-hinted load decodes as the latter |
| `safepoint` | Collect only if `FlatHeap` is over its adaptive threshold (**paced**); also **compacts** (relocates objects) when `FlatHeap::should_compact` says fragmentation warrants it — the same shared policy `gc-core-capi`'s `__gc_safepoint` consults |
| `gc_collect` | Collect unconditionally, non-moving |

`alloc_array`/`array_get`/`array_set` (E5 arrays) remain on `ctx.arrays`, a
plain Rust bump arena that is never collected — a separate, still-open gap,
not an oversight.

Root-finding is **precise by construction** — no conservative stack scan.
`dispatch::collect_now` walks every `Value::HeapRef` across every frame's
registers, `globals`, `memory`, and `arrays`, and hands their exact storage
addresses to `FlatHeap::collect_mixed`: an interpreter already knows exactly
where every reference lives. A `safepoint` op and an automatic check every
4096 dispatched instructions (`AUTO_SAFEPOINT_INTERVAL`) both call the paced
path, so a long loop with no explicit `safepoint` still collects under
allocation pressure.

Fields are raw 64-bit words with no NaN-boxing — the same convention the
native cons-cell path uses — so a field holds either a nested `HeapRef`'s raw
address or a plain integer; storing anything else (a `Str`/`Float`/`Bool`)
traps. See `tests/gc_heap.rs` for the full round-trip and reclamation proofs.

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

94 tests across unit + integration suites, plus 6 doctests, all green.
