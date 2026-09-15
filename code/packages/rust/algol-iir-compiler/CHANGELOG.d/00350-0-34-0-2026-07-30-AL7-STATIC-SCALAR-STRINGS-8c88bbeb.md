## 0.34.0 — 2026-07-30 — AL7 static scalar strings

Procedures can now assign and read scalar `string` variables declared by their
enclosing ALGOL block. These captures lower to typed module globals, allowing
the shared LANG backends to use their native string-handle representation.
`own string` has ALGOL static lifetime: a module-global initialization flag
installs its empty-string value on the first procedure call, then subsequent
calls reuse the same stored string.

The regression invokes a procedure that assigns an enclosing string and a
second procedure that writes and rereads an `own string` across calls. It
executes to 3 on Native AOT, LLVM, WASM, JVM, CLR, VM, and JIT.

