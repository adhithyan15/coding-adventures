# R00 — The R Language

## Status

Active spec / roadmap for the **R** language frontend. R is "an implementation
of the S language" (Ihaka & Gentleman, 1993), so this frontend is built as a
sibling of the historical-S stack (`s-lexer`/`s-parser`/`s-runtime`/`s-repl`,
specced in [`S00-s-language.md`](S00-s-language.md)) and **reuses it heavily**.
This document defines what we support, the S↔R differences that matter, and the
item-by-item rollout.

## §1 Why R, on top of S

The S00 work already built a vector-first interpreter — recycling, NA
propagation, the coercion lattice, closures with lexical scope, S3 dispatch,
factors, and data frames — driven by a grammar-defined frontend over the shared
`r-vector` / `statistics-core` substrate. R shares all of that semantics and
~98% of S's grammar. So the R frontend is mostly: a token/parser grammar that
captures R's lexical and syntactic differences, plus thin crates that reuse the
S evaluator. We get an R REPL for a fraction of the cost of S because the hard
part (the evaluator and value model) already exists.

This also realizes the plan recorded in
[`ST00-r-stats-roadmap.md`](ST00-r-stats-roadmap.md), which always intended R to
sit on the generic language pipeline; S00 §9 noted "a future R frontend can
reuse most of `s-lexer`/`s-parser` with adjusted token/keyword sets."

## §2 The differences from S that matter

| Area | S | R |
|------|---|---|
| `_` | the **assignment** operator; never inside a name | an ordinary **name character** (`data_frame` is one name) — since R 1.9 (2004) |
| `=` | binds named call arguments only | also a top-level **assignment** operator |
| Right super-assign | — | `->>` |
| Typed `NA` | — | `NA_integer_`, `NA_real_`, `NA_character_` |
| Integer / complex / hex literals | — | `1L`, `1i`, `0x1F` *(later item)* |
| Pipe / lambda | — | `|>`, `\(x)` *(out of scope for now)* |

Everything else in S00 (vector semantics, operators incl. the v2 `%op%` infixes,
`$`/`[[`, control flow, function values, S3, factors, data frames) carries over
unchanged.

## §3 Rollout (one item = one PR)

- **R-1 — R00 spec + `r-lexer`** *(this PR)*. `code/grammars/r.tokens` (the S
  token grammar with `_` moved into `NAME`, no `UNDERSCORE`, plus `->>` and the
  `NA_*` constants) and the `r-lexer` crate (a sibling of `s-lexer`, reusing the
  identical bracket-interior newline hook).
- **R-2 — `r-parser`**. `code/grammars/r.grammar`, mirroring `s.grammar`'s rule
  names (so the S evaluator can consume the tree unchanged) and adding `=` as a
  top-level assignment operator.
- **R-3 — `r-runtime` + `r-repl` + the `R` binary**. A vertical slice to a
  working R REPL. The S runtime is refactored to expose evaluation of an
  externally-parsed tree; `r-runtime` parses with `r-parser` and evaluates with
  the shared `s-runtime`. `r-repl` mirrors `s-repl`.
- **R-4+** — R literal types (`L`/`i`/`0x`), the `NA_*` constants in the
  runtime, then R-specific built-ins and the `d/p/q/r` distribution family wired
  to `statistics-core`.

## §4 Reuse strategy

- **Lexer/parser:** the grammar-tools framework, exactly as S uses it. `r.tokens`
  / `r.grammar` compile to committed `_grammar.rs` in `r-lexer` / `r-parser`.
- **Runtime:** the `s-runtime` evaluator and `SValue` model are language-neutral
  — they walk a `GrammarASTNode` by rule name. By keeping `r.grammar`'s rule
  names identical to `s.grammar`'s, `r-runtime` can evaluate R programs through
  the same `Interpreter`. (R-3 adds the small public entry point for this.)
- **REPL:** `r-repl` mirrors `s-repl`'s single-threaded driver.

## §5 Out of scope (for now)

Pipes (`|>`) and backslash lambdas (`\(x)`); environments/`<<-` semantics beyond
the S subset; S4/R5/R6 OO; namespaces and `library()`; the C interface; graphics.
These layer on later, following ST00.

## §6 References

Internal: [`S00-s-language.md`](S00-s-language.md),
[`ST00-r-stats-roadmap.md`](ST00-r-stats-roadmap.md), `grammar-tools`,
`r-vector` / `statistics-core`.

External:

- R. Ihaka & R. Gentleman, *R: A Language for Data Analysis and Graphics*
  (J. Computational and Graphical Statistics, 1996).
- R Core Team, *The R Language Definition*.
