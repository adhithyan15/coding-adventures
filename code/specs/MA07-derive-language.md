# MA07 — Derive (a subset)

## Status

Active spec / roadmap for a **Derive** frontend — Wave 5 of the historical-math
roadmap ([HML00 §7](HML00-historical-math-languages-roadmap.md)), called out
there as "small and historically charming." Derive (Soft Warehouse, 1988,
successor to muMATH; later licensed to Texas Instruments and discontinued
2007 at version 6.1) is a symbolic CAS with an expression-oriented,
`:=`-assignment linear syntax — much closer in spirit to Macsyma/Wolfram
than to the array languages (APL/J). Like Wolfram, its surface syntax is
genuinely its own (uppercase-by-convention named functions called with
ordinary parentheses — `DIF(u, x)`, not `f[x]`; `:=` for both variable and
function definition; `[...]`/`[...;...]` for vectors/matrices) — so it needs
a real new frontend (lexer + parser) — but the *engine* underneath is the
same shared substrate every symbolic-family language in this repo already
drives: every Derive expression lowers to [`symbolic-ir`](../packages/rust/symbolic-ir)
and is evaluated by [`symbolic-vm`](../packages/rust/symbolic-vm), with
[`cas-pattern-matching`](../packages/rust/cas-pattern-matching)/
[`cas-simplify`](../packages/rust/cas-simplify) backing the calculus/algebra
built-ins.

## §1 Why Derive is "expressions and worksheets," not "everything is `f[x]`"

Unlike Wolfram (where even assignment and lists are just `Head[args]`
sugar), Derive's core mental model is closer to a scientific calculator with
memory: a *worksheet* (`.mth` file) is a sequence of **expressions** —
ordinary algebraic expressions (`2 + 3*x`), variable/function definitions
(`x := 5`, `F(x) := x^2 + 1`), and named-function calls that are themselves
just ordinary expressions (`DIF(SIN(x), x)`) — each one entered, then
*simplified* or otherwise transformed (differentiate, integrate, solve) as
its own step. There is no `f[x]`-style universal application syntax and no
pattern/rewrite-rule vocabulary (`_`, `->`, `/.`) the way Wolfram has one —
Derive's "transform this expression" operations (`DIF`, `INT`, `LIM`,
`SOLVE`, `TAYLOR`, …) are each their own named function, called with
ordinary parentheses, not a generic rewrite engine exposed at the surface.
This maps directly onto `symbolic-ir`'s `IRNode::Apply { head, args }` the
same way Macsyma/Wolfram do — parsing Derive is desugaring its infix
operators, `:=` assignment, and vector/matrix literals into the canonical
heads (`Plus`/`Times`/`Power`/`List`/`Assign`/`Define`/…) `symbolic-vm`
already rewrites — but there is no analogue of Wolfram's W-19/W-20 pattern
items to port: Derive's built-ins are ordinary functions with fixed
argument shapes, not a general pattern matcher exposed to the user.

## §2 The pieces (one item = one PR)

Following [HML00 §6](HML00-historical-math-languages-roadmap.md)'s
breakdown, mirroring [MA06](MA06-j-language.md)'s finer-grained MA-6a
(spec) / MA-6b (tokens + lexer) / MA-6c (grammar + parser) / MA-6d
(runtime + repl + binary) split rather than Wolfram's coarser W-1
(spec + grammar combined) — this PR is a **design-only** kickoff, with
no grammar files yet:

- **D-1 — this spec** *(this PR)*. Fixes language scope (§4) and the
  surface grammar shape (§3) the next items implement against; no
  `code/grammars/` files, no crate, yet.
- **D-2 — `derive.tokens` + `derive-lexer`.** Authored in the grammar-tools
  format and validated with `grammar-tools validate`; the committed
  `_grammar.rs` compiled from `derive.tokens`, a sibling of
  `wolfram-lexer`/`macsyma-lexer`.
- **D-3 — `derive.grammar` + `derive-parser`.** The committed `_grammar.rs`
  compiled from `derive.grammar`, over the generic `parser::GrammarParser`,
  with an explicit `MAX_RULE_DEPTH` measured the same way `apl-parser`/
  `j-parser` measured theirs (per [MA06](MA06-j-language.md) §6's
  precedent) rather than assumed.
