# Changelog

## 0.1.0 -- 2026-03-21

### Added
- Initial implementation of the Lisp VM, ported from the Python `lisp-vm` package.
- `LispVm` struct with stack, variables, locals, heap, and symbol table.
- Heap with cons cells (`ConsCell`), symbols (`HeapSymbol`), and closures (`LispClosure`).
- Full opcode execution: stack ops, variable ops, arithmetic, comparison, control flow, function calls, Lisp-specific ops, I/O, halt.
- Closure support with environment capture.
- Tail call optimization via frame reuse.
- `run()` convenience function for compile-and-execute.
- `run_with_output()` for capturing print output.
- Comprehensive test suite covering all opcodes, cons cells, symbols, predicates, functions, tail calls, print output, and end-to-end programs (factorial, fibonacci, currying, higher-order functions).
