---
category: Compiler / VM / language pipeline
---

# An IIR type hint is also a native signedness choice: verify a hint change on the native backends

In #18453, the `cmp_lt` / `cmp_gt` after `str_cmp` in `(string<? a b)` was
re-hinted from `[i64]` to `[bool]` to make the hint agree with the recorded
type. Every IR-level check passed: the compiler unit tests, the soundness
report, twig-vm, and the backend validators. But `aot-core`'s `spec_type`
names the CIR op after the *result* hint (`cmp_lt_bool`). The x86-64 and
AArch64 backends then read the suffix as the operand signedness and took
`bool` as unsigned. So `str_cmp`'s -1 compared as `0xFFFF…FFFF < 0`, which is
false, and the `lang_matrix` `string<?` row exited 0 instead of 42 natively.
The same flaw was already live for Twig's `(< -5 0)`; no row had a negative
operand to show it.

The fix: both backends treat `bool` as signed (`is_signed_ty`). For a real
0/1 boolean, signed and unsigned agree.

What to do differently: a change to a `type_hint` on an arithmetic or
comparison op is a codegen change, not only a typing change. Run the
programs it touches through a native AOT probe before calling it done. For
example, use `twig_aot::compile_module_to_linux_executable` from a scratch
test, then delete the test. Give corpus rows negative operands, so a
signed/unsigned mix-up cannot stay hidden.
