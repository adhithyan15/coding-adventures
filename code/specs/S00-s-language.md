# S00 — The S Language (Bell Labs), v1

## Status

Active spec. Defines the historical **S** statistical language as implemented
in this repo: a Rust vertical slice — lexer → parser → tree-walking runtime →
REPL — sufficient to evaluate real S sessions such as
`x <- c(1, 2, 3); mean(x)`.

This is the **language** half of the S/R effort. It is a deliberate companion
to, and a divergence from, [`ST00-r-stats-roadmap.md`](ST00-r-stats-roadmap.md):
ST00 specs the *R* frontend (R00–R05) with S noted only as "a thin alias added
later." We instead build the **historical S frontend first**, because this
repository's identity is faithful historical reconstruction (Macsyma, VisiCalc,
PowerPC 601, Dartmouth BASIC). Where this spec and ST00 disagree, the divergence
is called out explicitly in §9.

## §1 What S is, and why

S was created at Bell Laboratories beginning in **1976** by John Chambers,
Rick Becker, and Allan Wilks as an interactive environment for data analysis —
"an interactive environment for data analysis and graphics." It is the direct
ancestor of R (R is "an implementation of the S language"). The "New S"
described in *The New S Language* (Becker, Chambers & Wilks, 1988 — the
"Blue Book") crystallized the design this spec targets.

S is built on one observation that shapes the entire language:

> Statistics happens to **vectors**, not to scalars.

In S, `c(1, 2, 3) + 10` is `c(11, 12, 13)`: arithmetic broadcasts over vectors.
There is no scalar type — what other languages call "a number" is a vector of
length one. Aggregations (`mean`, `sum`, `sd`) are part of the language's core
vocabulary. That single decision cascades into recycling, missing-value
propagation, and coercion rules, all specified below.

### The iconic historical detail: `_` is assignment

The single most distinctive feature of historical S is that the **underscore
`_` is the assignment operator**, equivalent to `<-`:

```s
x _ c(1, 2, 3)     # historical S: assign c(1,2,3) to x  (same as x <- ...)
x <- c(1, 2, 3)    # the arrow form, also valid
```

This is *why*, to this day, R programmers are taught never to put `_` inside a
name: in S an underscore was never part of an identifier — it meant "assign."
We honor that faithfully: **`_` is a token, and identifiers may not contain it.**
`=` is **not** a general assignment operator in S (it only binds named
arguments inside a call) — also faithful to the era.

## §2 Scope of v1

### In scope (the supported subset)

- **Atomic vectors**: `double` (numeric) and `character` (string), plus a
  `logical` type (`TRUE`/`FALSE`/`T`/`F`) and `NULL`. Length-1 is the smallest
  legal vector; there is no scalar.
- **Literals**: numbers (`1`, `3.14`, `1e3`, `.5`), strings (`"..."`/`'...'`),
  `TRUE`/`FALSE`/`T`/`F`, `NULL`, `NA`, `Inf`, `NaN`.
- **Assignment**: `<-`, `_` (historical), and right-assignment `->`. `=` binds
  named call arguments only.
- **`c(...)`** — the combine function building vectors (with type coercion).
- **Arithmetic** `+ - * / ^` with **recycling**, **NA propagation**, and
  numeric coercion. Unary minus. The sequence operator `:` (`1:5`).
- **Comparison** `== != < > <= >=` (return logical vectors).
- **Indexing** `x[i]` by a positive-integer index vector.
- **Function values**: `function(args) body`, lexical scoping, closures,
  positional and named arguments, default argument values.
- **Control flow as expressions**: `if/else`, `for`, `while`, `{ ... }` blocks.
- **Built-ins**: `c`, `length`, `print`, `seq`, and the statistics glue
  `mean`, `sum`, `sd`, `var`, `median`, `min`, `max`, `prod` (dispatched to
  `statistics-core`).
- **REPL**: interactive prompt with continuation, auto-print of top-level
  results, and the `[i]` index-prefix print convention.

### Out of scope for v1 (documented follow-ups)

S3/S4 method dispatch; lazy-evaluation promises; data frames, factors, and
formulas (`y ~ x`); the `d/p/q/r` distribution family and modeling (`lm`);
user-defined `%op%` infix operators; complex numbers and
the integer-`L` suffix; negative/logical/named indexing and `[[ ]]`/`$`;
attributes beyond `names`; graphics. The numeric/statistical depth lives in
ST02–ST12 and `statistics-core`; this spec is only the language frontend.

## §3 Lexical structure (`code/grammars/s.tokens`)

| Token | Pattern / value | Notes |
|-------|-----------------|-------|
| `ASSIGN` | `<-` | primary assignment |
| `SUPER_ASSIGN` | `<<-` | super-assignment (R-21): rebinds in the nearest enclosing scope, else global |
| `RIGHT_ASSIGN` | `->` | right assignment |
| `UNDERSCORE` | `_` | **historical assignment** |
| `NUMBER` | `[0-9]*\.?[0-9]+([eE][+-]?[0-9]+)?` | double literal |
| `STRING` | `"..."` or `'...'` | both quote styles |
| `NAME` | `[A-Za-z.][A-Za-z0-9.]*` | **no `_`** (it is assignment) |
| `LE GE EQ_EQ NE` | `<= >= == !=` | multi-char first |
| `LT GT` | `< >` | |
| `PLUS MINUS STAR SLASH CARET` | `+ - * / ^` | `^` right-assoc |
| `COLON` | `:` | sequence operator |
| `ASSIGN_EQ` | `=` | named-arg binding only |
| `LPAREN RPAREN LBRACE RBRACE LBRACKET RBRACKET` | `( ) { } [ ]` | |
| `COMMA SEMICOLON` | `, ;` | |
| `NEWLINE` | `\r?\n` | significant terminator |
| keywords | `if else for while repeat function break next in` | promoted from `NAME` |

`#` introduces a comment to end of line (skipped). Horizontal whitespace is
skipped. Multi-character operators are declared before their single-character
prefixes so `<-` never lexes as `<` then `-`.

### Newlines as statement terminators

S, like Macsyma, ends a statement at a newline — **unless** the newline falls
inside an open `(` or `[`, where it is insignificant (so a call's arguments may
span lines). Inside `{ }`, newlines **remain significant**: they separate the
statements of a block, exactly as in R. The lexer applies a post-tokenize hook
that tracks *paren/bracket* depth (not brace depth) and **drops** newlines that
occur at depth > 0, keeping the rest as `NEWLINE` terminators. (This mirrors the
`relabel`/`suppress` hook pattern used by the Dartmouth BASIC lexer.)

## §4 Grammar (`code/grammars/s.grammar`)

Precedence is encoded by rule nesting (lowest binds loosest), matching S:

```
program     →  { (statement)? NEWLINE | (statement)? SEMICOLON | statement }
statement   →  expr
expr        →  assignment
assignment  →  or_expr ( (ASSIGN | UNDERSCORE | RIGHT_ASSIGN) assignment )?
or_expr     →  and_expr            (logical OR — reserved for later)
and_expr    →  comparison
comparison  →  range ( (LT|GT|LE|GE|EQ_EQ|NE) range )?     # non-associative
range       →  add ( COLON add )?                          # 1:5
add         →  term { (PLUS|MINUS) term }
term        →  unary { (STAR|SLASH) unary }
unary       →  MINUS unary | power
power       →  postfix ( CARET unary )?                    # ^ right-assoc
postfix     →  primary { call_args | index }               # f(...) / x[...]
primary     →  NUMBER | STRING | NAME | TRUE | FALSE | NULL | NA
            |  func_def | if_expr | for_expr | while_expr | block | group
```

