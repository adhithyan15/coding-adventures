---
category: Security boundaries
---

# A frontend that reserves generated function names must reserve generated register names too

Twig's first two security fixes reserved the prefixes of the *functions* the
toolchain generates (`__lisp_`, `__dyn_`, `__lambda_`, `__twig_`). The third
review found the same take-over one level down. A Twig parameter or `let`
name becomes an IIR register verbatim, and the compiler and every later pass
invent registers in the same function. The compiler's temporaries were
`_n1`, `_n2`, so `(let ((_n2 40)) (+ _n2 2))` compiled the literal `2` into
the program's `_n2` and computed 4 on every column. twig-aot's string
lowering hands its `__aot_str{n}_len` register to the runtime's print as the
byte count. closure_heap, dyn_repr and iir-to-llvm all add `__…` registers
of their own.

**Fix:** reserve the whole `__` namespace in the frontend, the way C does,
instead of listing prefixes, and generate every temporary and label inside
it (`__t{n}_{what}`, `__l{n}_{what}`). The passes that run on IIR from every
frontend also defend themselves. closure_heap skips names the function
already uses, and twig-aot refuses a module that uses an `__aot_` register.
An unspellable *character* was not an option, because LLVM already uses `.`
for its own derived names (`%x.slot`) and labels share the `%` namespace
with values.

**Do differently:** when a name collision is a security bug, list every
namespace that generated names land in (functions, registers, labels,
backend-local names), not just the one the repro used. Write the repro that
proves each one, and check that the test fails with the fix reverted. The
first version of this test used two bindings and passed against the old
naming.
