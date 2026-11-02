---
category: Security boundaries
---

# Every function a whole-program LLVM module defines except the entry must be internal or it can replace libc for the linked runtime

`iir-to-llvm` emitted every program function as `define <ty> @name`, which has
external linkage. In the final executable a global symbol has one definition,
and every reference binds to it, including references from the C runtime,
the Rust collector and libc's own callers. A Twig program that defined
`abort`, `malloc` or `calloc` therefore replaced libc's for all of them:

- `(define (abort) 0) (car 5)`: the pair guard's `abort()` returned. Only the
  guard's backstop `__builtin_trap` stopped the program (SIGILL).
- `(define (malloc n) 0)`: gc-core-capi's allocations failed.
- `(define (calloc a b) 0)`: `__dyn_pair_new` got a null cell.

The native backends were not affected, because their link step never
exported program functions.

**Fix:** in a module with an entry point, emit every function except the
entry as `define internal`. Check what the runtime calls back into by name
first. Here nothing did: closures dispatch through the module's own
`__dyn_call_closure`. A module without an entry point is a library and keeps
external linkage.

**Do differently:** when a compiler emits a whole program for linking with a
runtime, decide the linkage of every symbol on purpose. A program-defined name
must never be able to satisfy a reference the runtime makes. Test it by
defining the libc functions the runtime uses (`abort`, `malloc`, `calloc`) in
a program, and check the observable result (the exit status, or the signal),
not just that the build succeeds.
