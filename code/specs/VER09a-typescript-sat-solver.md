# VER09a — TypeScript Level 3 CDCL SAT Solver & Tseitin Bit-Blaster (P034)

**Status:** Draft. Target package: `@coding-adventures/test-bootstrap/sat`.
**Layer:** Verification ladder, Level 3 (Formal Boolean Verification & SMT engine).
**Builds on:** [VER00](VER00-verification-manifesto.md), [VER09](VER09-cdcl-sat.md), [TF01a](TF01a-typescript-test-bootstrap.md).
**Consumed by:** Level 3 invariant checking, Level 4 bounded model checking, Warrant solver bindings, and symbolic execution.

---

## 1. Context & Motivation

Level 1 unit testing and Level 2 property-based / fuzz testing can find counterexamples when test inputs hit failing paths, but they cannot prove the *absence* of bugs. Level 3 formal verification provides mathematical certainty: a problem formula is encoded such that an **UNSAT** result constitutes a formal proof over all $2^N$ possible states, while a **SAT** result extracts a concrete violating assignment (counterexample model).

Per VER00 §7.1 and repository standards, `@coding-adventures/test-bootstrap` must remain completely self-contained with **zero npm runtime dependencies** (`"dependencies": {}`). This slice delivers the Level 3 CDCL (Conflict-Driven Clause Learning) SAT solver and Tseitin bit-blaster in pure, idiomatic TypeScript.

---

## 2. Solver Architecture

The solver implements a modern CDCL architecture based on MiniSat and [VER09](VER09-cdcl-sat.md):

### 2.1 Variable and Literal Encoding
- **Variable index (`Var`)**: Non-negative 32-bit integer `0..N-1`.
- **Literal (`Lit`)**: Packed into a single 32-bit integer:
  - `lit = (var << 1) | (sign ? 1 : 0)`
  - `var(lit) = lit >> 1`
  - `sign(lit) = lit & 1` (`1` = negated, `0` = positive)
  - `neg(lit) = lit ^ 1`
- **Three-Valued Logic (`LBool`)**:
  - `True = 1`, `False = -1`, `Undef = 0`.

### 2.2 Two Watched Literals (2WL) Unit Propagation
- Each clause watches two non-falsified literals.
- When literal $L$ becomes false, watcher lists for $L$ are traversed.
- Blocker literal optimization avoids dereferencing full clauses when the other watched literal or blocker is already satisfied.
- Unit clauses immediately propagate assignments to the trail; conflicts return the conflicting clause reference.

### 2.3 1-UIP Conflict Analysis & Non-Chronological Backjumping
- When a conflict occurs at decision level $d$, the implication graph is traversed backwards from the conflict clause.
- Literal resolution continues until exactly one literal from the current decision level remains (the **First Unique Implication Point** or 1-UIP).
- The resulting learned clause contains the negated 1-UIP literal as an asserting literal, plus literals from earlier decision levels.
- The solver backjumps to the second highest decision level in the learned clause, making the 1-UIP literal immediately unit-propagated.

### 2.4 Decision Heuristics & Restarts
- **VSIDS (Variable State Independent Decaying Sum)**:
  - Every variable maintains an activity score incremented during conflict analysis.
  - Scores decay exponentially by dividing by a decay factor (or scaling the activity bump by $1 / 0.95$).
  - Fast priority selection via binary heap.
- **Phase Saving**:
  - Remembers the last polarity assigned to each variable and reuses it upon subsequent decisions.
- **Luby Restarts**:
  - Relieves solver from unproductive branches using the Luby sequence ($1, 1, 2, 1, 1, 2, 4, \dots$).
  - Learned clauses and phase saving are preserved across restarts.

### 2.5 Incremental Solving & Assumptions
- Supports `solveWithAssumptions(assumptions: Lit[])`.
- Decisions at negative levels enforce assumptions; if unsatisfiable under assumptions, the solver extracts the **failed assumption core** (the minimal subset of assumptions that caused the conflict).
- Allows adding clauses between successive `solve()` invocations.

### 2.6 Hostile Input & Resource Bounding
- Guarded by explicit conflict budget (`maxConflicts`) and propagation step budget (`maxSteps`).
- Returns `SolveResult.Unknown` when budgets are exhausted, preventing denial-of-service on adversarial formulas.

---

## 3. Tseitin Transformation & Bit-Blaster

Directly converting boolean expressions to CNF via distributive laws causes exponential clause blowup. The Tseitin transformation introduces fresh auxiliary variables for each subformula, yielding an equisatisfiable 3-CNF formula with linear size growth:

1. **Logical Gates**:
   - `AND(y, x1, x2)`: $(y \lor \neg x_1 \lor \neg x_2) \land (\neg y \lor x_1) \land (\neg y \lor x_2)$
   - `OR(y, x1, x2)`: $(\neg y \lor x_1 \lor x_2) \land (y \lor \neg x_1) \land (y \lor \neg x_2)$
   - `XOR(y, x1, x2)`: $(\neg y \lor x_1 \lor x_2) \land (\neg y \lor \neg x_1 \lor \neg x_2) \land (y \lor \neg x_1 \lor x_2) \land (y \lor x_1 \lor \neg x_2)$
   - `NOT(y, x)`: $(y \lor x) \land (\neg y \lor \neg x)$
   - `ITE(y, c, t, e)`: $(\neg c \lor \neg t \lor y) \land (\neg c \lor t \lor \neg y) \land (c \lor \neg e \lor y) \land (c \lor e \lor \neg y)$
   - `EQUIV(y, x1, x2)`: $(\neg y \lor \neg x_1 \lor x_2) \land (\neg y \lor x_1 \lor \neg x_2) \land (y \lor x_1 \lor x_2) \land (y \lor \neg x_1 \lor \neg x_2)$

2. **Bit-Vector Primitives**:
   - Bit-vector equality: pairwise equivalence conjunction.
   - Ripple-carry bit-vector adder: generates sum and carry bits using Tseitin XOR/AND/OR full adder circuits.

---

## 4. DIMACS CNF Transport

- **Reader (`parseDimacs`)**: Strict parsing of `p cnf <vars> <clauses>`, rejecting malformed literals, non-zero-terminated clauses, and trailing garbage.
- **Writer (`writeDimacs`)**: Formats CNF into standard DIMACS text.

---

## 5. Verification & Acceptance Criteria

1. **Self-Contained**: 0 npm runtime dependencies in `package.json`.
2. **Correctness**:
   - Differential testing against a brute-force oracle across 100+ random 3-SAT instances at the phase transition ($m/n \approx 4.26$).
   - Verification of pigeonhole principle ($PHP^n_{n-1}$ is provably UNSAT).
   - Valid model verification for satisfiable instances (N-queens, full adders).
   - Failed-assumption core soundness.
3. **Resource Safety**: Strict termination within `maxConflicts` / `maxSteps`.
4. **Coverage**: $\ge 90\%$ line coverage and $\ge 85\%$ branch coverage.
