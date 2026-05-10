# ST00 — R-Style Statistics Roadmap

## Status

Roadmap / master spec. Surveys what an S- and R-class statistical
programming environment requires, maps that surface area onto a layered
sequence of per-feature specs, and explains how the R language frontend
plugs into the existing generic language pipeline (LANG00…LANG17).

This document does not duplicate `ST01-stats.md`; it places ST01 in the
larger map and identifies the further specs that need to be written.

## Update — Rust Foundation Shift

The roadmap below was originally drafted with per-host-language
implementations (Python ST01 shipped, Ruby ST01 shipped, etc.). After
the VisiCalc reconstruction work began, the *real implementation* of
the statistics stack consolidated into Rust. The new layered home:

- **Substrate (Layer 0)**: `numeric-tower`, `na-semantics`, `r-vector`,
  `vectorization-rules` — see `numeric-tower.md`, `na-semantics.md`,
  `r-vector.md`, `vectorization-rules.md`.
- **Domain core (Layer 1)**: `statistics-core` — see
  `statistics-core.md`. Comprehensive: every statistical function across
  VisiCalc, Lotus 1-2-3, Symphony, Multiplan, Excel, R, and S, with
  cross-cutting contracts for error/NA propagation, RNG state,
  distribution trait, numerical accuracy.
- **Spreadsheet engine (Layer 3)**: `spreadsheet-core` — see
  `spreadsheet-core.md`. Owns the formula language, dependency DAG,
  recalc, and the dispatch table that maps Excel/Lotus/VisiCalc names
  to Layer 1 function pointers.
- **Frontends (Layer 4)**: `visicalc-modern` (Rust on Mosaic) and
  `visicalc-faithful` (Python on the existing 6502 simulator) — see
  `visicalc-modern.md` and `visicalc-faithful.md`. The future R and S
  runtimes plug into the same Layer 1 cores via their own dispatch.

What this means for the roadmap below:

- **ST01 (descriptive stats)** in Python, Ruby, TS, etc. **stays as
  shipped**. It serves the per-language ecosystems (the Ruby tooling
  in this repo, the Python notebooks, the TS web demos). It is no
  longer the canonical implementation; that is the Rust
  `statistics-core` crate. Cross-language parity tests verify the two
  agree.
- **ST02-ST09 (distributions, inference, regression, multivariate,
  time-series, smoothing, resampling, clustering)** are subsumed into
  `statistics-core.md` as families within the catalog. The individual
  ST0N specs may still be written for didactic depth, but the
  implementation home is the Rust crate, not per-host-language ports.
- **ST10-ST12 (vector semantics, data frames, formulas)** are
  redirected to `r-vector` (and a future `data-frame` crate). ST10's
  Rust home is `r-vector`; ST11 becomes a follow-up `data-frame.md`;
  ST12 becomes a follow-up `r-formulas.md` once the R runtime needs it.
- **R00-R05 (the R language frontend)** now sits on top of the Rust
  Layer 1 cores. The R runtime's dispatcher maps R names (`mean`,
  `lm`, `dnorm`) to the same Rust function pointers that
  `spreadsheet-core` dispatches `AVERAGE`, `LINEST`, `NORMDIST` to.

The architectural rule, locked in: **the statistical core is named for
what it *is* (statistics math), not how it's *consumed* (formulas in a
spreadsheet).** "Formula" is a presentation concern at the spreadsheet
layer; the core is agnostic. Every consumer (spreadsheet,
R runtime, S runtime) gets its own dispatch layer; the cores have no
frontend awareness.

The original roadmap below remains accurate as a *taxonomy* of the
statistical surface area. Read it for the breadth of what an R/S
environment requires; treat the implementation guidance as
superseded by the Rust specs.

## Purpose

The eventual goal is a self-hosted, educational R implementation that:

1. Reproduces the **statistical computing experience** of S/R: a REPL,
   vectorized data, missing values, factors, data frames, formulas,
   distributions, hypothesis tests, regression, time series.
2. Runs on the **same VM substrate** that already executes Tetrad, Nib,
   Lattice, MACSYMA, Brainfuck, and the other languages in this repo
   (`vm-core`, `jit-core`, `compiler-ir`, `interpreter-ir`).
3. Builds **bottom-up** from primitive statistical packages (ST01 …
   ST09) that are usable by every host language in the repo, not just
   by R itself.

There are two halves to that goal:

- **The library half** — a comprehensive statistics package surface
  (descriptive, distributions, tests, regression, multivariate, time
  series, clustering, smoothing, resampling) usable from any host
  language and idiomatic in each.
- **The language half** — an R lexer / parser / runtime that sits on
  top of the generic language pipeline and exposes the library half
  through R's own syntax and semantics.

Each half breaks down into a layered set of specs. This document is the
table of contents.

---

## Why R/S?

R is the *lingua franca* of statistical computing — and S, its commercial
ancestor (Bell Labs, 1976; commercialized as S-PLUS), is where the design
was crystallized. The languages share a single observation that makes
them worth implementing for educational purposes:

