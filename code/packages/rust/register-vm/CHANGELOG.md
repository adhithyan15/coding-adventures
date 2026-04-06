# Changelog — register-vm

All notable changes to this package will be documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] — 2026-04-06

### Added

- Initial implementation of the register-based VM modelled on V8 Ignition.

#### `src/opcodes.rs`
- ~70 opcode constants across 13 categories: accumulator loads, register moves,
  arithmetic, comparison, logical/bitwise, control flow, property access, array
  access, calls, construction, context/closure, type ops, stack check, and halt.
- `opcode_name(op: u8) -> &'static str` — human-readable name for disassembly
  and error messages.

#### `src/types.rs`
- `VMValue` enum — Integer, Float, Str, Bool, Null, Undefined, Object, Array,
  Function.  Implements `is_truthy()`, `type_name()`, `Display`, `PartialEq`.
- `VMObject` — JS-style property bag with `hidden_class_id` tracking and a
  `set_property` method that bumps the class id on shape change.
- `CodeObject` — immutable compiled function: instructions, constant pool,
  name table, register count, feedback-slot count, parameter count.
- `RegisterInstruction` — decoded instruction with opcode, operand list, and
  optional feedback-slot index.
- `CallFrame` — per-invocation state: accumulator, register file, feedback
  vector, lexical context, caller-frame link.
- `VMResult` — return value + output lines + optional error.
- `VMError` — runtime error with message, instruction index, and opcode byte.
  Implements `Display` and `std::error::Error`.

#### `src/feedback.rs`
- `FeedbackSlot` enum — Uninitialized → Monomorphic → Polymorphic → Megamorphic.
- `next_hidden_class_id()` — atomic global counter for fresh class IDs.
- `new_vector(size)` — allocate a feedback vector of `Uninitialized` slots.
- `value_type(v)` — type-name string for a `VMValue`.
- `record_binary_op()` — record a (lhs-type, rhs-type) pair at an arithmetic site.
- `record_property_load()` — record a hidden-class id at a property-load site.
- `record_call_site()` — record callee type at a call site.

#### `src/scope.rs`
- `Context` struct — slot array + optional parent pointer.
- `new_context(parent, slot_count)` — allocate a new context.
- `get_slot(ctx, depth, idx)` — walk the parent chain and load a variable.
- `set_slot(ctx, depth, idx, value)` — walk the parent chain and store a variable.

#### `src/vm.rs`
- `VM` struct — globals map, output buffer, call-depth counter, max-depth limit.
- `VM::new()` — create a VM with empty state.
- `VM::execute(code)` — run a `CodeObject` from instruction 0; returns `VMResult`.
- `run_frame()` — the main dispatch loop implementing all opcodes.
- Arithmetic helpers: `do_add`, `do_sub`, `do_mul`, `do_div`, `do_mod`,
  `do_bitwise_and/or/xor`, `compare_lt`.
- 10 unit tests covering: constant loading, register round-trip, monomorphic
  feedback, feedback state transitions, conditional jumps, global variables,
  function calls, HALT semantics, property-load feedback, and stack-overflow
  detection.

#### Package files
- `Cargo.toml` — no external dependencies.
- `BUILD` / `BUILD_windows` — `cargo test -p register-vm -- --nocapture`.
- `README.md` — architecture overview, opcode table, quick-start example.
- `CHANGELOG.md` — this file.
