# @coding-adventures/sat-solver

A zero-dependency, pure TypeScript Level 3 Conflict-Driven Clause Learning (CDCL) Boolean Satisfiability (SAT) solver and Tseitin circuit bit-blaster (VER09a).

## Features

- **Conflict-Driven Clause Learning (CDCL)**:
  - Two-Watched Literals (2WL) unit propagation with blocker literal caching.
  - First Unique Implication Point (1-UIP) conflict resolution.
  - Non-chronological backjumping.
  - Variable State Independent Decaying Sum (VSIDS) decision heuristic via an indexed binary max-heap.
  - Luby restart schedule ($1, 1, 2, 1, 1, 2, 4, \dots$).
- **Incremental Solving & Assumptions**:
  - Solve under arbitrary sets of assumptions (`solveWithAssumptions`).
  - Sound failed-assumption core extraction via implication graph backward reachability.
  - Level-0 state isolation preventing stale assignments from dropping new constraints.
- **Tseitin Bit-Blaster**:
  - Circuit transformation for logic gates: `and`, `or`, `xor`, `not`, `ite`, `equiv`.
  - Bit-vector arithmetic: 1-bit full adder and multi-bit ripple-carry adder (`bvAdd`, `bvEq`).
- **DIMACS CNF Transport**:
  - Strict DIMACS reader (`parseDimacs`) and writer (`writeDimacs`).
- **Zero Runtime Dependencies**:
  - Requires only Node.js standard libraries and TypeScript.

## Installation

\`\`\`bash
npm install @coding-adventures/sat-solver
\`\`\`

## Quick Start

\`\`\`typescript
import { Solver, makeLit, negateLit, varOf } from "@coding-adventures/sat-solver";

const solver = new Solver();
const a = makeLit(solver.newVar(), false);
const b = makeLit(solver.newVar(), false);

// Clause: (not a or b)  (i.e. a -> b)
solver.addClause([negateLit(a), b]);

// Solve under assumptions: a = true, b = false
const result = solver.solveWithAssumptions([a, negateLit(b)]);
console.log(result); // "unsat"

// Extract minimal unsatisfiable assumption core
console.log(solver.getFailedCore()); // [a, not b]
\`\`\`

## Tseitin Circuit Bit-Blasting

\`\`\`typescript
import { CnfBuilder } from "@coding-adventures/sat-solver";

const builder = new CnfBuilder();
const a = builder.newLit();
const b = builder.newLit();

// Output = a XOR b
const xorOut = builder.xor(a, b);

// Verify that when a=1 and b=0, xorOut must be 1
const solver = builder.getSolver();
const sat = solver.solveWithAssumptions([a, builder.not(b), builder.not(xorOut)]);
console.log(sat); // "unsat"
\`\`\`

## License

MIT © Adhithya Rajasekaran