> Statistics happens to vectors, not to scalars. The shape of the
> language should reflect that.

In R, `c(1, 2, 3) + 10` is `c(11, 12, 13)` — addition broadcasts. There
is no scalar type; what other languages call a number is just a vector
of length 1. Aggregations like `mean`, `sd`, `quantile`, `summary` are
built into the kernel.

That single design choice forces a long list of semantic decisions:

- **Recycling.** What does `c(1, 2, 3) + c(10, 20)` do? (Recycle: `c(11, 22, 13)`.)
- **NA propagation.** What does `mean(c(1, 2, NA))` return? (`NA`, unless `na.rm = TRUE`.)
- **Coercion.** What does `c(1, "two")` do? (Coerces both to character.)
- **Factor levels.** How do categorical variables work without arithmetic accidents?
- **Formulas.** How do you say `lm(y ~ x + z)` and have `y`, `x`, `z` be looked up in a data frame?
- **Lazy evaluation.** When does `function(x = compute_default())` evaluate `compute_default()`?

Implementing R lets us examine these decisions concretely and explain
them in the literate-programming style this repo prefers. It also
unlocks a large slice of practical computing — the R standard library
has more statistical functionality in 50 MB than every other language
in this repo combined.

S vs R: for our purposes the languages are interchangeable. R is the
de-facto standard, free, and has the larger surviving ecosystem. We
spec **R** below; an S frontend can be added later as a thin alias if
useful (the languages share grammar to a high degree).

---

## §1 The Two Halves

The roadmap divides cleanly:

```text
  ┌────────────────────────────────────────────────────────────────┐
  │  Application: an R program (lm(y ~ x), kmeans(data, 3), …)     │
  └────────────────────────────────────────────────────────────────┘
                                 │
  ┌────────────────────────────────────────────────────────────────┐
  │  R LANGUAGE FRONTEND  (R00 … R05)                              │
  │   r-lexer   →   r-parser   →   r-bytecode-compiler             │
  │                                              │                 │
  │   r-runtime  (R-specific built-ins, NA semantics, recycling)   │
  │                                              │                 │
  │   r-repl, r-notebook-kernel                  ▼                 │
  └─────────────────────────────────  vm-core / jit-core ──────────┘
                                 │
  ┌────────────────────────────────────────────────────────────────┐
  │  STATISTICS LIBRARY HALF  (ST00 … ST09 + supporting)           │
  │                                                                │
  │  ST09 smoothing / density estimation                           │
  │  ST08 resampling / bootstrap                                   │
  │  ST07 clustering                                               │
  │  ST06 time series                                              │
  │  ST05 multivariate analysis                                    │
  │  ST04 regression (lm / glm / nls)                              │
  │  ST03 hypothesis testing                                       │
  │  ST02 probability distributions                                │
  │  ST01 descriptive statistics  ◄── shipped foundation           │
  └────────────────────────────────────────────────────────────────┘
                                 │
  ┌────────────────────────────────────────────────────────────────┐
  │  R-DATA SUBSTRATE  (ST10 … ST12, used by both halves)          │
  │   ST12 formulas (`y ~ x + z`)                                  │
  │   ST11 data frames + factors                                   │
  │   ST10 vector semantics (recycling, NA, coercion)              │
  └────────────────────────────────────────────────────────────────┘
                                 │
  ┌────────────────────────────────────────────────────────────────┐
  │  EXISTING FOUNDATION                                           │
  │   ML03 matrix + matrix-extensions                              │
  │   LANG00 generic language pipeline                             │
  │   vm-core, jit-core, compiler-ir, interpreter-ir               │
  └────────────────────────────────────────────────────────────────┘
```

The data substrate (ST10–ST12) is the cross-cut: the statistics layer
*can* use plain arrays in any host language, but it *must* use
R-flavored vectors when invoked through the R frontend (so that NA,
recycling, and factors behave the way R users expect).

---

## §2 Existing Foundation

What already ships on `main` that the roadmap builds on:

- **`LANG00-generic-language-pipeline.md`** — the two-IR architecture
  (InterpreterIR + CompilerIR). Any new language only needs a
  lexer/parser/AST → bytecode-compiler → InterpreterIR. The VM, JIT,
  optimizer, debugger, REPL, language server, and notebook kernel are
  reused without modification.
- **`vm-core` / `jit-core` / `compiler-ir` / `interpreter-ir`** — the
  shared substrate. Tetrad, Nib, MACSYMA, Lattice, Brainfuck all run on
  it. R will too.
- **`grammar-tools`** — the `.tokens` / `.grammar` format for declaring
  lexers and parsers; lets us treat R's grammar as data and reuse the
  generic GrammarLexer / GrammarParser instead of hand-rolling.
- **`ML03-matrix.md` + `ML03-matrix-extensions.md`** — the matrix
  package that ST01's matrix variants depend on. Available in 9 host
  languages.
- **`ST01-stats.md`** — descriptive statistics + frequency analysis +
  cryptanalysis helpers (mean, median, mode, variance, sd,
  index_of_coincidence, entropy, chi_squared). The foundation layer of
  this roadmap.
