---
category: Compiler / VM / language pipeline
---

# The JIT's float registers refuse mov and cmp, so a frontend that copies an f64 into a binding register runs on the interpreter tier

**What went wrong.** LANG84 phase 6e's first JavaScript lowering kept the
pre-6e shape for `let x = 2`: compute into a temporary, then `mov js.x <-
js0` (`f64`). `jit-core`'s `GenericCirJit` carries floats as bit patterns in an
i64 register file and lets a float register meet only float-aware
instructions (`const_f64`, the PR6c `f64_*` ops, `f64_signbit`, the float
writers, `ret_f64`). A `mov` (specialised to `mov_f64`) or a `cmp_*` on one
refuses the whole function. The JIT column still printed the right answer,
because the function fell back to the vm-core interpreter, so nothing failed.

**Fix.** A binding's initializer writes `js.<name>` directly: when the value
is the temporary the last instruction just wrote, the compiler renames that
instruction's destination instead of adding a `mov` (a `mov` remains only for
`let y = x`). The `-0` test compares two `f64_signbit` results, which are
integers, rather than comparing the float with `0.0`.
`javascript_rows_compile_and_run_on_the_jit_without_its_interpreter_fallback`
compiles every row's `main` with `JITCore::compile`, asserts `is_compiled`,
and checks the compiled run's output.

**Next time.** Before emitting an instruction on a float, check the JIT's
float-register rule (`generic_jit.rs`, "LANG83 PR6a — the `f64` registers"),
and add a compile-and-run JIT test for any new frontend whose values are
floats. A green JIT cell does not show which tier ran.
