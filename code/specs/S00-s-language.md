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
`<<-` superassignment; user-defined `%op%` infix operators; complex numbers and
the integer-`L` suffix; negative/logical/named indexing and `[[ ]]`/`$`;
attributes beyond `names`; graphics. The numeric/statistical depth lives in
ST02–ST12 and `statistics-core`; this spec is only the language frontend.

## §3 Lexical structure (`code/grammars/s.tokens`)

| Token | Pattern / value | Notes |
|-------|-----------------|-------|
| `ASSIGN` | `<-` | primary assignment |
| `SUPER_ASSIGN` | `<<-` | lexed (reserved); v1 treats as `<-` |
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
`src/_grammar.rs` (regenerated with `scripts/generate-compiled-grammars.sh`),
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
vectors carry a `names` attribute, which v2 does not yet add.)

### V2.6 Data frames

`data.frame(name = column, …)` builds a list of equal-length columns (length-1
columns recycle; positional columns are auto-named `V1`, `V2`, …). Access:
`df$name`, `df[["name"]]` / `df[[i]]`, and 2-subscript `df[rows, cols]` (a single
selected column drops to a vector, otherwise a narrower data frame). `nrow`,
`ncol`, `names`, `dim`, `colnames`, and `head` are provided; data frames print as
an aligned table. (Empty-subscript forms `df[, c]` / `df[r, ]` are deferred —
they need an empty grammar argument, which v2 does not yet add; pass an explicit
row/column vector instead.)

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
