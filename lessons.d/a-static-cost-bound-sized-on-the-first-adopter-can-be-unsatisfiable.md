---
category: Compiler / VM / language pipeline
---

# A static cost bound sized on the first adopter can be unsatisfiable for the next, and only an independent evaluator measures it

VER04r fixed `MAX_IR_COST = 2,000` node evaluations per expression, charging every
`model_call` at its body's full cost, because ct-compare's models are one node
deep. Building the independent oracle (python/warrant-oracle) and evaluating
gf256's `.warrant` models showed the bound cannot hold the second adopter:
charging every branch of an `if`, `xor8` alone costs 294,900 and `ginv`
92,014,191; charging only the larger branch `gmul` is 2,011 and `ginv` 26,204.
The generator never noticed because it evaluates models by running them, not by
costing them, and the spec text was written before any model recursed eight
levels. Seven of gf256's 38 suites are refused under the spec bound.

Fix (in the oracle, not the spec): the bound is a parameter; the spec value stays
the default so the finding is visible, `--max-ir-cost` is a labelled diagnostic
override, and the output records the bound and cost model it ran under. The spec
(or gf256's models) must change before slice S8.

Do differently: when a spec states a static bound, compute it for every adopter
that will be held to it *before* the spec is accepted, with the evaluator that
will enforce it. A bound that only the smallest adopter satisfies is a refusal
waiting for the second adopter, and it surfaces as a missing suite rather than a
red test unless the refusal is recorded by name.
