---
category: Testing & coverage
---

# A rejection-sampling branch is unreachable by random draws: invert the generator's finalizer to build a seed that reaches it

`SplitMix64::bounded(n)` rejects any 64-bit output below `t = (2^64 - n) mod n`.
Since `t < n <= 2^32`, a rejection needs an output below 2^32 out of 2^64:
about one draw in four billion. A hand-made mutant that deleted the threshold
(`let threshold = 0`) therefore survived the whole suite, including a 300,000
draw uniformity test and an exact comparison against an independent sampler for
every `n` up to 3000, because none of those draws ever hit the branch. The
mutant was behaviourally identical on every sequence the tests could reach.

SplitMix64's finalizer is a bijection, so the branch can be reached on
purpose. The state that outputs 0 is 0, so the seed is `-gamma`
(`0x61c8864680b583eb`): its first output is 0, which `bounded(3)` must reject
(`t = 1`) and then answer from the second output. A test built on that seed
(`bounded_rejects_outputs_below_the_threshold` in
`code/packages/rust/test-bootstrap/tests/prng_contracts.rs`) kills the mutant.

What to do differently:

- When a mutant survives, ask whether the mutated branch is reachable by the
  generator at all before writing a statistical test. A statistical test cannot
  cover a branch with probability 2^-32.
- For a generator built from a bijective finalizer, invert it to construct the
  input that forces the rare branch; assert on the exact consumed-output count.
- Keep an exhaustive tiny-word analogue of the algorithm (a 4-bit word, `n = 6`)
  as a second, independent check of the arithmetic.
