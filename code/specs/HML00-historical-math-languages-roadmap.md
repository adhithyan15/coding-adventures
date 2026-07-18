# HML00 — Historical Math Languages Roadmap (CAS & array languages)

## Status

Roadmap / master spec. Surveys the great historical mathematical programming
environments — the symbolic *computer algebra systems* and the numerical *array
languages* — and lays out an item-by-item plan to reconstruct each on this
repo's existing substrate. It is the math-language analogue of
[`ST00-r-stats-roadmap.md`](ST00-r-stats-roadmap.md), and it generalizes the
pattern proven by the S/R work ([`S00`](S00-s-language.md), [`R00`](R00-r-language.md))
and the Macsyma work.

## §1 Why — and the two principles

This repository's identity is **faithful historical reconstruction**: bringing
back the systems that shaped computing so a reader can learn from them in a
modern, literate codebase. Macsyma — the first great computer algebra system —
was an early instance. The goal here is to **keep going**: reconstruct the rest
of the historical math-software lineage, both the symbolic CAS branch and the
numerical/array branch.

Two principles guide every item below:

1. **Reuse the substrate (reconstruct the *language*, not the math).** The hard
   parts — symbolic simplification, equation solving, matrix algebra — already
   ship as reusable crates. Each new language is mostly a *frontend* (grammar +
   lexer + parser) plus a thin runtime that lowers to a shared evaluator. This
   is exactly how R was built on S in days rather than months.
2. **Use modern hardware when it helps.** A faithful *language* reconstruction
   should still run on a 2020s machine's strengths. Numerical/array languages
   lower their matrix operations to `matrix-ir` and let `matrix-runtime`'s
   cost-based planner dispatch each op to **CPU, CUDA, or Metal** automatically —
   so `A * B` in our MATLAB or APL runs on the GPU when the matrices are large
   enough to win, and on the CPU otherwise, with no language-level change. See
   §4.

## §2 The key taxonomy — two families, two substrates

"CAS" colloquially covers two families that are computationally very different
and therefore reuse different substrate:

| Family | What it is | Reuses | Reference frontend (already built) |
|--------|-----------|--------|-------------------------------------|
| **Symbolic CAS** | exact symbolic math: term rewriting, pattern matching, simplification, solving | `symbol-core`, `symbolic-ir`, `symbolic-vm`, the `cas-*` crates | **Macsyma** (`macsyma-lexer/parser/compiler/runtime`) |
| **Numerical / array** | matrix-first floating-point computing | `numeric-tower`, `matrix` + `matrix-ir` + `matrix-runtime`, `array-core`, `statistics-core` | **S / R** (`s-runtime`, `r-runtime`) |

