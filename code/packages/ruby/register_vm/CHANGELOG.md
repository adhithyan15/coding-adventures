# Changelog

All notable changes to `coding_adventures_register_vm` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] – 2026-04-06

### Added

- **`Opcodes` module** — ~60 opcode constants organized into categories:
  accumulator loads, register moves, arithmetic, bitwise, comparison, control
  flow, function call, scope chain, object/array, type coercion, logical,
  I/O, and VM control. Each constant has a corresponding entry in the `NAMES`
  lookup table for disassembler and trace output.

- **`types.rb`** — Core data structures:
  - `UNDEFINED` singleton sentinel (distinct from `nil`) representing JavaScript
    `undefined`.
  - `VMObject` — a JS-style object backed by a Ruby `Hash` with a `hidden_class_id`
    for JIT inline-cache tracking.
  - `VMFunction` — a first-class function value pairing a `CodeObject` with a
    captured lexical `Context` (closure semantics).
  - `CodeObject` — compiled bytecode for one function: instruction array,
    constants pool, names array, register count, feedback slot count,
    parameter count, and a debug name.
  - `RegisterInstruction` — one bytecode instruction with opcode, operand array,
    and optional feedback slot index.
  - `CallFrame` — the execution context of one function invocation: IP,
    accumulator, register file, feedback vector, scope context, and caller frame
    back-pointer.
  - `VMResult` — the return value, accumulated PRINT output, and optional error
    from `execute`.
  - `VMError` — a runtime error carrying the faulting instruction index and opcode.
  - `Context` — one level of the lexical scope chain (slots array + parent pointer).
  - `TraceStep` — a snapshot of one instruction's execution for tracing and debugging.

- **`feedback.rb`** — Inline-cache state machine:
  - `Feedback.new_vector(size)` — allocates a fresh feedback vector.
  - `Feedback.record_binary_op(vector, slot, left, right)` — records a type pair
    for arithmetic/comparison feedback slots.
  - `Feedback.record_property_load(vector, slot, hidden_class_id)` — records the
    receiver shape for property-load feedback slots.
  - `Feedback.record_call_site(vector, slot, callee_type)` — records the callee
    type for call-site feedback slots.
  - `Feedback.update_slot(slot, pair)` — implements the one-way IC state machine:
    `uninitialized → monomorphic → polymorphic → megamorphic`.
    Duplicate type pairs do not advance the state.
  - `Feedback.value_type(v)` — classifies any Ruby value into a JS-style type
    string (`"number"`, `"string"`, `"boolean"`, `"null"`, `"undefined"`,
    `"object"`, `"array"`, `"function"`).
  - `Feedback.new_hidden_class_id` / `reset_hidden_class_counter!` — monotonic
    counter for object shapes.

- **`scope.rb`** — Lexical scope chain helpers:
  - `Scope.new_context(parent, slot_count)` — allocates a new scope level.
  - `Scope.get_slot(ctx, depth, idx)` — walks `depth` parent links and reads
    `slots[idx]`.
  - `Scope.set_slot(ctx, depth, idx, value)` — walks `depth` parent links and
    writes `slots[idx]`.

- **`interpreter.rb`** — Main bytecode execution engine:
  - `Interpreter#execute(code)` — runs a `CodeObject` to completion, returning a
    `VMResult`.
  - `Interpreter#execute_with_trace(code)` — same, but returns an `Array<TraceStep>`
    recording every instruction executed.
  - Full dispatch for all ~60 opcodes in a `case/when` loop.
  - JS-style truthiness (`false`, `nil`, `UNDEFINED`, `0`, `""` are falsy).
  - JS-style `typeof` semantics (including the `typeof null === "object"` quirk).
  - String concatenation when either operand of `ADD` is a `String`.
  - Division-by-zero and modulo-by-zero raise `VMError`.
  - `CALL` instruction supports `VMFunction` callees and host `Proc`/`Method`
    objects.
  - `CREATE_CLOSURE` captures the current lexical context.
  - `PUSH_CONTEXT` / `POP_CONTEXT` manage the scope chain.
  - `CALL_BUILTIN` dispatches to host-registered functions in `@globals`.
  - Pre-seeded built-ins: `Math.abs`, `Math.floor`, `Math.ceil`, `Math.round`,
    `Math.max`, `Math.min`, `String.length`.
  - Call-depth guard (default 500) raises `VMError` on infinite recursion.

- **Test suite** (857 lines, 10 test classes):
  - `TestArithmetic` — integer arithmetic, MOD, EXP, unary NEG/INC/DEC.
  - `TestBitwise` — AND, OR, XOR, NOT, left/right shifts.
  - `TestStrings` — string concatenation, `TYPEOF`, `TO_STRING`.
  - `TestComparison` — all six comparison operators, predicate tests.
  - `TestControlFlow` — conditional jumps, counting loop with `LOOP` back-edge.
  - `TestObjects` — `CREATE_OBJECT`, `LOAD/STORE/DELETE/HAS_PROPERTY`.
  - `TestArrays` — `CREATE_ARRAY`, `PUSH/LOAD/STORE_ELEMENT`, `ARRAY_LENGTH`.
  - `TestFunctions` — `CALL`, `RETURN`, recursive Fibonacci, `CREATE_CLOSURE`,
    closure captures scope correctly, `CALL_BUILTIN`.
  - `TestFeedback` — IC state machine transitions: uninitialized → monomorphic →
    polymorphic → megamorphic; deduplication; property-load shape tracking.
  - `TestErrors` — division by zero, unknown opcode, maximum call depth exceeded.

[0.1.0]: https://github.com/adhithyan15/coding-adventures/releases/tag/register_vm-v0.1.0
