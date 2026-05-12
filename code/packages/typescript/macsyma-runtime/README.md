# @coding-adventures/macsyma-runtime

Pure TypeScript MACSYMA runtime session over the grammar-driven MACSYMA
compiler and symbolic VM.

This first runtime slice is intentionally small and browser-friendly:

- compiles MACSYMA source through `@coding-adventures/macsyma-compiler`
- evaluates statements through `@coding-adventures/symbolic-vm`
- preserves `;` display versus `$` suppress metadata
- tracks `%`, `%iN`, and `%oN` history
- pre-binds `%pi`, `%e`, and `%i`
- dispatches `linsolve` / `Solve(List(...), List(...))` to the exact
  `cas-solve` linear-system solver
- dispatches `Solve(inequality, var)` to the `cas-solve` polynomial
  inequality solver for interval predicates
- dispatches direct `Solve(f(linear) = constant, var)` transcendental
  equations to the `cas-solve` inverse-family solver
- dispatches deterministic list operations such as `Length`, `First`, `Rest`,
  `Last`, `Append`, `Reverse`, `Range`, `Map`, `Apply`, `Sort`, `Part`,
  `Flatten`, and `Join` to `cas-list-operations`
- exposes a JSON helper whose recursive IR representation is safe for
  `JSON.stringify`
