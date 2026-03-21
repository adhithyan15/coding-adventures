# Changelog

## 0.1.0 — 2026-03-20

### Added

- **Lisp VM plugin** for GenericVM — registers all Lisp opcodes
- **LispOp enum** — 27 opcodes organized by category (stack, variables, arithmetic, comparison, control flow, functions, Lisp-specific, I/O, VM control)
- **NIL sentinel** — distinct Python object for Lisp's empty list / false / nothing
- **LispFunction** — callable wrapper around LispClosure heap addresses
- **Tail call optimization** — TAIL_CALL opcode reuses call frames for O(1) stack space
- **Closure support** — MAKE_CLOSURE captures environment, supports nested closures
- `create_lisp_vm(gc=None)` factory with pluggable garbage collector
- Pretty-printing for cons cells, symbols, and nested structures
- 41 tests (including hand-compiled factorial and TCO), 93% coverage
- Full literate programming documentation
