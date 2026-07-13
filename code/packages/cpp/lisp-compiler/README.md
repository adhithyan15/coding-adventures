# lisp-compiler (C++)

A **Lisp bytecode compiler** — S-expression AST → bytecode — **header-only** in
pure ISO C++17 (namespace `ca::lisp_compiler`). A faithful port of the Rust
[`lisp-compiler`](../../rust/lisp-compiler) crate, completing the toolchain on
top of the header-only [`lisp-parser`](../lisp-parser) and
[`lisp-lexer`](../lisp-lexer).

## What it does

The compiler inspects the first element of each list to assign meaning: special
forms (`define`, `lambda`, `cond`, `quote`), arithmetic / comparison operators,
cons cells, predicates, or an ordinary call. It tracks tail position so tail
calls emit `LispOp::TailCall`. The output `CodeObject` (instructions + constant
pool + name pool) is a plain struct; lambda bodies are `Value::Code` constants.

## API

- `CodeObject compile(const std::string& source)` — throws `CompileError` on a
  parse or compile error.
- `CodeObject compile_ast(const std::vector<lisp_parser::SExpr>& program)`.
- `Value` (with `kind`, `is_falsy()`, structural `operator==`), `CodeObject`,
  `Instruction`, and the `LispOp` opcode enum.

## Dependency

Depends on `cpp/lisp-parser` and `cpp/lisp-lexer` (both header-only); `run.sh`
adds their include paths. (This port also adds AST child accessors —
`child_count` / `child` / `dotted_last` / `quoted_inner` — to `lisp-parser`'s
`SExpr` for tree walking.)

## Design notes

- **Exceptions + value semantics.** Rust's `Result`/`CompileError` becomes a
  thrown `CompileError`; state unwinds via RAII. `Value::Code` uses a
  `std::shared_ptr<CodeObject>` for cheap copies, but equality (used for
  constant deduplication) is structural.
- **Header-only.** `#include "lisp_compiler.hpp"` and go.

## Building

```sh
sh BUILD           # POSIX: g++ and/or clang++ via the shared iso-harness
```

Compiles under GCC, Clang and MSVC with `-pedantic-errors` / `/permissive-` and
warnings-as-errors.