`func_def → "function" "(" [params] ")" expr`,
`if_expr → "if" "(" expr ")" expr [ "else" expr ]`,
`call_args` allows positional and `NAME = expr` named arguments. Because the
grammar references `NEWLINE`, the `GrammarParser` runs in newlines-significant
mode automatically.

## §5 Semantics

- **Everything is a vector.** A literal `3` is a `double` of length 1.
- **Recycling.** A binary op over vectors of unequal length recycles the
  shorter to the longer's length: `c(1,2,3,4) + c(10,20)` → `c(11,22,13,24)`.
  Length 0 with anything non-empty yields length 0.
- **NA propagation.** `NA` participates in arithmetic/comparison by producing
  `NA`. `NA` uses R/S's reserved double bit pattern (`r_vector::na_real`).
- **Coercion lattice.** `logical < double < character`. `c(1, "a")` coerces all
  to character; `c(TRUE, 2)` coerces to double (`TRUE`→1, `FALSE`→0).
- **Assignment** binds a name in the current environment. `x <- v`, `x _ v`,
  and `v -> x` are equivalent. Assignment is an expression returning `v`
  invisibly (it does not auto-print at the REPL).
- **Lexical scoping.** A `function` captures its defining environment; calls
  push a new child environment whose parent is the closure's environment.
- **`if`/`for`/`while` are expressions.** `if` returns its taken branch's value
  (or `NULL` invisibly when no `else` and the test is false); loops return
  `NULL` invisibly.

### Evaluation model — tree-walking interpreter

v1 evaluates the `GrammarASTNode` parse tree directly with a recursive
tree-walker (`s-runtime`), in the spirit of `macsyma-runtime`. It does **not**
lower to bytecode / InterpreterIR / `vm-core`. Numeric work is delegated to the
shipped substrate: vectors are `r_vector::Double` / `Character`; scalar
arithmetic and coercion go through `numeric-tower`; `mean`/`sd`/… call
`statistics_core::descriptive`. This is the fastest faithful path to a working
REPL and keeps the substrate authoritative for the math.

## §6 Built-in functions (v1)

| S name | Behavior | Backed by |
|--------|----------|-----------|
| `c(...)` | combine into one vector (coercing) | s-runtime |
| `length(x)` | element count as length-1 double | s-runtime |
| `seq(a, b)` / `a:b` | integer-step sequence | s-runtime |
| `print(x)` | print with `[i]` prefixing; returns `x` invisibly | s-repl/s-runtime |
| `mean sum sd var median min max prod` | reductions (`na.rm=FALSE` default) | `statistics-core` |

`na.rm = TRUE` is accepted as a named argument and threads through to the
`statistics-core` `na_rm` parameter.

## §7 Worked examples

```s
> x <- c(1, 2, 3)
> mean(x)
[1] 2
> x * 10 + c(1, 2)      # recycling: c(11,22,31)
[1] 11 22 31
> sd(x)
[1] 1
> sq <- function(v) v * v
> sq(1:4)               # 1:4 is c(1,2,3,4)
[1]  1  4  9 16
> y _ c(5, NA, 7)       # historical underscore assignment
> mean(y)
[1] NA
> mean(y, na.rm = TRUE)
[1] 6
```

## §8 Pipeline integration

Mirrors the Dartmouth BASIC / Macsyma frontends and `LANG00`:

| Component | S supplies |
|-----------|-----------|
| Lexer | `code/grammars/s.tokens` + `s-lexer` (wraps `lexer::GrammarLexer`) |
| Parser | `code/grammars/s.grammar` + `s-parser` (wraps `parser::GrammarParser`) |
| Runtime | `s-runtime` (tree-walker over `r-vector`/`numeric-tower`/`statistics-core`) |
| REPL | `s-repl` (impl `repl::Language`) + `s` binary |

`s-lexer` / `s-parser` embed their compiled grammar as a committed
`src/_grammar.rs` (regenerated with `code/scripts/generate-compiled-grammars.sh`),
never re-parsing the `.tokens` / `.grammar` files at runtime.

## §V2 — Language v2 additions

v2 deepens the language toward R-class expressiveness. Everything in §1–§8
still holds; this section layers on top.

### V2.1 Corrected operator precedence

v1 placed the `:` sequence operator *looser* than `+ - * /`, so `1:3+1` parsed
as `1:(3+1)`. R binds `:` tighter than arithmetic. The cascade is corrected to
(loosest → tightest):

```
assignment  <- _ <<- ->        (right-assoc)
comparison  == != < > <= >=
additive    + -
multiplicative  * /
special     %op%               (NEW — user and built-in infix)
range       :                  (now tighter than * /, matching R)
unary       - (prefix)
power       ^                  (right-assoc)
postfix     f(...) x[...] x[[...]] x$name   (NEW: [[ ]] and $)
```

So `1:3+1` is now `c(2,3,4)`, and `2 %in% 1:3` parses as `2 %in% (1:3)`.

### V2.2 Infix operators (`%op%`)

