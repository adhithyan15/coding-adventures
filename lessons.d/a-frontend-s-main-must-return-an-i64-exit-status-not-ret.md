---
category: Compiler / VM / language pipeline
---

# A frontend's main must return an i64 exit status, not ret_void, or BEAM reads [] back

**What went wrong.** JavaScript 6e's `main` ended in `ret_void` with return
type `void`. Native, LLVM, WASM, JVM, the VM and the JIT all treat that as
exit status 0, so every local cell passed. On BEAM, though, the matrix
harness prints `main()`'s result between `<<R>>` markers and parses it as an
integer, and a void Erlang function returns `[]`. So every JavaScript BEAM
cell failed in macOS CI with `stdout "[]"`, even though the program had
printed the right text. Neither `erl` nor `dotnet` exists locally, so the
first real run of those columns was in CI. Python and Ruby already return
`const 0` / `ret` (i64) for exactly this reason, and say so in a comment
(the LANG84 4a/5a lesson), but the JavaScript compiler predates the
AOT path and kept its VM-era `ret_void`.

**Fix.** `main` returns `i64`: `const 0`, then `ret`, pushed even after a
ReferenceError's trap. The release-class digests change with the IIR, and
the class partition does not.

**Next time.** When a frontend first joins lang-aot, compare its entry
function's epilogue with an existing frontend's (`ret` of an `i64` exit
status). And treat the columns you cannot run locally (CLR, BEAM) as
unproven until you have checked what their launchers read back from `main`.
