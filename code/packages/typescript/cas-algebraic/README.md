# cas-algebraic

Pure TypeScript factoring over quadratic extensions `Q[sqrt(d)]`.

Polynomials use ascending coefficient order. Coefficients in extension factors
are represented as `{ rational, radical }`, meaning `rational + radical*sqrt(d)`.
