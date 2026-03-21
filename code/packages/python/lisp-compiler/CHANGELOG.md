# Changelog

## 0.1.0 — 2026-03-20

### Added

- **Lisp compiler** that transforms McCarthy's 1960 Lisp into GenericVM bytecode
- All 7 McCarthy special forms: `quote`, `atom`, `eq`, `car`, `cdr`, `cons`, `cond`
- `lambda` and `define` for functions and variable bindings
- Arithmetic operators (`+`, `-`, `*`, `/`) and comparisons (`eq`, `<`, `>`, `=`)
- **Tail call optimization**: compiler detects tail position and emits `TAIL_CALL`
- Quoted data construction: builds cons chains at runtime for `(quote (1 2 3))`
- `'x` shorthand for `(quote x)`
- Closure support: inner lambdas correctly capture outer lambda parameters
- `compile_lisp(source)` and `run_lisp(source)` convenience functions
- 48 compiler tests + 48 end-to-end tests (96 total), 95% coverage
- Full literate programming documentation
