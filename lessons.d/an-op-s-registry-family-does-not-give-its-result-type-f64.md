---
category: Compiler / VM / language pipeline
---

# An op's registry family does not give its result type: f64_signbit is Float-family but returns an integer

**What went wrong.** LANG83 PR6c registered `f64_signbit` in the `Float`
family (its operand is an `f64`) with an integer 0/1 result. jit-core's
`declared_support_matches_what_generic_jit_compiles` builds one function per
registry op and picks the result's `type_hint` from the op's *family*
(`Float` → `f64`). So the probe function was `f64_signbit r <- p0 : f64;
ret_f64 r`: the JIT correctly refused to treat an integer as a float register,
and the test reported `Lowered` vs "refuses" — a false failure from the
helper, not the compiler.

**Fix.** The helper special-cases `f64_signbit` → `i64`. The same mistake is
easy in any registry-driven test or tool (conformance builders, matrix
generators, docs tables).

**Do differently.** When adding a registry op whose result type differs from
its family's usual type (`f64_signbit`, and later any `*_is_nan`,
`*_to_bits`, comparisons on floats), grep every consumer of `OpFamily` and
`operand_types` for a family-to-type mapping and check it. Better still,
derive a result type from the op itself (its `type_hint` in the conformance
case) rather than from its family.
