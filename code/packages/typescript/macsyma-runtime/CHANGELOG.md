# Changelog

## [0.3.0] — 2026-05-14

- Add TypeScript MACSYMA parity for EllipticE (second kind) integration:
  - `integrate(sqrt(1-k^2*sin(theta)^2), theta, 0, %pi/2)` → `EllipticE(k)`
  - `integrate(sqrt(1-k^2*sin(theta)^2), theta)` → `EllipticE(theta, k)`
- Add TypeScript MACSYMA parity for EllipticPi (third kind) integration:
  - `integrate(1/((1+n*sin(theta)^2)*sqrt(1-k^2*sin(theta)^2)), theta, 0, %pi/2)` → `EllipticPi(n, k)`
- Fix numeric modulus extraction: `(1/2)^2` now correctly yields modulus `1/2`
  for EllipticE/Pi recognition (same fix as `symbolic-vm` v0.3.1).
- Add pipeline tests for numeric modulus forms (k=1/2, n=2, k=1/2).

## [0.2.0] — 2026-05-14

- Add TypeScript MACSYMA parity for elliptic first-kind integration, so
  `integrate(1/sqrt(1-k^2*sin(theta)^2), theta)` returns
  `EllipticF(theta, k)` and the `[0, %pi/2]` definite form returns
  `EllipticK(k)`.
- Add TypeScript MACSYMA parity for perfect-cube expansions, so
  `factor(x^3 + 3*x^2*y + 3*x*y^2 + y^3)` returns `(x+y)^3` and
  `factor(x^3 - 3*x^2*y + 3*x*y^2 - y^3)` returns `(x-y)^3` through the
  shared symbolic VM handler.
- Delegate `factor` to the canonical TypeScript symbolic VM handler so common
  multivariate terms such as `factor(x^2*y - y)` reduce through the shared CAS
  substrate.
- Add TypeScript MACSYMA parity for shared integer content in multivariate
  common-factor expressions, so `factor(2*x*y + 2*x*z)` returns `2*x*(y+z)`.
- Add TypeScript MACSYMA parity for the bivariate perfect-square factoring
  foothold, so `factor(x^2 + 2*x*y + y^2)` returns `(x+y)^2`.
- Add TypeScript MACSYMA parity for the bivariate difference-of-squares
  factoring foothold, so `factor(x^2 - y^2)` returns `(x-y)*(x+y)`.
- Add TypeScript MACSYMA parity for bivariate cubic identities, so
  `factor(x^3 - y^3)` and `factor(x^3 + y^3)` return the textbook two-factor
  decompositions through the shared symbolic VM handler.
- Add TypeScript MACSYMA parity for four-term bilinear grouping, so
  `factor(x*y + x*z + y + z)` returns `(x+1)*(y+z)` through the shared
  symbolic VM handler.
- Add `?` / `? topic` help-query handling for TypeScript MACSYMA sessions and
  JSON responses.
- Wire `assume`, `forget`, `is`, `declare`, `properties`, and `propvars` to a
  TypeScript MACSYMA session assumption context so declared properties feed
  property queries and assumption-backed relation checks.
- Route `ev(expr, display2d)` JSON and browser display text through the
  TypeScript `cas-pretty-printer` 2D MACSYMA box renderer while preserving the
  symbolic IR result for history and downstream evaluation.
- Wire `TrigReduce` runtime dispatch to the TypeScript `cas-trig` package.
- Wire `Simplify`, `RatSimplify`, `TrigSimplify`, and `TrigExpand` runtime
  heads to the TypeScript `cas-simplify` and `cas-trig` packages.
- Wire `Subst(value, variable, expr)` to the TypeScript `cas-substitution`
  structural substitution package.
- Wire deterministic MACSYMA list heads (`Length`, `First`, `Rest`, `Last`,
  `Append`, `Reverse`, `Range`, `Map`, `Apply`, `Sort`, `Part`, `Flatten`,
  and `Join`) to the TypeScript `cas-list-operations` package.
- Wire direct `Solve(f(linear) = constant, var)` transcendental equations to
  the TypeScript `cas-solve` `trySolveTranscendental` handler, returning
  `List(...)` symbolic inverse and periodic-family solutions.
- Wire `Solve(inequality, var)` to the TypeScript `cas-solve`
  `trySolveInequality` handler for polynomial inequalities, returning
  `List(...)` interval predicates and preserving unevaluated fallback behavior.
- Wire `Solve(List(...), List(...))` / `linsolve` to the TypeScript
  `cas-solve` exact linear-system solver, returning `List(Rule(var, value))`
  for square systems and preserving unevaluated fallback behavior.

## 0.1.0

- Add pure TypeScript MACSYMA runtime sessions, history lookup, display
  metadata, constants, and JSON result helpers.