- **D-4 — `derive-runtime` + `derive-repl`.** Lowers the parsed
  `GrammarASTNode` into `symbolic-ir`, evaluates with `symbolic-vm`'s shared
  `SymbolicBackend`, and wires the §3-specified named built-ins (`DIF`, `INT`, `LIM`,
  `SUM`, `SOLVE`, `IF`, …) as thin calls into the same handler-table pattern
  Wolfram's W-4/W-5/W-22 already use — each one, where the semantics match,
  calling the *exact same* `cas-*` function its Macsyma/Wolfram counterpart
  calls (`DIF` → the same differentiation `cas-*` already implements for
  `D`/`DIF`-under-Macsyma-names, `INT` → the same integration engine, `SOLVE`
  → the same solver), so all three languages agree on every result these
  crates can produce. Plus the interactive `derive-repl` (`#n: ` numbered
  input, mirroring Derive's own numbered expression history, and the
  `derive` binary).
- **D-5 — vectors/matrices as structural `List` data.** `[a, b, c]` and
  `[a, b, c; d, e, f]` lower to nested `List` heads (mirroring Wolfram's
  `{a, b}` → `List[a, b]`), giving programs a way to *hold* vector/matrix
  *data* immediately. Actual linear-algebra evaluation (matrix multiply,
  determinant, `#5.4`) is a separate, later item — out of scope for D-5,
  which only needs the literal syntax and structural representation.

## §3 The supported surface (the grammar)

This spec's grammar (implemented by D-2/D-3) captures this subset of Derive syntax, verified against the
Derive 6.1 online help (the last released version; most of its content
applies to 5.x too). Everything is desugared to a `head(args)`-shaped
`IRNode::Apply` (right column) in D-4.

