---
category: Compiler / VM / language pipeline
---

# Exempting a construct from a new ordering rule can strand its arguments with no emit path, so test it with a compound argument

The #18482 review made the C backend evaluate operands left to right.
`is_simple` became stricter: `a.get() + b.get()` was no longer simple,
because two of its operands run code.  The OOP dispatch builtins
(`__new__`, `__method__`, ...) had no hoisted (compound) emit path, so they
were exempted and kept the old rule, on the theory that this changed
nothing for them.

It did change something.  A dispatch call is simple only if each of its
arguments is, and `a.get() + b.get()` no longer was.  So `P(a.get() +
b.get())` went down the compound path that dispatch did not have, and
emitted `_sir_unknown_builtin("__new__")`, exiting 1 at runtime.
`a.add(a.get() + b.get())` was refused at compile time.  Both had worked
before.  The sir-conformance table missed it twice: no program passed a
compound argument to a constructor or method, and a backend refusal is
reported as a *skip*, not a failure.

The fix gave the dispatch builtins a real compound path (`emit_dispatch`,
which hoists the value arguments and keeps class and method names
literal).  That also closed the exemption's ordering gap (#18768).

What to do:

- When tightening a predicate like `is_simple`, check every caller that
  branches on it, including the constructs you exempt.  An exemption of
  the *outer* rule does not stop the stricter rule from reaching its
  *arguments*.
- Test the exempted construct with an argument that is newly non-simple.
- In a conformance table where every program should run everywhere, count
  a backend refusal as a failure.
