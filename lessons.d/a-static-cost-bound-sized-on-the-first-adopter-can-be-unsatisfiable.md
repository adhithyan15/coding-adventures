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

Fix: VER04r was amended to cost the worst path (one branch of an `if`) with a
default of 2,000 that a corpus raises only through a `max_ir_cost` profile line
(ceiling 65,536), a per-suite `ir_cost` and a total IR-work bound. The oracle
has no override flag, so the limit lives in exactly one place.

Do differently: when a spec states a static bound, compute it for every adopter
that will be held to it *before* the spec is accepted, with the evaluator that
will enforce it. A bound that only the smallest adopter satisfies is a refusal
waiting for the second adopter, and it surfaces as a missing suite rather than a
red test unless the refusal is recorded by name.
