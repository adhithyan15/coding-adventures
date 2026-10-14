---
category: Compiler / VM / language pipeline
---

# Reconcile materialized value proof separately from mutable root proof

CV04a's pre/post alias guard correctly rejected selector effects that promote
local roots, but a newer upstream family returned computed name-actual scalars
from the branches themselves. Preparing those actuals also promotes their source
dependencies. Rechecking the old AST afterward then rejected eight previously
admitted materialized results at the formatter boundary.

Specify provenance at the value boundary. Prove both arms after the condition's
effects, carry proof only on the successfully materialized fresh real result,
and keep the original pre-lowering gate. Ordinary constructors carry no such
proof. Preserve selector-induced alias, initial shared-root and tracked-literal
refusals; pair them with exact native stdout for accepted snapshots. Keep failed
logs and source heads. A stopped partial run is not a completed negative PASS,
and neither a local repair nor its specification establishes final acceptance.