| Surface | Meaning | Lowers to |
|---------|---------|-----------|
| `123`, `1.5` | integer / real literal | `Integer` / `Float` |
| `1/3` | exact rational (division of two integer literals stays unreduced-to-float) | `Times[1, Power[3, -1]]` (D-4 recognizes and folds to `Rational`) |
| `SIN`, `x`, `foo` | symbol (built-ins conventionally, but not enforced, uppercase) | `Symbol` |
| `F(a, b)` | function/named-built-in call (ordinary parentheses) | `F[a, b]` |
| `[a, b, c]` | vector | `List[a, b, c]` (D-5) |
| `[a, b, c; d, e, f]` | matrix (rows separated by `;`) | `List[List[a, b, c], List[d, e, f]]` (D-5) |
| `a + b`, `a - b` | additive | `Plus` / `Subtract` |
| `a * b` | multiply *(explicit `*` required — see below)* | `Times` |
| `a / b` | divide | `Times[a, Power[b, -1]]` |
| `a ^ b` | power (right-assoc: `4^3^2` is `4^(3^2)`) | `Power` |
| `-a` | negation | `Times[-1, a]` |
| `a = b` | equation (a Boolean-valued expression, e.g. as `SOLVE`'s first argument) | `Equal` |
| `a <= b`, `<`, `>`, `>=` | comparison | `LessEqual`/`Less`/`Greater`/`GreaterEqual` |
| `a AND b`, `a OR b`, `NOT a` | logic (Derive's boolean-algebra keywords, not symbols) | `And` / `Or` / `Not` |
| `x := e` | variable assignment | `Assign[x, e]` |
| `F(x) := e` | function definition | `Define[F, [x], e]` |
| `( … )` | grouping | — |

**Precedence**, loosest → tightest: assignment (`:=`) → `OR` → `AND` → `NOT`
→ comparison (`=`/`<=`/…) → additive → multiplicative → unary minus →
`Power` → function application `F(…)` / vector-literal `[…]` → atoms.

Comments and the `.mth` worksheet-file format's other authoring conveniences
(named utility-function files, `Declare`-style domain annotations) are not
part of this grammar; see §4.

## §4 Honest scope — what is *out* (for now)

This is a clearly-scoped subset (per HML00 §9 and as S00/R00/MA03/MA04 do).
This spec's grammar deliberately omits, to be added later if warranted:

- **Implicit multiplication by juxtaposition.** Real Derive reads `2x` and
  `2(3+5)` as multiplication with no operator at all — confirmed directly
  against the Derive 6.1 help (`IF(h <= 40, 10h, ...)`, `2(3+5)`). Exactly
  like [MA04](MA04-wolfram-language.md) §4's identical call for Wolfram's
  `2 x`, this is genuinely hard in a context-free grammar without heavy
  lookahead/ambiguity cost, so this subset **requires an explicit `*`**
  the same way the Wolfram subset does. (This is a scope-narrowing
  decision this repo is making, not a claim that real Derive lacked
  juxtaposition — it clearly had it.)
- **`%` (percent), one-sided `LIM(u, x, a, dir)`, multi-variable
  `LIM(u, [x,y], [a,b])`, higher-order `DIF(u, x, n)`/antiderivative
  `DIF(u, x, -n)` forms.** The base one/two/three-argument `DIF`/`INT`/`LIM`
  forms are D-4; these richer forms are later items, added once the base
  forms are proven out.
- **Prime notation (`F'(x)`, `F''(x)`) for derivatives of user-defined
  functions.** A real, documented Derive convenience, but sugar over `DIF`
  — deferred; write `DIF(F(x), x)` in this subset for now.
- **`SUM`/`PRODUCT`, `TAYLOR`, `SOLVE`/`SOLUTIONS`, `NSOLVE`/`NSOLUTIONS`,
  `ITERATE`/`ITERATES`.** Confirmed real Derive functions with confirmed
  syntax (`SUM(expr, var, start, end)`, `TAYLOR(expr, var, point, order)`,
  `SOLVE(equation, var)`, `ITERATE(expr, var, init)`) — deferred to their
  own D-4-follow-on items rather than landing all at once, matching how
  Wolfram's W-6 through W-22 items each landed the built-in surface
  incrementally rather than in one PR.
- **Matrix/vector *algebra*** (multiply, determinant, inverse, eigenvalues).
  D-5 only gives vectors/matrices a structural `List` representation and
  literal syntax; evaluating linear-algebra operations on them is separate,
  later work (and may reuse `array-runtime`/`matrix-runtime` rather than
  `symbolic-vm`, since that's this repo's established substrate for actual
  matrix numerics — a decision for that later item, not this one).
- **Comments and worksheet-level conveniences** (`Declare`, imported
  utility files, piecewise-function helper macros). Not part of this
  grammar; this subset's programs are a flat sequence of expressions and
  `:=` definitions, matching how MA03/MA04's own subsets start minimal.
- **Full boolean algebra beyond `AND`/`OR`/`NOT`** (`XOR`, quantifiers).
  `AND`/`OR`/`NOT` cover this subset; anything richer is a later item.

These are surface-syntax gaps only where the *engine* (rewrite +
differentiation + integration + solving, all already implemented for
Macsyma/Wolfram) already supports the corresponding operation — each is a
grammar/lexer/wiring addition in a later item, not an engine change.

## §5 Reuse strategy

- **Frontend:** the grammar-tools framework, exactly as Macsyma/MATLAB/
  Wolfram/APL/J use it. `derive.tokens`/`derive.grammar` compile to
  committed `_grammar.rs` in `derive-lexer`/`derive-parser` (D-2/D-3).
- **Lowering + engine (D-4):** the parsed tree lowers to
  [`symbolic_ir::IRNode`](../packages/rust/symbolic-ir) (surface operators
  and `:=` → canonical `Plus`/`Times`/`Power`/`Assign`/`Define`/`List`
  heads), evaluated by [`symbolic_vm::VM`](../packages/rust/symbolic-vm)
  with a Derive `Backend` over the shared `build_handler_table` pattern —
  the same rewrite engine Macsyma and Wolfram already drive, unchanged.
  `DIF`/`INT`/`LIM`/`SOLVE` are thin calls into the same `cas-*` crates
  Wolfram's W-22 items call under Wolfram names, called here under Derive
  names — one function, three languages agreeing on its result.
- **REPL (D-4):** a single-threaded driver mirroring
  `wolfram-repl`/`maxima-repl`, with a numbered-history prompt matching
  Derive's own `#n:` worksheet convention.

## §6 References

Internal: [`HML00`](HML00-historical-math-languages-roadmap.md) (roadmap,
Wave 5), [`MA03`](MA03-maxima-language.md)/[`MA04`](MA04-wolfram-language.md)
(the two prior symbolic-family kickoffs this spec mirrors most closely),
`symbolic-ir`, `symbolic-vm`, `cas-pattern-matching`, `cas-simplify`.

External: Stoutemyer & Rich, *Derive* (Soft Warehouse, 1988, successor to
muMATH); the Derive 6.1 online help (Texas Instruments, last released
version — most content applies to 5.x too), specifically its
Differentiate/Integrate/Limit/IF-Expressions/Matrix/Programming-in-DERIVE/
Solving-Equations-Numerically pages, consulted directly to verify every
syntax claim in §3/§4 rather than relying on general CAS-family assumption.
