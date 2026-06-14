# Statistics Core

## Overview

The `statistics-core` Rust crate is the canonical home of every
statistical computation that the repo's spreadsheets, R runtime, S
runtime, or any future numerical tool will call. The list of consumers
is long — the list of *implementations* is exactly one. Every
function lives here, has a single Rust signature, and is referenced by
name from every frontend.

The crate's purpose is captured in one rule:

> **A function in `statistics-core` knows nothing about who is calling
> it. No code in this crate mentions Excel, Lotus, VisiCalc, R, S, or
> spreadsheet cells. It receives `Vector`s and `Number`s; it returns
> `Vector`s and `Number`s; it reports errors through one shared error
> type. Frontends translate to and from these signatures.**

That separation is what lets `=AVERAGE(A1:A10)` in Modern VisiCalc and
`mean(c(...))` in the future R runtime resolve to the same line of
Rust. It is also what lets future frontends (Mathematica-flavored,
APL-flavored, S+) plug in without disturbing the math.

This spec defines (a) the cross-cutting contracts every function in
the crate obeys, (b) the function catalog organized by family, and
(c) the per-frontend alias tables that map external names onto
canonical Rust signatures. The crate lives at
`code/packages/rust/statistics-core/` and depends on `numeric-tower`,
`r-vector`, and `num-traits`.

This is the largest single spec in the substrate-and-cores series. It
is large because the surface is large: VisiCalc had 25 functions,
Lotus 1-2-3 had 80, Multiplan had 70, Excel today has roughly 100
statistical functions, R `stats` exposes around 250, and S has its
own variants. The deduplication across this set is enormous — there
are about 80 distinct mathematical operations underneath all the
frontends. Listing them once with all aliases is the best forcing
function for getting the implementation right.

---

## Where It Fits

```
   visicalc-modern   r-runtime   s-runtime    (Layer 4 frontends)
        │                │            │
        │ AVERAGE        │ mean       │ mean
        ▼                ▼            ▼
   ┌────────────────────────────────────────────────────────────┐
   │  spreadsheet-core / r-runtime / s-runtime  (Layer 3)       │
   │  Each owns a name table → core fn dispatch.                │
   └────────────────────────────────┬───────────────────────────┘
                                    │
                                    │  call by canonical Rust signature
                                    ▼
   ┌────────────────────────────────────────────────────────────┐
   │   statistics-core  (Layer 1)              ← THIS SPEC      │
   │   descriptive · rank · distributions · inference · regression
   │   · ANOVA · multivariate · time-series · smoothing · RNG   │
   │   · special functions · linalg                              │
   └────────────────────────────────┬───────────────────────────┘
                                    │
                                    ▼
                  numeric-tower · r-vector · na-semantics
```

**Used by:** spreadsheet-core (every Excel/Lotus/VisiCalc statistical
function dispatches through it); future r-runtime; future s-runtime;
future visicalc-modern UI for any in-cell helper that calls a statistic
directly; the financial-core crate (where statistics-core supplies
mean/variance to financial functions like Sharpe ratio).

**Not used by:** `cas-*` symbolic stack (different mathematical
universe — numeric vs symbolic); `text-core` / `datetime-core`
(non-numerical domains).

---

# Part I — Cross-Cutting Contracts

The function catalog in Part II refers back to these contracts. Read
this part first; the catalog is unreadable without it.

## §1 Error Model

Every public function in `statistics-core` returns one of three things:

| Return form                  | When                                                              |
|------------------------------|-------------------------------------------------------------------|
| `Result<T, StatsError>`       | Function may fail for input-shape, domain, or numeric reasons    |
| `Vector<T>` (no `Result`)     | Function cannot fail short of allocation; NA in inputs becomes NA in outputs |
| `Number` (no `Result`)        | Reductions on a known-good vector that propagate NA via `Number::Float(NA_REAL)` |

The error type:

```rust
pub enum StatsError {
    /// Input vector empty when not allowed (e.g. shapiro.test on n < 3)
    EmptyInput { function: &'static str, min_n: usize },
    /// Domain error (e.g. log of negative, sqrt of negative real)
    DomainError { function: &'static str, what: String },
    /// Inputs have mismatched lengths after frontend was supposed to align them
    ShapeMismatch { expected: usize, found: usize },
    /// Singular matrix in a regression / decomposition
    Singular { function: &'static str },
    /// Iterative method failed to converge in given iterations
    NoConvergence { function: &'static str, iters: u32 },
    /// Parameter out of allowed range (e.g. negative df)
    BadParameter { name: &'static str, value: String },
    /// Numerical overflow that NA cannot represent (rare)
    Overflow { function: &'static str },
}
```

The error is structured so frontends can translate to their native
error sentinel:

| Frontend       | Translation |
|----------------|-------------|
| Excel          | `#NUM!` (DomainError, Singular, NoConvergence, Overflow), `#VALUE!` (ShapeMismatch, BadParameter), `#N/A` (EmptyInput) |
| R              | `stop(msg)` for all variants; the `msg` is built from the error fields |
| S              | Same as R |

### Error contract under NA

NA in input does **not** produce a `StatsError`. NA propagates per
`na-semantics.md`. A function returns `StatsError` only when the
operation cannot proceed *even if all values were known*.

`mean(c(1, NA, 3))` returns `NA` (a `Number::Float` carrying the NA bit
pattern). It does not return `Err(EmptyInput)` because the input was
not empty.

`mean(empty vector)` returns `Err(EmptyInput)` (with `na_rm = false`)
or `NA` (with `na_rm = true`, matching R's `mean(numeric(0), na.rm = TRUE) = NaN`
— note `NaN`, not NA, because the math is "0 ÷ 0").

## §2 NA Propagation

Single rule applied identically across the whole catalog:

> **If any input element is NA, the result element is NA, unless the
> function is documented as a reduction taking `na_rm: bool` and the
> caller passed `na_rm = true`.**

Reductions that take `na_rm`:

`sum`, `prod`, `mean`, `median`, `var`, `sd`, `min`, `max`, `range`,
`quantile`, `IQR`, `mad`, `geomean`, `harmean`, `trimmean`, `sumsq`,
`devsq`, `avedev`, `cov`, `cor`, `cumsum`, `cumprod`, `cummin`,
`cummax`, every distribution `r*` (random sample) is unaffected,
every distribution `d*`/`p*`/`q*` propagates element-wise.

Functions that do **not** take `na_rm` (always propagate):

- All element-wise math (`add`, `sub`, `mul`, `div`, `log`, `exp`, …)
- All distributions (`dnorm`, `pnorm`, `qnorm`, …) — these are
  vectorized over their arguments, NA in any arg → NA at that position
- Predicates (`is_finite`, `is_nan`) — these test bit patterns, NA is
  reported as NA-the-input rather than NA-the-output
- Constructors (`seq`, `rep`)

The empty-after-`na_rm` table:

| Reduction | All-NA input with na_rm=true returns |
|-----------|--------------------------------------|
| sum, cumsum  | 0 (additive identity) |
| prod, cumprod | 1 (multiplicative identity) |
| mean      | NaN (0/0 — math really is undefined) |
| var, sd   | NaN |
| median, quantile, IQR, mad | NaN |
| min       | +Inf |
| max       | -Inf |
| range     | (+Inf, -Inf) |

