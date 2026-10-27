---
category: Security boundaries
---

# A trap written after a call to abort is dead code unless the call goes through a volatile pointer

The security review of the C runtime's pair guard (`dynval_runtime.c`,
`lispy_not_a_pair`) pointed out that `abort()` is a symbol the linker
resolves. On LLVM a Twig program's functions had external linkage (#18416),
so `(define (abort) 0)` replaced libc's `abort` for the whole executable, and
the runtime's `abort()` returned. The obvious fix is `abort();
__builtin_trap();`. It does nothing. libc declares `abort` `noreturn`, so
clang and GCC treat every instruction after the call as unreachable and
delete the trap. Checked with a returning `abort` linked in at -O0 and -O2:
the plain form fell off the end of the helper into the next function and
looped, printing refusals (one of them a heap address).

**Fix:** load the callee from a `volatile` function pointer
(`void (*volatile stop)(void) = abort; stop();`). The compiler can no longer
see the `noreturn` promise, so it keeps the `__builtin_trap()` (MSVC:
`__fastfail`). With that change the same probe exits with SIGILL under both
compilers and both optimisation levels.

**Do differently:** a "belt and braces" line after a `noreturn` call is
worthless unless something stops the compiler from proving it unreachable.
Test it by linking a returning replacement of the function and running the
binary, not by reading the source.
