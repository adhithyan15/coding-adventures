# Go SAT solver

A dependency-free incremental CDCL solver with watched-literal propagation,
first-UIP learning, assumption queries, and Tseitin gate/bit-vector builders.
Packed literals match the dedicated TypeScript package: `2*variable + sign`,
where the low bit marks negation.

```go
solver, err := satsolver.NewSolver()
if err != nil { panic(err) }
x, _ := solver.NewVar()
lit, _ := satsolver.MakeLit(x, false)
_, _ = solver.AddClause([]satsolver.Lit{lit})
result, err := solver.Solve()
model, valid := solver.Model()
```

`Solve` and `SolveWithAssumptions` return `sat`, `unsat`, or `unknown`.
`SolverOptions` uses optional `*uint64` budgets; a nil limit is unlimited and a
pointer to zero permits zero propagation/conflict work. A returned model and a
failed-assumption core are caller-owned copies. Failed cores are sound
assumption subsets, not guaranteed minimal. Adding a clause or resetting
invalidates snapshots.

`NewCnfBuilder(nil)` creates a solver automatically. The builder exposes
AND/OR/XOR/equivalence/ITE, full-adder, and little-endian bit-vector add/equal
operations. Addition keeps the low input-width bits and drops final carry.
`ParseDimacs` and `WriteDimacs` implement strict single-formula DIMACS transport
with CRLF, comments, spanning clauses, empty clauses, and SATLIB `%` support.

Variable, clause, clause-literal, and stored-literal ceilings bound memory use;
invalid inputs and resource refusals return errors. The solver returns no
proof certificate. Tests compare models and cores against independent bounded
enumeration and consume the shared SAT corpus also used by the TypeScript
package; this does not claim a source proof or DRAT validation.
