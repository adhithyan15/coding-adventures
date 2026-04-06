# Changelog — @coding-adventures/register-vm

All notable changes to this package are documented here.
Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [0.1.0] — 2026-04-06

### Added

- **`src/opcodes.ts`** — 70+ opcodes in 12 categories (0x0_–0xF_), matching the V8 Ignition instruction set. Each opcode has JSDoc comments explaining semantics, operand encoding, and analogies to real V8 bytecode. Exports `Opcode` const enum and `opcodeName(op)` for human-readable disassembly.

- **`src/types.ts`** — Complete type system for the VM:
  - `VMValue` — union of all representable values (number, string, boolean, null, undefined, VMObject, VMArray, VMFunction)
  - `VMObject` — property bag with `hiddenClassId` for inline-cache simulation
  - `VMFunction` — closure with captured `Context`
  - `CodeObject` — compiled bytecode unit (instructions, constants, names, register/feedback counts)
  - `RegisterInstruction` — decoded instruction with opcode, operand list, and optional feedback slot
  - `CallFrame` — per-invocation execution state (ip, accumulator, registers, feedbackVector, context)
  - `FeedbackSlot` — discriminated union (`uninitialized | monomorphic | polymorphic | megamorphic`)
  - `Context` — lexical scope chain node
  - `TraceStep` — per-instruction execution snapshot for debugging
  - `VMResult` / `VMError` — return types

- **`src/feedback.ts`** — Feedback vector utilities implementing the IC state machine:
  - `newVector(size)` — allocate all-uninitialized vector
  - `valueType(v)` — classify VMValue into type string (distinguishes null, undefined, array from JS typeof)
  - `recordBinaryOp` — uninitialized→mono→poly→mega transitions for arithmetic sites
  - `recordPropertyLoad` — records hiddenClassId at property-load sites
  - `recordCallSite` — records callee type at call sites
  - Polymorphic budget: up to 4 distinct type pairs before going megamorphic

- **`src/scope.ts`** — Lexical scope chain operations:
  - `newContext(parent, slotCount)` — allocate a new scope level
  - `getSlot(ctx, depth, idx)` — walk `depth` parent links and read slot
  - `setSlot(ctx, depth, idx, value)` — walk and write

- **`src/vm.ts`** — Main `RegisterVM` class:
  - `execute(code)` — run bytecode, return VMResult (catches VMErrors internally)
  - `executeWithTrace(code)` — run with per-instruction TraceStep collection
  - Full dispatch switch for all 70+ opcodes
  - `doAdd` with JS-style string coercion: `number+number→number`, `string+?→string`
  - Hidden-class transitions on `STA_NAMED_PROPERTY` (new property → new class ID)
  - Closure support: `CREATE_CLOSURE` captures current context; calls restore it
  - Stack overflow: `STACK_CHECK` throws `VMError` when `callDepth > maxDepth`
  - `newObject()` / `objectWithHiddenClass()` exported for tests and compilers
  - Simplified iterator protocol (`GET_ITERATOR`, `CALL_ITERATOR_STEP`, etc.)

- **`src/index.ts`** — Clean re-export surface

- **`tests/register-vm.test.ts`** — 60+ test cases covering:
  1. `LDA_CONSTANT` + `HALT` → returnValue === 42
  2. `STAR` + `LDAR` register round-trip
  3. `ADD` same types → monomorphic feedback
  4. `ADD` mixed types → uninitialized→mono→poly→mega transitions
  5. `JUMP_IF_FALSE` conditional branch
  6. `LDA_GLOBAL` / `STA_GLOBAL` global variable access
  7. `CALL_ANY_RECEIVER` with closure
  8. `HALT` returns accumulator immediately (RETURN also stops frame)
  9. `LDA_NAMED_PROPERTY` → monomorphic hidden-class feedback
  10. `STACK_CHECK` throws on recursion overflow
  - Plus: all arithmetic opcodes, all bitwise opcodes, comparisons, `TYPEOF`, object/array creation, keyed property access, context/scope chain, `opcodeName`, feedback utilities, error handling, `executeWithTrace` trace validation

- **`BUILD` / `BUILD_windows`** — `npm install --silent\nnpx vitest run --coverage`

- **`required_capabilities.json`** — pure computation; no capabilities required

- **`README.md`** — package overview, stack-vs-register comparison, quick-start examples, feedback vector demo, closure example, API reference

- **`CHANGELOG.md`** — this file
