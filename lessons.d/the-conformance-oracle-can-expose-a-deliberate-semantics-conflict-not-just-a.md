# The conformance oracle can expose a *deliberate* semantics conflict, not just a bug

Building the SIR21 integer/division reference oracle (`sir-conformance::oracle`)
and probing division across backends surfaced that Ruby's integer `/` (which
*floors*: `-7 / 2 == -4`) is implemented **four different wrong ways**: Python's
runtime `div` truncates (`int(a/b)` → `-3`) and is *deliberately* documented +
unit-tested that way ("to match SIR semantics"); JavaScript true-divides (`7 / 2`
→ `3.5`); Go/Rust crash on the negative path. So the oracle didn't just find
bugs — it found a **genuine design conflict** between a pre-existing, tested,
documented runtime decision (truncate) and the newer reference authority (Ruby
floor).

Lesson: when the reference model disagrees with a deliberately-chosen, tested,
documented behaviour in shared/published code, do **not** unilaterally flip it in
an autonomous loop — that overrides someone's decision and risks breaking the
tests that pin it. Capture the divergence as an `#[ignore]`d, oracle-judged
frontier test (so it's tracked and flips green when resolved), record the
conflict, and surface the *decision* (here: div_floor/div_trunc split vs. flip,
plus the `Integer#/` floors / `Float#/` true-divides polymorphism) to the human.
The split is exactly what SIR21 §E3 prescribes precisely to avoid one overloaded
`/` that different sources read differently.

Discovered: 2026-07-04, SIR21 division frontier (sir-conformance 0.10.0).

---
