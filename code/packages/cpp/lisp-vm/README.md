# lisp-vm (C++)

A **bytecode virtual machine** for Lisp — header-only, pure ISO C++17. A
faithful port of the Rust [`lisp-vm`](../../rust/lisp-vm) crate and the final
stage of the Lisp toolchain: [`lisp-lexer`](../lisp-lexer) →
[`lisp-parser`](../lisp-parser) → [`lisp-compiler`](../lisp-compiler) → **VM**.
It executes the `ca::lisp_compiler::CodeObject` bytecode the compiler emits.

## What it does

The VM is a **stack machine** in the `ca::lisp_vm` namespace. It walks a code
object's instructions with a program counter, pushing and popping
`ca::lisp_compiler::Value`s on a value stack:

- **globals** — a variable table (`define`), keyed by name;
- **locals** — indexed slots for a call frame's parameters;
- **a grow-only heap** — a `std::vector<HeapObject>` where `HeapObject` is a
  `std::variant<ConsCell, HeapSymbol, LispClosure>`; stack values refer to heap
  entries by address;
- **closures** — a lambda captures the environment it was created in, so
  `(lambda (y) (+ x y))` still sees `x` after the enclosing frame returns;
- **tail-call optimisation** — a call in tail position reuses the current frame
  instead of recursing in C++, so `(countdown 10000)` runs in constant stack.

```
   source ──lex──▶ tokens ──parse──▶ AST ──compile──▶ bytecode ──VM──▶ value
                                                      (CodeObject)
```

Where the C port hand-manages every allocation, this port leans on C++ value
semantics: `std::vector`, `std::string`, `std::unordered_map`, `std::variant`,
and `std::shared_ptr<CodeObject>` (reused from `lisp-compiler`) do the ownership
bookkeeping. Errors are C++ exceptions (`ca::lisp_vm::VmError`) rather than
status codes.

## API

```cpp
#include "lisp_vm.hpp"

using namespace ca::lisp_vm;

Value v = run("(define f (lambda (n) (cond ((eq n 0) 1) (t (* n (f (- n 1)))))))"
              "(f 5)");
// v.kind == ValueKind::Integer, v.integer == 120

auto [result, lines] = run_with_output("(print 42)");
// lines == {"42"}
```

- `class LispVm` — `execute(code)`, plus public `stack`, `variables`, `heap`,
  `output` for inspection, and `format_value(v)`.
- `Value run(const std::string& source)` — compile + execute source, returns the
  top-of-stack value (or `nil`). Throws `VmError` on a compile or runtime error.
- `std::pair<Value, std::vector<std::string>> run_with_output(source)` — same,
  but also returns the lines captured from `(print ...)`.

## Building

This package builds through the shared [`iso-harness`](../../c/iso-harness)
multi-compiler engine, which compiles the test suite (and its sibling
`lisp-compiler`, `lisp-parser`, `lisp-lexer` sources) under every ISO C++
compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`.
