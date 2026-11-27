---
category: Rust
---

# A printer rule that refuses shadowed binders must be tried on the committed prelude before it is specified

VER32 3.6 says the PIF printer "refuses shadowed binders". The first `pif-pp` implemented that
literally (refuse any repeated binder name in scope) and the gate then refused the committed
PIF prelude: its generator names every non-dependent arrow binder `a`, so `Prop -> Prop ->
Prop` is two nested binders named `a`, and `Pif.Not` nests an arrow named `a` inside a `fun
(a : Prop)`. The worked example's own proof also nests two used `b` binders (the case-split
motives sit inside `fun b`).

What to do differently: before pinning a printer or lint rule into a gate, run it over every
committed file it will meet (the prelude, the worked example, the model fixtures), not only
over hand-written goals. Here variables print as `#n`, so a repeated name cannot change a
meaning; the rule became "refuse when the text can refer to either binder" (the new binder
is used in its body, or the old one is used inside the new scope). Unused binders, such as
the prelude's arrows, are allowed, and the whole-file worked example is still refused, which
the tests record. G7 keeps the strict literal rule for goals, where the elaborator controls
the names. If the spec's literal rule is wanted for files, the prelude generator must name
its arrows `_` (a digest change and a re-signing of `pif.prelude.faithful`).
