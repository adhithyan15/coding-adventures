# Changelog

## 0.1.0 — 2026-03-20

### Added
- Initial release
- GenericVM: pluggable fetch-decode-execute bytecode interpreter
- Immutable functional design with handler signature `fn(vm, instruction, code) -> {output, vm}`
- Type structs: Instruction, CodeObject, VMTrace, CallFrame, BuiltinFunction
- Stack operations (push, pop, peek)
- Call stack (push_frame, pop_frame)
- Extra state map for language-specific extensions
- Builtin function registration
- Error types: VMError, StackUnderflowError, InvalidOpcodeError, etc.
- Step-by-step execution tracing
- 55 tests passing
