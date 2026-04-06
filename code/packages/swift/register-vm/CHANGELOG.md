# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-06

### Added

- `Opcode` enum with 80+ opcodes across 13 categories (0x0_–0xF_), mirroring
  V8 Ignition's opcode layout. Each case is documented with operand semantics.
- `VMValue` indirect enum: integer, float, string, boolean, null, undefined,
  object, array, function. `isTruthy` property implements JavaScript coercion rules.
- `VMObject` class: hidden-class-ID property bag simulating V8's object model.
- `CodeObject` struct: compiled bytecode unit (instructions, constants, names,
  register count, feedback slot count, parameter count, debug name).
- `RegisterInstruction` struct: opcode + operand list + optional feedback slot index.
- `CallFrame` class: per-invocation state (IP, accumulator, registers,
  feedback vector, lexical context, caller frame back-link).
- `Context` class: scope-chain node for closure variables; `getSlot(depth:idx:)`
  and `setSlot(depth:idx:value:)` walk the parent chain.
- `FeedbackSlot` enum: uninitialized / monomorphic / polymorphic / megamorphic
  state machine with `newVector(size:)` factory.
- Free functions `valueType(_:)`, `recordBinaryOp(vector:slot:left:right:)`,
  `recordPropertyLoad(vector:slot:hiddenClassId:)`, `recordCallSite(vector:slot:calleeType:)`.
- `RegisterVM` struct: full interpreter with `execute(_:) -> VMResult`.
  Implements all 13 opcode categories in a single `switch` dispatch loop.
  Supports recursive `CodeObject`-backed function calls and a native function
  registry (`nativeFunctions`).
- `VMResult` struct: return value, output lines, optional `VMError`.
- `VMError` struct: message, instruction index, opcode byte.
- `nextHiddenClassId()` global counter for unique hidden class IDs.
- 10-test XCTest suite covering: constant loads, register round-trips,
  monomorphic add feedback, feedback state transitions, conditional branches,
  global variables, function calls, halt semantics, named-property feedback,
  and stack-overflow detection.
- `BUILD` and `BUILD_windows` scripts using the platform-aware
  `xcrun swift test` / `swift test` idiom.
- `.gitignore` excluding the `.build/` directory.
- `required_capabilities.json` (no capabilities; pure computation).