The lexer emits one `PERCENT_OP` token for a whole `%…%` block. Built-in:
`%%` (modulo, result takes the divisor's sign), `%/%` (floor division), `%in%`
(membership → logical), `%o%` (outer product). A **user-defined** `%foo%` is any
function bound to the name `"%foo%"`; `a %foo% b` calls it `(a, b)`. Defining one
needs a string assignment target: `"%between%" <- function(x, r) x >= r[1] & x <= r[2]`.

### V2.3 Expanded built-in library

- **Vectorized math:** `abs sqrt exp log log10 floor ceiling round sin cos tan`.
- **Utilities:** `rev sort order rep unique which any all is.na cumsum cumprod
  paste paste0`. (No underscore-named helpers like R's `seq_len`/`seq_along`:
  in historical S `_` is assignment, so such names cannot be written — `seq()`
  and `1:n` cover the need.)
- **Set operations & ranking** *(R-29)*: `union(x, y)` (distinct elements of
  `c(x, y)`, first-occurrence order — i.e. `unique(c(x, y))`), `intersect(x, y)`
  (elements in both, in `x`'s order, deduplicated), `setdiff(x, y)` (elements of
  `x` not in `y`, deduplicated), `is.element(el, set)` (the function spelling of
  `el %in% set`), `duplicated(x)` (a logical vector, `TRUE` where an element
  repeats an earlier one), and `rank(x)` (sample ranks with **average** tie
  handling — `rank(c(1,1,2))` is `c(1.5, 1.5, 3)`). All are pure, numeric- and
  character-aware (they key on the same coerced-character form as `unique`/`%in%`)
  and reuse the existing `as_character` / `index` / `membership` / `combine`
  machinery.
- **Ordering refinements** *(R-30)*: extensions of the R-29/R-13 ordering builtins
  (no new value type, no grammar change). **Multi-key `order(x, y, ...)`** sorts the
  index permutation lexicographically by the first key, breaking ties by the second,
  etc., with remaining ties kept in original order (stable); each key is coerced
  independently (numeric compares numerically with `NA` last, character
  lexicographically), all keys must share the first key's length, and the single-key
  R-13 form is the arity-1 special case (`order(c(2,1,2), c(1,2,1))` is `c(2,1,3)`).
  **`rank(x, ties.method=)`** gains `"min"` (lowest position in a tie run), `"max"`
  (highest), and `"first"` (consecutive ranks in original order) alongside the default
  `"average"` (`rank(c(1,1,2))` → `c(1.5,1.5,3)`/`c(1,1,3)`/`c(2,2,3)`/`c(1,2,3)`);
  the result is always numeric. **`duplicated(x, fromLast=TRUE)`** runs the dup scan
  right-to-left, so the **last** occurrence is the keeper and the earlier repeats are
  `TRUE` (`duplicated(c(1,2,1), fromLast=TRUE)` is `c(TRUE,FALSE,FALSE)`).
  **`anyDuplicated(x)`** returns the 1-based index of the first duplicated element, or
  `0` if none (`anyDuplicated(c(1,2,1))` is `3`). The keyword args reuse the existing
  readers (`truthy()` for `fromLast=`, `as_character()` for `ties.method=`). Deferred
  to **R-31**: `incomparables=` on `duplicated` / `anyDuplicated` / `unique`,
  `unique(fromLast=)`, and `rank`'s `"random"` method (needs an RNG-seed contract).
- **Set-op & ordering refinements** *(R-31)*: extensions of the R-29/R-30 dedup &
  ranking builtins (no new value type, no grammar change). **`incomparables=`** on
  `duplicated(x, incomparables=)`, `anyDuplicated(x, incomparables=)`, and
  `unique(x, incomparables=)`: the default `FALSE` means "no incomparables", while a
  **vector** lists the elements to treat as *incomparable* — such a value is **never
  equal to anything**, so it is never flagged as a duplicate and never removed as one.
  We coerce the `incomparables` vector to the same `as_character` key and short-circuit
  any element whose key is in that set to "not a duplicate" (and never insert it into
  `seen`). `duplicated(c(1,1,2,2), incomparables=1)` is `c(FALSE,FALSE,FALSE,TRUE)`;
  `unique(c(1,1,2,2), incomparables=1)` is `c(1,1,2)`; `anyDuplicated(c(1,2,1),
  incomparables=1)` is `0`. **`unique(x, fromLast=TRUE)`** keeps the **last** occurrence
  of each distinct value (gathered in ascending index order), mirroring R-30's
  `duplicated(fromLast=)`. **`rank(x, ties.method="random")`** assigns a run of tied
  values the consecutive ranks `lo..=hi` but permutes them with a Fisher–Yates shuffle
  driven by the **session RNG** (the `set.seed`-seeded generator shared with the R-8
  distribution family via `Interpreter::sample_with`/`RngState::next_u32`), so the
  result is fully reproducible under `set.seed`. Numeric and character vectors are both
  supported. Deferred to **R-32**: `incomparables=`/`fromLast=` on the binary set ops
  (`union`/`intersect`/`setdiff`), whose R semantics are ambiguous enough to warrant
  their own pass.
- **Matrix cross products** *(R-36)*: `crossprod`/`tcrossprod`, defined purely in
  terms of the existing R-11 `t()` transpose and `%*%` matrix product (no new
  linear algebra, no new value type). **`crossprod(x, y)` = `t(x) %*% y`** and
  **`crossprod(x)` = `t(x) %*% x`** (the Gram matrix `X'X`); **`tcrossprod(x, y)` =
  `x %*% t(y)`** and **`tcrossprod(x)` = `x %*% t(x)`** (`XX'`). The second argument
  defaults to the first. The implementation calls the public `t()` builtin (`b_t`)
  and the evaluator's `matrix_multiply` (the `%*%` handler, exposed `pub(crate)`),
  so it **inherits** that handler's `MAX_SEQ_LEN` allocation guard on the
  `nrow * ncol` result and its `"non-conformable arguments"` error (raised before
  any indexing when inner dims disagree) — no new unbounded multiplier, no OOB.
  `crossprod(matrix(c(1,2,3,4), nrow=2))` is `[[5,11],[11,25]]`;
  `tcrossprod` of the same is `[[10,14],[14,20]]`. As in R a bare vector flows
  through `%*%`'s existing vector promotion (left = row, right = column); the
  dense-matrix case is the solid, tested core.
- **Triangular solves** *(R-41)*: `backsolve(r, x)` and `forwardsolve(l, x)`,
  the two substitution-based linear solvers for a *triangular* coefficient
  matrix — available to both S and R through the shared tree-walker.
  **`backsolve(r, x)`** solves `r %*% y = x` for `y` with `r` **upper**-triangular
  (`n×n`), by **back-substitution** (last row first):
  `y[i] = (x[i] − Σ_{j>i} r[i,j]·y[j]) / r[i,i]` for `i = n-1 … 0`.
  **`forwardsolve(l, x)`** solves `l %*% y = x` with `l` **lower**-triangular, by
  **forward-substitution** (first row first):
  `y[i] = (x[i] − Σ_{j<i} l[i,j]·y[j]) / l[i,i]` for `i = 0 … n-1`. In both, `x`
  is either an `n`-vector (single RHS → an `n`-vector result) or an `n×k` matrix
  (`k` RHS columns → an `n×k` matrix result, solved column-by-column). Only the
  relevant triangle is read — upper for `backsolve`, lower for `forwardsolve` —
  matching R's defaults (`upper.tri = TRUE`, `transpose = FALSE`); this is the
  `O(n²·k)` fast path, half the work of the `O(n³)` general `solve`. The
  implementation **reuses** the existing `square_matrix` helper (shared with
  `det`/`solve`/`chol` — non-matrix / non-square / over-`MAX_SOLVE_DIM`
  rejection), the same vector-vs-matrix RHS reading (`n`-row check, `MAX_SOLVE_DIM`
  column cap) `solve` uses, column-major indexing (`r[i,j]` at `j·n + i`), and the
  `SValue::Matrix` constructor. **Error paths, no panic / no NaN / no Inf:**
  non-square or non-numeric `r`/`l` → error before any indexing; a RHS whose row
  count ≠ `n` → error before the loop; a **zero on the diagonal** (singular) is
  tested `== 0` **before** the division, so it is a clean *"apparently singular"*
  error rather than a propagated `NaN`/`Inf`; `NA` in `r`/`l`/`x` → error. Worked
  examples: `backsolve(matrix(c(2,0,1,3), nrow=2), c(5,9))` is `c(1,3)` (and
  `r %*% c(1,3) == c(5,9)`); `forwardsolve(matrix(c(2,1,0,3), nrow=2), c(4,11))`
  is `c(2,3)`. The optional `k =`, `transpose = TRUE`, and `upper.tri` flips are
  shipped in **R-42** (see next item); this item ships the default full-triangle
  dense core (vector and multi-column matrix RHS).
- **Triangular-solve options** *(R-42)*: the same shared `triangular_solve`
  helper, extended with base-R's three named arguments —
  `backsolve(r, x, k = ncol(r), upper.tri = TRUE,  transpose = FALSE)` and
  `forwardsolve(l, x, k = ncol(l), upper.tri = FALSE, transpose = FALSE)`.
  **`upper.tri`** selects which triangle of the first argument to read (its
  per-builtin default — `TRUE` for `backsolve`, `FALSE` for `forwardsolve` — is
  overridden by an explicit value); the substitution direction follows the
  triangle read (upper ⇒ back-substitution, lower ⇒ forward-substitution).
  **`transpose = TRUE`** solves `t(R) %*% y = x`, which flips the direction and
  reroutes every coefficient read through the transposed column-major index
  (`R[j,i]` at `i·n + j` wherever the untransposed solve used `R[i,j]` at
  `j·n + i`); the effective direction is back-substitution iff
  `upper.tri != transpose`. **`k`** restricts the solve to the leading `k×k`
  block and the first `k` RHS rows (result has `k` rows); indexing keeps the
  full stride `n`, so no data is copied — the loops simply range over `0..k`.
  **Safety (preserved from R-41):** `k` is range-checked `0 ≤ k ≤ n` (a
  malformed/out-of-range `k` is a clean error, never an out-of-bounds read); the
  zero-on-the-*used*-diagonal singular check is unchanged (the diagonal index is
  transpose-invariant); RHS dimensions are validated before indexing. Worked
  examples: `backsolve(m, c(4,11), upper.tri = FALSE)` equals `forwardsolve(m)`;
  `backsolve(matrix(c(2,0,1,3), nrow=2), c(4,11), transpose = TRUE)` solves the
  lower-triangular `t(R) = [[2,0],[1,3]]` ⇒ `c(2,3)`. Exotic full
  cross-products of all three options with a *wide* multi-column matrix RHS are
  **deferred to a later linear-algebra item** (R-43 itself is now `norm()`); R-42
  ships each option independently plus the common combinations.
- **Matrix norms** *(R-43)*: `norm(x, type = "O")` reduces a numeric matrix to a
  single non-negative "size". `type` is a one-letter, **case-insensitive** string
  selecting the norm: **`"O"`/`"1"`** the one-norm (max absolute *column* sum,
  R's default), **`"I"`** the infinity-norm (max absolute *row* sum),
  **`"F"`/`"E"`** the Frobenius/Euclidean norm (`sqrt(Σ x[i,j]²)`), **`"M"`** the
  max-modulus (max `|x[i,j]|`). A plain numeric **vector** is treated as an `n×1`
  matrix, so `norm(c(3,4), "F") == 5`. **Reuse:** dims+data come from the shared
  **`matrix_parts`** helper (column-major `(data, nrow, ncol)` — *not*
  `square_matrix`, since norms apply to rectangular matrices); the vector case
  promotes through `as_double`; `type =` is read as a named-or-positional string
  (`as_character`); the result is a `SValue::scalar`. **Safety:** any `NA` entry
  ⇒ `NA`; an **unknown `type`** is a clean error (never a panic); an empty matrix
  does not panic (reductions start from `0`); the Frobenius sum-of-squares
  accumulates in `f64` (no integer overflow). Worked examples (column-major):
  `norm(matrix(c(1,2,3,4), nrow=2))` is `7` (one-norm), `…, "I")` is `6`,
  `…, "F")` is `sqrt(30) ≈ 5.477`, `norm(matrix(c(1,-5,3,4), nrow=2), "M")` is `5`.
  **`type = "2"`** (the spectral norm = largest singular value) needs an SVD and
  is **deferred to R-48**; for now it returns a clear *"norm type '2' (spectral)
  not yet supported"* error rather than a wrong number.
- **Cholesky factorization** *(R-40)*: `chol(X)`, the Cholesky factor of a real
  symmetric positive-definite `n×n` matrix. Returns the **upper-triangular**
  matrix `R` with **`t(R) %*% R == X`** (R's convention — the upper factor, so
  `R'R = X`). The Cholesky–Banachiewicz recurrence walks columns `i = 1..n`:
  `R[i,i] = sqrt(X[i,i] − Σ_{k<i} R[k,i]²)` and, for `j > i`,
  `R[i,j] = (X[i,j] − Σ_{k<i} R[k,i]·R[k,j]) / R[i,i]`; sub-diagonal entries are
  `0`. Only the **upper triangle** of `X` is read (matching R's default), so an
  asymmetric lower triangle is ignored. The implementation reuses the existing
  `square_matrix` helper (shared with `det`/`solve` — it rejects non-matrix,
  non-square and over-`MAX_SOLVE_DIM` inputs and hands back column-major data and
  the order `n`), indexes column-major (`X[i,j]` at `j·n + i`), and emits an
  `SValue::Matrix` directly. **Error paths, faithful to R:** non-square →
  error; `NA` in the upper triangle → error; and if the pivot
  `X[i,i] − Σ_{k<i} R[k,i]²` is `≤ 0` (or non-finite) the matrix is not
  positive-definite and `chol` errors with *"the leading minor of order i is not
  positive definite"*. The `≤ 0` check runs **before** the `sqrt`, so a non-SPD
  matrix is a clean error — never `sqrt` of a negative, never `NaN`, never a
  panic. Worked example: `chol(matrix(c(4,2,2,3), nrow=2))` is `[[2,1],[0,√2]]`
  and `t(R) %*% R` reconstructs the input; `chol(diag(3))` is the identity.
  `pivot=TRUE` (pivoted Cholesky), the `chol2inv()` companion, and complex
  (Hermitian) matrices are **deferred to R-42** (R-41 ships the triangular solves
  `backsolve`/`forwardsolve`); this item ships the real-SPD dense core only.
- **Kronecker product** *(R-38)*: `kronecker(X, Y)`, the block-outer-product of
  two matrices. For `X` `m×n` and `Y` `p×q` the result is `(m·p)×(n·q)` with
  **`result[(i-1)·p + k, (j-1)·q + l] = X[i, j] · Y[k, l]`** (1-based,
  column-major to match `SValue::Matrix`); equivalently each result cell `(r, c)`
  splits into an outer `X` index (`(r-1) div p`, `(c-1) div q`) and an inner `Y`
  index (`(r-1) mod p`, `(c-1) mod q`). The implementation reuses the existing
  `matrix_parts` accessor and emits an `SValue::Matrix` directly. Because the
  output is *quadratic* in the inputs, the result dimensions `m·p`, `n·q` and
  their product are formed with `checked_mul` and bounded by the same
  `MAX_SEQ_LEN` cap `matrix()`/`matrix_multiply` enforce, so a Kronecker of two
  moderately large matrices raises a clean "result too large" error rather than
  OOMing; `0×n`/`m×0` degenerate inputs yield an empty result with the correct
  zero dimension and never index out of bounds. A bare numeric vector promotes
  to an `n×1` column (the `matrix()` bare-column default). Worked example:
  `kronecker(matrix(c(1,2,3,4), nrow=2), matrix(c(0,1,1,0), nrow=2))` is the
  4×4 block matrix whose `(i, j)` block is `X[i,j]·[[0,1],[1,0]]`. The R `%x%`
  infix alias is **deferred to R-40** (grammar work); this item ships the
  `kronecker(X, Y)` function form only.
- **Apply family:** `sapply(x, f)` / `lapply(x, f)` map a function over elements
  (`sapply` simplifies length-1 atomic results to a vector).

### V2.4 S3 method dispatch

Single-dispatch on an object's class. `class(x)` returns the explicit class (set
by `structure(x, class=…)` or ``class<-``) or the implicit one (`"numeric"`,
`"character"`, `"factor"`, `"data.frame"`, `"function"`, …). `print` is a
**generic**: it dispatches on the first class to `print.<class>`, falling back to
`print.default`. Users define methods as ordinary functions: `print.myc <-
function(x) …`. The REPL's auto-print routes visible values through this generic.
`inherits(x, "cls")` and `unclass(x)` are provided. (S4/R5 remain out of scope.)

### V2.5 Factors

`factor(x, levels=, labels=)` stores integer codes + a `levels` character vector
(class `"factor"`). `levels`, `nlevels`, `as.character`, and `as.integer` are
provided; factors print as their labels followed by a `Levels:` line.
Arithmetic on a factor is an error, faithful to S. (`table` is deferred until
vectors carry a `names` attribute, which v2 does not yet add.) R-35 adds an
**ordered** flag to the factor value (`ordered()`/`as.ordered()`/`is.ordered()`,
level-index comparison, `cut(ordered_result=, dig.lab=)`) — see §9 item 9.

### V2.6 Data frames

`data.frame(name = column, …)` builds a list of equal-length columns (length-1
columns recycle; positional columns are auto-named `V1`, `V2`, …). Access:
`df$name`, `df[["name"]]` / `df[[i]]`, and 2-subscript `df[rows, cols]` (a single
selected column drops to a vector, otherwise a narrower data frame). `nrow`,
`ncol`, `names`, `dim`, `colnames`, and `head` are provided; data frames print as
an aligned table. (Empty-subscript forms `df[, c]` / `df[r, ]` are deferred —
they need an empty grammar argument, which v2 does not yet add; pass an explicit
row/column vector instead.)

### V2.7 Named vectors (the `names` attribute)

Atomic vectors may carry a *names* attribute — the R feature that the deferred
`table` (V2.5) was waiting on. A new transparent wrapper
`SValue::Named { names, values }` holds a `Vec<Option<String>>` (one slot per
element, `None` = an unset/`NA` name) beside a boxed atomic value. Like
`SValue::Classed` it is **see-through**: `length`, `type_name`, the coercions,
arithmetic, comparison, and `class` all delegate to the inner value, so names are
silently dropped exactly where R drops them (binary arithmetic, comparison, `c()`
of unnamed pieces) and preserved where R keeps them (positional/character
indexing, `rev` via index, the value as returned). Construction:
`c(a = 1, b = 2)` attaches argument names, and nested named pieces combine
R-style (`c(x = c(a = 1), 2)` → names `"x.a"`, `""`). `names(x)` returns the
character names vector (or `NULL`); `names(x) <- value` sets them with R's
NA-padding recycling (too-short pads with `NA`, `NULL` clears) via a general
**replacement-function** lvalue path (`f(x) <- v` ≡ ``x <- `f<-`(x, v)``);
`setNames(x, nm)` is the functional form. `x["b"]` / `x[c("a","c")]` select by
name (unmatched → `NA`). A named vector prints names above values in aligned
columns instead of the `[i]` prefix.

### V2.8 General attributes (`attr`, `attributes`, `structure`)

V2.7's `names` is *one* attribute; V2.8 adds the **general attribute system** —
the open key→value metadata map R attaches to any object. The *special*
attributes `names`, `class`, and `dim` keep their dedicated representations
(`SValue::Named`, `SValue::Classed`, `SValue::Matrix`); a new transparent wrapper
`SValue::Attributed { attrs: Vec<(String, SValue)>, inner }` stores every *other*
attribute as an insertion-ordered association list. Like `Named`/`Classed` it is
**see-through** (`length`, `type_name`, coercions, arithmetic, comparison,
`class`, indexing, printing all delegate to `inner`); only the attribute builtins
read the map.

Because the three special attributes are *never* duplicated into the general map,
the consistency invariant is structural: `attr(x, "names")` reads the same field
`names(x)` does, `attr(x, "class")` agrees with `class(x)`/`class_of`, and
`attr(x, "dim")` agrees with the matrix `dim` (R-11). `attr(x, which)` gets one
attribute (or `NULL`); `attr(x, which) <- value` sets/replaces it (`NULL`
removes) through the V2.7 replacement-function lvalue path (`\`attr<-\``).
`attributes(x)` returns all attributes as a named list (or `NULL`);
`attributes(x) <- list(...)` replaces them. `structure(x, ...)` attaches each
named `...` argument as an attribute, routing special names appropriately. The
general map is bounded (`MAX_ATTRIBUTES`) and a `"dim"` reshape is length-checked
against `MAX_SEQ_LEN`, so attacker-controlled attribute input cannot exhaust
memory or panic.

### V2.9 `switch()` and error handling (`stop`/`warning`/`tryCatch`)

V2.9 adds R's value-returning multi-way branch and its condition-based error
handling. The headline implementation note is **laziness**: `switch` and
`tryCatch` are **special forms**, not ordinary (eager) builtins. The evaluator's
call dispatcher (`eval_postfix`) intercepts a bare-name call to `switch` or
`tryCatch` and hands it the *unevaluated* argument expressions, evaluating only
the selected arm / the protected expression / the chosen handler. This is
essential: `switch("a", a = stop("no"), b = "ok")` must not raise, and a
`tryCatch` handler must not run unless its expression actually errors.

- **`switch(EXPR, ...)`** — choose one arm by the value of `EXPR`.
  - **Character `EXPR`** matches against the *names* of the arms. An **unnamed
    final arm** is the **default**, used when no name matches. With no match and
    no default the result is an **invisible `NULL`**. An **empty arm**
    (`a = ,` — a named arm with no value expression) **falls through** to the
    next non-empty arm's value: `switch("a", a = , b = "hit")` → `"hit"`, and the
    fall-through chains across several empties: `switch("a", a = , b = , c = "z")`
    → `"z"`. If only empty arms follow the match (the last matched arm is empty
    with nothing after it), the result is an invisible `NULL`. *(Implemented in
    R-19.* The shared S/R grammar previously had only `arg = NAME EQ expr`, with
    no empty-value production, so `a = ,` was a parse error. R-19 extends the
    grammar to `arg = NAME EQ [expr] | expr` — a named argument may omit its
    value — so an empty arm now parses as an `arg` node with a `NAME` and `=`
    token but no `expr` child. The evaluator's `eval_switch` already consumed
    such empty arms via `arm_body() == None`; the grammar change activates it.
    An empty value is accepted **generally** by the grammar but is only
    *meaningful* in `switch`: an empty arg in an ordinary call (e.g. `f(x = )`)
    is rejected at **eval time** with a parse-style error — `eval_arg`'s
    `only_node` requires a value node — matching R, which treats a missing
    argument value outside `switch`/indexing as an error.)*
  - **Numeric `EXPR`** selects the `n`-th arm by **position** (1-based), ignoring
    names; out of range (or `NA`/`< 1`) → `NULL`. Only the chosen arm is
    evaluated.
- **`stop(...)`** raises an error whose message is the concatenation of its
  arguments (coerced to character, like `cat`/`paste0`). It surfaces as a new
  typed error variant `SError::User(String)` so a `stop()` is distinguishable
  from an internal type/index error — but both are catchable by `tryCatch`.
- **`warning(...)`** emits a warning (message concatenated as for `stop`) into a
  session-held buffer and prints it as `Warning message:\n<msg>`; it does **not**
  abort and returns an **invisible `NULL`**.
- **`tryCatch(expr, error = handler, finally = cleanup)`** evaluates `expr`; if it
  raises *any* error, the `error` handler is called with a **condition object**
  and its value becomes the result; otherwise the value of `expr` is the result.
  The `finally` expression, if present, is evaluated for its side effects **after**
  `expr`/handler regardless of success or failure. The condition object is a
  minimal `list(message = <chr>, call = NULL)` with S3 class
  `c("simpleError", "error", "condition")`, so `conditionMessage(e)` and
  `e$message` both yield the message (full R condition machinery —
  `simpleCondition`, custom classes, `withCallingHandlers`, restarts — is out of
  scope). The internal `break`/`next` control signals are **not** caught (they are
  not user errors). Handler re-entry is bounded by the evaluator's existing
  `MAX_EVAL_DEPTH`, so a handler that re-raises cannot overflow the stack.
- **`conditionMessage(e)`** returns the `message` element of a condition object
  (the character message), for parity with R's accessor.

## §9 Divergences from ST00 (spec-sync)

1. **S before R.** ST00 specs R first; we implement historical S first. The two
   share grammar to a high degree, so a future R frontend can reuse most of
   `s-lexer`/`s-parser` with adjusted token/keyword sets.
2. **Tree-walking, not bytecode.** ST00's R03 lowers to InterpreterIR for the
   `vm-core`/JIT path. v1 S evaluates the parse tree directly. Bytecode/VM
   lowering and JIT specialization (ST00 Wave 10) are deferred.
3. **Substrate, not CAS.** S evaluation uses `r-vector` + `statistics-core`,
   **not** the symbolic-algebra stack (`symbolic-ir`/`symbolic-vm`/`cas-*`),
   which serves Macsyma. Only the *pluggable tree-walk pattern* is shared.
4. **v1 → v2 precedence correction.** v1 bound `:` looser than `+ - * /`; v2
   corrects the cascade to match R (`:` tighter than arithmetic) — see §V2.1.
   This changes the parse of expressions mixing `:` with arithmetic
   (e.g. `1:3+1` was `1:4`, is now `c(2,3,4)`).
5. **First-class environments (R-22, shared value).** The shared `SValue` gains
   an `Environment(Env)` variant boxing the scope handle (`Rc<RefCell<Scope>>`),
   so a scope can be a value: passed, stored, and mutated **by reference**. The
   surface (`new.env()`, `environment()`, `envir =`, `ls(envir=)`) is driven from
   the R frontend (R-22), but the variant and the `Scope::parent` link's change
   from strong `Rc` to **`Weak`** (to break the parent-edge cycle an
   environment-holding environment would otherwise form) live in `s-runtime` so S
   benefits too. The global env and each live call frame retain strong ownership
   (interpreter / native call stack), so parents are referenced but never owned by
   children — **no cycle through the parent chain is constructible**. A cycle
   *through a value binding* (e.g. `assign("self", e, envir = e)`) remains
   possible — a strong `Rc` stored inside its own scope, which `Rc` cannot reclaim
   without a tracing GC — and is a documented limitation bounded by a per-session
   `MAX_ENVIRONMENTS` cap rather than collected.
6. **Closure environments & call-frame reflection (R-23, shared value).** Builds
   on the R-22 variant. `s-runtime` gains a second long-lived interpreter handle —
   the **empty** environment (a parentless, bindingless root, owned for the whole
   session like the global env) — plus a closure's captured-env accessor and
   replacer, and a **caller environment** recorded alongside the R-20 call-stack
   closure so the caller's frame can be reflected. The S/R surface
   (`environment(f)`, `environment(f) <-`, `environmentName`,
   `globalenv()`/`emptyenv()`/`baseenv()`, `parent.frame()`, `is.environment()`)
   is driven from the R frontend (R-23) but lives in `s-runtime`. No new ownership
   risk: the caller env on the call stack is dropped when its frame is popped (RAII
   guard), and the captured-env exposure is the same strong-`Rc` situation as
   `environment()`, bounded by the same `MAX_ENVIRONMENTS` cap. `parent.frame(n)`
   **clamps** to the global env past the bottom of the stack rather than indexing
   out of bounds.
7. **Binning & cross-product utilities (R-32, shared builtins).** `s-runtime`
   gains two binning builtins that R (R-32) reuses verbatim through the shared
   tree-walker; both are pure functions over the existing `r-vector` `Double` and
   the existing `SValue::Factor` value, so no new value type or cap is added.
   - **`findInterval(x, vec)`** — for a **non-decreasing** breakpoint vector `vec`,
     the 1-based index of the last breakpoint not exceeding each `x` (`0` below the
     first, `length(vec)` at/above the last); `NA`/non-finite `x` → `NA`. A linear
     scan, `O(len(x)·len(vec))` with both lengths `MAX_SEQ_LEN`-bounded.
   - **`cut(x, breaks)`** — bins `x` into the `k-1` right-closed intervals `(lo,hi]`
     of the sorted `breaks` (length `k`), returning a real `SValue::Factor` whose
     `levels` are the auto-generated `"(lo,hi]"` labels; values outside all intervals
     (or `NA`) get a `NA` code. Built directly on `findInterval` (the interval index
     is the level code), so `levels()`/`as.integer()`/`as.character()`/`table()` all
     work on the result. The existing `tabulate(bin, nbins)` (R-28)
     rounds out the family (its `nbins` stays capped at `MAX_SEQ_LEN`).
8. **`cut()` option completeness (R-33, shared builtin).** The R-32 `cut` handler
   gains four options, all layered onto the same `find_interval_index` kernel and
   factor builder:
   - **`labels =`** — `labels = FALSE` returns the **integer bin codes** (a plain
     numeric vector, not a factor); a character vector of length `length(breaks)-1`
     is used verbatim as the levels (a length mismatch is a clean error); absent /
     `TRUE` keeps the auto-generated interval labels.
   - **`right = FALSE`** — left-closed `[lo,hi)` intervals (the scan counts breaks
     strictly `< x` instead of `<= x`); auto-labels become `"[lo,hi)"`.
   - **`include.lowest = TRUE`** — folds the extreme boundary (the lowest break for
     `right = TRUE`, the highest for `right = FALSE`) into the adjacent interval so
     `x == breaks[1]` (resp. `breaks[k]`) bins instead of going `NA`.
   - **integer `breaks` (a single number `N`)** — `N` equal-width bins over the
     range of `x`, with the range extended by `dx/1000` on each side
     (`dx = max-min`, degenerate `dx == 0` falling back to `abs(min)` then `1`).
     `N` is capped at `MAX_SEQ_LEN` before the break vector is built; the spacing
     uses finite/checked arithmetic so a degenerate range never divides by zero.
   `dig.lab=` and `ordered_result=` land in R-35 (item 9 below).
9. **Ordered factors & `cut()` label polish (R-35, shared builtin).** The
   `SValue::Factor` value gains a single boolean field —
   `Factor { codes, levels, ordered }` — so an *ordered* factor (one whose levels
   carry a meaningful order) needs no parallel value type. `ordered` defaults to
   `false`, so every prior factor is unchanged; when `true`, `class()` reports
   `c("ordered", "factor")` instead of `"factor"`.
   - **`ordered(x, levels=, labels=)`** — reuses the `factor` builder, then sets
     `ordered = true`. `factor(x, ordered = TRUE)` is an accepted synonym.
   - **`as.ordered(x)`** — coerce to an ordered factor (a factor keeps its
     codes/levels and flips the flag; any other vector is `factor`-encoded first).
   - **`is.ordered(x)`** — `TRUE` iff `x` is an ordered factor; `FALSE` otherwise
     (never errors).
   - **Ordered-factor comparison.** `<`, `<=`, `>`, `>=`, `==`, `!=` between two
     ordered factors compare by **level index** (the 1-based code), *not* by the
     label string. The `compare` kernel gains an early ordered-factor branch that
     runs before numeric/character coercion, comparing recycled codes numerically
     (an `NA` code → `NA`). Two ordered factors with **different level sets** is a
     clean error (`"level sets of factors are different"`).
   - **`cut(..., ordered_result = TRUE)`** — flips the returned factor's `ordered`
     flag (intervals are naturally ordered low→high). Default `FALSE`.
   - **`cut(..., dig.lab = k)`** — significant digits (default **3**) used when
     auto-generating the `"(lo,hi]"` break labels, via a `format_break_sig`
     helper. `dig.lab` is validated and **clamped to `1..=22`** so an extreme
     value can never drive an unbounded allocation or a formatter panic; a
     malformed value falls back to the default. A custom `labels =` vector still
     overrides auto-labels (so `dig.lab` is then ignored), as in R.
   - **Security.** Ordered comparison works on the integer `codes` (out-of-range
     or `NA` code → `NA`, never a panic) and rejects differing level sets before
     any compare; `dig.lab` is clamped before formatting. No new unbounded
     multiplier — the level vector is still bounded by the (`MAX_SEQ_LEN`-bounded)
     break count.
   - **Deferred to R-39.** S3 `Ops.ordered` group-generic *dispatch* and order
     statistics on ordered factors (`sort`/`max`/`min`/`range` by level order).
10. **String utilities (R-34, shared builtins).** `s-runtime` gains five base-R
   string builtins that R (R-34) reuses verbatim through the shared tree-walker.
   They build on the existing string machinery (`as_character` coercion, the
   `Option<String>` NA convention, `SValue::Character`/`SValue::Logical`), add no
   new value type, and operate on Unicode `char`s throughout (never raw byte
   indices), so multibyte UTF-8 input can never panic or split a code point.
   - **`startsWith(x, prefix)`** / **`endsWith(x, suffix)`** — a logical vector,
     `TRUE` where `x[i]` begins/ends with the (recycled) `prefix[i]`/`suffix[i]`.
     Both arguments are **recycled** to the longer length; `NA` in either position
     → `NA`. Implemented with `str::starts_with`/`ends_with` (code-point safe).
   - **`trimws(x, which = "both", whitespace = "[ \t\r\n]")`** — strip leading and/or
     trailing whitespace from each element; `which ∈ {"both","left","right"}` (second
     positional or `which =`), any other value a clean `Err`. `NA` → `NA`. The
     **`whitespace =`** argument (R-37) is a **regex** (default `"[ \t\r\n]"`),
     compiled with the same RE2-based `regex` engine `grepl`/`gsub` use and anchored
     to the matched edge (`^(?:p)+` / `(?:p)+$`); RE2's linear-time matching rules out
     ReDoS. `trimws("xxhixx", whitespace = "x")` → `"hi"`. An invalid pattern is a
     clean `Err`.
   - **`chartr(old, new, x)`** — translate each char of `x` found at position *i* of
     `old` to position *i* of `new`. `old`/`new` are length-one and must have equal
     `nchar`, else an `Err`. Vectorized over `x`; `NA` → `NA`.
   - **`strtoi(x, base = 10L)`** — parse each string as an integer in `base` (2..36
     *or* the special `0`, second positional or `base =`), returning a `Double` vector
     (`NA` for unparseable). Follows C `strtol`: leading whitespace and an optional
     sign are honored, base 16 accepts an `0x`/`0X` prefix, the whole remaining string
     must be consumed (trailing garbage → `NA`), an empty string → `NA`, an
     out-of-range digit → `NA`, and a `base` outside `{0} ∪ 2..36` makes every element
     `NA`. Parsing uses checked `i64` accumulation (overflow → `NA`, never a panic).
     **`base = 0L` auto-detection (R-37):** the radix is inferred from each string's
     prefix — `0x`/`0X` → hex, a leading `0` followed by another digit → octal (so
     `"08"` → `NA`), a lone `"0"` → zero, otherwise decimal. `strtoi("0x1F", 0L)` →
     `31`; `strtoi("010", 0L)` → `8`; `strtoi("12", 0L)` → `12`.
11. **Base R Date support (R-44, shared builtins).** `s-runtime` gains the base R
   calendar type and its builtins, which R (R-44) reuses verbatim through the
   shared tree-walker. A **`Date`** introduces **no new `SValue` variant**: it is a
   numeric vector of **days since the Unix epoch `1970-01-01`** wrapped in the
   existing transparent `SValue::Classed { inner: Double, class: ["Date"] }`
   (the same wrapper explicit S3 classes already use). Because `Classed` is
   see-through, every coercion (`as_double`/`as_character`/`as_logical`) and the
   `arithmetic` kernel already reach the day count with no special case.
   - **The civil-date kernel.** Two pure, dependency-free helpers implement the
     proleptic Gregorian calendar with Howard Hinnant's algorithms:
     `days_from_civil(y, m, d)` → days since the epoch (era / year-of-era /
     day-of-era decomposition; handles leap years and negative/pre-epoch dates)
     and its exact inverse `civil_from_days(z)` → `(y, m, d)`. Round-trips are
     unit-tested across leap days (`2000-02-29`, `2020-02-29`) and pre-epoch dates
     (`1969-12-31` → `-1`).
   - **`as.Date(x, format =)`** — parse a character vector (default `"%Y-%m-%d"`,
     or `"%Y/%m/%d"` etc. via `format =`, recognising `%Y`/`%m`/`%d` + literals),
     or wrap a numeric vector as days-since-epoch directly. Malformed / out-of-range
     input → `NA`. Vectorised.
   - **`format.Date(d, format =)` / `format(d, fmt)`** — render a `Date` to a
     string; default `"%Y-%m-%d"`, fields `%Y`/`%m`/`%d`/`%j` (day-of-year). The
     shared `format` builtin detects the `"Date"` class and dispatches here.
   - **`Sys.Date()`** — today as a length-1 `Date`. There is no pre-existing
     deterministic clock in the runtime, so this reads `SystemTime::now()` and
     converts to whole days since `UNIX_EPOCH` (pre-epoch handled without panic).
     Non-deterministic, so tested only for structure (class + single numeric).
   - **`difftime(d1, d2)` / `d1 - d2`** — difference in **days** (numeric).
     Subtraction needs no special case (the `Classed` wrapper is transparent to
     `arithmetic`). `as.numeric(d)` returns the raw day count; **`as.numeric`** is
     added as a base coercion (drops the class, reusing `as_double`).
   - **`weekdays(d)`** — English weekday name; anchored on `1970-01-01 = Thursday`
     with `(days + 3).rem_euclid(7)` so pre-epoch (negative) counts never panic.
   - **Security.** Untrusted date strings parse with **bounded `i64`** digit
     accumulation (a digit cap rejects absurd years before the day arithmetic can
     overflow); malformed/over-range input → `NA`, never a panic. All weekday /
     day-of-year modulo uses `rem_euclid` (Rust `%` can go negative).
   - **Deferred to R-45.** Full `strptime`/`strftime` fields (`%B`/`%A`/`%H`/…);
     `POSIXct`/`POSIXlt` date-times & timezones; `seq.Date`; `months()`/`quarters()`;
     `difftime` units other than days.

12. **Date/time completeness (R-45, shared builtins).** Extends the R-44 Date
   builtins **in place** — same civil-date kernel, same `Date` class machinery,
   same parse-safety guards, no new dependency (English month/weekday name tables
   are hand-rolled `const` arrays).
   - **Extended `strftime` fields** in `format.Date`/`format`: `%B` (full month
     `"January"`..`"December"`), `%b` (abbrev `"Jan"`..`"Dec"`), `%A` (full
     weekday `"Monday"`..`"Sunday"`), `%a` (abbrev `"Mon"`..`"Sun"`), `%e`
     (space-padded day of month, width 2). Weekday reuses R-44's
     `(days + 4).rem_euclid(7)` Sunday-based index.
   - **Extended `strptime` fields** in `as.Date`: `%B`/`%b` parse month names
     **case-insensitively** (`"january"`/`"JAN"`/`"Jan"`); `%A`/`%a` parse and
     spell-check weekday names (consumed but not used to constrain the date, as in
     base R); `%e` parses an optionally space-padded day. So
     `as.Date("January 15, 2021", "%B %d, %Y")` and `as.Date("15 Jan 2021",
     "%d %b %Y")` parse correctly; a malformed name → `NA`, never a panic
     (name-matching scans a fixed, length-bounded table with ASCII case-folding).
   - **`seq.Date(from, to, by)`** (also reached via the `seq` generic when `from`
     is a `Date`): `by` is a number of days or a unit string
     `"day"`/`"week"`/`"month"`/`"year"` with an optional leading integer
     multiplier (`"2 weeks"`). Day/week step a fixed day count; month/year step
     the civil Y/M/D, **clamping** day-of-month to the target month length
     (`2021-01-31 + 1 month` → `2021-02-28`). `length.out =` supported as an
     alternative to `to`. **Output length is `MAX_SEQ_LEN`-bounded** with checked
     arithmetic before allocation.
   - **`months(d)`** → full month name (= `format(d, "%B")`); **`quarters(d)`** →
     `"Q1"`..`"Q4"`. Both vectorised, `NA`-preserving.
   - **Deferred to R-46.** `POSIXct`/`POSIXlt`; timezones; sub-day
     `%H`/`%M`/`%S`/`%p`; `%U`/`%W` week-of-year; locale (non-English) names;
     compound `"N units"` `by=` beyond a single leading integer multiplier.

13. **POSIXct date-times (R-46, shared builtins).** Adds the first *date-time*
   type on top of the R-44/R-45 calendar machinery, again **in place** and with
   **no new dependency**. A `POSIXct` is — exactly like `Date` — an ordinary
   numeric vector, but of **seconds since 1970-01-01 00:00:00 UTC**, carrying the
   two-element class `c("POSIXct", "POSIXt")` via the same transparent
   `SValue::Classed` wrapper. The key reuse: a POSIXct's **date part** is
   `seconds.div_euclid(86400)` (= the R-44 `Date` day count) and its **time part**
   is `seconds.rem_euclid(86400)` (intraday seconds → H/M/S), so the civil kernel
   (`days_from_civil`/`civil_from_days`), the English name tables, and the
   `format_date_days` `%`-field renderer are all reused **unchanged** — only the
   seconds↔(days, h, m, s) split is new.
   - **`as.POSIXct(x, tz = "UTC")`** — parse a character vector of
     `"YYYY-MM-DD HH:MM:SS"` (or `"YYYY-MM-DD"` → midnight) to a POSIXct, or wrap a
     numeric vector as raw seconds directly. The date half reuses R-44's
     `parse_date_str`; an optional ` HH:MM:SS` time half is parsed with H 0–23,
     M 0–59, S 0–60 (leap-second slot accepted). Malformed → `NA`, never a panic.
     Only `tz = "UTC"` honoured.
   - **`Sys.time()`** — current time as a length-1 POSIXct (reads the wall clock
     like `Sys.Date`; pre-epoch handled without panic). Structure-tested only.
   - **`format.POSIXct(x, format =)` / `format(x, fmt)`** — render to character.
     Default `"%Y-%m-%d %H:%M:%S"`. New fields `%H`/`%M`/`%S`; reuses every R-44/
     R-45 date field (`%Y %m %d %B %b %A %a %j %e`) by feeding the day count to
     `format_date_days`. The `format()` generic checks `"POSIXct"` before `"Date"`.
   - **POSIXct subtraction & `as.numeric`** — no special case: `as.numeric` peels
     the wrapper to raw seconds, and `t1 - t2` flows through `arithmetic("-", …)`,
     giving the difference **in seconds**.
   - **Parse-safety.** A new `MAX_POSIXCT_SECONDS` (≈ `MAX_DATE_DAYS * 86400`)
     bounds any parsed/supplied seconds *before* the civil kernel; the date half
     still passes through `MAX_DATE_DAYS`/`MAX_DATE_DIGITS`; H/M/S range-checked;
     the seconds→(days, intraday) split uses `div_euclid`/`rem_euclid` so negative
     (pre-epoch) seconds split without a negative index.
   - **Deferred to R-47.** Non-UTC timezones (and DST); `POSIXlt`; fractional
     seconds; `%z`/`%Z`; standalone `strptime`/`strftime`; `as.POSIXlt`.

## §10 References

Internal: [`ST00-r-stats-roadmap.md`](ST00-r-stats-roadmap.md),
[`LANG00-generic-language-pipeline.md`](LANG00-generic-language-pipeline.md),
[`dartmouth_basic_lexer.md`](dartmouth-basic-lexer.md) (frontend model),
`grammar-tools`, `r-vector` / `numeric-tower` / `statistics-core` crates.

External:

- R. A. Becker, J. M. Chambers, A. R. Wilks, *The New S Language* (1988) — the
  "Blue Book"; the dialect this spec targets.
- J. M. Chambers, *Programming with Data* (1998).
- R. A. Becker, *A Brief History of S* (1994) — the origin of the `_`
  assignment operator and the vector-first design.
