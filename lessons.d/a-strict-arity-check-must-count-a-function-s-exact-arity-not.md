---
category: Compiler / VM / language pipeline
---

# A strict-arity check must count a function's exact arity, not its positional parameters, or defaulted parameters become required

Issue #18466 made a strict closure's arity error SIR's `ArgumentError` on
every backend.  On Python the check went into the runtime's `apply`.  It
compared the call's argument count against `Closure.arity`, the closure's
introspected positional-parameter count.  That count includes defaulted
parameters: the emitter writes `def body(a, b=_SIR_MISSING)` for Ruby's
`->(a, b = 10) { a + b }`, so `arity` was 2.  `.call(1)` had returned 11;
after the change it raised `ArityError`.  The security review caught the
regression.

Every other backend keys its check on `Function::exact_arity()`, which is
`None` for a defaulted, variadic, keyword or block parameter, and checks
nothing in that case.  The fix made the emitter the only source of the
count: it writes `_sir_as_lambda(c, n)` only when `exact_arity()` is
`Some(n)`, and the runtime checks only a count it was given.

**When a backend enforces "the call must match the function", take the
number from the one definition the other backends share
(`Function::exact_arity`), not from whatever the target language can
introspect.**  Then test a closure whose function has a default: the
Twig-only regression table had no such row, because Twig has no defaults.
The new test gives a Twig lambda's parameter a default by hand.
