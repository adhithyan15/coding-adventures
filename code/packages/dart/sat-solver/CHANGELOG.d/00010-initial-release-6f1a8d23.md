## 0.1.0

- Initial Dart SAT lane (VER09d): CDCL solver with 2WL propagation,
  1-UIP learning, VSIDS + phase saving + Luby restarts, learned DB
  reduction, assumption solving with sound failed cores, conflict/step
  budgets with sticky root conflicts, strict DIMACS transport, and
  Tseitin gate/bit-vector builders.
- 29 tests: 14/14 sat-cnf-v1 corpus cases (SHA-pinned), 100 3-SAT
  differentials, 64 stateful traces, and a 2,500-iteration mutation
  campaign through the Dart fuzz lane; zero runtime dependencies; 95%
  branch+function coverage gate.
