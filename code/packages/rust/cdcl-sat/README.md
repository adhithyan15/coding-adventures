# `cdcl-sat`

A conflict-driven clause-learning (CDCL) SAT solver, written from scratch
with no dependencies. **VER09** of the verified shipping program
([`code/specs/VER09-cdcl-sat.md`](../../../specs/VER09-cdcl-sat.md)).

It answers one question: *can this Boolean formula be made true?* In
VER00 §7.6 that question is a proof technique. Encode "there is an input
where the implementation and the specification disagree" as clauses. An
UNSAT answer then proves they agree on **every** input, and a SAT answer
hands back the counterexample.

## Where it fits

```text
  VER10 encodings (Tseitin, AIGs, bit-blasting)     constraint-engine::SatTactic
                     │                                        │ (adapter)
                     ▼                                        ▼
               cdcl-sat  ─────── (VER11) DRAT proof ──►  drat-check
                                                        (the trusted kernel)
```

The solver is not trusted: VER11's small proof checker is. So the solver may
use every trick that makes it fast, and a wrong UNSAT answer is caught by
its certificate.

## What is inside

| Feature | Where |
|---|---|
| Literals packed as `2·var + sign` in one `u32` | `src/lit.rs` |
| Two watched literals with blocker literals | `Solver::propagate` |
| First-UIP clause learning, recursive minimisation | `Solver::analyze` |
| Non-chronological backjumping | `Solver::search` |
| VSIDS activity heap, phase saving | `src/heap.rs`, `Solver::bump_var` |
| Luby restarts | `src/luby.rs` |
| Learned-clause reduction (LBD, then activity) | `Solver::reduce_db` |
| Assumptions, failed-assumption cores, incremental clause addition | `Solver::solve_with_assumptions` |
| Conflict budget (`Unknown` instead of running forever) | `Solver::set_conflict_budget` |
| Strict DIMACS CNF reader and writer | `src/dimacs.rs` |

Every source file is written to be read top to bottom. The module comments
explain the algorithm before the code implements it.

## Usage

```rust
use cdcl_sat::{parse_dimacs, Solver, SolveResult, Var};

// From DIMACS text.
let cnf = parse_dimacs("p cnf 2 2\n1 2 0\n-1 0\n").unwrap();
let mut s = Solver::from_cnf(&cnf);
assert_eq!(s.solve(), SolveResult::Sat);
assert!(cnf.is_satisfied_by(s.model()));

// Built directly, solved under assumptions.
let mut s = Solver::new();
let a = s.new_var();
let b = s.new_var();
s.add_clause(&[a.negative(), b.positive()]);   // a → b
assert_eq!(s.solve_with_assumptions(&[a.positive(), b.negative()]), SolveResult::Unsat);
assert_eq!(s.failed_assumptions().len(), 2);   // both assumptions are to blame
assert_eq!(s.solve(), SolveResult::Sat);        // nothing was learned as a fact
```

## How it is tested

- **Brute force:** 3000 random formulas with 1–12 variables, near the 3-SAT
  phase transition. Each answer is compared with exhaustive enumeration, and
  every model is re-checked against the formula.
- **Assumptions:** answers match brute force with the assumptions added as
  units. Every failed-assumption core is a subset of the assumptions, and
  is UNSAT on its own.
- **Incremental:** clauses are added in batches, with assumption probes in
  between, as bounded model checking does.
- **Pigeonhole** PHP(n+1, n) for n = 1…7 is UNSAT. PHP(n, n) is SAT, with a
  checked model.
- **DIMACS:** every error path, and writer/reader round trips.
- **Old DPLL:** `constraint-engine` cross-checks its adapter against the
  pre-VER09 DPLL and against truth tables.

Line coverage is above 99%.

## Capabilities

None. This is pure computation over in-memory data. Reading a `.cnf` file is
the caller's job (and the caller's capability). See
`required_capabilities.json`.