These match R exactly. They are the empty-set identities.

## §3 RNG State Model

All randomness goes through one PRNG: **Mersenne Twister 19937**
(MT19937), R's default. The `r*` family (`rnorm`, `rbinom`, …) plus
`sample` are the only functions that consume randomness.

```rust
pub struct RngState {
    state: [u32; 624],
    index: usize,
    seed: u64,
}

impl RngState {
    pub fn new(seed: u64) -> Self;
    pub fn from_entropy() -> Self;
    pub fn next_u32(&mut self) -> u32;
    pub fn next_f64(&mut self) -> f64;       // uniform [0, 1)
}

pub fn set_seed(state: &mut RngState, seed: u64);
```

### Reproducibility

A `set_seed(state, k)` followed by any sequence of `r*` calls
produces, byte-for-byte, the same vector across every machine,
operating system, and Rust toolchain version. This is non-negotiable
because cross-language parity tests against R compare exact bytes.

The MT19937 initialization follows R's:

```
state[0] = seed
state[i] = 1812433253 * (state[i-1] ^ (state[i-1] >> 30)) + i   for i = 1..623
```

Then 624 numbers are warmed up before any are consumed. This matches
the `init_genrand` function in R's `RNG.c`, which is itself the
Matsumoto-Nishimura 1998 reference.

### Stream independence

Multiple `RngState`s with different seeds are independent. We do
not implement L'Ecuyer-style stream splitting in v1; users who want
parallel reproducibility seed multiple streams manually with
distinct seeds.

### Frontend integration

| Frontend | RNG state location |
|----------|--------------------|
| R runtime | A single `.Random.seed` per session, modifiable from R code |
| Excel    | A new RNG seeded per workbook from the file save state, advanced on every recalc |
| VisiCalc Modern | Same as Excel |
| S        | Same as R |

The frontend owns the state; `statistics-core` is given a `&mut RngState`
for any random call. There is no global mutable RNG in the crate.

### Non-uniform sampling algorithms

| Distribution | Algorithm | Reference |
|--------------|-----------|-----------|
| Normal       | Box-Muller (default) or ziggurat (opt-in) | Marsaglia & Tsang 2000 |
| Exponential  | Inverse CDF (`-ln(U)`) | trivial |
| Gamma         | Marsaglia-Tsang (shape ≥ 1), Ahrens-Dieter (shape < 1) | Marsaglia & Tsang 2000 |
| Beta          | Two gammas, ratio | Cheng 1978 |
| Poisson       | Knuth's product-of-uniforms (small λ); rejection (large λ) | Devroye 1986 |
| Binomial      | BTPE for n large; inverse-CDF for n small | Kachitvichyanukul-Schmeiser 1988 |
| Chi-squared   | Sum of squared normals (df small); gamma (df large) | trivial |
| Student-t     | Normal / sqrt(chi-squared / df) | trivial |
| F             | Ratio of chi-squareds | trivial |

The sampling functions all match R's output bit-for-bit when called
through the same MT19937 seed. Cross-language parity tests assert
this.

## §4 The `Distribution` Trait

Every continuous distribution implements:

```rust
pub trait ContinuousDistribution {
    /// Probability density function — f(x)
    fn pdf(&self, x: f64) -> f64;
    /// Log of pdf — for numerical stability in tails
    fn log_pdf(&self, x: f64) -> f64;
    /// Cumulative distribution function — F(x) = P(X ≤ x)
    fn cdf(&self, x: f64) -> f64;
    /// Survival function — 1 - F(x), accurate in upper tail
    fn sf(&self, x: f64) -> f64;
    /// Log CDF — for numerical stability
    fn log_cdf(&self, x: f64) -> f64;
    /// Inverse CDF — quantile function
    fn quantile(&self, p: f64) -> f64;
    /// Random sample
    fn sample(&self, rng: &mut RngState) -> f64;
    /// Mean of the distribution (None if undefined, e.g. Cauchy)
    fn mean(&self) -> Option<f64>;
    /// Variance (None if undefined or infinite)
    fn variance(&self) -> Option<f64>;
    /// Support — the set of x where pdf > 0
    fn support(&self) -> Support;
}

pub trait DiscreteDistribution {
    fn pmf(&self, x: i64) -> f64;
    fn log_pmf(&self, x: i64) -> f64;
    fn cdf(&self, x: i64) -> f64;
    fn sf(&self, x: i64) -> f64;
    fn quantile(&self, p: f64) -> i64;
    fn sample(&self, rng: &mut RngState) -> i64;
    fn mean(&self) -> f64;
    fn variance(&self) -> f64;
    fn support(&self) -> DiscreteSupport;
}
```

Adding a new distribution (Cauchy, Wilcoxon, …) is: implement the
trait, register the constructor in the dispatcher table, write
parity tests against R. No other crate code changes.

The four R-named functions `dnorm` / `pnorm` / `qnorm` / `rnorm` are
generated by macro from a single `Normal` struct that implements the
trait:

```rust
distribution_family! {
    Normal { mean: f64, sd: f64 } => normal {
        d: dnorm, p: pnorm, q: qnorm, r: rnorm
    }
}
```

The macro expands to four free functions plus the trait `impl`. Same
shape for every distribution.

### Numerical conventions for distributions

- All `pdf` / `cdf` / `quantile` accept `f64` (not generic). NA
  propagates via the NA bit pattern in `f64`.
- All return `f64`. Out-of-support inputs return `0.0` for pdf/pmf,
  `0.0` or `1.0` for cdf depending on which tail, `NaN` for quantile
  outside `[0, 1]`.
- `lower_tail: bool` and `log_p: bool` parameters exist on every
  CDF/quantile (matching R) but live as separate functions
  (`pnorm_lower`, `pnorm_upper`, `pnorm_log`) — Rust doesn't have R's
  default-argument flexibility cheaply, and four separate functions
  beat `Result`-returning option types.

## §5 Numerical Accuracy Contract

Every reduction follows specific algorithms chosen for stability, not
the textbook formula. The contract:

| Reduction      | Algorithm                                            | Expected ULP error |
|----------------|------------------------------------------------------|--------------------|
| sum            | Kahan compensated summation                           | ≤ 2 ULP regardless of n |
| mean           | sum / n via Kahan; or Welford if streaming           | ≤ 2 ULP             |
| var, sd        | Welford's online algorithm                            | ≤ 4 ULP             |
| cov            | Welford-style two-pass                                | ≤ 4 ULP             |
| cor            | Standardized Welford                                  | ≤ 4 ULP             |
| log-sum-exp    | Subtract max before exp, then add max back            | ≤ 2 ULP             |
| beta-cdf, gamma-cdf | Continued fractions in tails; series in body | ≤ 5 ULP             |
| normal CDF     | erf via Cody's rational approximations                | ≤ 1 ULP             |

Test infrastructure:

```rust
// In tests/numerical_accuracy.rs
fn assert_ulp_eq(actual: f64, expected: f64, max_ulp: u64) { … }
```

