---
category: Compiler / VM / language pipeline
---

# A native comparison's signedness comes from the CIR suffix the specialiser picks, so dump the CIR before trusting a doc comment or an issue about which op is emitted

**What went wrong.** Issue #18483 said twig-aot's untyped native path
compared Twig integers with `cmp_*_u64`, citing a doc comment that called
the untyped path "u64". The code had moved on: params were already
normalised to `i64`. A probe test that ran `prepare_module_for_aot` and
printed `aot_specialise`'s output for `(if (< -8 100) 1 2)` showed
`cmp_lt_bool`, not `cmp_lt_u64`. The frontend hints `(< a b)` with its
*result* type, `bool`. `propagate_aot_types` only fills `"any"` hints, so
the `bool` hint survived and became the suffix. The x86-64 and AArch64
backends read `bool` as unsigned. Fixing "u64 → i64" where the doc pointed
would have changed nothing.

**Fix.** twig-aot's `sign_word_comparisons` retypes a `bool`-hinted
comparison over 64-bit integer words `i64` (its operand type, the IIR
convention). A unit test asserts every comparison in the negative-operand
programs specialises to `_i64`.

**Do differently.** For a native signedness bug, dump the CIR first:
`aot_specialise(f, Some(&infer_types(f)))` after the same preparation the
column runs. The suffix printed there is what the backend sees. Then fix
where that suffix comes from, and pin it with a test on the CIR, not only
on the exit code.