- **MACSYMA stack** — concrete proof that a domain-specific language
  with a rich function library can be layered on top of the VM:
  `macsyma-grammar-extensions.md`, `macsyma-runtime.md`,
  `macsyma-repl.md`, `symbolic-computation.md`, and the per-feature
  CAS specs (`cas-factor`, `cas-simplify`, `cas-solve`, `cas-matrix`,
  …). The R rollout follows the same shape.

---

## §3 Layered Package Roadmap — Statistics Library

Each layer is one spec and one publishable package per host language.
Higher layers depend only on layers beneath them and on ML03.

### ST01 — Descriptive Statistics  ✅ shipped

Already on main: `mean`, `median`, `mode`, `variance`,
`standard_deviation`, `min`, `max`, `range`; matrix variants
`mean_matrix`, `variance_matrix`, `std_matrix`; frequency analysis
(`frequency_count`, `frequency_distribution`, `chi_squared`,
`chi_squared_text`); cryptanalysis (`index_of_coincidence`, `entropy`).

What it doesn't yet have but should be added in a follow-up: `quantile`,
`summary` (the five-number summary + mean), `IQR`, `mad` (median
absolute deviation), `weighted.mean`, `cumsum`/`cumprod`/`cummin`/
`cummax`, `prod`. These are still descriptive; no need for a new spec —
extend ST01.

### ST02 — Probability Distributions

R's distribution interface is famously regular: every distribution
exposes four functions with prefixes `d`/`p`/`q`/`r`:

- `d<dist>(x, …)` — probability density (continuous) or mass (discrete).
- `p<dist>(q, …)` — cumulative distribution function (CDF).
- `q<dist>(p, …)` — inverse CDF (quantile function).
- `r<dist>(n, …)` — generate `n` random samples.

ST02 specifies exactly that interface, in language-idiomatic form, for
the standard distribution roster:

| Distribution | Continuous? | Parameters | R prefix |
|---|---|---|---|
| Normal (Gaussian) | yes | `mean`, `sd` | `norm` |
| Student's t | yes | `df` | `t` |
| Chi-squared | yes | `df` | `chisq` |
| F | yes | `df1`, `df2` | `f` |
| Exponential | yes | `rate` | `exp` |
| Gamma | yes | `shape`, `rate` | `gamma` |
| Beta | yes | `shape1`, `shape2` | `beta` |
| Uniform (continuous) | yes | `min`, `max` | `unif` |
| Cauchy | yes | `location`, `scale` | `cauchy` |
| Log-normal | yes | `meanlog`, `sdlog` | `lnorm` |
| Weibull | yes | `shape`, `scale` | `weibull` |
| Logistic | yes | `location`, `scale` | `logis` |
| Binomial | discrete | `size`, `prob` | `binom` |
| Poisson | discrete | `lambda` | `pois` |
| Negative binomial | discrete | `size`, `prob` | `nbinom` |
| Geometric | discrete | `prob` | `geom` |
| Hypergeometric | discrete | `m`, `n`, `k` | `hyper` |
| Multinomial | discrete | `size`, `prob` | `multinom` |

A pseudo-random generator must back the `r…` family. ST02 specifies
**Mersenne Twister 19937** as the default, matching R's behavior, with
a pluggable interface so cryptographic-quality or alternate generators
can substitute. The seed surface is `set.seed(seed)` at the R layer.

ST02 also specifies the special functions on which the distributions
depend (and which R exposes directly):

- `gamma`, `lgamma` (log-gamma)
- `beta`, `lbeta`
- `choose`, `lchoose` (binomial coefficient and its log)
- `factorial`, `lfactorial`
- `digamma`, `trigamma`
- `erf`, `erfc` (error function and complement, used by `pnorm`)
- `bessel_*` (Bessel functions — at least J, Y, I, K)

### ST03 — Hypothesis Testing

R's hypothesis tests share a return shape: a list-with-class `"htest"`
containing fields like `statistic`, `parameter`, `p.value`,
`conf.int`, `estimate`, `null.value`, `alternative`, `method`,
`data.name`. ST03 standardizes that envelope across host languages.

The roster:

- **One- and two-sample tests of location.** `t_test` (one-sample,
  two-sample, paired); `wilcox_test` (Wilcoxon signed-rank and
  Mann-Whitney U); `sign_test`.
- **Tests of variance.** `var_test` (F-test for ratio of variances);
  `bartlett_test` (homogeneity across k samples); `levene_test`.
