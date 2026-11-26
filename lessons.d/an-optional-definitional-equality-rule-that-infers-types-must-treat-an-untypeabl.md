---
category: Rust
---

# An optional definitional-equality rule that infers types must treat an untypeable term as 'rule does not apply', not as an error

Proof irrelevance and eta need `infer` on the terms being compared. The first version of lean-kernel slice 3a propagated every `infer` error out of `is_def_eq`, so comparing terms over an undeclared constant (which slice 2 answered with a plain "not equal") started failing with `UnknownConstant`, and two slice-2 tests broke. Because these rules only ever add a "yes" verdict, the safe handling is: if the term cannot be typed, skip the rule and carry on exactly as before; propagate only `ResourceLimit`, `ForeignExpr` and `Internal` so that an exhausted budget never turns into an answer. Also remember that a new rule can legitimately change an old test: `theorem t : False -> False` and its value are now def-eq by proof irrelevance, so the slice-2 assertion that they differ had to be rewritten to say why (whnf still does not unfold the theorem). Separately, a Pi telescope of more than about 128 binders exceeds the universe-level depth limit (nested `imax`), so a hostile-input test that expects "moderately many" constructors to succeed must stay under roughly 120.
