# KRN-S0: Lean 4 feasibility spike

A learning spike, not a published package. It tests the design of
`code/specs/VER11a-lean4-proof-kernel.md` (sections 6, 8 and 9) in official
Lean 4 before anything is built on it: can a Warrant claim about a WIR program
be proved for every input, with the semantics, step lemmas and lockstep rule
the spec proposes?

The verdict, measurements and the changes the spec needs are in the KRN-S0 hand-back
(the harness refused to let the spike agent write REPORT.md; the coordinator adds it from that hand-back).

## What is here

| File | Part | Contents |
|---|---|---|
| `KrnS0/Semantics.lean` | 1 | Minimal WIR in Lean: expressions, memory with tagged allocations, frame stack, `step`, `run` (structural on the step count), cost schedule, events, `obs`. |
| `KrnS0/CtEq.lean` | 1 | `ct_eq_equal_length` as WIR (hand transcription of the lowering). |
| `KrnS0/StepLemmas.lean` | 2 | One step lemma per instruction and terminator kind, including `step_branch_true` and `step_branch_false`. |
| `KrnS0/Relation.lean` | 3, 4 | The lockstep relation `R` (VER11a 8.3), memory and frame helper lemmas. |
| `KrnS0/Obligations.lean`, `KrnS0/Lockstep.lean` | 3 | One obligation lemma per instruction and terminator kind, over symbolic states, and `lockstep`. |
| `KrnS0/CtTheorem.lean` | 3, 4 | `lockstep_obs`, the label table, the loop-body block point by point, and `ct_eq_equal_length_ct` for every length. |
| `KrnS0/Witness.lean` | 5 | A concrete run to `ret`, closed by `decide`. |
| `KrnS0/Sabotage.lean` | control | The check refuses a wrong label table and a branch on a secret. |
| `KrnS0/Audit.lean` | 6 | `#print axioms` for the headline results. |

## Building

Lean core only: no Mathlib, no Batteries, no other dependencies. The toolchain
is pinned in `lean-toolchain` (Lean v4.35.0).

```sh
# one-time: install elan (https://github.com/leanprover/elan) with
#   sh elan-init.sh -y --no-modify-path --default-toolchain leanprover/lean4:v4.35.0
~/.elan/bin/lake build            # about 4 seconds from clean
~/.elan/bin/lake env lean KrnS0/Audit.lean   # axiom audit
```
