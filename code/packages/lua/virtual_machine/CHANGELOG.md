# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- Full port of Go virtual-machine package to Lua 5.4
- VirtualMachine: hard-coded opcode dispatch via Lua table lookup
  - Stack manipulation: LOAD_CONST, POP, DUP
  - Variable access: STORE_NAME, LOAD_NAME, STORE_LOCAL, LOAD_LOCAL
  - Arithmetic: ADD (number + string concat), SUB, MUL, DIV (integer)
  - Comparison: CMP_EQ, CMP_LT, CMP_GT (push 1/0)
  - Control flow: JUMP, JUMP_IF_FALSE, JUMP_IF_TRUE
  - Functions: CALL (with CallFrame save/restore), RETURN
  - I/O: PRINT (appends to output array)
  - HALT
- GenericVM: handler-based pluggable opcode interpreter
  - register_opcode() / register_builtin() for extensibility
  - push/pop/peek stack operations exposed as methods
  - push_frame/pop_frame with configurable max recursion depth
  - advance_pc / jump_to for program counter control
  - Freeze/unfreeze for sandboxing (prevents new handler registration)
  - reset() clears runtime state but preserves handlers and config
- Coroutine-based step-through debugging for both VMs (create_stepper)
- VMTrace snapshots at every instruction (pc, stack before/after, variables, output)
- assemble_code() constructor for CodeObject tables
- Exported helper functions: is_falsy, vm_add, copy_array, copy_map
- Comprehensive busted test suite (100 tests, targeting 95%+ coverage)
- Literate programming style with inline explanations, truth tables, and diagrams
