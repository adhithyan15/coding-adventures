# Changelog

## [0.1.0] - 2026-04-06

### Added

- Initial implementation of `coding-adventures-register-vm`.
- `Opcode` enum with 70+ opcodes across 13 functional groups (0x0_–0xF_),
  mirroring V8 Ignition's bytecode layout.
- `CodeObject` — compiled unit of bytecode (instructions, constants, names,
  register count, feedback slot count, parameter count, name).
- `RegisterInstruction` — single bytecode instruction with opcode + operand list.
- `CallFrame` — per-activation runtime state: accumulator, register file,
  feedback vector, lexical context, caller frame.
- `RegisterVM` class with:
  - `execute(code)` — run a `CodeObject` and return a `VMResult`.
  - `execute_with_trace(code)` — run with per-instruction `TraceStep` recording.
  - `max_depth` guard against runaway recursion via `STACK_CHECK`.
  - Pre-seeded `print` global that captures output for deterministic tests.
- Module-level `execute()` and `execute_with_trace()` convenience wrappers.
- `feedback.py` — inline-cache feedback recording:
  - `new_vector(size)` — allocate a fresh feedback vector.
  - `record_binary_op`, `record_property_load`, `record_call_site` — update IC slots.
  - `_update_slot` — state machine: Uninitialized → Monomorphic → Polymorphic → Megamorphic.
  - `value_type(v)` — JavaScript-style type name for any VM value.
- `scope.py` — lexical context chain:
  - `new_context(parent, slot_count)` — allocate a new scope.
  - `get_slot(ctx, depth, idx)` / `set_slot(ctx, depth, idx, value)` — walk chain and access slots.
- `types.py` — all shared data structures with full literate docstrings.
- `_Undefined` singleton sentinel (`UNDEFINED`) distinct from `None` (null).
- `VMError` exception dataclass with `message`, `instruction_index`, `opcode`.
- `FeedbackSlot` discriminated union: `SlotUninitialized`, `SlotMonomorphic`,
  `SlotPolymorphic`, `SlotMegamorphic`.
- `Context` dataclass for lexical scoping.
- `TraceStep` dataclass for execution-trace entries.
- `py.typed` marker (PEP 561).
- Comprehensive test suite (`tests/test_register_vm.py`) covering:
  - Accumulator loads (all literal opcodes)
  - Register moves (STAR / LDAR / MOV)
  - Arithmetic (ADD, SUB, MUL, DIV, MOD, POW, ADD_SMI, NEGATE, bitwise ops)
  - Comparisons (TEST_EQUAL, TEST_LESS_THAN, TYPEOF, LOGICAL_NOT)
  - Control flow (JUMP, JUMP_IF_FALSE, JUMP_LOOP — including a counter loop)
  - Variable access (LDA/STA_GLOBAL, LDA/STA_LOCAL, context slots)
  - Objects (CREATE_OBJECT_LITERAL, named and keyed property access, array length)
  - Function calls (CREATE_CLOSURE, CALL_ANY_RECEIVER, built-in print, stack overflow)
  - Feedback vectors (Monomorphic, Polymorphic, Megamorphic transitions)
  - Execution tracing (length, accumulator progression, frame depth)
  - Scope/context (depth-0 R/W, parent chain, out-of-range guard)
  - Error handling (THROW, unknown opcode, incompatible ADD types, HALT)
- `BUILD` file for the monorepo build tool.
- `README.md` with architecture overview, opcode table, quick-start examples.
