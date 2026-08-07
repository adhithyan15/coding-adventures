# lisp-vm (C)

A **bytecode virtual machine** for Lisp — in pure ISO C17. A faithful port of
the Rust [`lisp-vm`](../../rust/lisp-vm) crate. It is the final stage of the
Lisp toolchain: [`lisp-lexer`](../lisp-lexer) →
[`lisp-parser`](../lisp-parser) → [`lisp-compiler`](../lisp-compiler) → **VM**.
It executes the `LcCodeObject` bytecode the compiler emits and produces a value.

## What it does

The VM is a **stack machine**. It walks a code object's instructions with a
program counter, pushing and popping `LcValue`s on a value stack:

- **globals** — a variable table (`define`), keyed by name;
- **locals** — indexed slots for a call frame's parameters;
- **a grow-only heap** — cons cells, interned symbols, and closures live here,
  and stack values refer to them by address (`LC_VAL_*ADDR`);
- **closures** — a lambda captures the environment it was created in, so
  `(lambda (y) (+ x y))` still sees `x` after the enclosing frame returns;
- **tail-call optimisation** — a call in tail position reuses the current frame
  instead of recursing in C, so `(countdown 10000)` runs in constant C stack.

```
   source ──lex──▶ tokens ──parse──▶ AST ──compile──▶ bytecode ──VM──▶ value
                                                      (LcCodeObject)
```

## Memory model

This is the tricky part of the port. Rust's `lisp-vm` leans on move semantics
and `Rc`/ownership; C has neither, so the VM is **explicitly malloc-owned**:
every value pushed onto the stack, stored in a variable, or placed on the heap
is **cloned** (`lc_value_clone`), and every value consumed off the stack is
**freed** (`lc_value_free`). Closures deep-copy their captured environment.
The heap is freed wholesale when the VM is destroyed. The test suite is run
under AddressSanitizer + UndefinedBehaviorSanitizer to prove there are no leaks,
double-frees, or use-after-frees.

## API

```c
#include "lisp_vm.h"

LcValue out;
LvError err;
if (lv_run("(define f (lambda (n) (cond ((eq n 0) 1) (t (* n (f (- n 1)))))))"
           "(f 5)", &out, &err)) {
    /* out.kind == LC_VAL_INTEGER, out.integer == 120 */
    lc_value_free(&out);
} else {
    /* err.message describes the compile or runtime error */
}
```

- `lv_new` / `lv_free` — create / destroy a VM.
- `lv_execute(vm, code, err)` — run a compiled `LcCodeObject`; `1` on success,
  `0` on a runtime error (fills `err`).
- `lv_stack_len` / `lv_stack_at` / `lv_stack_top` — inspect the value stack.
- `lv_heap_len` / `lv_heap_at` + `lv_heap_kind` / `lv_cons_car` / `lv_cons_cdr`
  / `lv_symbol_name` — inspect the heap.
- `lv_output_len` / `lv_output_at` — lines emitted by `(print ...)`.
- `lv_format_value` — render a value the way `print` does (owned string).
- `lv_run(source, out, err)` — compile + execute source in one call.
- `lv_run_with_output(source, out, out_lines, n_lines, err)` — same, but also
  returns the captured `print` output.

## Building

This package builds through the shared [`iso-harness`](../iso-harness)
multi-compiler engine, which compiles the test suite (and its sibling
`lisp-compiler`, `lisp-parser`, `lisp-lexer` sources) under every ISO C compiler
on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`.