Every distribution function has parity tests against R-generated
expected values, with ULP tolerance per the table. Catastrophic-cancellation
tests (sum of nearly-equal opposites, var of nearly-constant series)
are in `tests/cancellation.rs`.

### Reduction order and parallelism

Floating-point summation is non-associative. The crate guarantees
*deterministic* reduction order even when SIMD or parallel execution
is enabled: the parallel reducer combines partial sums in tree order,
identical regardless of thread count or scheduling. Tests assert
that single-threaded and multi-threaded results agree bit-for-bit.

## §6 Linalg Sub-Module

`statistics-core::linalg` provides the linear algebra needed for
regression, principal components, and inference. It does **not**
delegate to the existing `matrix` or `cas-matrix` crates because:

- `matrix` is ML03-basic (no QR, no SVD, no Cholesky).
- `cas-matrix` is symbolic — its determinant is a polynomial.
- We need numeric, pivoted, condition-aware operations tuned for
  least-squares.

The sub-module covers exactly what the catalog needs:

| Operation           | Used by                                    |
|---------------------|--------------------------------------------|
| QR (Householder)     | lm, glm (IRLS)                            |
| SVD (Golub-Reinsch) | princomp, prcomp, rank-deficient lm       |
| Cholesky             | mvtnorm density, multivariate sampling   |
| LU with partial pivoting | solve(), determinant                  |
| Eigendecomposition   | princomp, factanal                        |
| Backsolve            | regression coefficient extraction         |

All operations return `Result<_, StatsError>` with `StatsError::Singular`
or `StatsError::NoConvergence` as appropriate.

The linalg module's public types: `Matrix<f64>`, `Vector<f64>` (not
the `r-vector::Double`; this is a pure dense float matrix without
NA support — regression demands no NA in the design matrix). The
boundary between r-vector and linalg is `to_dense_matrix()` / `from_dense_matrix()`,
which checks for NA and errors if found (callers strip or impute first).

If a second consumer ever appears, the module gets extracted to
`numeric-linalg`. Until then it lives here. Premature extraction
would force interface decisions before the second use case names them.

## §7 Vectorization Contract Recap

From `vectorization-rules.md`: every public function assumes inputs
are already shape-aligned. The frontend has done recycling (R) or
broadcasting (Excel) before calling. `statistics-core` does not
attempt to recycle.

The exception: scalar-args-with-vector-arg functions in the
distribution family. `dnorm(x: &Double, mean: f64, sd: f64)` accepts a
vector `x` and scalar `mean`/`sd`; it does not accept vector `mean`/`sd`.
For vectorized parameters, the frontend builds N separate calls or
uses the bulk overload `dnorm_vec(x: &Double, mean: &Double, sd: &Double)`
where all three must be the same length (frontend has aligned them).

---

# Part II — Function Catalog

Functions are grouped by family. Each entry lists:

