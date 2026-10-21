---
category: Compiler / VM / language pipeline
---

# JITCore's optimiser propagates constants into call_builtin operands, so a JIT builtin lowering must accept a literal where it expects a variable

`JITCore::compile_fn` runs `specialise` and then the CIR optimiser (constant
folding plus DCE) before `GenericCirJit` sees the function. Constant
propagation rewrites `const_f64 x 1.5; call_builtin "write_f64_py", x` into
`call_builtin "write_f64_py", Float(1.5)` and drops the const. A lowering
that pattern-matches `[_, CIROperand::Var(arg)]` therefore refuses every
constant argument, and the function silently falls back to the interpreter
tier.

The jit-core unit tests compile hand-written CIR, never optimised CIR, so they
passed. lang-aot's `op_conformance` JIT column caught it, because it asserts
`jit.is_compiled("main")`.

What to do: every JIT arm that reads an operand must accept the literal form
too (`resolve_operand` already does this for `Int`). Test new JIT lowerings
through `JITCore::compile`, not only `GenericCirJit::compile` on hand-built
CIR.
