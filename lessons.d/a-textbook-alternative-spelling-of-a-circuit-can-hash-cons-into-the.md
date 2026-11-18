---
category: Testing & coverage
---

# A textbook alternative spelling of a circuit can hash-cons into the primary one: assert the two are different nodes before calling it a divergence

VOS-028b's capacity probe compares a code side with a reference side whose instructions are "encoded
differently": `a ^ b` as `(a | b) & !(a & b)`, an add with a constant as `a - (-c)`, a barrel shifter
with its stages reversed. Each spelling is logically equal to the primary and was checked equal on random
inputs, and several were *structurally identical* to it: `bit-blaster`'s own XOR is exactly `(a | b) & !(a & b)`,
a constant `neg` folds to a constant, and a shift by a constant has no stages left to reorder. The AIG
hash-conses them into the very same nodes, the miter folds to the constant false, and the "divergent"
row silently becomes a collapsed one that proves nothing (the VER-P030 failure in a new coat).

The fix is a test that builds the primary and every alternative in one AIG and asserts they are *different*
edges (`sem::an_alternative_is_a_different_circuit_from_the_primary`, and the same with a constant operand),
plus a per-operand table of which alternative survives (`sem::imm_alternative`). Equality on random inputs is
necessary and says nothing about whether the solver will have any work to do.
