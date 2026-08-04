# Changelog

## [0.6.1] — 2026-07-16

### Changed

- `expand(...)`/`ev(expr, expand)` now collect like terms — `expand((x+1)^2)`
  returns `1 + 2*x + x^2`, not the raw `1 + x + x + x*x` from before. No
  code change in this crate: `expand_handler` delegates to
  `cas_simplify::expand` unchanged, which gained a `collect_terms` pass (see
  that crate's 0.5.0 CHANGELOG entry). Updated this crate's own
  `expand_distributes_polynomial_multiplication` test and the
  `expand_handler` doc comment, which had pinned/described the old
  uncollected shape.

## [0.6.0] — 2026-07-03

### Fixed

- **`expand(...)` and `ev(expr, expand)` now actually expand.** Previously
  `expand((x+1)^2)` returned the input unevaluated — `symbolic-vm`'s shared
  `build_handler_table` never registered a handler for the `Expand` head
  the name table already routed `expand`/the `expand` `ev` flag through, so
  the head silently had nowhere to land. Fixed by registering a new
  `expand_handler` in `MacsymaBackend` (the same decorator-over-the-shared-
  table pattern already used for `Simplify`/`RatSimplify`/`Radcan`), backed
  by the new `cas_simplify::expand` (see that crate's 0.4.0 CHANGELOG entry
  for the algorithm and its honestly-documented scope: correct distribution,
  no like-term collection yet). `expand((x+1)^2)` now returns
  `1 + x + x + x*x`.
- `ev_routes_supported_flags_and_preserves_unsupported_heads` renamed to
  `ev_routes_ratsimp_trigsimp_and_expand_flags` and its `expand` case
  updated to assert the new (correct) output — it previously demonstrated
  `expand` as an example of an *unsupported* passthrough head, which is no
  longer true.

## [0.5.0] — 2026-05-29

- Track M2 — port the MACSYMA `load("name")` runtime package directive
  from Python (committed in `dc78e0931`).  Surface contract:
  - `load("orthopoly")` flips a per-session gate on the backend that
    turns on the closed-form orthogonal polynomial evaluators.
  - Until then, `legendre_p(3, x)`, `chebyshev_t(4, x)`, `hermite(2, x)`,
    and friends round-trip unevaluated, matching the Python runtime.
  - After `load("orthopoly")`:
    - `legendre_p(n, x)` reduces via the Bonnet recurrence.
    - `chebyshev_t(n, x)`, `chebyshev_u(n, x)` reduce via the standard
      two-term recurrence.
    - `hermite(n, x)` reduces via the physicists' two-term recurrence
      (Maxima's convention: `H_0 = 1`, `H_1 = 2x`).
    - `legendre_q`, `bessel_j`, `bessel_y` are passthrough — symbol is
      "known" but no closed form is applied.
  - Non-integer / negative `n` keeps the expression unevaluated.
  - `load("nonexistent")`, `load("../etc/passwd")`, `load("os")`, etc.,
    panic with a `MacsymaUserError` that advertises the available
    packages.
  - Loading is idempotent and per-session (two backends stay
    independent).
- Security note: the load handler dispatches via a compile-time-constant
  `LOAD_ALLOWLIST: &[&str]` and a static `match` arm.  There is no
  `libloading`, dynamic FFI, `Path` resolution, or `eval`-equivalent
  code path; a hostile name string can never become an executable
  code path.
- New surface names in `macsyma_name_table()`: `load`, `legendre_p`,
  `legendre_q`, `chebyshev_t`, `chebyshev_u`, `hermite`, `bessel_j`,
  `bessel_y`.
- New public items: `LOAD`, `MacsymaUserError`, and
  `MacsymaSession::loaded_packages()`.

## [0.4.0] — 2026-05-29

- Feed the Rust MACSYMA session assumption context into direct `abs`, `sqrt`,
  and `log` evaluation, matching Python reference behavior for cases such as
  `assume(x >= 0); sqrt(x^2)`, `log(x^3)`, and `abs(x)`.

## [0.3.0] — 2026-05-16

**EllipticE (2nd kind) and EllipticPi (3rd kind) integration pipeline tests.**

Added pipeline-level tests verifying that the complete and incomplete elliptic
integrals of the second and third kind round-trip correctly through the
macsyma-runtime dispatcher:

- `recognizes_elliptic_second_kind_integrals_through_runtime` — incomplete
  EllipticE: `∫ √(1−k²sin²θ) dθ` → `EllipticE(θ, k)`
- `recognizes_complete_elliptic_second_kind_integrals_through_runtime` —
  complete EllipticE: `∫₀^(π/2) √(1−k²sin²θ) dθ` → `EllipticE(k)`
- `recognizes_complete_elliptic_third_kind_integrals_through_runtime` —
  complete EllipticPi: `∫₀^(π/2) 1/((1+n·sin²θ)√(1−k²sin²θ)) dθ` → `EllipticPi(n, k)`
- `elliptic_first_kind_regression_still_works` — regression guard confirming
  EllipticK recognition is unaffected

## [0.2.0] — 2026-05-14

### Added

- Added Rust MACSYMA parity for elliptic first-kind integration, so
  `integrate(1/sqrt(1-k^2*sin(theta)^2), theta)` returns
  `EllipticF(theta, k)` and the `[0, %pi/2]` definite form returns
  `EllipticK(k)`.
- Delegated Rust MACSYMA `factor` evaluation to the shared `symbolic-vm`
  canonical handler, including coverage for common multivariate factors.
- Added Rust MACSYMA parity for multivariate integer content extraction, so
  `factor(2*x + 4*y)` returns `2*(x + 2*y)`, `factor(2*x*y + 2*x*z)` returns
  `2*x*(y + z)`, and `factor(2*x^2*y - 2*y)` returns `2*y*(x+1)*(x-1)` through
  the shared symbolic VM handler. Brings Rust to parity with Python (#3120) and
  TypeScript (#3124).
- Added Rust MACSYMA parity for bivariate perfect-square factoring, so
  `factor(x^2 + 2*x*y + y^2)` returns `(x + y)^2` through the shared symbolic
  VM handler.
- Added Rust MACSYMA parity for bivariate difference-of-squares factoring, so
  `factor(x^2 - y^2)` returns `(x - y) * (x + y)` through the shared symbolic
  VM handler.
- Added Rust MACSYMA parity for bivariate cubic-identity factoring, so
  `factor(x^3 - y^3)` and `factor(x^3 + y^3)` return their linear/quadratic
  products through the shared symbolic VM handler.
- Added Rust MACSYMA parity for four-term bilinear grouping, so
  `factor(x*y + x*z + y + z)` returns `(x + 1) * (y + z)` through the shared
  symbolic VM handler.
- Added Rust MACSYMA parity for shared multivariate integer content, so
  `factor(2*x*y + 2*x*z)` returns `2*x*(y + z)` through the shared symbolic VM
  handler.
- Added Rust MACSYMA parity for four-term perfect-cube expansions, so
  `factor(x^3 + 3*x^2*y + 3*x*y^2 + y^3)` returns `(x + y)^3` and
  `factor(x^3 - 3*x^2*y + 3*x*y^2 - y^3)` returns `(x - y)^3` through the
  shared symbolic VM handler.
- Added `?` / `? topic` help-query handling for Rust MACSYMA sessions.
- Wired `assume`, `forget`, `is`, `declare`, `properties`, and `propvars` to a
  Rust MACSYMA session assumption context so declared properties feed property
  queries and assumption-backed relation checks.

## [0.1.1] - 2026-05-12

### Added

- Routed `ev(expr, display2d)` result presentation through the Rust
  `cas-pretty-printer` 2D MACSYMA box renderer while preserving the symbolic IR
  result for history and downstream evaluation.
- Wired `TrigReduce(expr)` and `ev(expr, trigreduce)` to the Rust `cas-trig`
  power-reduction walker.
- Wired `Subst(value, variable, expr)` to the Rust `cas-substitution`
  structural substitution package.
- Wired deterministic MACSYMA list heads (`Length`, `First`, `Rest`, `Last`,
  `Append`, `Join`, `Reverse`, `Range`, `Part`, `Map`, `Apply`, `Sort`, and
  `Flatten`) to the Rust `cas-list-operations` package.
- Wired direct `Solve(f(linear) = constant, variable)` transcendental equations
  to the Rust `cas-solve` `try_solve_transcendental` handler, returning
  `List(...)` symbolic inverse and periodic-family solutions.
- Wired `Solve(inequality, variable)` to the Rust `cas-solve`
  `try_solve_inequality` handler, returning `List(...)` interval predicates
  for supported one-variable polynomial inequalities.
- Wired `linsolve` / `Solve(List(...), List(...))` to the Rust `cas-solve`
  exact linear-system solver.
- Added runtime coverage for integer systems, rational systems, and non-linear
  fallback.

## [0.1.0] - 2026-05-08

### Added

- Initial Rust MACSYMA runtime session facade.
- Source-to-IR compilation, symbolic VM evaluation, display/suppress result metadata, and in-memory input/output history.