- **Canonical Rust signature** (the one that lives in the crate)
- **Mathematical formula** (where it isn't obvious from the name)
- **Edge cases** (empty input, NA behavior, domain errors)

Per-frontend names live in the alias tables in Part III.

## §A Descriptive Statistics

| Function | Rust signature | Notes |
|----------|---------------|-------|
| sum      | `fn sum(x: &Double, na_rm: bool) -> Number` | Kahan; identity 0 |
| prod     | `fn prod(x: &Double, na_rm: bool) -> Number` | identity 1 |
| mean     | `fn mean(x: &Double, na_rm: bool) -> Number` | sum / n; NaN on empty |
| median   | `fn median(x: &Double, na_rm: bool) -> Number` | linear-time selection (quickselect); average of middle two when even |
| mode     | `fn mode(x: &Double, na_rm: bool) -> Double` | returns ALL modes (R's behavior); empty if all unique |
| var      | `fn var(x: &Double, na_rm: bool) -> Number` | Welford; sample variance (Bessel's correction, divides by n-1) |
| sd       | `fn sd(x: &Double, na_rm: bool) -> Number` | sqrt(var) |
| var_pop  | `fn var_pop(x: &Double, na_rm: bool) -> Number` | divides by n (population variance — VAR.P) |
| sd_pop   | `fn sd_pop(x: &Double, na_rm: bool) -> Number` |  |
| min      | `fn min(x: &Double, na_rm: bool) -> Number` | identity +Inf |
| max      | `fn max(x: &Double, na_rm: bool) -> Number` | identity -Inf |
| range    | `fn range(x: &Double, na_rm: bool) -> (Number, Number)` | (min, max) |
| iqr      | `fn iqr(x: &Double, na_rm: bool) -> Number` | Q3 - Q1 |
| mad      | `fn mad(x: &Double, center: Option<f64>, constant: f64, na_rm: bool) -> Number` | median absolute deviation; default constant 1.4826 |
| geomean  | `fn geomean(x: &Double, na_rm: bool) -> Number` | exp(mean(log(x))); rejects ≤ 0 |
| harmean  | `fn harmean(x: &Double, na_rm: bool) -> Number` | n / sum(1/x); rejects 0 |
| trimmean | `fn trimmean(x: &Double, fraction: f64, na_rm: bool) -> Number` | drops symmetric tails |
| sumsq    | `fn sumsq(x: &Double, na_rm: bool) -> Number` | Σx² |
| devsq    | `fn devsq(x: &Double, na_rm: bool) -> Number` | Σ(x - mean)² |
| avedev   | `fn avedev(x: &Double, na_rm: bool) -> Number` | mean(\|x - mean\|) |
| cumsum   | `fn cumsum(x: &Double) -> Double` | length n; NA → NA from that point |
| cumprod  | `fn cumprod(x: &Double) -> Double` |  |
| cummin   | `fn cummin(x: &Double) -> Double` |  |
| cummax   | `fn cummax(x: &Double) -> Double` |  |

**Edge cases for descriptive family:**

- Empty input with `na_rm = false`: returns `StatsError::EmptyInput`
- Empty input with `na_rm = true`: returns the empty-set identity per §2
- All-NA with `na_rm = false`: returns NA
- All-NA with `na_rm = true`: returns the empty-set identity (per §2)
- Single-element input: var/sd return NaN (n-1 = 0); mean returns the element

## §B Counting

| Function | Rust signature | Notes |
|----------|---------------|-------|
| length   | `fn length<V: Vector>(x: &V) -> usize` | structural; counts NAs |
| count_non_na | `fn count_non_na<V: Vector>(x: &V) -> usize` | excludes NAs |
| count_blank  | `fn count_blank(x: &Character) -> usize` | empty strings |
| count_if   | `fn count_if<V: Vector, P: Fn(&V::Element) -> bool>(x: &V, pred: P) -> usize` | predicate-based; underlies COUNTIF |

`COUNTIF` and `COUNTIFS` from Excel are spreadsheet-core's job; they
parse criteria strings and lower to `count_if` calls.

## §C Rank and Order

| Function | Rust signature | Notes |
|----------|---------------|-------|
| sort     | `fn sort<V: Vector + Ord>(x: &V, decreasing: bool) -> V` | NAs sink to end (R: na.last = TRUE default) |
| order    | `fn order<V: Vector + Ord>(x: &V) -> Integer` | permutation indices |
| rank     | `fn rank(x: &Double, ties: TieMethod) -> Double` | TieMethod = Average\|Min\|Max\|First\|Random |
| large    | `fn large(x: &Double, k: usize) -> Number` | k-th largest (Excel LARGE) |
| small    | `fn small(x: &Double, k: usize) -> Number` | k-th smallest (Excel SMALL) |
| quantile | `fn quantile(x: &Double, probs: &Double, qtype: u8, na_rm: bool) -> Double` | qtype 1-9, default 7 (R/Excel inclusive) |
| percentile | alias for quantile when probs is a single value — no separate fn |
| percent_rank | `fn percent_rank(x: &Double, value: f64, kind: PercentRankKind) -> f64` | kind = Inclusive\|Exclusive |

**Quantile types:** Excel's `QUARTILE` and `PERCENTILE.INC` correspond
to qtype 7 (R default). `QUARTILE.EXC` and `PERCENTILE.EXC` correspond
to qtype 6. The `qtype` parameter exposes all 9 R definitions for
users who care about the difference; frontends pick the right value.

## §D Distributions (d/p/q/r)

The eighteen distributions, listed by family with parameter signatures.
Each has implicit `dXXX`, `pXXX`, `qXXX`, `rXXX` functions plus the
trait implementation.

| Distribution | Parameters                            | Continuous? |
|--------------|----------------------------------------|-------------|
| normal       | mean, sd                                | yes |
| t            | df                                      | yes |
| chisq        | df, ncp = 0                             | yes |
| f            | df1, df2, ncp = 0                       | yes |
| beta         | shape1, shape2, ncp = 0                 | yes |
| gamma        | shape, rate (or 1/scale)                | yes |
| exp          | rate                                    | yes |
| weibull      | shape, scale                            | yes |
| lnorm        | meanlog, sdlog                          | yes |
| unif         | min, max                                | yes |
| cauchy       | location, scale                         | yes |
| logis        | location, scale                         | yes |
| binom        | size, prob                              | no  |
| pois         | lambda                                  | no  |
| nbinom       | size, prob (or mu)                      | no  |
| geom         | prob                                    | no  |
| hyper        | m, n, k                                 | no  |
| wilcox       | m, n (Wilcoxon rank-sum)                | no  |

Every parameter is `f64`. Vectorization happens at the call site:
`dnorm(x: &Double, mean: f64, sd: f64) -> Double` is the primary form;
`dnorm_vec(x: &Double, mean: &Double, sd: &Double) -> Double` is the
fully-vectorized form (frontends pre-align).

**Tail accuracy:** `pnorm(x = 10)` should return ≈ 1.0 with the
remaining mass available via `pnorm_upper(10) ≈ 7.6e-24`, not by
`1 - pnorm(10)` (which loses precision). All distributions provide
both tails as separate functions for this reason.

## §E Correlation and Covariance

| Function | Rust signature | Notes |
|----------|---------------|-------|
| cor      | `fn cor(x: &Double, y: &Double, method: CorMethod, na_action: NaAction) -> Number` | method = Pearson\|Spearman\|Kendall |
| cov      | `fn cov(x: &Double, y: &Double, method: CovMethod, na_action: NaAction) -> Number` |  |
| cor_matrix | `fn cor_matrix(x: &Matrix<f64>, method: CorMethod) -> Matrix<f64>` | pairwise correlations |
| cov_matrix | `fn cov_matrix(x: &Matrix<f64>, method: CovMethod) -> Matrix<f64>` |  |
| autocor  | `fn autocor(x: &Double, lag_max: usize) -> Double` | ACF |
| partial_autocor | `fn partial_autocor(x: &Double, lag_max: usize) -> Double` | PACF |

`NaAction` enum covers R's `use` argument: `EverythingNa`, `AllObs`,
`Complete`, `NaOrComplete`, `PairwiseComplete`. The default is
`EverythingNa` (NA in either input → NA result), matching R.

## §F Inference (Hypothesis Tests)

Every test returns the standard `HTest` envelope:

```rust
pub struct HTest {
    pub statistic: f64,
    pub p_value: f64,
    pub df: Option<f64>,            // some tests have no df (e.g. KS)
    pub estimate: Vec<(String, f64)>, // named estimates
    pub null_value: Vec<(String, f64)>,
    pub conf_int: Option<(f64, f64)>,
    pub method: String,             // "Welch's two-sample t-test" etc.
    pub alternative: Alternative,    // TwoSided, Less, Greater
    pub data_name: String,
}
```

Tests:

| Function | Rust signature | Notes |
|----------|---------------|-------|
| t_test_one_sample | `fn t_test_one_sample(x: &Double, mu: f64, alt: Alternative, conf_level: f64) -> Result<HTest, StatsError>` |  |
| t_test_two_sample | `fn t_test_two_sample(x: &Double, y: &Double, paired: bool, var_equal: bool, alt: Alternative, conf_level: f64) -> Result<HTest, StatsError>` | Welch by default |
| var_test_f | `fn var_test_f(x: &Double, y: &Double, ratio: f64, alt: Alternative, conf_level: f64) -> Result<HTest, StatsError>` | F-test for variance ratio |
| chisq_test_gof | `fn chisq_test_gof(x: &Integer, p: &Double) -> Result<HTest, StatsError>` | goodness of fit |
| chisq_test_indep | `fn chisq_test_indep(x: &Matrix<i64>) -> Result<HTest, StatsError>` | contingency table |
| ks_test_one | `fn ks_test_one(x: &Double, dist: &dyn ContinuousDistribution, alt: Alternative) -> Result<HTest, StatsError>` |  |
| ks_test_two | `fn ks_test_two(x: &Double, y: &Double, alt: Alternative) -> Result<HTest, StatsError>` |  |
| shapiro_test | `fn shapiro_test(x: &Double) -> Result<HTest, StatsError>` | n in [3, 5000] |
| wilcox_test_one | `fn wilcox_test_one(x: &Double, mu: f64, alt: Alternative, conf_level: f64) -> Result<HTest, StatsError>` |  |
| wilcox_test_two | `fn wilcox_test_two(x: &Double, y: &Double, paired: bool, mu: f64, alt: Alternative, conf_level: f64) -> Result<HTest, StatsError>` | Mann-Whitney when paired = false |
| kruskal_test | `fn kruskal_test(groups: &[&Double]) -> Result<HTest, StatsError>` |  |
| friedman_test | `fn friedman_test(x: &Matrix<f64>) -> Result<HTest, StatsError>` |  |
| fisher_test | `fn fisher_test(x: &Matrix<i64>, alt: Alternative, conf_level: f64) -> Result<HTest, StatsError>` | exact 2×2 |
| mcnemar_test | `fn mcnemar_test(x: &Matrix<i64>, correct: bool) -> Result<HTest, StatsError>` | 2×2 paired |
| prop_test | `fn prop_test(successes: &Integer, totals: &Integer, p: Option<f64>, alt: Alternative, conf_level: f64) -> Result<HTest, StatsError>` |  |
| binom_test | `fn binom_test(successes: i64, total: i64, p: f64, alt: Alternative, conf_level: f64) -> Result<HTest, StatsError>` | exact |
| poisson_test | `fn poisson_test(x: i64, t: f64, r: f64, alt: Alternative, conf_level: f64) -> Result<HTest, StatsError>` |  |
| z_test | `fn z_test(x: &Double, mu: f64, sigma: f64, alt: Alternative, conf_level: f64) -> Result<HTest, StatsError>` | known sigma |

All tests have parity tests against R.

## §G Regression

```rust
pub struct LmFit {
    pub coefficients: Double,        // length p, named by term
    pub residuals: Double,
    pub fitted_values: Double,
    pub rank: usize,
    pub df_residual: usize,
    pub qr: QrDecomposition,         // for re-use in summary()
    pub design_matrix: Matrix<f64>,
    pub response: Double,
    pub call: String,                // for printing
}
```

| Function | Rust signature | Notes |
|----------|---------------|-------|
| lm       | `fn lm(y: &Double, x: &Matrix<f64>) -> Result<LmFit, StatsError>` | OLS via QR; rank-deficient handled by aliasing |
| lm_summary | `fn lm_summary(fit: &LmFit) -> LmSummary` | R's summary.lm: coefficients table with SE/t/p, R², F-stat |
| predict_lm | `fn predict_lm(fit: &LmFit, newdata: &Matrix<f64>, interval: PredInterval) -> PredOutput` | confidence and prediction intervals |
| residuals | `fn residuals(fit: &LmFit, kind: ResidKind) -> Double` | raw / standardized / studentized |
| fitted    | `fn fitted(fit: &LmFit) -> Double` |  |
| coef      | `fn coef(fit: &LmFit) -> Double` | with names |
| anova_lm  | `fn anova_lm(fit: &LmFit) -> AnovaTable` | sequential SS |

GLM:

```rust
pub struct GlmFit { /* coefficients, fitted, deviance, family, link, … */ }
```

| Function | Rust signature | Notes |
|----------|---------------|-------|
| glm      | `fn glm(y: &Double, x: &Matrix<f64>, family: Family, link: Link, weights: Option<&Double>) -> Result<GlmFit, StatsError>` | IRLS |

Families: `Gaussian`, `Binomial`, `Poisson`, `Gamma`, `InverseGaussian`,
`NegativeBinomial`. Links: `Identity`, `Logit`, `Probit`, `CLogLog`,
`Log`, `Inverse`, `Sqrt`. The default link per family follows R's
canonical: Gaussian-Identity, Binomial-Logit, Poisson-Log, etc.

NLS:

| Function | Rust signature | Notes |
|----------|---------------|-------|
| nls      | `fn nls(y: &Double, x: &Matrix<f64>, model: ModelFn, start: &Double, control: NlsControl) -> Result<NlsFit, StatsError>` | Gauss-Newton with optional Levenberg-Marquardt |

Excel's `LINEST`, `LOGEST`, `TREND`, `GROWTH`, `INTERCEPT`, `SLOPE`,
`STEYX`, `RSQ`, `FORECAST`, `FORECAST.LINEAR` are all thin wrappers
over `lm` / `predict_lm`. They live in `spreadsheet-core` and call
this crate.

## §H ANOVA

| Function | Rust signature | Notes |
|----------|---------------|-------|
| aov      | `fn aov(y: &Double, factors: &[&Integer]) -> Result<AovFit, StatsError>` | balanced and unbalanced |
| anova    | `fn anova(fit1: &LmFit, fit2: Option<&LmFit>) -> AnovaTable` | one or two models |
| manova   | `fn manova(y: &Matrix<f64>, factors: &[&Integer]) -> Result<ManovaFit, StatsError>` | Wilks, Pillai, Hotelling-Lawley, Roy |

## §I Multivariate

| Function | Rust signature | Notes |
|----------|---------------|-------|
| princomp | `fn princomp(x: &Matrix<f64>, cor: bool, scores: bool) -> Result<PrincompFit, StatsError>` | eigendecomposition of cov/cor |
| prcomp   | `fn prcomp(x: &Matrix<f64>, center: bool, scale: bool) -> Result<PrcompFit, StatsError>` | SVD on centered/scaled data |
| factanal | `fn factanal(x: &Matrix<f64>, factors: usize, rotation: Rotation) -> Result<FactanalFit, StatsError>` | maximum-likelihood; rotations: None, Varimax, Promax |
| cancor   | `fn cancor(x: &Matrix<f64>, y: &Matrix<f64>, xcenter: bool, ycenter: bool) -> Result<CancorFit, StatsError>` |  |
| lda      | `fn lda(x: &Matrix<f64>, grouping: &Integer) -> Result<LdaFit, StatsError>` |  |
| qda      | `fn qda(x: &Matrix<f64>, grouping: &Integer) -> Result<QdaFit, StatsError>` |  |
| mahalanobis | `fn mahalanobis(x: &Matrix<f64>, center: &Double, cov: &Matrix<f64>) -> Double` |  |

## §J Time Series

| Function | Rust signature | Notes |
|----------|---------------|-------|
| ts       | `fn ts(data: &Double, start: TimeIndex, end: Option<TimeIndex>, freq: f64) -> TimeSeries` | sets `tsp` attribute |
| lag      | `fn lag(x: &TimeSeries, k: i32) -> TimeSeries` |  |
| diff     | `fn diff(x: &Double, lag: usize, differences: usize) -> Double` |  |
| acf      | `fn acf(x: &Double, lag_max: usize, kind: AcfKind) -> AcfResult` | kind: Correlation, Covariance, Partial |
| pacf     | `fn pacf(x: &Double, lag_max: usize) -> AcfResult` |  |
| ar       | `fn ar(x: &Double, order: AROrder, method: ArMethod) -> Result<ArFit, StatsError>` | Yule-Walker, Burg, MLE |
| arima    | `fn arima(x: &Double, order: (u32, u32, u32), seasonal: Option<(u32, u32, u32, u32)>, include_mean: bool) -> Result<ArimaFit, StatsError>` | (p, d, q) + (P, D, Q, period) |
| decompose | `fn decompose(x: &TimeSeries, kind: DecompKind) -> Decomposition` | classical: Additive, Multiplicative |
| stl      | `fn stl(x: &TimeSeries, s_window: STLWindow, t_window: Option<usize>, robust: bool) -> StlFit` | LOESS-based |
| forecast | `fn forecast<F: TsModel>(fit: &F, h: usize, level: &[f64]) -> Forecast` | h-step-ahead with prediction intervals |

## §K Smoothing

| Function | Rust signature | Notes |
|----------|---------------|-------|
| density  | `fn density(x: &Double, bw: Bandwidth, kernel: Kernel, n: usize) -> Density` | KDE |
| lowess   | `fn lowess(x: &Double, y: &Double, f: f64, iter: u32, delta: f64) -> Lowess` |  |
| loess    | `fn loess(y: &Double, x: &Matrix<f64>, span: f64, degree: u32, parametric: &[bool]) -> Result<LoessFit, StatsError>` |  |
| smooth_spline | `fn smooth_spline(x: &Double, y: &Double, df: Option<f64>, spar: Option<f64>, lambda: Option<f64>) -> SmoothSpline` |  |
| ksmooth  | `fn ksmooth(x: &Double, y: &Double, kernel: Kernel, bandwidth: f64) -> Ksmooth` |  |
| ecdf     | `fn ecdf(x: &Double) -> Ecdf` |  |
| histogram | `fn histogram(x: &Double, breaks: BreaksMode, freq: bool) -> Histogram` | breaks: Sturges (default), Scott, FD, Custom |

Bandwidth selectors: `Bandwidth::Nrd0` (R's default, Silverman's
rule-of-thumb), `Bandwidth::Sj` (Sheather-Jones), `Bandwidth::Custom(f64)`.

Kernels: `Gaussian`, `Epanechnikov`, `Triangular`, `Biweight`, `Cosine`,
`Optcosine`, `Rectangular`.

## §L Resampling

| Function | Rust signature | Notes |
|----------|---------------|-------|
| sample   | `fn sample<V: Vector>(x: &V, size: usize, replace: bool, prob: Option<&Double>, rng: &mut RngState) -> V` |  |
| bootstrap | `fn bootstrap<F, T>(x: &Double, statistic: F, b: usize, rng: &mut RngState) -> BootstrapResult<T> where F: Fn(&Double) -> T` |  |
| jackknife | `fn jackknife<F, T>(x: &Double, statistic: F) -> JackknifeResult<T>` |  |
| permutation_test | `fn permutation_test<F>(x: &Double, y: &Double, statistic: F, b: usize, alt: Alternative, rng: &mut RngState) -> PermutationResult where F: Fn(&Double, &Double) -> f64` |  |

## §M Special Functions

The mathematical primitives every statistical formula needs.

| Function | Rust signature | Notes |
|----------|---------------|-------|
| gamma_fn | `fn gamma_fn(x: f64) -> f64` | Γ(x); Lanczos approximation |
| lgamma   | `fn lgamma(x: f64) -> f64` | log(\|Γ(x)\|); avoids overflow |
| beta_fn  | `fn beta_fn(a: f64, b: f64) -> f64` | B(a,b) = Γ(a)Γ(b)/Γ(a+b) |
| lbeta    | `fn lbeta(a: f64, b: f64) -> f64` |  |
| choose   | `fn choose(n: f64, k: f64) -> f64` | binomial coefficient; supports non-integer n |
| lchoose  | `fn lchoose(n: f64, k: f64) -> f64` |  |
| erf      | `fn erf(x: f64) -> f64` | Cody's rational approximations |
| erfc     | `fn erfc(x: f64) -> f64` | accurate in tails |
| beta_inc | `fn beta_inc(x: f64, a: f64, b: f64) -> f64` | regularized incomplete beta I_x(a,b) |
| gamma_inc_lower | `fn gamma_inc_lower(x: f64, a: f64) -> f64` | P(a,x) |
| gamma_inc_upper | `fn gamma_inc_upper(x: f64, a: f64) -> f64` | Q(a,x) = 1 - P(a,x) |
| digamma  | `fn digamma(x: f64) -> f64` | ψ(x) = d/dx log Γ(x) |
| trigamma | `fn trigamma(x: f64) -> f64` | ψ'(x) |
| bessel_j | `fn bessel_j(nu: f64, x: f64) -> f64` | J_ν(x) |
| bessel_y | `fn bessel_y(nu: f64, x: f64) -> f64` |  |
| bessel_i | `fn bessel_i(nu: f64, x: f64, expon_scaled: bool) -> f64` |  |
| bessel_k | `fn bessel_k(nu: f64, x: f64, expon_scaled: bool) -> f64` |  |
| atanh    | `fn atanh(x: f64) -> f64` | also Excel's FISHER transform |
| tanh     | `fn tanh(x: f64) -> f64` | also Excel's FISHERINV |

Numerical accuracy: each special function has a parity test against
R's value with ULP tolerance per §5.

## §N RNG (Random Sampling)

`r_uniform`, `r_normal`, etc. are macro-generated from the `Distribution`
trait. Every `r*` takes `(n: usize, params…, rng: &mut RngState) -> Double`.
The full list, by distribution:

`r_uniform` (`runif`), `r_normal` (`rnorm`), `r_t` (`rt`),
`r_chisq` (`rchisq`), `r_f` (`rf`), `r_beta` (`rbeta`),
`r_gamma` (`rgamma`), `r_exp` (`rexp`), `r_weibull` (`rweibull`),
`r_lnorm` (`rlnorm`), `r_cauchy` (`rcauchy`), `r_logis` (`rlogis`),
`r_binom` (`rbinom`), `r_pois` (`rpois`), `r_nbinom` (`rnbinom`),
`r_geom` (`rgeom`), `r_hyper` (`rhyper`), `r_wilcox` (`rwilcox`).

Plus:

| Function | Rust signature | Notes |
|----------|---------------|-------|
| sample_with_replacement | `fn sample_with_replacement<V: Vector>(x: &V, size: usize, prob: Option<&Double>, rng: &mut RngState) -> V` |  |
| sample_without_replacement | `fn sample_without_replacement<V: Vector>(x: &V, size: usize, rng: &mut RngState) -> V` | reservoir for stream; partial Fisher-Yates for fixed input |

---

# Part III — Per-Frontend Aliases

These tables map external-frontend names to canonical Rust signatures.
The cores see only Rust names; these tables live in spreadsheet-core
(for VisiCalc/Lotus/Multiplan/Excel) and r-runtime (for R). No table is
authoritative *here*; each frontend owns its own and is the source of
truth. The tables below are the spec-time alignment, used to verify
that no frontend invents a function the core cannot serve.

## §A1 VisiCalc (1979)

The original 25-or-so function set. Statistical subset:

| VisiCalc | Rust core call |
|----------|---------------|
| `@SUM(range)`     | `sum(range, na_rm=true)` |
| `@AVERAGE(range)` | `mean(range, na_rm=true)` |
| `@MIN(range)`     | `min(range, na_rm=true)` |
| `@MAX(range)`     | `max(range, na_rm=true)` |
| `@COUNT(range)`   | `count_non_na(range)` |
| `@ABS(x)`         | `math_core::abs(x)` |
| `@INT(x)`         | `math_core::trunc(x)` |
| `@SQRT(x)`        | `math_core::sqrt(x)` |
| `@EXP(x)`         | `math_core::exp(x)` |
| `@LN(x)`          | `math_core::ln(x)` |
| `@LOG(x)`         | `math_core::log10(x)` |
| `@SIN(x)`         | `math_core::sin(x)` |
| `@COS(x)`         | `math_core::cos(x)` |
| `@TAN(x)`         | `math_core::tan(x)` |
| `@ASIN(x)`        | `math_core::asin(x)` |
| `@ACOS(x)`        | `math_core::acos(x)` |
| `@ATAN(x)`        | `math_core::atan(x)` |
| `@PI`             | `math_core::PI` constant |
| `@NPV(rate, range)` | `financial_core::npv(rate, range)` |
| `@IF(test, t, f)` | `spreadsheet_core::logical::if_then_else` |
| `@LOOKUP(value, range)` | `lookup_core::lookup` |
| `@ERROR`          | spreadsheet error sentinel |
| `@NA`             | spreadsheet NA sentinel |
| `@TRUE`, `@FALSE` | constants |
| `@AND`, `@OR`, `@NOT` | logical |

VisiCalc had no variance, sd, or rank functions. The subset is
intentionally small to honor the 1979 ceiling.

## §A2 Lotus 1-2-3 (1983) and Symphony (1984)

Statistical functions Lotus added beyond VisiCalc:

| Lotus 1-2-3 | Rust core call |
|-------------|---------------|
| `@AVG`      | `mean(.., na_rm=true)` (alias for AVERAGE) |
| `@COUNTA`   | `count_non_na` |
| `@STD`      | `sd_pop(.., na_rm=true)` (Lotus uses population SD) |
| `@VAR`      | `var_pop(.., na_rm=true)` |
| `@AVEDEV`   | `avedev` |
| `@DEVSQ`    | `devsq` |
| `@MEDIAN`   | `median` |
| `@MODE`     | `mode` (returns first only — Lotus convention) |
| `@RANK`     | `rank` with `TieMethod::Average` |
| `@LARGE`    | `large` |
| `@SMALL`    | `small` |
| `@PERCENTILE` | `quantile` qtype 7 |
| `@QUARTILE` | `quantile` qtype 7 at 0/0.25/0.5/0.75/1 |
| `@SUMSQ`    | `sumsq` |
| `@PRODUCT`  | `prod` |
| `@CORREL`   | `cor` Pearson |
| `@COVAR`    | `cov` |
| `@SLOPE`, `@INTERCEPT`, `@RSQ`, `@STEYX`, `@FORECAST`, `@TREND`, `@GROWTH`, `@LINEST`, `@LOGEST` | wrappers over `lm` / `predict_lm` |

Symphony adds `@MULREGRESS` for multiple regression — also `lm`.

## §A3 Multiplan (1982)

Multiplan used positional names that align with later Excel:

| Multiplan | Rust core call |
|-----------|---------------|
| `AVERAGE`, `SUM`, `MIN`, `MAX`, `COUNT`, `COUNTA`, `STDEV`, `STDEVP`, `VAR`, `VARP`, `MEDIAN`, `LARGE`, `SMALL`, `RANK` | as Lotus equivalents above |

## §A4 Microsoft Excel (current)

Excel's full statistical-function catalog. Both legacy names (e.g.
`STDEV`) and modern compatibility-suffixed names (e.g. `STDEV.S`) are
supported and dispatch to the same core function.

| Excel | Rust core call |
|-------|---------------|
| `SUM`, `SUMSQ`, `PRODUCT` | `sum`, `sumsq`, `prod` |
| `AVERAGE`, `AVERAGEA` | `mean(.., na_rm=true)`, `mean(.., na_rm=false)` |
| `AVERAGEIF`, `AVERAGEIFS` | predicate filter then `mean` |
| `MEDIAN`, `MODE`, `MODE.SNGL`, `MODE.MULT` | `median`, `mode` (single returns first; mult returns all) |
| `COUNT`, `COUNTA`, `COUNTBLANK`, `COUNTIF`, `COUNTIFS` | counting family + spreadsheet-core predicate compiler |
| `MIN`, `MAX`, `MINA`, `MAXA`, `MINIFS`, `MAXIFS` | min/max with na_rm flag |
| `STDEV`, `STDEV.S`, `STDEVA` | `sd(.., na_rm=true)` |
| `STDEV.P`, `STDEVP`, `STDEVPA` | `sd_pop` |
| `VAR`, `VAR.S`, `VARA` | `var` |
| `VAR.P`, `VARP`, `VARPA` | `var_pop` |
| `GEOMEAN`, `HARMEAN`, `TRIMMEAN` | `geomean`, `harmean`, `trimmean` |
| `AVEDEV`, `DEVSQ` | `avedev`, `devsq` |
| `LARGE`, `SMALL`, `RANK`, `RANK.AVG`, `RANK.EQ` | `large`, `small`, `rank` with TieMethod variants |
| `PERCENTILE`, `PERCENTILE.INC`, `PERCENTILE.EXC` | `quantile` qtypes 7, 7, 6 |
| `QUARTILE`, `QUARTILE.INC`, `QUARTILE.EXC` | as PERCENTILE at 4 standard probs |
| `PERCENTRANK`, `PERCENTRANK.INC`, `PERCENTRANK.EXC` | `percent_rank` |
| `CORREL`, `PEARSON` | `cor` Pearson |
| `RSQ` | `cor` squared |
| `COVARIANCE.S`, `COVARIANCE.P` | `cov` (Bessel-corrected vs not) |
| `FISHER`, `FISHERINV` | `atanh`, `tanh` |
| `NORM.DIST`, `NORM.S.DIST`, `NORMDIST`, `NORMSDIST` | `dnorm`, `pnorm` (with cumulative flag) |
| `NORM.INV`, `NORM.S.INV`, `NORMINV`, `NORMSINV` | `qnorm` |
| `T.DIST`, `T.DIST.2T`, `T.DIST.RT`, `TDIST` | `dt`, `pt`, `pt_upper` |
| `T.INV`, `T.INV.2T`, `TINV` | `qt` |
| `F.DIST`, `F.DIST.RT`, `FDIST` | `df`, `pf`, `pf_upper` |
| `F.INV`, `F.INV.RT`, `FINV` | `qf` |
| `CHISQ.DIST`, `CHISQ.DIST.RT`, `CHIDIST` | `dchisq`, `pchisq`, `pchisq_upper` |
| `CHISQ.INV`, `CHISQ.INV.RT`, `CHIINV` | `qchisq` |
| `BETA.DIST`, `BETADIST` | `dbeta`, `pbeta` |
| `BETA.INV`, `BETAINV` | `qbeta` |
| `BINOM.DIST`, `BINOMDIST`, `BINOM.DIST.RANGE` | `dbinom`, `pbinom` (range = pbinom(b) - pbinom(a-1)) |
| `BINOM.INV`, `CRITBINOM` | `qbinom` |
| `EXPON.DIST`, `EXPONDIST` | `dexp`, `pexp` |
| `GAMMA.DIST`, `GAMMADIST` | `dgamma`, `pgamma` |
| `GAMMA.INV`, `GAMMAINV` | `qgamma` |
| `GAMMA`, `GAMMALN`, `GAMMALN.PRECISE` | `gamma_fn`, `lgamma` |
| `HYPGEOM.DIST`, `HYPGEOMDIST` | `dhyper`, `phyper` |
| `LOGNORM.DIST`, `LOGNORMDIST` | `dlnorm`, `plnorm` |
| `LOGNORM.INV`, `LOGINV` | `qlnorm` |
| `NEGBINOM.DIST`, `NEGBINOMDIST` | `dnbinom`, `pnbinom` |
| `POISSON.DIST`, `POISSON` | `dpois`, `ppois` |
| `WEIBULL.DIST`, `WEIBULL` | `dweibull`, `pweibull` |
| `T.TEST`, `TTEST` | `t_test_two_sample` (paired/var.equal flags from `type` arg) |
| `F.TEST`, `FTEST` | `var_test_f` |
| `CHISQ.TEST`, `CHITEST` | `chisq_test_indep` (table from observed + expected) |
| `Z.TEST`, `ZTEST` | `z_test` |
| `CONFIDENCE.NORM`, `CONFIDENCE` | `qnorm`-based half-width |
| `CONFIDENCE.T` | `qt`-based half-width |
| `INTERCEPT`, `SLOPE`, `STEYX`, `FORECAST`, `FORECAST.LINEAR`, `TREND`, `GROWTH`, `LINEST`, `LOGEST` | `lm` / `predict_lm` |
| `PHI` | `dnorm(x, 0, 1)` |
| `PROB` | discrete probability via predicate over a finite list |
| `RANDARRAY` (Excel 365) | bulk `r_uniform` |
| `RAND`, `RANDBETWEEN` | `r_uniform`, `r_uniform`-then-floor |

The legacy/modern duplication (`STDEV` / `STDEV.S`) is a Microsoft
compatibility decision; both names dispatch to the same core function.

## §A5 R `stats` package

R's canonical names are the Rust signature names with R's punctuation:

| R | Rust core call |
|---|---------------|
| `mean`, `median`, `mode`, `var`, `sd`, `sum`, `prod`, `min`, `max`, `range`, `length`, `quantile`, `IQR`, `mad`, `summary`, `fivenum`, `geomean`, `harmean`, `cumsum`, `cumprod`, `cummin`, `cummax` | direct (R `summary` and `fivenum` compose; `summary` returns Min/Q1/Median/Mean/Q3/Max) |
| `cor`, `cov`, `acf`, `pacf` | direct |
| `t.test`, `var.test`, `chisq.test`, `ks.test`, `shapiro.test`, `wilcox.test`, `kruskal.test`, `friedman.test`, `fisher.test`, `mcnemar.test`, `prop.test`, `binom.test`, `poisson.test` | each maps to its `*_test` Rust signature |
| `lm`, `glm`, `nls`, `aov`, `anova`, `manova`, `predict`, `residuals`, `fitted`, `coef`, `summary` (S3 method) | regression family |
| `prcomp`, `princomp`, `factanal`, `cancor` | multivariate |
| `ts`, `lag`, `diff`, `acf`, `pacf`, `arima`, `ar`, `decompose`, `stl`, `forecast` (HoltWinters family lives here too in v2) | time series |
| `density`, `lowess`, `loess`, `smooth.spline`, `ksmooth`, `ecdf` | smoothing |
| `dnorm`, `pnorm`, `qnorm`, `rnorm`, … (every dist) | direct |
| `gamma`, `lgamma`, `beta`, `lbeta`, `choose`, `lchoose`, `digamma`, `trigamma`, `besselJ`, `besselY`, `besselI`, `besselK` | special |
| `set.seed`, `sample`, `replicate` | RNG |

## §A6 S / S-PLUS

S-PLUS function names match R for the statistical core. Differences
are mainly in optimization (`nlmin` vs `nlminb`) and graphics, which
are out of scope for this crate. The S frontend, when implemented,
will be a thin alias layer over the R names.

---

## Catalog Coverage Summary

| Family            | Functions (canonical) | Excel | Lotus | VisiCalc | R |
|-------------------|----------------------|-------|-------|----------|---|
| Descriptive       | 24                   | 38    | 14    | 5         | 22 |
| Counting          | 4                    | 5     | 2     | 1         | 4 |
| Rank/order        | 7                    | 14    | 4     | 0         | 7 |
| Distributions     | 18 × 4 = 72 + trait  | 35    | 0     | 0         | 72 |
| Correlation       | 6                    | 6     | 2     | 0         | 6 |
| Inference         | 17                   | 8     | 0     | 0         | 17 |
| Regression        | 7 + Excel wrappers   | 9     | 9     | 0         | 7 |
| ANOVA             | 3                    | 0     | 0     | 0         | 3 |
| Multivariate      | 7                    | 0     | 0     | 0         | 7 |
| Time series       | 11                   | 4     | 0     | 0         | 11 |
| Smoothing         | 7                    | 1     | 0     | 0         | 7 |
| Resampling        | 4                    | 0     | 0     | 0         | 4 |
| Special functions | 17                   | 5     | 0     | 0         | 17 |
| RNG               | 18 + sample          | 2     | 0     | 0         | 18 |

About 200 distinct mathematical operations, dispatching from ~330
external names across the four frontends. The deduplication pays for
itself.

---

## Implementation Phasing

The crate is too large for one PR. The following phases each become
their own implementation PR after the spec ships:

1. **Phase 1** — Descriptive (§A) + Counting (§B) + Rank (§C). About
   35 functions. Proves the harness, the test infrastructure, the
   parity testing against R, and the spreadsheet-core dispatch path.
2. **Phase 2** — Distributions (§D) and Special Functions (§M). The
   trait, the macro, all 18 distributions, all 17 specials. The
   numerical-accuracy contract gets exercised hard here.
3. **Phase 3** — Correlation (§E), Inference (§F). The `HTest` envelope
   ships. About 25 functions.
4. **Phase 4** — Regression (§G) and ANOVA (§H). Linalg sub-module
   lands. About 15 functions plus `LmFit` / `GlmFit` machinery.
5. **Phase 5** — Multivariate (§I). About 7 functions; needs full
   eigendecomposition and SVD.
6. **Phase 6** — Time series (§J) and Smoothing (§K). About 18
   functions.
7. **Phase 7** — Resampling (§L) and RNG (§N). About 22 functions
   including all `r*` family.

Each phase has Rust crate, tests, parity vectors against R, and
documentation ships together.

---

## Out of Scope

- Bayesian methods (`brms`, `rstan` equivalents). A separate
  `bayes-core` crate, or a future Phase 8.
- Survival analysis (`coxph`, `survreg`). Separate `survival-core`.
- Mixed-effects models (`lme4`). Separate.
- Spatial statistics. Separate.
- Plotting (`graphics` package in R). Renderer concern, not stats.
- Bioconductor-style genomics. Out of scope for the educational
  reconstruction.
- Symbolic statistics (closed-form expectations). That is the `cas-*`
  stack's lane.

Each "out of scope" item is a candidate for a future Layer 1 crate
that depends on `statistics-core`. None of them are blocked by this
crate's design.

---

## References

- *R Internals*, ch. 1-4 (the kernel against which we test parity)
- Becker, Chambers, Wilks, *The New S Language* (1988)
- Knuth, *TAOCP* vol. 2, ch. 3 (RNG fundamentals)
- Matsumoto & Nishimura, "Mersenne Twister" (1998)
- Marsaglia & Tsang, "A simple method for generating gamma variables" (2000)
- Wilks, *Statistical Methods in the Atmospheric Sciences*, 4th ed. (test-vector source for inference)
- Press et al., *Numerical Recipes*, 3rd ed. (special-function references)
- Golub & Van Loan, *Matrix Computations*, 4th ed. (linalg algorithms)
- Microsoft Excel documentation (function reference, primary alias source)
- Lotus Development Corp., *1-2-3 Reference Manual* (1983)
- Bricklin & Frankston, VisiCalc reference card (1979)