- **Tests of fit.** `chisq_test` (goodness-of-fit and contingency
  tables); `fisher_test` (Fisher's exact test for 2×2);
  `ks_test` (Kolmogorov-Smirnov, one- and two-sample);
  `shapiro_test` (Shapiro-Wilk normality);
  `anderson_darling_test` (optional).
- **Tests of proportion.** `binom_test`; `prop_test` (z-test of
  proportions, k-sample chi-squared).
- **Tests of association.** `cor_test` (Pearson, Spearman, Kendall
  with significance); `chi_squared_independence` (cross-tabulation).
- **Analysis of variance.** `aov` / `anova` — one-way and balanced
  multi-way ANOVA. (Mixed-effects and unbalanced designs are deferred
  to a later spec.)

ST03 depends on ST01 (for sums, means, variances) and ST02 (for the
reference distributions whose tail probabilities give the p-values).

### ST04 — Regression

The headline R function `lm(y ~ x1 + x2 + …, data = df)` collapses a
remarkable amount of machinery: formula parsing, model-matrix
construction (`model.matrix`), QR decomposition for the least-squares
fit, residuals and fitted values, standard errors, t-tests of
coefficients, F-test of overall fit, confidence and prediction
intervals, summary printing. ST04 specs each piece.

Layers within ST04:

- **Model matrices** (`model_matrix`) — turn a formula + data frame
  into a numeric design matrix `X`, expanding factors via contrast
  coding, and a response vector `y`. Depends on ST10 (vectors), ST11
  (data frames), ST12 (formulas).
- **Linear models** (`lm`) — fit `y = X β + ε` by QR decomposition
  (numerically stable; matches R). Coefficients `β`, residuals `e`,
  fitted values `ŷ`, residual standard error `σ̂`, R² and adjusted R²,
  F-statistic of overall fit. Depends on ML03 matrix (with QR added by
  ML03 extensions or a small dedicated spec), ST01 (mean/var), ST02
  (Student's t and F for inference).
- **Generalized linear models** (`glm`) — same skeleton, IRLS solver,
  link functions: identity (Gaussian), logit (binomial / logistic
  regression), log (Poisson), inverse (Gamma). Pluggable family
  interface so a future Tweedie / quasi-likelihood family can slot in.
- **Predictors** — `predict.lm` (point predictions, with optional
  confidence and prediction intervals), `predict.glm` (response or
  link scale), `fitted`, `resid`, `coef`, `summary.lm`, `anova.lm`
  (sequential / Type I sum of squares).
- **Nonlinear least squares** (`nls`) — Gauss-Newton with Marquardt
  damping. Depends on ML03 matrix and a tiny derivative helper.

QR decomposition is the linchpin. It belongs in ML03 (matrix-level
algorithm) rather than ST04 (statistics-level wrapper); the spec
identifies that as a small extension to ML03.

### ST05 — Multivariate Analysis

The classical multivariate toolkit:

- **Principal component analysis** (`prcomp`, `princomp`). Centered
  + scaled SVD (matches R's `prcomp`). Returns rotation matrix,
  standard deviations, and a `predict` method.
- **Factor analysis** (`factanal`). Maximum-likelihood factor model
  with optional rotation (varimax, promax).
- **Canonical correlation analysis** (`cancor`). Generalizes
  correlation to two sets of variables.
- **Linear and quadratic discriminant analysis** (`lda`, `qda`).
- **Mahalanobis distance** (`mahalanobis`). One line of math but
  pulls in matrix inversion / solve, so worth specifying for parity
  test vectors.

Depends on ML03 matrix (eigen, SVD, solve, det) and ST01 (means,
covariance).

ST05 also defines covariance and correlation at full strength:
`cov`, `cor`, `cov2cor`, with matrix variants. (ST01 has scalar
covariance only.)

### ST06 — Time Series

R's `ts` object is a vector with `start`, `end`, and `frequency`
attributes. The toolkit:

- **`ts` constructor** — wrap a vector with time metadata.
- **Lagged operations** — `lag`, `diff`, `na_locf` (last observation
  carried forward).
- **Autocorrelation** — `acf`, `pacf`, `ccf` (cross-correlation).
- **Decomposition** — `decompose` (classical), `stl` (seasonal-trend
  loess decomposition).
- **Smoothing** — `HoltWinters` (additive and multiplicative).
- **ARIMA** — `arima(p, d, q)` and seasonal `(P, D, Q)` orders;
  state-space (Kalman filter) implementation.
- **Forecasting** — point forecasts + prediction intervals from
  fitted models.

Depends on ST01 (means/var), ST05 (covariance), ML03 matrix (Kalman
filter is matrix-heavy), ST04 (linear regression for trend).

### ST07 — Clustering

Unsupervised partitioning and grouping:

- **K-means** (`kmeans`) — Lloyd's algorithm with
  k-means++ initialization. Within-cluster sum of squares, BSS / TSS
  ratio.
- **Hierarchical clustering** (`hclust`) — agglomerative with the
  classical linkages: single, complete, average, ward.D, ward.D2.
  Returns a dendrogram structure.
- **Distance matrices** (`dist`) — Euclidean, manhattan, maximum,
  binary, Minkowski.
- **Cluster cutting** (`cutree`) — convert a dendrogram into a flat
  partition.
- **Silhouette** (`silhouette`) — diagnostic for cluster quality.

Depends on ML03 matrix and ST01.

### ST08 — Resampling and Bootstrap

- **Sampling** — `sample(x, size, replace, prob)` (uniform or
  weighted; with or without replacement).
- **Bootstrap** — `boot(data, statistic, R)`: R bootstrap replicates
  of an arbitrary statistic. Standard error, bias, and three flavors
  of confidence interval (basic, percentile, BCa).
- **Jackknife** — leave-one-out resampling.
- **Permutation tests** — exchange labels under H₀, build the null
  distribution.

Depends on ST02 (uniform RNG underlies sampling) and the function-as-
value semantics from the host language (or R closures from R's side).

### ST09 — Smoothing and Density Estimation

- **Kernel density estimation** (`density`). Gaussian, Epanechnikov,
  rectangular kernels; bandwidth selection (`nrd0`, `nrd`, `ucv`,
  `bcv`, `SJ`).
- **Loess / lowess** (`loess`, `lowess`). Local polynomial regression.
- **Smoothing splines** (`smooth_spline`). Cubic spline with
  cross-validated smoothing parameter.
- **Kernel regression** (`ksmooth`).
- **Histograms** (`hist`) — bin selection (Sturges, Scott,
  Freedman-Diaconis).

Depends on ML03 matrix and ST01.

---

## §4 R-Data Substrate

These specs define the data shapes that R itself relies on. They are
shared between the R language frontend and the statistics library so
that ST functions can accept either plain host-language arrays *or*
R-flavored vectors and produce sensible behavior either way.

### ST10 — Vector Semantics

The "everything is a vector" rule. Specifies:

- **Atomic vector types** — `logical`, `integer`, `double`, `complex`,
  `character`, `raw`. Length 1 is the smallest legal length.
- **Coercion lattice** — `logical < integer < double < complex <
  character`. Mixing types in `c(...)` coerces to the highest type.
- **Recycling** — element-wise binary operators between vectors of
  unequal length recycle the shorter to match. R warns when the
  longer is not a multiple of the shorter; we do the same.
- **NA values** — every type has its own NA bit pattern (`NA_integer_`,
  `NA_real_`, `NA_character_`, …). Arithmetic and comparison with NA
  produces NA. Logical operators are tri-valued: `NA & FALSE = FALSE`
  but `NA & TRUE = NA`. Aggregations propagate NA unless the caller
  passes `na.rm = TRUE`.
- **Indexing** — `x[i]` with positive integers (selection), negative
  integers (omission), logical vector (filter, with recycling),
  character vector (by name), `0` (drop). `x[[i]]` for atomic
  extraction. `which(cond)` to convert logical to indices.
- **Names attribute** — every vector can carry a names vector of the
  same length; names survive arithmetic ("the names of the result are
  the names of the longer operand").
- **NULL** — distinct from NA. `NULL` is the empty list; `c(1, NULL,
  2) == c(1, 2)`; `length(NULL) == 0`.

ST10 is implemented as a Rust crate (`r-vector`) with bindings exposed
to the other host languages — vector semantics are subtle enough that
we do not want to re-derive them per language.

### ST11 — Data Frames and Factors

- **Factors.** A vector of integer codes plus a `levels` character
  vector. Ordered vs unordered. Drop unused levels. Factor arithmetic
  is undefined; comparisons require ordered factors. Display and
  printing match R.
- **Data frames.** A list of equal-length vectors with a common row
  names attribute. `[`, `[[`, and `$` access. Column types preserved.
  `rbind` / `cbind`, `subset`, `merge`. CSV / TSV read/write.

Depends on ST10.

### ST12 — Formulas

A formula like `y ~ x + z + I(x^2) + factor(group):x` is a *first-class
unevaluated expression* in R. ST12 specifies:

- **Formula AST** — preserves the LHS, RHS, and operators
  (`+`, `-`, `:`, `*`, `^`, `I()`).
- **Term expansion** — turn the RHS into a list of terms (e.g. `a*b`
  expands to `a + b + a:b`), respecting `-1` (drop intercept) and
  `I()` (treat as literal arithmetic).
- **Model matrix construction** — combine a formula + data frame into
  the design matrix `X`. Hands off to ST04 contrast-coding logic for
  factor terms.
- **`update()`** — extend an existing formula (`update(f, . ~ . +
  z)`).

Depends on ST10 + ST11. Used by ST04 (regression) and ST06 (time
series with regressors).

---

## §5 R Language Frontend

Five sub-specs, mirroring the layout we already use for Tetrad
(`TET01-tetrad-lexer.md` … `TET05-tetrad-jit.md`) and MACSYMA
(`macsyma-grammar-extensions.md`, `macsyma-runtime.md`,
`macsyma-repl.md`).

### R00 — R Language Specification

The master language doc. Defines:

- The R syntax we support (a subset of base R; R has corners we
  intentionally do not chase).
- Semantics: lexical scoping, lazy evaluation of arguments,
  copy-on-modify, function dispatch (S3 method dispatch is in scope;
  S4 and R5/Reference classes are out of scope for v1).
- The parts of R we deliberately exclude: the C interface (`.Call`,
  `.External`), `eval(parse())` magic beyond the predictable cases,
  packages with namespaces (we ship a flat global), interactive
  graphics device (`plot()` etc. — replaced by SVG output through
  the html-renderer / forme-vision specs).
- Worked examples: assignments, arithmetic, indexing, function
  definitions, formulas, a complete `lm` call.

R00 is parallel to TET00 in scope and length.

### R01 — R Lexer

R's lexical structure has a few items worth specifying carefully:

- **Assignment operators.** `<-`, `<<-`, `->`, `->>`, `=`. The
  right-pointing forms (`->`, `->>`) are a wart but in scope.
- **Numeric literals.** `1`, `1L` (integer), `1.0`, `1e3`, `0x1F`,
  `1i` (complex). The `L` suffix forces integer; `i` forces complex.
- **String literals.** Single and double quotes, with `\n`, `\t`,
  `\u{1F600}`, `\x41` escapes.
- **Special operators.** `%any%` is user-defined infix; the lexer
  emits a single token for the whole `%…%` block (including `%%`,
  `%/%`, `%*%`, `%o%`, `%in%` plus user-named ones).
- **Backtick-quoted identifiers.** `` `weird name` `` is a legal
  identifier.
- **Newline as expression separator.** Inside parentheses, brackets,
  or braces a newline is whitespace; outside, a newline ends an
  expression. The lexer emits an `IMPLICIT_SEMICOLON` token tracked
  by a paren-depth counter (this is the same trick MACSYMA's lexer
  uses for terminators).

We use the existing `grammar-tools` framework to declare the lexer.
Output: `code/grammars/r.tokens` plus a tiny per-host language wrapper
that consumes it via `GrammarLexer`. Test vectors per `LANG00`.

### R02 — R Parser

R's grammar is unusual in two ways:

- **Operator precedence.** Different from C; assignment binds *more
  loosely* than the right-pointing arrows; comparison is non-
  associative; unary minus interacts with `^`.
- **Function arguments.** Both positional and named, in any order:
  `f(1, x = 2, 3, y = 4)`. Default values are unevaluated expressions
  (lazy). The parser must record both the value expression and
  whether each argument was named.
- **Special forms.** `if`/`else` is an *expression* (returns a
  value); `for`/`while`/`repeat` are also expressions (return
  invisible NULL). `function(args) body` is a literal (R closures
  carry their defining environment, but the parser only needs to
  emit the AST).

The output AST mirrors R's own internal language objects; later specs
(R03 onward) compile that AST to InterpreterIR.

### R03 — R Bytecode Compiler

Compiles the R AST to InterpreterIR. The interesting compilation
problems:

- **Lazy argument promises.** When a function is called, each argument
  is wrapped in a *promise* — a thunk that captures the expression
  and the calling environment. The promise is forced (evaluated) on
  first use inside the callee. The compiler emits a `MAKE_PROMISE`
  bytecode at the call site and a `FORCE_PROMISE` whenever the
  argument is read.
- **Vectorized arithmetic.** `a + b` where `a` and `b` are both
  R vectors does not compile to a scalar `Add` op; it compiles to a
  call into the vector runtime that handles type coercion, recycling,
  and NA propagation. The JIT can specialize on observed types
  (e.g. "both sides observed as `double` of length > 1") to hoist out
  the type-check and inline the loop.
- **S3 dispatch.** `print(x)` first reads the class attribute of `x`,
  then looks up `print.<class>` in the environment, falling back to
  `print.default`. Compile to a `DISPATCH` opcode that the runtime
  resolves (using feedback to specialize).
- **Copy-on-modify.** R values are conceptually copied on assignment
  (`y <- x; y[1] <- 99` does not modify `x`). The runtime uses
  reference counting + COW under the hood; the compiler emits hints
  about whether the LHS is unique, allowing in-place mutation when
  safe.

### R04 — R Runtime

The R-equivalent of `macsyma-runtime`: things that are genuinely
R-flavored and would not belong in the generic VM.

- **The R global environment** (`.GlobalEnv`) and the search path.
- **The base package** (`base`) and core built-ins: arithmetic
  operators, `c`, `length`, `names`, `class`, `attr`, `attributes`,
  `is.*`, `as.*`, `seq`, `rep`, `sort`, `order`, `unique`, `table`,
  `apply`, `sapply`, `lapply`, `mapply`, `Map`, `Reduce`, `Filter`,
  `paste`, `sprintf`, `cat`, `print`, `format`, `nchar`, `substr`,
  `tolower`, `toupper`, `strsplit`, `regmatches`, `gsub`, `sub`,
  `grep`, `grepl`.
- **Statistical built-ins** that R users expect to be in scope
  without `library()`: this is the R-side glue from the language
  frontend to the ST01..ST09 packages.
- **`stopifnot`, `tryCatch`, `withCallingHandlers`** — R's condition
  system.
- **`options()`** — the R session preferences (`digits`, `scipen`,
  `na.action`, `stringsAsFactors`, ...).

R04 is the only deliberately non-reusable spec in the language layer.

### R05 — R REPL

Parallel to `macsyma-repl.md`. Owns:

- The interactive prompt (`> ` and continuation `+ `).
- Expression continuation across multiple lines (parser tells us when
  the expression is incomplete and we should print the continuation
  prompt).
- `[<n>]` indexing in printed vector output (R's "wrap and prefix
  with `[i]`" convention).
- Auto-print rule (top-level expression is printed unless wrapped in
  `invisible()`).
- Tab completion (object names, function arguments).
- Help (`?function`) — opens the spec's literate documentation.

R05 reuses `LANG08-repl-integration.md` and adds R-specific behaviors.

---

## §6 Pipeline Integration

This section maps the R frontend onto LANG00:

| LANG00 component | R frontend supplies |
|---|---|
| Lexer | `code/grammars/r.tokens` + `r-lexer` package |
| Parser | `code/grammars/r.grammar` + `r-parser` package |
| AST | R AST types (vector literal, formula, function, …) |
| Bytecode compiler | `r-bytecode-compiler` (R03) |
| Type checker | optional in v1; v2 adds simple type sketches for JIT feedback |
| Backend selection | InterpreterIR for v1; CompilerIR via existing `ir-optimizer` for hot paths |
| Runtime | `r-runtime` (R04) — R-specific built-ins |
| REPL | `r-repl` (R05) — uses LANG08 |
| Notebook kernel | reuse LANG09 (`notebook-kernel`) |
| Language server | reuse LANG07 (`lsp-integration`) |

The vm-core, jit-core, debugger, GC, profiler, and metrics
infrastructure are reused **without modification**. R does not need
its own VM.

---

## §7 Implementation Order (PR Plan)

A reasonable ordering that keeps each PR independently valuable and
testable:

### Wave 0 — Foundations (mostly exist)

ST01 already shipped. ML03 matrix shipped. LANG00 shipped. To finish
Wave 0 we add:

- ST01 extension: `quantile`, `summary`, `IQR`, `mad`,
  `weighted.mean`, `cumsum`/`cumprod`/`cummin`/`cummax`, `prod`.
- ML03 extension: `qr`, `svd`, `eigen`, `solve` (linear systems),
  `det`. These are needed by ST04 / ST05.

### Wave 1 — Distributions

ST02 in all 9 host languages plus the special functions
(`gamma`, `lgamma`, `beta`, `lbeta`, `choose`, `lchoose`,
`erf`, `erfc`).

### Wave 2 — Hypothesis tests + correlation

ST03 (the test roster) plus `cov`/`cor` (the matrix-strength
covariance/correlation that ST05 will build on).

### Wave 3 — R-data substrate

ST10 (vector semantics, in Rust with bindings), ST11 (factors + data
frames), ST12 (formulas). At this point the R frontend has enough
substrate to start.

### Wave 4 — R lexer + parser + minimal runtime

R00 (master), R01 (lexer), R02 (parser), R03 (bytecode compiler).
Minimal runtime (R04): only the arithmetic/comparison operators,
`c`, `length`, `print`. No stats yet — just "can run a hello-world R
program through the VM".

### Wave 5 — R REPL

R05. The first user-visible artifact: an interactive R prompt that
runs in the same VM as Tetrad / MACSYMA / Lattice.

### Wave 6 — Stats library exposed in R

Glue layer in R04 that exposes ST01..ST03 to R-side code as built-in
functions. By the end of this wave, an R user can write:
`x <- rnorm(100); mean(x); sd(x); t.test(x)`.

### Wave 7 — Regression

ST04: model matrices, `lm`, `glm`, `predict`, `summary`. Glue into R
runtime so `lm(y ~ x, data = df)` works.

### Wave 8 — Multivariate + time series

ST05 + ST06.

### Wave 9 — Clustering + resampling + smoothing

ST07 + ST08 + ST09.

### Wave 10 — JIT specialization

Type-feedback driven specialization of vectorized arithmetic in the
R compiler → CompilerIR path. This is where we benefit from the
two-IR design: pure InterpreterIR is fine for getting started; hot
loops over numeric vectors are exactly what the JIT targets.

### Wave 11 — Notebook + LSP

R notebook kernel (LANG09) and language server (LANG07) for the R
frontend. Reuses generic infrastructure; thin per-language adapter.

### Wave 12 — Plotting

Out of scope for this roadmap as a *language* feature, but at this
point we can layer an SVG/Canvas plotting library on top of
forme-vision (FM00) and the html-renderer to give R users `plot()`,
`hist()`, `boxplot()`. That is its own roadmap (`PLT00` etc.).

---

## §8 Cross-Cutting Conventions

- **Errors.** Each spec defines a typed error category for its
  preconditions (shape mismatch, NA where not allowed, singular
  matrix in `solve`, …). Errors use the language's idiomatic
  mechanism (Result in Rust, Either in Haskell, exceptions in
  Python/Ruby/Java). Error *types* — not just messages — are part
  of the public interface and must be parity-tested.
- **Logging.** Statistical packages do not log. Diagnostic output is
  the caller's responsibility. The R runtime adds `message()` /
  `warning()` / `stop()` on top.
- **Determinism.** With a fixed `set.seed`, all `r…` distribution
  samplers, `kmeans` initializations, bootstrap replicates, and
  permutation tests must produce identical output across host
  languages. ST02's parity test vectors include the first 10 draws
  from each distribution at seed 42.
- **NA handling.** Every aggregation accepts `na.rm = FALSE` (default,
  propagate NA) or `na.rm = TRUE` (drop). Every reduction documents
  what an all-NA input returns (`NA` for `mean`, error or `NA`
  according to R for `min`/`max`).
- **Numeric tolerance.** Float-equality in tests uses tolerance 1e-9
  unless the spec specifies otherwise. Distribution function tests
  inherit R's tolerance (1e-6 for tail probabilities, 1e-8 for
  density values).
- **Naming.** Language-idiomatic. R uses `t.test`; Python uses
  `t_test`; Rust uses `t_test` in `snake_case`; TypeScript uses
  `tTest` in `camelCase`. The spec lists the canonical name in the
  R column and each language column derives from it predictably.

---

## §9 Out of Scope

We are deliberately not chasing the entire R standard library. The
following are explicitly out of scope for this roadmap (and may be
revisited in a successor roadmap):

- **R5 / Reference classes / S4.** S3 dispatch is in; the OO systems
  built on top of S3 (R5, S4, R6) are not.
- **`Rcpp` / C-level interop.** We will not be implementing R's C
  API. Native extensions, if needed, route through the existing FFI
  bridges (`DS01`).
- **Bioconductor and CRAN packages.** The spec covers core R only.
  Domain-specific extensions (`Bioconductor`, `tidyverse`, `caret`,
  `lme4`, `ggplot2`, …) are out of scope.
- **Plotting devices.** As noted, plotting is its own roadmap.
- **Spatial / GIS** (`sp`, `sf`, raster).
- **Unicode collation** beyond the host language's default.
- **Localized output.** No locale-dependent number formatting, no
  translated error messages.
- **Shiny-style interactive web apps.** Possibly fits forme-vision +
  venture-browser later, but not part of the R language spec.
- **JIT specialization for non-numeric types.** Wave 10 only
  specializes vectorized numeric operations. String operations stay
  in the interpreter loop until proven a bottleneck.

---

## §10 Spec Production Order — Concrete Next PRs

This is the actionable list, in the order the specs should be written:

1. `ST00-r-stats-roadmap.md` — this document. ✅
2. Extend `ST01-stats.md` with `quantile`, `summary`, `IQR`, `mad`,
   `weighted.mean`, `cumsum`/`cumprod`/`cummin`/`cummax`, `prod`.
3. Extend `ML03-matrix-extensions.md` with `qr`, `svd`, `eigen`,
   `solve`, `det`.
4. `ST02-distributions.md` — full d/p/q/r interface for the
   distribution roster.
5. `ST10-r-vector-semantics.md` — recycling, NA, coercion lattice.
6. `ST03-hypothesis-tests.md`.
7. `ST11-data-frames-factors.md`.
8. `ST12-formulas.md`.
9. `R00-r-language.md` — master.
10. `R01-r-lexer.md` and `code/grammars/r.tokens`.
11. `R02-r-parser.md` and `code/grammars/r.grammar`.
12. `R03-r-bytecode-compiler.md`.
13. `R04-r-runtime.md`.
14. `R05-r-repl.md`.
15. `ST04-regression.md`.
16. `ST05-multivariate.md`.
17. `ST06-time-series.md`.
18. `ST07-clustering.md`.
19. `ST08-resampling.md`.
20. `ST09-smoothing-density.md`.

Each spec follows the existing repo conventions: literate-programming
in the source, parity test vectors, package-per-language matrix,
coverage > 80% on libraries, BUILD/CHANGELOG/README per package.

---

## §11 References

Internal:

- `LANG00-generic-language-pipeline.md` — the two-IR architecture
- `TET00-tetrad-language.md` … `TET05-tetrad-jit.md` — model for a
  complete language stack on this VM
- `macsyma-runtime.md`, `macsyma-repl.md`,
  `macsyma-grammar-extensions.md` — model for a domain language
  with a rich function library
- `ML03-matrix.md`, `ML03-matrix-extensions.md`
- `ST01-stats.md` — the existing foundation layer
- `grammar-tools.md` — the `.tokens` / `.grammar` format
- `LANG08-repl-integration.md`
- `LANG09-notebook-kernel.md`
- `LANG07-lsp-integration.md`

External:

- *The R Language Definition*, R Core Team. The authoritative R
  language reference; this spec departs from it only where noted.
- *Statistical Models in S*, Chambers and Hastie (1992). The
  canonical S reference; the conceptual ancestor of R's modeling
  interface (`lm`, `glm`, formulas).
- *Modern Applied Statistics with S*, Venables and Ripley. The
  practical companion to *Statistical Models in S*; useful as a
  scope check for which statistical functions a "complete" S/R
  implementation needs.
- *Mersenne Twister: A 623-dimensionally equidistributed uniform
  pseudo-random number generator*, Matsumoto and Nishimura (1998).
  The default RNG ST02 specifies.
- IEEE 754 — the float representation we assume for `double`. R's NA
  is a specific NaN payload; ST10 nails down the exact bit pattern.
