/// Level 3 CDCL SAT solver & Tseitin bit-blaster: the Dart lane of VER09d.
///
/// Conflict-driven clause learning with 2WL propagation, 1-UIP learning,
/// VSIDS + phase saving + Luby restarts, assumption solving with failed
/// cores, budgets, strict DIMACS transport, and Tseitin circuit builders.
/// Zero runtime dependencies.
library coding_adventures_sat_solver;

export 'src/sat.dart';
