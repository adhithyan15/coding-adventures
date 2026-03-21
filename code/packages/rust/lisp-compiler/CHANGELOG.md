# Changelog

## 0.1.0 -- 2026-03-21

### Added
- Initial implementation of the Lisp compiler, ported from the Python `lisp-compiler` package.
- `LispOp` enum with all Lisp bytecode opcodes (stack, variables, arithmetic, comparison, control flow, functions, Lisp-specific, I/O).
- `Instruction` struct with opcode and optional operand.
- `CodeObject` struct containing instructions, constants, and names.
- `Value` enum for runtime values (integer, string, boolean, nil, symbol, cons cell, closure, code object).
- `compile()` function that transforms Lisp source code into a `CodeObject`.
- Special form compilation: define, lambda, cond, quote, cons, car, cdr, atom, eq, print.
- Arithmetic operators: +, -, *, /.
- Comparison operators: =, <, >.
- Tail call optimization: emits TAIL_CALL for calls in tail position.
- Quoted data construction: builds cons chains from right to left.
- Custom `CompileError` type.
- Comprehensive test suite covering atoms, arithmetic, comparisons, defines, cons cells, predicates, quotes, conditionals, lambdas, function calls, tail calls, and multi-expression programs.
