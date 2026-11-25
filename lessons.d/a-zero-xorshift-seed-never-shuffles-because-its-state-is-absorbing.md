---
category: Testing & coverage
---

# A zero xorshift seed never shuffles because its state is absorbing

The xorshift PRNG used by Dart's paired collector updates its state only with
shift-and-XOR operations. If the seed is zero, every update remains zero, so
the Fisher-Yates choices are all index zero and the balanced schedule stays in
its original alternating order.

Do differently: when a lane uses xorshift, map only the all-zero internal state
to a documented nonzero constant. Keep the user's configured seed in the
report, and test that zero is deterministic, balanced, and actually shuffled.
