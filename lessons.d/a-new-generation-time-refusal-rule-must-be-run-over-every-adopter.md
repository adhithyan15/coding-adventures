---
category: Compiler / VM / language pipeline
---

# A new generation-time refusal rule must be run over every adopter document before it is committed, because a rule that is conservative on a synthetic fixture can block a real adopter

CV2-S3e added "a call of an op outside its `admit` is refused", and the first draft also refused
any call of an admit-bearing op whose argument contained a subject call. The new tests, the
ct-compare fixture and the synth fixture all passed; the oracle's existing gf256 tests did not
(`power(add(a, b), 2u64)` is a real gf256 claim, and `power`'s `admit` reads only `exp`), so the
draft would have blocked the gf256 adopter slice.

The fix narrowed the static refusal to the parameters the `admit` actually reads. What to do
differently: a refusal rule is a new way for a real document to fail, so run it over every adopter
`.warrant` (here `emit` over gf256, wasm-leb128, ... and the oracle's whole suite) before the
first commit, not only over the fixtures the slice owns; and when a rule needs a decision the
owner has not made, take the narrowest reading and record the wider one as OPEN.
