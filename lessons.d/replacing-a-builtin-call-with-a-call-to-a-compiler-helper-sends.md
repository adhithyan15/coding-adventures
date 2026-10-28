---
category: Compiler / VM / language pipeline
---

# Replacing a builtin call with a call to a compiler helper sends the caller to the JIT's interpreter tier, so measure which functions the JIT compiles before and after

**What went wrong.** LANG83 PR7f-2 moved Lisp's truthiness, `not`,
`equal?` and exit coercion out of the tagged-word runtime (`dyn_truthy`,
`dyn_not`, `dyn_equal`, `dyn_to_exit_code` builtins) into compiler-emitted
IIR. The first draft emitted four module-local helper *functions* and a
`call` at every use, as PR7c did for float layout. Every program still
printed the right answer. A probe that listed `JITCore::is_compiled` for
each McCarthy JIT test program showed the cost: `main` of `(ATOM 7)` and
`(EQ 7 7)`, and the lambda of `((LAMBDA (X) (ATOM X)) 7)`, had been compiled
JIT bytecode and now ran on the interpreter tier. `GenericCirJit` compiles a
`call_builtin` to a registered handler, but it has no bytecode for `call`,
and a function containing `jmp`/`label` is `PartiallyTyped` and is never
eagerly compiled. Output tests cannot see this: the fallback tier gives the
same answer.

**Fix.** The two rules that are straight-line (truthiness is
`(x ≠ 3) × (x ≠ 1)` over the whole word, `not` is `5 − 2 × truthy`) are expanded inline,
every instruction typed `i64`, so a function using them stays `FullyTyped`
and compiles as before. Only `equal?` (recursive) and the exit-code switch
(which applies to a `call`'s result, so its caller already had a call) are
functions. The probe, re-run, differs from the baseline only where `EQ`
appears, which needs a recursive walk and cannot avoid a call.

**Do differently.** Before moving work from a builtin into emitted IIR,
record which functions the JIT compiles for the affected programs, and
compare after. Emit short straight-line rules inline; reserve helper
functions for work that needs branches, loops or recursion, and say in the
hand-off which functions moved tiers and why.
