# MA08 — Reduce (a subset)

## Status

Active spec / roadmap for a **Reduce** frontend — Wave 5 of the
historical-math roadmap ([HML00 §7](HML00-historical-math-languages-roadmap.md)),
named there alongside Derive and Maple as "more symbolic CAS on the shared
engine," with Reduce and Maple explicitly called out as "remain[ing]
unstarted" once [`MA07`](MA07-derive-language.md) (Derive) landed. Reduce
(Anthony C. Hearn, Stanford/Rand Corporation, 1968; open-sourced 2008;
still actively maintained) is **one of the two oldest CAS ever built**
(alongside Macsyma, both 1968) and, unlike Derive or Wolfram, is built
*directly* on top of Lisp's own Algol-like "algebraic mode" surface syntax
— assignment (`:=`), `if`/`while`/`for` statements, and `procedure`
definitions read like a 1970s Algol/Pascal-family language, not a
Lisp-style parenthesized one, even though the underlying evaluator is a
Standard Lisp (later PSL/CSL) system throughout. Every claim in §3/§4
below was checked directly against the current REDUCE User's Manual
(Hearn & Schöpf, free version, 2025/2026 build — see §6) rather than
assumed from family resemblance to Macsyma/Derive/Wolfram.

## §1 Why Reduce is "Algol surface, Lisp engine, algebraic-mode expressions"

Reduce's "algebraic mode" (the mode every interactive session and this
subset both use — REDUCE also has a raw "symbolic mode" that is direct
Lisp, out of scope entirely) reads, on the surface, like an ordinary
imperative language: statements are separated by `;` or `$`, `x := 5`
assigns, `if x=5 then a:=b+c else d:=e+f` branches, and multi-statement
bodies are wrapped `<< stmt; stmt; ... >>` (REDUCE's own token for what
Wolfram spells `CompoundExpression[...]` and Algol spells
`begin ... end`). But every *expression* — the right-hand side of an
assignment, an `if`'s branches, an operator's arguments — is an ordinary
algebraic expression built from `+ - * / ^` and parenthesized operator
calls (`log(y/m)`, `df(x,z)`), exactly the same `IRNode::Apply
{ head, args }` shape Macsyma/Maxima/Wolfram/Derive all already lower to.
So — like Derive — Reduce needs a real new frontend (its surface syntax,
statement keywords, and precedence table are its own, not reused from any
other language in this repo) but the *engine* underneath is the same
shared substrate: every Reduce expression lowers to
[`symbolic-ir`](../packages/rust/symbolic-ir) and is evaluated by
[`symbolic-vm`](../packages/rust/symbolic-vm), with
[`cas-pattern-matching`](../packages/rust/cas-pattern-matching)/
[`cas-simplify`](../packages/rust/cas-simplify) backing the calculus/algebra
built-ins — precisely the reuse story [`MA07`](MA07-derive-language.md) §1
already tells for Derive.

What's genuinely new relative to Derive: Reduce's algebraic mode is a
*statement* language, not an *expression-and-worksheet* one — it has
real `if`/statement-sequencing control flow (Derive's `IF` docs describe
a value-returning conditional *expression*, not a statement), and its own
native list type (`{a,b,c}`, curly braces — not Derive's `[a,b,c]` square
brackets, and not APL/J/MATLAB array syntax either) with `first`/`rest`/
`part`/`append`/`reverse`/cons (`.`) operators, predating and structurally
foreshadowing Wolfram's `{a,b}` list literal by two decades.

## §2 The pieces (one item = one PR)

Following [HML00 §6](HML00-historical-math-languages-roadmap.md)'s
breakdown, mirroring [MA06](MA06-j-language.md)'s four-part split
(spec / tokens+lexer / grammar+parser / runtime+repl+binary) rather than
[MA07](MA07-derive-language.md)'s five-part one — Reduce's list literals
are simple enough (unlike Derive's separate row/column matrix-literal
syntax) to fold into the base runtime item rather than needing their own
follow-on. This PR is a **design-only** kickoff, with no grammar files yet:

- **R-1 — this spec** *(this PR)*. Fixes language scope (§4) and the
  surface grammar shape (§3) the next items implement against; no
  `code/grammars/` files, no crate, yet.
- **R-2 — `reduce.tokens` + `reduce-lexer`.** Authored in the grammar-tools
  format and validated with `grammar-tools validate`; the committed
  `_grammar.rs` compiled from `reduce.tokens`, a sibling of
  `derive-lexer`/`wolfram-lexer`/`macsyma-lexer`.
- **R-3 — `reduce.grammar` + `reduce-parser`.** The committed `_grammar.rs`
  compiled from `reduce.grammar`, over the generic `parser::GrammarParser`,
  with an explicit `MAX_RULE_DEPTH` measured the same way `apl-parser`/
  `j-parser`/`derive-parser` measured theirs, not assumed (per
  [MA06](MA06-j-language.md) §6's precedent).
- **R-4 — `reduce-runtime` + `reduce-repl`.** (✅ done) Lowers the parsed
  `GrammarASTNode` into `symbolic-ir`, evaluates with `symbolic-vm`'s shared
  `SymbolicBackend` — reused *unchanged*, with no custom `Backend` at all,
  the same reuse `derive-runtime` already demonstrated: R-4's in-scope
  surface (§3) needs nothing the shared handler table doesn't already
  provide for arithmetic, comparison, logic, and the held `Assign`/
  `Define`/`If` forms. Plus the interactive `reduce-repl` (Reduce's own
  session transcript has no numbered-input convention the way Derive's
  `#n:` or Wolfram's `In[n]:=` do — a plain read-eval-print loop, no
  numbering) and the `reduce` binary.
  **Two things this line originally claimed turned out not to hold once
  R-4 actually grepped `symbolic-vm` rather than assuming family
  resemblance to Macsyma/Wolfram/Derive — see §3/§5's own corrected text
  and `reduce-runtime`'s CHANGELOG for the full accounting:** (1) `List`/
  `first`/`rest`/`append`/`reverse` are **not** "already implemented …
  the exact same functions" for the *shared* `SymbolicBackend` — only
  `List` has a handler there; Macsyma's list functions and Wolfram's
  `CompoundExpression` are each wired through a bespoke `Backend` specific
  to that language, which R-4 does not build, so `first`/`second`/`third`/
  `rest`/`part`/`append`/`reverse`/`Cons`/`CompoundExpression` all lower to
  the structurally-correct head but evaluate as an unresolved call (no
  crash, just no reduction) until a follow-on item wires real handlers.
  (2) the arithmetic heads are `Add`/`Sub`/`Mul`/`Div`/`Pow`/`Neg` (what
  `derive-runtime` already reuses), not `Plus`/`Subtract`/`Times`/`Power`
  (spellings that do not exist in `symbolic-ir` at all).

## §3 The supported surface (the grammar)

This spec's grammar (implemented by R-2/R-3) captures this subset of
Reduce syntax, verified directly against the REDUCE User's Manual's
"Structure of Programs" (ch. 2), "Expressions" (ch. 3), "Statements"
(ch. 5), and "Lists" (§4.1) chapters — see §6 for the exact pages
consulted. Everything is desugared to a `head(args)`-shaped
`IRNode::Apply` (right column) in R-4.

| Surface | Meaning | Lowers to |
|---------|---------|-----------|
| `123`, `1.5` | integer / real literal | `Integer` / `Float` |
| `x`, `foo`, `df` | identifier (symbol or, applied, an operator/procedure name) | `Symbol` |
| `f(a, b)` | operator/procedure call (ordinary parentheses) | `f[a, b]` |
| `a(5)`, `b(i, q)` | array-name-with-subscript *or* operator call — both spellings are the same `f(args)` production in the manual's own grammar (§3.1) | `f[a, b]` (array *declaration*/backing-store semantics are out of scope, §4 — this subset only supports the call-shaped read) |
| `{a, b, c}` | list literal (curly braces — **not** square brackets) | `List[a, b, c]` |
| `list(a, b, c)` | list literal, function-call spelling (equivalent to `{a,b,c}`) | `List[a, b, c]` |
| `a . {b, c}` | cons (prepend `a` onto list `{b,c}`) | `Cons[a, List[b, c]]` (R-4 folds a `Cons` onto a literal `List` immediately into one `List`) |
| `first(l)`, `second(l)`, `third(l)`, `rest(l)`, `part(l, n)` | list accessors | `First`/`Second`/`Third`/`Rest`/`Part` |
| `append(l1, l2)`, `reverse(l)` | list construction | `Append`/`Reverse` |
| `a + b`, `a - b` | additive | `Add` / `Sub` |
| `a * b` | multiply *(explicit `*` required — see §4)* | `Mul` |
| `a / b` | divide | `Div` |
| `a ^ b`, `a ** b` | power (`^` and `**` are the same operator — manual §2.7's own precedence table lists them as one tier) | `Pow` |
| `-a` | negation | `Neg` |
| `a = b` | equation (boolean-valued; distinct from `:=` — manual §3.4 confirms `=` never assigns, only `on evallhseqp` even evaluates its left side) | `Equal` |
| `a < b`, `a > b`, `a <= b`, `a >= b`, `a neq b` | relational | `Less`/`Greater`/`LessEqual`/`GreaterEqual`/`NotEqual` |
| `a and b`, `a or b`, `not a` | logical | `And` / `Or` / `Not` |
| `x := e` | assignment (a *statement*, per manual §5.1 — not a value-producing expression the way `=` inside a boolean is) | `Assign[x, e]` |
| `h(l, m) := e` | operator/procedure definition via assignment to a call form (manual §3.1's own example: `h(l,m) := x-2*y`, "where h is an operator") — the direct analogue of [Derive](MA07-derive-language.md)'s `F(x) := e` | `Define[h, [l, m], e]` |
| `if b then s1 else s2` | conditional statement (right-associative per manual §5.3; usable as an expression, returning whichever branch ran) | `If[b, s1, s2]` |
| `<< s1; s2; ... >>` | group statement (manual §5.2 — sequences statements where exactly one is expected; evaluates to its last statement's value) | `CompoundExpression[s1, s2, ...]` |
| `( … )` | grouping | — |

**Precedence**, loosest → tightest (manual §2.7's own `⟨infix operator⟩`
production, read left-to-right as lowest-to-highest — this subset omits
that production's `where` substitution-operator and `member`/`memq`/`eq`
list-membership/equality tiers, §4): assignment (`:=`) → `or` → `and` →
equation/comparison (`=`/`neq`/`>=`/`>`/`<=`/`<`, in that relative order
per the manual's own list, though the manual doesn't spell out whether
each is its own strict precedence level or several share one — this
subset treats the whole group as one flat, non-chaining comparison tier,
a deliberate simplification for a first cut, not a verified claim about
real Reduce's exact grammar) → additive (`+`/`-`) → multiplicative
(`*`/`/`) → `^`/`**` → function application `f(…)` / list-literal `{…}`
→ atoms. `:=` is right-associative (manual §2.7: "`a:=b:=c` evaluates as
`a:=(b:=c)`"); this subset's `if`/`else` is likewise right-associative
per §5.3.

Note this table (and the manual's own `⟨infix operator⟩` production it
transcribes) never places the cons operator `.` (§3, `a . {b, c}`)
anywhere in this chain — a genuine gap, not an omission of this spec's own
making. R-3's `reduce.grammar` resolves it by binding `cons` looser than
`additive` but tighter than the comparison tier: an already-fully-reduced
arithmetic expression on each side (`1+2 . {3,4}` means `(1+2) . {3,4}`,
never `1 + (2 . {3,4})`), while still nesting inside an equation (`a . {b}
= c . {d}`). This is a first-cut, disclosed simplification in the same
spirit as this section's own comparison-tier note above, not a verified
claim about real Reduce's exact grammar; see `reduce.grammar`'s own header
comment for the full reasoning.

**R-4 correction to this table** (added once `reduce-runtime` actually
lowered against `symbolic-ir`/`symbolic-vm` rather than assuming their
exact head spellings from family resemblance to Macsyma/Wolfram/Derive):
the arithmetic/negation rows above originally read `Plus`/`Subtract`/
`Times`/`Power`, with `a / b` and `-a` expanded to `Times[a, Power[b,
-1]]` and `Times[-1, a]` respectively. None of `Plus`/`Subtract`/`Times`/
`Power` exist as head names in `symbolic-ir` — `grep -n '"Plus"\|
"Subtract"\|"Times"\|"Power"' symbolic-ir/src/lib.rs` returns nothing.
The real, already-implemented-and-reused heads are `Add`/`Sub`/`Mul`/
`Div`/`Pow`/`Neg` (exactly what `derive-runtime`/`macsyma-compiler`
already lower `+`/`-`/`*`/`/`/`^`/unary-`-` to), which is what the table
above now shows and what `reduce-runtime` actually lowers to — using the
real heads is *more* faithful to this spec's own reuse promise (§5: "the
exact same functions, so all four languages agree on every result") than
literally expanding division/negation into `Times`/`Power` applications
would have been, since that would have sidestepped the very `Div`/`Neg`
handlers already shared with Derive and Macsyma.

This table's `Cons`/`First`/`Second`/`Third`/`Rest`/`Part`/`Append`/
`Reverse`/`CompoundExpression` entries remain accurate as *lowering
targets* — `reduce-runtime` does produce exactly these heads — but §5's
claim that they are "already implemented … the exact same functions" for
the shared `SymbolicBackend` did not hold: see §5's own corrected text.

Comments, Reduce's mode-switch mechanism (`on rounded;`, `on complex;`,
…), and its symbolic (raw-Lisp) mode are not part of this grammar; see §4.

## §4 Honest scope — what is *out* (for now)

This is a clearly-scoped subset (per HML00 §9 and as every prior kickoff
in this family does). This spec's grammar deliberately omits, to be added
later if warranted:

- **Implicit multiplication by juxtaposition.** Not actually a gap in real
  Reduce's own grammar the way it is for Wolfram/Derive/MATLAB — the
  manual's own examples (`x^3 - 2*y/(2*z^2 - df(x,z))`) always show an
  explicit `*`, so this subset's requirement of an explicit `*` is not a
  scope-narrowing decision here, just what real Reduce already does.
- **`for`/`while`/`repeat...until` loops** (manual §5.4/§5.5/§5.6).
  Confirmed real syntax (`for i:=1:10 do ...` with `step`/`until` bounds
  and `in`/`on` list-iteration forms, five action keywords `do`/`sum`/
  `product`/`collect`/`join`; `while ⟨bool⟩ do ⟨stmt⟩`) — but
  `symbolic-vm`'s shared handler table has no existing `While`/`For`
  handler for the CAS-family languages the way it does for the
  array-family MATLAB/APL/J side (SIR16's `Loops` feature), so wiring
  these would be new engine code, not reuse — deferred to its own
  follow-on item, the same reasoning [MA07](MA07-derive-language.md) §4
  used to defer `LIM`/`SUM`/`PRODUCT`/`TAYLOR`/`SOLVE`.
- **Block-structured `procedure` definitions** (`procedure f(x); begin
  scalar m; m:=x^2; return m end;` — manual's own factorial example uses
  `begin scalar m,s; ...; if s=0 then return m; ...; go to l1 end`).
  Confirmed real syntax with local (`scalar`) variable declarations,
  explicit `return`, and `go to`/label (`l1:`) support — a substantially
  bigger feature than the simple assignment-based `h(l,m) := e`
  definition form this subset already covers (§3), and one with no
  precedent anywhere else in this repo (no other frontend has GOTO/labels
  today). Deferred to its own item once genuinely needed.
- **`let` rules — Reduce's own native pattern-rewrite-rule vocabulary**
  (`for all n let factorial n => ...`; rule lists with `~x`-marked free
  variables and `such that` guards — manual ch. 11/§15). This is the one
  respect in which Reduce is *not* like Derive: Derive "has no analogue of
  Wolfram's pattern items" ([MA07](MA07-derive-language.md) §1), but
  Reduce genuinely does — `let`'s `for all ... let pattern => replacement`
  shape is a real, confirmed, general rewrite-rule mechanism, structurally
  closer to Wolfram's `SetDelayed`/pattern vocabulary
  (already reusing `cas-pattern-matching`) than to anything Derive needed.
  Deferred as its own follow-on item precisely *because* it's substantial
  enough to deserve one, not because it's out of character for the
  language — when it lands, it should reuse `cas-pattern-matching`'s
  existing matcher the way Wolfram's W-19/W-20 items already do,
  transliterating Reduce's `~x` free-variable marker onto the same pattern
  vocabulary Wolfram's `x_` already lowers to, rather than inventing a
  second pattern representation.
- **Arrays** (`array a(10,10);`-style declarations with subscript
  *assignment* into backing storage — manual §3.1 mentions "array names
  with subscripts" as one of scalar-expression's building blocks, and
  chapter 6 covers array declarations directly). This subset's grammar
  parses `a(5)`/`b(i,q)` only as an ordinary call-shaped read (§3); array
  *declaration* and indexed *write* are deferred, mirroring how
  [Derive](MA07-derive-language.md) D-5 deferred matrix/vector *algebra*
  while still giving the literal syntax a structural representation.
- **Matrices** (ch. 14 — Reduce's own dedicated matrix type and linear-
  algebra operators, distinct from its list type). Deferred entirely for
  now; if picked up later it should likely reuse `array-runtime`/
  `matrix-runtime` for the actual numerics, matching
  [Derive](MA07-derive-language.md) §4's identical note about its own
  deferred matrix algebra.
- **Mode switches** (`on rounded;`, `on complex;`, `off exp;`, …) — a
  global, session-wide evaluation-mode mechanism (confirmed real, e.g.
  floating-point numbers convert to exact integer ratios *unless*
  `on rounded` is active, per manual §3.1) with no analogue in this
  repo's existing CAS frontends. Deferred; this subset always behaves as
  if no mode switch has been set (exact-rational arithmetic throughout,
  matching every other CAS frontend's own default).
- **Strings** (manual §2.5) and **comments** (§2.6) — not part of this
  grammar; this subset's programs are a flat sequence of expression
  statements, assignments, and simple operator/procedure definitions,
  matching how MA03/MA04/MA07's own subsets all start minimal.
- **`eq`, `memq`, `member`, and `where`** — manual §2.7's own precedence
  table lists these as ordinary infix operators (list/structural equality
  and membership, and a substitution operator respectively), but they are
  more advanced list/substitution primitives than this subset's core
  arithmetic-and-list-accessor surface needs; deferred alongside the
  richer list operator surface (`delete`, `length`, `remainder`
  list-specific overloads, etc.) chapter 4 documents beyond §3's table.
- **Symbolic (raw Lisp) mode** — Reduce's `symbolic procedure ...` escape
  hatch into direct Lisp is a different language entirely (S-expressions,
  not algebraic-mode infix syntax); out of scope permanently, not just
  "for now" — this repo's Reduce frontend targets algebraic mode only,
  the same mode every real interactive Reduce session and every one of
  Reduce's own algebraic-CAS-family peers in this repo use.

These are surface-syntax gaps only where the *engine* (rewrite +
simplification, already implemented for Macsyma/Maxima/Wolfram/Derive)
already supports the corresponding operation — each is a grammar/lexer/
wiring addition in a later item, not an engine change, except `let`
rules and block-structured procedures, which are genuinely new engine
surface (pattern matching / local-scope-with-GOTO respectively) deferred
for that reason specifically, called out above rather than lumped in
with the others.

## §5 Reuse strategy

- **Frontend:** the grammar-tools framework, exactly as Macsyma/MATLAB/
  Wolfram/APL/J/Derive use it. `reduce.tokens`/`reduce.grammar` compile to
  committed `_grammar.rs` in `reduce-lexer`/`reduce-parser` (R-2/R-3).
- **Lowering + engine (R-4, ✅ done, `reduce-runtime`):** the parsed tree
  lowers to [`symbolic_ir::IRNode`](../packages/rust/symbolic-ir) (surface
  operators, `:=`/equations, and list operations → canonical `Add`/`Sub`/
  `Mul`/`Div`/`Pow`/`Neg`/`Assign`/`Define`/`If`/`CompoundExpression`/
  `List`/`First`/`Second`/`Third`/`Rest`/`Part`/`Append`/`Reverse`/`Cons`
  heads), evaluated by [`symbolic_vm::VM`](../packages/rust/symbolic-vm)
  over the *stock* [`SymbolicBackend`](../packages/rust/symbolic-vm) —
  reused directly, unchanged, with **no** Reduce-specific `Backend` at
  all (this spec's original wording, "a Reduce `Backend` over the shared
  `build_handler_table` pattern," implied a per-language `Backend` the
  way Macsyma/Wolfram each have their own; R-4 does not build one) — the
  same rewrite engine Macsyma, Wolfram, and Derive already drive.
  **Corrected once implemented, rather than left to quietly mismatch
  reality:** this spec originally claimed "No new engine code is needed
  for R-4's in-scope surface (§3): every head it lowers to already has a
  handler, shared verbatim across all four symbolic-family languages" —
  grepping `symbolic_vm::handlers::build_handler_table` shows that is
  true only for the arithmetic/comparison/logic/`Assign`/`Define`/`If`/
  `List` heads. `CompoundExpression`, `First`/`Second`/`Third`/`Rest`/
  `Part`/`Append`/`Reverse`, and `Cons` have **no handler at all** in the
  shared table — Macsyma's list functions and Wolfram's
  `CompoundExpression` are each wired through a bespoke per-language
  `Backend`, which is exactly what "no custom `Backend` at all" (above)
  rules out building here. `reduce-runtime` still lowers to these exact
  heads (so a later item that adds real handlers to the shared table, or
  a narrowly-scoped Reduce `Backend` if that turns out to be the right
  shape, needs no lowering change), but evaluating one of these calls
  today does not perform the operation — arguments still evaluate (so
  `Assign`/`Define` side effects inside a `<< ... >>` genuinely happen,
  in order), the call itself just stays unevaluated, like calling an
  undefined user function. See `reduce-runtime`'s own module doc comment
  and CHANGELOG for the full accounting.
- **REPL (R-4, ✅ done, `reduce-repl`):** a single-threaded driver
  mirroring `wolfram-repl`/`maxima-repl`/`derive-repl`; a plain
  (non-numbered) read-eval-print loop, matching real Reduce's own
  interactive session transcript style.

## §6 References

Internal: [`HML00`](HML00-historical-math-languages-roadmap.md) (roadmap,
Wave 5), [`MA03`](MA03-maxima-language.md)/[`MA04`](MA04-wolfram-language.md)/
[`MA07`](MA07-derive-language.md) (the three prior symbolic-family
kickoffs this spec mirrors most closely — MA07 especially, as the other
Wave-5 language), `symbolic-ir`, `symbolic-vm`, `cas-pattern-matching`,
`cas-simplify`.

External: Anthony C. Hearn, *REDUCE User's Manual* (Rand Corporation,
1968 origin; free/open-source since 2008; Hearn & Schöpf, current build
consulted directly rather than assumed from CAS-family resemblance):
<https://reduce-algebra.sourceforge.io/manual/manual.html> — specifically
§2.7 "Operators" (precedence table, infix/prefix operator list),
§3.1 "Scalar Expressions", §3.3 "Boolean Expressions", §3.4 "Equations",
§4.1 "Operations on Lists", §5.1 "Assignment Statements", §5.2 "Group
Statements", §5.3 "Conditional Statements", §5.4 "FOR Statements",
§5.5 "WHILE...DO", and the "Procedures" / "LET Rules as Procedures"
chapters (ch. 11/15) covering `procedure`/`begin...end`/`return`/`go to`
and `for all ... let ... =>` rule syntax — every syntax claim in §3/§4
above traces to one of these pages, not to general CAS-family assumption.
