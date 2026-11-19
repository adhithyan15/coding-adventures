---
category: Testing & coverage
---

# Hand-encoded SAT gate controls must test the solver literal polarity before proofs

A bounded game-clock CNF draft treated `Var::lit(bool)` as the desired Boolean value, but the
`cdcl-sat` API parameter is `negated`: `false` means the positive literal and `true` means its
negation. A final miter can still return a plausible UNSAT result when constants or gate clauses
are reversed. Before relying on any hand-encoded theorem, exhaustively compare the AND/OR/XOR/mux
CNF truth tables and constant encodings with their truth functions, then replay concrete model
assignments against an independent scalar oracle. Treat a solver verdict as untrusted until those
small controls pass.
