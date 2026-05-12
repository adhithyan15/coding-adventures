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
- exposes a JSON helper whose recursive IR representation is safe for
  `JSON.stringify`
