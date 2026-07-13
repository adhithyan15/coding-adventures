# Changelog

All notable changes to the C `lisp-compiler` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-13

### Added

- Initial pure-ISO C17 port of the Rust `lisp-compiler` crate — a
  recursive-descent compiler from the parser's S-expression AST to Lisp
  bytecode. Depends on the sibling `c/lisp-parser` and `c/lisp-lexer`.
- `lc_compile(source, out, err)` and `lc_compile_ast(program, out, err)`
  producing an `LcCodeObject` (a plain struct: instructions + constant pool +
  name pool, with nested lambda bodies as `LC_VAL_CODE` constants);
  `lc_code_object_free`, `lc_value_is_falsy`.
- The full opcode set (`LcOp`), special forms (`define`, `lambda`, `cond`,
  `quote`), arithmetic/comparison operators, cons/car/cdr, predicates, function
  calls, and tail-call optimisation (`LC_TAIL_CALL` in tail position).
- Recursive owned `LcValue` with structural (deep) constant deduplication; a
  `failed`-flag error model so bad syntax / OOM unwind cleanly and discard the
  partial output.
- 96 checks mirroring the Rust crate's own unit tests, run under every available
  C compiler via the shared `iso-harness`; the suite also passes clean under
  AddressSanitizer + UndefinedBehaviorSanitizer.
