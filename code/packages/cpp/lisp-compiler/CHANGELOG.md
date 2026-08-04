# Changelog

All notable changes to the C++ `lisp-compiler` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-13

### Added

- Initial header-only pure-ISO C++17 port of the Rust `lisp-compiler` crate
  (namespace `ca::lisp_compiler`) — a recursive-descent compiler from the
  parser's S-expression AST to Lisp bytecode. Depends on the header-only
  `cpp/lisp-parser` and `cpp/lisp-lexer`.
- `compile(source)` and `compile_ast(program)` → `CodeObject`; throws
  `CompileError` on a parse or compile error. `Value` (structural `operator==`,
  `is_falsy`), `Instruction`, `CodeObject`, and the `LispOp` opcode enum.
- The full special-form set, arithmetic/comparison operators, cons/car/cdr,
  predicates, function calls, and tail-call optimisation. `Value::Code` bodies
  use a shared `CodeObject` for cheap copies; constant deduplication is
  structural.
- 48 checks mirroring the Rust crate's own unit tests, run under every available
  C++ compiler via the shared `iso-harness`; the suite also passes clean under
  AddressSanitizer + UndefinedBehaviorSanitizer.
