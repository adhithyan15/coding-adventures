# coding_adventures_sat_solver (Dart)

Level 3 CDCL SAT solver and Tseitin bit-blaster: the Dart lane of
[VER09d](../../../specs/VER09d-dart-sat-solver.md).

Where testing finds bugs by luck, a SAT solver proves their absence: encode
"a bug exists" as a formula and an UNSAT verdict rules out all 2^N states at
once, while SAT hands back a concrete counterexample model.

## API

```dart
import 'package:coding_adventures_sat_solver/coding_adventures_sat_solver.dart';

// Literals pack (variable, sign) into one int
int v = solver.newVar();
int lit = makeLit(v, /*negated=*/ false);
int varOf(lit); int signOf(lit); int negateLit(lit); String litToString(lit);

// Solving
solver.addClause([a, b, negateLit(c)]);
String r = solver.solve();                        // sat | unsat | unknown
String r2 = solver.solveWithAssumptions([a], SolverOptions(maxSteps: 1000));
List<bool> model = solver.getModel();             // index = variable
List<int> core = solver.getFailedCore();          // sound assumption subset
solver.reset();                                   // backjump to level 0

// Circuits (Tseitin)
final b = CnfBuilder();
b.and(a, x); b.or(a, x); b.xor(a, x); b.not(a); b.ite(c, t, e); b.equiv(a, x);
b.fullAdder(a, x, cin); b.bvEq(a, x); b.bvAdd(a, x);

// DIMACS transport
CnfFormula f = parseDimacs('p cnf 2 1\n1 -2 0\n');
String text = writeDimacs(f);
```

* Conflict budgets (`maxConflicts`/`maxSteps`) bound one call; exhaustion
  returns `unknown` and the next call resumes honestly — including pending
  root work, which stays observable to a later solve or `propagate()`.
* Assumption conflicts extract sound duplicate-free cores and never poison
  the solver; only genuine root contradictions clear `okay()`.
* `parseDimacs` is strict (exact header, `%` terminator, `0`-terminated
  clauses, exact clause count) and throws `FormatException` otherwise.
* Zero runtime dependencies. Pure computation: no clock, no I/O.

## Tests

```bash
dart pub get
dart test
```

The suite mirrors the sibling lane: literal/DIMACS checks, solving and
1-UIP learning, pigeonhole UNSAT, 4-queens, assumptions with core soundness,
budget isolation, 100 seeded 3-SAT rounds vs brute force, Tseitin
gate/adder equivalence with sabotage, a strict DIMACS rejection battery, the
14-case sat-cnf-v1 stateful corpus (SHA-pinned, models and cores validated
against the independently decoded signed CNF), 64 seeded differential traces
with exact operation accounting, and a bounded 2,500-iteration mutation
campaign through the Dart `fuzz` lane. Coverage gate: 95% branch+function.
