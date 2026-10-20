---
category: Compiler / VM / language pipeline
---

# An operation defined on i64 must count as wide-model evidence, or the JVM scalar concretizer narrows its module to int32

**What went wrong (caught before it shipped).** `lang_aot::compile_iir_to_jvm_class`
runs `concretize_scalar_any_for_jvm`, which rewrites every `i64` in a "scalar"
module to `i32`. The in-repo `jvm-simulator` is a 32-bit machine. The pass
keeps a module wide only if it prints through `print_i64`, reads input, uses
reals, uses the tagged heap model, or has an i64 literal that does not fit in
32 bits. The LANG83 PR1b ops `i64_div_floor` and `i64_mod_floor` are
*defined* on i64: `MIN / -1` traps `Trap.Overflow` only because `MIN` is the
i64 minimum. A module that used them with `write_i64` or `putchar` output, and
no `print_i64`, would therefore be narrowed. The backend would then see `int`
operands for an op it lowers on `long`, which is a typed refusal at best and a
silent semantic change at worst.

**Fix.** The wide-evidence lists now name `write_i64` (next to `print_i64`)
and the ops `i64_div_floor` / `i64_mod_floor`.

**Next time.** When a registry operation's semantics depend on its width
(`i64_*` names, overflow edges), add it to every narrowing pass's evidence
list in the same PR: JVM `concretize_scalar_any_for_jvm` and the binary-path
`concretize_scalar_any_for_cil`. Also add a conformance case that uses the op
*without* `print_i64`, so the narrowing path is actually exercised.