The popular examples land on **opposite sides**: **Mathematica** is symbolic;
**MATLAB** and **Octave** are numerical array languages (Octave is a free MATLAB
clone — they share a language, the way R shares S's). Maple and Mathematica are
hybrids that do both; we implement the dominant side first and graft the other
on later.

## §3 The existing substrate this builds on

### Symbolic (CAS) substrate
- **`symbolic-ir`** — the immutable symbolic-expression tree (`IRSymbol`,
  `IRInteger`, `IRRational`, `IRApply`, …).
- **`symbolic-vm`** — a tree-walking evaluator with **pluggable backends** and a
  **rewrite-rule** engine. This is the engine Macsyma already runs on; new CAS
  frontends plug in the same way.
- **`cas-*` (≈20 crates)** — `cas-simplify`, `cas-solve`, `cas-factor`,
  `cas-trig`, `cas-complex`, `cas-pattern-matching`, `cas-substitution`,
  `cas-summation`, `cas-limit-series`, `cas-ode`, `cas-laplace`, `cas-fourier`,
  `cas-multivariate`, `cas-number-theory`, `cas-pretty-printer`, … — the
  algorithm library every CAS language exposes under its own names.
- **`macsyma-runtime`** — the reference: a complete CAS language on `symbolic-vm`.

### Numerical / array substrate
- **`numeric-tower`** — the Integer→Rational→Float→Complex coercion lattice.
- **`matrix-ir`** — a pure tensor-algebra IR (the upper IR of the matrix layer).
- **`matrix-runtime`** — the planner + executor registry: a **cost model**
  (FLOPs vs host↔device transfer) selects a backend per op; **CPU is the
  always-available fallback**, with `matrix-cpu`, `matrix-cuda`, and
  `matrix-metal` executors.
- **`array-core`**, **`data-matrix`**, **`statistics-core`**, **`r-vector`**,
  **`math-core`**, **`trig`**, **`dsp-*`** — vectors, frames, stats, signal ops.
- **`s-runtime` / `r-runtime`** — the reference: vector-first array languages,
  with `r-runtime` already demonstrating multi-frontend reuse of one evaluator.

## §4 GPU acceleration — by lowering, not by special-casing

Numerical/array runtimes do **not** hand-roll GPU code. They lower array
operations to **`matrix-ir`** and submit the graph to **`matrix-runtime`**, whose
planner already:

- estimates each op's cost on each registered executor (CPU/CUDA/Metal) from a
  FLOP + dtype + transfer-cost model;
- keeps data resident on a device across a chain of ops to amortize transfers
  (so `A*B + C` doesn't round-trip to host between ops);
- falls back to CPU when no accelerator is present or when the matrices are too
  small for the GPU to win after transfer overhead.

The contract for a new array language is therefore: **represent values as
tensors and emit `matrix-ir`; hardware dispatch is automatic.** A MATLAB `A \ b`
or an APL `+/⍳n` over a large array uses the GPU exactly when it pays off, with
zero language-specific GPU code. Scalar/small-array work stays on the CPU.

This also defines a substrate task (Wave 0): an **`array-runtime`** value model —
N-D arrays with broadcasting, column-major storage, `end`/logical/range indexing
— that lowers to `matrix-ir`. It is shared by *every* array-family frontend
(MATLAB, Octave, Scilab, APL, J, IDL), so it is built once.

## §5 Survey — the systems and their histories

### Symbolic CAS family (reuse `symbolic-vm` + `cas-*`; model on Macsyma)

| System | Origin | Creators | Notes for reconstruction |
|--------|--------|----------|--------------------------|
| **Macsyma** | MIT Project MAC, 1968–82 | Moses, Engelman, Martin | The first great CAS; Lisp-based. **Already built.** |
| **Maxima** | DOE Macsyma → GPL 1998 | maint. W. Schelter | Open Macsyma descendant — *essentially the same language*. The smallest possible delta (a Macsyma↔Maxima alias, like R↔S). |
| **Reduce** | 1968; OSS 2008 | Anthony Hearn | One of the oldest; Lisp-based; algebraic mode. |
| **muMATH / Derive** | 1979 / 1988 | Stoutemyer & Rich | Ran on tiny machines; Derive shipped on TI graphing calculators. Delightful historical target. |
| **Mathematica / Wolfram** | 1988 (after SMP, 1981) | Stephen Wolfram | Everything-is-an-expression; `f[x]` syntax; term rewriting + pattern matching (`/.`, `:>`) — a direct fit for `symbolic-vm` + `cas-pattern-matching`. The flagship symbolic. |
| **Maple** | 1980–82, U. Waterloo | Geddes & Gonnet | C kernel + library in the Maple language; procedural + symbolic. |
| **Axiom** | IBM Scratchpad, 1971; OSS 2002 | R. Jenks et al. | Strongly *typed* CAS (category/type system) — the hardest; later. |
| also | PARI/GP (1985, number theory), MuPAD (1989; later MATLAB's Symbolic Toolbox), GAP / Magma / Singular / Macaulay2 (group & commutative algebra), Yacas, Xcas | | domain-specific; opportunistic later items |

### Numerical / array family (reuse `matrix*` + `numeric-tower` + `statistics-core`; model on S/R)

| System | Origin | Creators | Notes for reconstruction |
|--------|--------|----------|--------------------------|
| **APL** | 1966 (Iverson notation, 1957–62) | Kenneth Iverson (IBM) | The array-programming root; special glyph charset; reduce/scan/inner/outer products. Iconic; main new work is a Unicode-glyph lexer. |
| **MATLAB** | late 1970s; MathWorks 1984 | Cleve Moler | "Matrix Laboratory"; a wrapper over LINPACK/EISPACK. Matrix-first: `[1 2;3 4]`, `A\b`, `A'`, `1:10`, `end`, command syntax. The flagship numeric. |
| **Octave** | 1988/1992 | John W. Eaton | GNU, MATLAB-compatible — *shares MATLAB's language*. A thin second frontend over the same evaluator (the R↔S move). |
| **Scilab** | 1990, INRIA | | MATLAB-like with syntax differences. |
| **J** | 1990 | Iverson & Hui | ASCII APL; tacit (point-free) programming. |
| **K / Q / kdb+** | 1993 | Arthur Whitney | Terse APL-family; finance. |
| **IDL** | 1977 | David Stern | Array language for science/astronomy. |
| **Julia** (subset) | 2012 | Bezanson, Karpinski, Shah, Edelman | Modern; multiple dispatch + JIT — the one genuine research lift. |

(NumPy / SymPy / Pandas are *libraries*, not languages — out of scope as
"languages," though useful as builtin references.)

## §6 The per-language pattern

Identical to the S/R and Macsyma builds; proven 6× already this cycle:

```
spec  →  code/grammars/<lang>.tokens + <lang>.grammar   (validated, compiled to _grammar.rs)
      →  <lang>-lexer   (wraps lexer::GrammarLexer)
      →  <lang>-parser  (wraps parser::GrammarParser; rule names matched to a
                         reference grammar so a shared evaluator can be reused)
      →  <lang>-runtime (lowers to symbolic-vm + cas-*   OR   matrix-ir + array-runtime)
      →  <lang>-repl + binary
```

Two reuse shortcuts already demonstrated:
- **Clone languages share a frontend + evaluator.** Octave reuses MATLAB's; a
  Maxima frontend reuses Macsyma's runtime. (Exactly R reusing S's `eval_program`.)
- **The runtime is the thin part.** It maps the language's names/operators onto
  the shared algorithm crates and the shared value model.

## §7 Implementation roadmap (waves)

Ordered by reuse leverage (cheapest, highest-impact first). Each wave is a
sequence of one-PR items run through the autonomous loop (§8).

- **Wave 0 — `array-runtime` substrate.** The N-D array value model + `matrix-ir`
  lowering + GPU dispatch (§4). Shared by the entire array family. *(Also extend
  S/R to lower their matrix ops here, so existing work benefits.)*
- **Wave 1 — Maxima.** ≈ Macsyma + GPL-era surface. Warm-up win; reuses the
  Macsyma frontend/runtime almost wholesale.
- **Wave 2 — MATLAB + Octave.** The flagship numeric pair. New matrix-first
  frontend on `array-runtime`; **GPU-accelerated via `matrix-runtime`**. Octave
  is a thin second frontend over the same evaluator.
- **Wave 3 — Mathematica / Wolfram (subset).** The flagship symbolic. New
  `f[x]` M-expression frontend + replacement/rule operators; engine reuses
  `symbolic-vm` term rewriting + `cas-pattern-matching` + `cas-simplify`.
- **Wave 4 — APL.** Iconic historical array language; glyph-token lexer + array
  primitives (reduce/scan/outer) on `array-runtime` (GPU-accelerated).
  *(Kickoff: see [`MA05`](MA05-apl-language.md) — the design item fixing
  language scope, the function/operator grammar shape, and a substrate gap
  (`array-runtime` needs generalized reduce/scan/outer-product kernels
  first, tracked as item AR-2) before the lexer/parser/runtime land.)*
- **Wave 5 — Reduce, Derive, Maple (subset).** More symbolic CAS on the shared
  engine (Derive is small and historically charming). *(Kickoff: see
  [`MA07`](MA07-derive-language.md) for Derive — the design item fixing
  language scope and the `:=`-assignment expression grammar, verified
  against the Derive 6.1 online help rather than assumed from the family
  resemblance to Macsyma/Wolfram, before the lexer/parser/runtime land.
  See [`MA08`](MA08-reduce-language.md) for Reduce — one of the two
  oldest CAS ever built (1968, alongside Macsyma), an Algol-surfaced
  algebraic-mode language over a Lisp engine, verified against the current
  REDUCE User's Manual. See [`MA09`](MA09-maple-language.md) for Maple —
  Wave 5's third and final language: three distinct aggregate types
  (expression sequences, lists, sets) sharing brackets that mean something
  different in every sibling CAS already in this repo, and an `f(x) := e`
  spelling that is *not* the general function definition Reduce's/Derive's
  own identical-looking idiom is (real Maple's own remember-table
  mechanism instead — the arrow operator `f := x -> e` is the general
  form) — verified against the current Maplesoft online Help system
  rather than assumed from family resemblance.)*
- **Wave 6 — J, K/Q, Scilab, IDL.** More array languages, each a frontend on
  `array-runtime`. *(Kickoff: see [`MA06`](MA06-j-language.md) for J — the
  design item fixing language scope, the verb/adverb/conjunction grammar
  (reusing APL's verb/noun split almost wholesale, plus one genuinely new
  production for tacit hook/fork trains) before any lexer/parser/runtime
  code lands. No substrate gap this time — array-runtime + AR-2, already
  built for APL, cover J's in-scope value model unchanged.)*
- **Wave 7 — Axiom, Julia (subset).** The research-grade lifts (typed CAS;
  multiple dispatch) — last, and only if warranted.

### Item breakdown for the first three waves (illustrative)

- **Maxima:** M-1 spec + grammar diff from Macsyma; M-2 lexer/parser; M-3 runtime
  alias over `macsyma-runtime`; M-4 GPL-era builtins. *(Delivered: see
  [MA03](MA03-maxima-language.md). The grammar diff is empty for the supported
  subset — Maxima parses identically to Macsyma — so M-1/M-2 collapsed and Maxima
  shipped as just the runtime alias + REPL, the symbolic-CAS analogue of
  Octave-over-MATLAB. No `maxima.tokens`/`maxima.grammar`.)*
- **MATLAB/Octave:** ML-1 spec + `array-runtime` (Wave 0); ML-2 `matlab.tokens/grammar`
  + lexer; ML-3 parser; ML-4 `matlab-runtime` (matrix literals, `\`/`'`/`end`,
  ranges, broadcasting → `matrix-ir`); ML-5 `matlab-repl` + binary; ML-6 Octave
  frontend reusing the evaluator; ML-7 toolbox builtins.
- **Wolfram:** W-1 spec + `wolfram.tokens/grammar` (M-expressions); W-2 lexer;
  W-3 parser; W-4 `wolfram-runtime` (rewrite rules + `/.`/`:>` over `symbolic-vm`);
  W-5 repl + binary; W-6 the `cas-*` function surface under Wolfram names.
  *(W-1 delivered: see [MA04](MA04-wolfram-language.md). Unlike Maxima, the
  syntax genuinely differs from Macsyma — `f[x]`/`{a,b}`/`/.`/`->`/`x_` — so this
  is a real new frontend over the shared `symbolic-vm`/`cas-*` engine, not a
  reuse of the Macsyma grammar.)*

## §8 Plugging into the autonomous loop

Each item runs the established cycle: a fresh git worktree off `origin/main`,
implement → `cargo test`/`fmt`/`clippy` → `/security-review` → PR → babysit CI to
green → on merge, auto-advance to the next item. The user merges asynchronously;
the loop never auto-merges. (This is the same machinery currently driving the R
items.)

## §9 Cross-cutting conventions

- **Literate programming**, per-package BUILD/README/CHANGELOG, >80% coverage,
  parity tests against the reference system's documented behavior — as elsewhere
  in the repo.
- **Honesty about subsets.** Each language ships a clearly-scoped subset with an
  explicit out-of-scope list (as S00/R00 do); we do not pretend to be the full
  product.
- **GPU is opt-in by cost, not by syntax.** No language exposes a "use GPU"
  keyword; the planner decides. Determinism and CPU-fallback parity are tested.

## §10 References

Internal: [`S00`](S00-s-language.md), [`R00`](R00-r-language.md),
[`ST00`](ST00-r-stats-roadmap.md), `macsyma-runtime.md`, `cas.md`,
[`MX00`](MX00-matrix-execution-overview.md), [`MX01`](MX01-matrix-ir.md),
`symbolic-computation.md`, `grammar-tools.md`.

External (histories): Iverson, *A Programming Language* (1962); Moler, *MATLAB*
origins; Eaton, GNU Octave; Wolfram, *Mathematica* / SMP; Moses & Martin,
Macsyma; Hearn, Reduce; Geddes & Gonnet, Maple; Jenks & Sutor, *Axiom*;
Bezanson et al., *Julia* (2017, SIAM Review).
