# Changelog

## 0.1.0 — 2026-04-06

### Added

- Initial implementation of the generic register-based virtual machine
- Accumulator-centric execution model (same pattern as V8 Ignition)
- ~70 opcodes across 12 categories:
  - **0x0_ Accumulator loads**: LdaConstant, LdaZero, LdaSmi, LdaUndefined, LdaNull, LdaTrue, LdaFalse
  - **0x1_ Register moves**: Ldar, Star, Mov
  - **0x2_ Variable access**: LdaGlobal, StaGlobal, LdaLocal, StaLocal, LdaContextSlot, StaContextSlot, LdaCurrentContextSlot, StaCurrentContextSlot
  - **0x3_ Arithmetic**: Add, Sub, Mul, Div, Mod, Pow, AddSmi, SubSmi, BitwiseAnd, BitwiseOr, BitwiseXor, BitwiseNot, ShiftLeft, ShiftRight, ShiftRightLogical, Negate
  - **0x4_ Comparisons**: TestEqual, TestNotEqual, TestStrictEqual, TestStrictNotEqual, TestLessThan, TestGreaterThan, TestLessThanOrEqual, TestGreaterThanOrEqual, TestIn, TestInstanceOf, TestUndetectable, LogicalNot, TypeOf
  - **0x5_ Control flow**: Jump, JumpIfTrue, JumpIfFalse, JumpIfNull, JumpIfUndefined, JumpIfNullOrUndefined, JumpIfToBooleanTrue, JumpIfToBooleanFalse, JumpLoop
  - **0x6_ Calls**: CallAnyReceiver, CallProperty, CallUndefinedReceiver, Construct, ConstructWithSpread, CallWithSpread, Return, SuspendGenerator, ResumeGenerator
  - **0x7_ Property access**: LdaNamedProperty, StaNamedProperty, LdaKeyedProperty, StaKeyedProperty, LdaNamedPropertyNoFeedback, StaNamedPropertyNoFeedback, DeletePropertyStrict, DeletePropertySloppy
  - **0x8_ Object/array creation**: CreateObjectLiteral, CreateArrayLiteral, CreateRegExpLiteral, CreateClosure, CreateContext, CloneObject
  - **0x9_ Iteration**: GetIterator, CallIteratorStep, GetIteratorDone, GetIteratorValue
  - **0xA_ Exceptions**: Throw, Rethrow
  - **0xB_ Context/scope**: PushContext, PopContext, LdaModuleVariable, StaModuleVariable
  - **0xF_ VM control**: StackCheck, Debugger, Halt
- Per-function feedback vectors with 4-state slot progression:
  - `:uninitialized` → `{:monomorphic, [type_pair]}` → `{:polymorphic, [pairs]}` → `:megamorphic`
  - State machine deduplicates type pairs (stays monomorphic if same pair repeated)
  - Polymorphic threshold: 4 distinct type pairs before going megamorphic
- Hidden class tracking for property access feedback using `:erlang.phash2/1` on sorted keys
- Scope chain management: flat globals map + linked context frames for closures
- Call frame stack with configurable depth limit (default 500)
- `execute/1` — runs a CodeObject, returns `{:ok, %VMResult{}}`
- `execute_with_trace/1` — runs with full per-instruction trace, returns `{:ok, %VMResult{}, [TraceStep]}`
- `VMResult.final_feedback_vector` field exposing top-level frame's feedback after execution
- Full ExUnit test suite covering all 10 required scenarios plus additional edge cases
- Literate programming style throughout: all modules, functions, and significant blocks have inline documentation explaining what, why, and how
- Abstract equality (`==`) with type coercion (number vs. string comparison)
- JavaScript-style truthiness rules (false, nil, :undefined, 0, 0.0, "" are falsy)
- JavaScript-style `typeof` semantics including `typeof null === "object"` quirk
- String concatenation when either operand of Add is a binary
- Built-in `{:builtin, :print}` function support for observable output without IO
