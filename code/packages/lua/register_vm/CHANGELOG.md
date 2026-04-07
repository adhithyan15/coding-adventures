# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-06

### Added

- Complete register-based VM with accumulator model, modeled after V8's Ignition interpreter
- ~70 opcodes covering:
  - Immediate loads: `LDA_CONSTANT`, `LDA_ZERO`, `LDA_SMI`, `LDA_UNDEFINED`, `LDA_NULL`, `LDA_TRUE`, `LDA_FALSE`
  - Register moves: `LDAR`, `STAR`, `MOV`
  - Global/context variables: `LDA_GLOBAL`, `STA_GLOBAL`, `LDA_CONTEXT_SLOT`, `STA_CONTEXT_SLOT`, `LDA_CURRENT_CONTEXT_SLOT`, `STA_CURRENT_CONTEXT_SLOT`
  - Arithmetic: `ADD`, `SUB`, `MUL`, `DIV`, `MOD`, `POW`, `ADD_SMI`, `SUB_SMI`, `NEGATE`
  - Bitwise: `BITWISE_AND`, `BITWISE_OR`, `BITWISE_XOR`, `BITWISE_NOT`, `SHIFT_LEFT`, `SHIFT_RIGHT`, `SHIFT_RIGHT_LOGICAL`
  - Comparisons: `TEST_EQUAL`, `TEST_NOT_EQUAL`, `TEST_STRICT_EQUAL`, `TEST_STRICT_NOT_EQUAL`, `TEST_LESS_THAN`, `TEST_GREATER_THAN`, `TEST_LE`, `TEST_GE`, `TEST_IN`, `TEST_INSTANCE_OF`, `TEST_UNDETECTABLE`, `LOGICAL_NOT`, `TYPE_OF`
  - Control flow: `JUMP`, `JUMP_IF_TRUE`, `JUMP_IF_FALSE`, `JUMP_IF_NULL`, `JUMP_IF_UNDEFINED`, `JUMP_IF_NULL_OR_UNDEFINED`, `JUMP_IF_TO_BOOLEAN_TRUE`, `JUMP_IF_TO_BOOLEAN_FALSE`, `JUMP_LOOP`
  - Calls: `CALL_ANY_RECEIVER`, `CALL_PROPERTY`, `CALL_UNDEFINED_RECEIVER`, `CONSTRUCT`, `RETURN`, `SUSPEND_GENERATOR`, `RESUME_GENERATOR`
  - Property access: `LDA_NAMED_PROPERTY`, `STA_NAMED_PROPERTY`, `LDA_KEYED_PROPERTY`, `STA_KEYED_PROPERTY`, `LDA_NAMED_PROPERTY_NO_FEEDBACK`, `STA_NAMED_PROPERTY_NO_FEEDBACK`, `DELETE_PROPERTY_STRICT`, `DELETE_PROPERTY_SLOPPY`
  - Object creation: `CREATE_OBJECT_LITERAL`, `CREATE_ARRAY_LITERAL`, `CREATE_REGEXP_LITERAL`, `CREATE_CLOSURE`, `CREATE_CONTEXT`, `CLONE_OBJECT`
  - Iterators: `GET_ITERATOR`, `CALL_ITERATOR_STEP`, `GET_ITERATOR_DONE`, `GET_ITERATOR_VALUE`
  - Exceptions: `THROW`, `RETHROW`
  - Context/module: `PUSH_CONTEXT`, `POP_CONTEXT`, `LDA_MODULE_VARIABLE`, `STA_MODULE_VARIABLE`
  - VM meta: `STACK_CHECK`, `DEBUGGER`, `HALT`
- Feedback slot state machine: `uninitialized → monomorphic → polymorphic → megamorphic`
  - Deduplication: same type pair seen again does not advance the state
  - Megamorphic is a terminal state (no transitions out)
  - Arithmetic operations record `typeA:typeB` string pairs
  - Property access records `hclass:ID:hclass:ID` pairs using hidden class IDs
- Hidden class registry with global counter and sorted-key-string lookup
- Scope/context chain with `new_context` and `walk_context` helpers
- `execute(code_object, globals)` — returns `{value, error}`, never throws
- `execute_with_trace(code_object, globals)` — returns result + trace array
- `make_instruction` and `make_code_object` convenience constructors
- Exported helpers for testing: `new_vm_object`, `new_context`, `record_feedback`, `new_feedback_slot`
- Dispatch table architecture (not if/elseif chain) for O(1) opcode dispatch
- Call depth tracking with configurable `max_depth` (default 500) for stack overflow protection
- Literate programming style with inline comments explaining VM concepts for beginners
- Comprehensive busted test suite covering all 10 required test cases plus additional coverage:
  - LDA_CONSTANT + RETURN
  - STAR / LDAR round-trip
  - ADD + monomorphic feedback
  - Feedback state machine transitions (uni → mono → poly → mega)
  - JUMP / JUMP_IF_FALSE control flow
  - LDA_GLOBAL / STA_GLOBAL
  - CALL_ANY_RECEIVER frame push/pop
  - HALT
  - LDA_NAMED_PROPERTY + hidden class feedback
  - STACK_CHECK stack overflow detection
  - Additional: arithmetic, bitwise, TYPE_OF, context slots, TEST_EQUAL, LOGICAL_NOT, execute_with_trace, module API
