# MA04 — Wolfram / Mathematica (a subset)

## Status

Active spec / roadmap for a **Wolfram Language** (Mathematica) frontend — Wave 3
of the historical-math roadmap
([HML00 §7](HML00-historical-math-languages-roadmap.md)), "the flagship
symbolic." Unlike [Maxima](MA03-maxima-language.md), which is byte-for-byte
Macsyma and so reused that frontend wholesale, **Wolfram's surface syntax is
genuinely different** — `f[x]` function application in *square* brackets,
`{a, b}` list braces, `_`/`x_` patterns, and the `/.`, `->`, `:>` replacement
operators. So Wolfram needs a *real new frontend* (lexer + parser). What it
reuses is the **engine underneath**: every M-expression lowers to the shared
[`symbolic-ir`](../packages/rust/symbolic-ir) term representation and is
evaluated by [`symbolic-vm`](../packages/rust/symbolic-vm) term rewriting, with
[`cas-pattern-matching`](../packages/rust/cas-pattern-matching) backing `/.` and
[`cas-simplify`](../packages/rust/cas-simplify) backing simplification — the same
substrate Macsyma drives.

This is the symbolic-CAS analogue of how the MATLAB *numeric* frontend was a new
grammar over the shared `array-runtime`: a new surface, a shared engine.

## §1 Why Wolfram is "everything is an expression"

Wolfram's defining idea is that *every* program fragment is a single expression
tree `head[arg, …]` — `2 + 3` is `Plus[2, 3]`, `{1, 2, 3}` is `List[1, 2, 3]`,
`x = 5` is `Set[x, 5]`, even `f[x]` is just `f` applied to `x`. The infix
operators are syntactic sugar for those heads. This maps **directly** onto
`symbolic-ir`'s [`IRNode::Apply { head, args }`](../packages/rust/symbolic-ir):
parsing a Wolfram program is, essentially, desugaring its surface operators into
the canonical heads (`Plus`/`Times`/`Power`/`List`/`Rule`/`ReplaceAll`/…) that
`symbolic-vm` already knows how to rewrite. The pattern/rewrite core (`_`, `x_`,
`->`, `:>`, `/.`) is a *direct fit* for `cas-pattern-matching`'s
`Blank`/`Pattern`/`match_pattern`/`rewrite`.

## §2 The pieces (one item = one PR)

Following [HML00 §6](HML00-historical-math-languages-roadmap.md)'s breakdown:

- **W-1 — this spec + the grammar** *(this PR)*. `code/grammars/wolfram.tokens`
  and `code/grammars/wolfram.grammar`, authored in the grammar-tools format and
  validated with `grammar-tools validate`. No crate yet — the grammar is the
  contract the next items implement against.
- **W-2 — `wolfram-lexer`.** A sibling of the other grammar-driven lexers, with
  the committed `_grammar.rs` compiled from `wolfram.tokens`, plus a
  bracket-interior-newline hook (a `NEWLINE` inside `[ ]`, `{ }`, or `( )` is
  not a statement terminator) — the same shape as the R/S lexer's hook.
- **W-3 — `wolfram-parser`.** The committed `_grammar.rs` compiled from
  `wolfram.grammar`, over the generic `parser::GrammarParser`.
- **W-4 — `wolfram-runtime` + `wolfram-repl`.** *(implemented — see §7.)* A
  `WolframSession` that lowers the parsed `GrammarASTNode` into `symbolic-ir`,
  evaluates with `symbolic-vm` (the shared `SymbolicBackend` over
  `build_handler_table`), and applies `cas-pattern-matching` for `/.`. String-in /
  string-out, like the Maxima/Octave facades, plus the interactive
  `wolfram-repl` (`In[n]:= ` / `Out[n]= `, the `wolfram`/`math` binary).
  `Simplify`/`Expand` and the full `cas-*` surface are W-6.
- **W-5 — more built-ins & evaluation.** *(implemented — see §8.)* List,
  functional, control, and numeric built-ins lowered onto the *same* symbolic
  substrate (no bespoke evaluator): `Length`, `First`, `Last`, `Part`, `Append`,
  `Range`, `Map`, `Apply`, `If`, `N`, and the comparison/equality/logical heads.
- **W-6 — operator sugar `/@`, `@@`, `[[ ]]`.** *(implemented — see §9.)* The
  Tier-2 surface forms deferred from W-5: `f /@ x` ≡ `Map[f, x]`, `f @@ x` ≡
  `Apply[f, x]`, `x[[i]]` ≡ `Part[x, i]`. A grammar + lexer change (new tokens
  `MAP`/`APPLY`/`LDBRACKET`/`RDBRACKET`) desugaring to the *same* W-5 Tier-1
  heads, so each sugar form evaluates identically to its head form.
- **W-7 — iteration constructs `Table`, `Do`, `Sum`, `Product`.**
  *(implemented — see §10.)* Iterator-bound evaluation over a local index,
  lowered onto the *same* substrate: `Table[expr, {i, imax}]` builds a list of
  `expr` with `i` bound over a range, `Do[expr, {i, n}]` evaluates `expr` `n`
  times for side effects (returns `Null`), `Sum`/`Product` fold `+`/`×` over the
  range. The iterator spec `{i, …}` reuses the W-5 `Range` span machinery and the
  W-4 `substitute` used for user-function parameter binding; the per-iteration
  count is DoS-capped exactly like `Range` (§10.3). No grammar change — these are
  ordinary `Head[args]` forms the existing grammar already parses.
- **Future — the `cas-*` function surface under Wolfram names** (`Expand`,
  `Factor`, `Solve`, `D`, `Integrate`, …) wired to the existing `cas-*` crates.

## §3 The supported surface (the grammar)

The W-1 grammar captures this subset of Wolfram syntax. Everything is desugared
to a `head[args]` form (shown in the right column) in W-4.

| Surface | Meaning | Lowers to |
|---------|---------|-----------|
| `123`, `1.5` | integer / real literal | `Integer` / `Float` |
| `"text"` | string literal | `Str` |
| `Sin`, `x`, `foo` | symbol (case-sensitive; built-ins are Capitalized) | `Symbol` |
| `f[a, b]` | function application (square brackets) | `f[a, b]` |
| `x[[i]]` | part sugar (double brackets, postfix) | `Part[x, i]` (W-6) |
| `f /@ x` | map sugar (infix) | `Map[f, x]` (W-6) |
| `f @@ x` | apply sugar (infix) | `Apply[f, x]` (W-6) |
| `{a, b, c}` | list | `List[a, b, c]` |
| `a + b`, `a - b` | additive | `Plus` / `Subtract` |
| `a b`* / `a * b` | multiply *(explicit `*` required — see below)* | `Times` |
| `a / b` | divide | `Times[a, Power[b, -1]]` (W-4) |
| `a ^ b` | power (right-assoc) | `Power` |
| `-a` | negation | `Times[-1, a]` |
| `a == b`, `!=`, `<`, `>`, `<=`, `>=` | comparison | `Equal`/`Unequal`/`Less`/… |
| `a && b`, `a \|\| b`, `!a` | logic | `And` / `Or` / `Not` |
| `_`, `x_`, `_h`, `x_h` | pattern blanks | `Blank[]` / `Pattern[x, Blank[]]` / `Blank[h]` / `Pattern[x, Blank[h]]` |
| `a -> b` | Rule (right-assoc) | `Rule[a, b]` |
| `a :> b` | RuleDelayed | `RuleDelayed[a, b]` |
| `expr /. rules` | ReplaceAll (left-assoc) | `ReplaceAll[expr, rules]` |
| `x = e`, `x := e` | Set / SetDelayed (right-assoc) | `Set` / `SetDelayed` |
| `( … )` | grouping | — |
| `e1 ; e2` | statement separator (`;` suppresses output) | *(see below)* |

**Precedence**, loosest → tightest (each binds tighter than the line above):
`Set`/`SetDelayed` → `ReplaceAll` → `Rule`/`RuleDelayed` → `Or` → `And` → `Not`
→ comparison → additive → multiplicative → unary minus → `Power` → map/apply
sugar (`/@`, `@@`) → application `f[…]` / part `[[…]]` → atoms. This matches
Wolfram's operator precedences for the subset (e.g. `x /. a -> b` is
`ReplaceAll[x, Rule[a, b]]`; `a && b -> c` is `Rule[And[a, b], c]`; `-x^2` is
`Times[-1, Power[x, 2]]`). The `[[…]]` part sugar is a postfix that binds as
tightly as `f[…]` application (so `x[[1]][[2]]` chains left-to-right); `/@` and
`@@` are infix operators just below application (`f /@ {1, 2}` maps `f` over the
whole list). Exact precedence of `/@`/`@@` against the arithmetic operators is
not exercised by the subset's tests — write parentheses when mixing them.

Comments are `(* … *)`. Newlines terminate a top-level statement; a `;`
separates statements on one line and suppresses the preceding result's display
(the REPL convention). Inside `[ ]`/`{ }`/`( )` a newline is whitespace (the W-2
lexer hook), so a call or list may span lines.

## §4 Honest scope — what is *out* (for now)

This is a clearly-scoped subset (per convention 9 and as S00/R00/MA03 do). The
W-1 grammar deliberately omits, to be added later if warranted:

- **Implicit multiplication by juxtaposition.** Real Wolfram reads `2 x` as
  `2*x`; that is genuinely hard in a context-free grammar, so this subset
  **requires an explicit `*`**. (`2 x` will not parse; write `2*x`.)
- **`;;` `Span`**, `@` (prefix application), `&` pure functions and `#`/`#1`
  slots, `~f~` infix, `|` `Alternatives`. The `[[ … ]]` `Part` sugar, `/@` map
  sugar, and `@@` apply sugar ship in W-6 (§9), desugaring to the W-5 `Part`,
  `Map`, and `Apply` *head* forms (§8); prefix `@`, spans, and pure functions
  remain out of scope.
  `..`/`...` repeated patterns, `/;` `Condition`, `:` `Optional`/`Pattern`
  binding, `'` `Derivative`, `%` `Out`, contexts/backtick symbols.
- **`CompoundExpression` inside an expression** — `;` is supported only as a
  top-level statement separator, not as `(a; b)`. (Deferred to a later item.)
- Big-integer / arbitrary-precision and complex literals; `&&`/`||` short
  circuit is a runtime concern (W-4), not lexical.

These are surface-syntax gaps only; the *engine* (rewrite + pattern matching +
simplify) already supports the corresponding heads, so each is a grammar/lexer
addition, not an engine change.

## §5 Reuse strategy

- **Frontend:** the grammar-tools framework, exactly as Macsyma/MATLAB/R use it.
  `wolfram.tokens`/`wolfram.grammar` compile to committed `_grammar.rs` in
  `wolfram-lexer`/`wolfram-parser` (W-2/W-3).
- **Lowering + engine (W-4):** the parsed tree is lowered to
  [`symbolic_ir::IRNode`](../packages/rust/symbolic-ir) (surface operators →
  canonical heads), then evaluated by
  [`symbolic_vm::VM`](../packages/rust/symbolic-vm) with a Wolfram `Backend`
  over `build_handler_table`. `/.`/`->`/`:>` use
  [`cas_pattern_matching`](../packages/rust/cas-pattern-matching)
  (`match_pattern`/`rewrite`/`rule`/`rule_delayed`); `Simplify`/`Expand`/… use
  the `cas-*` crates (W-6).
- **REPL (W-5):** a single-threaded driver mirroring `s-repl`/`maxima-repl`.

## §7 W-4 runtime — lowering + evaluation (implemented)

The `wolfram-runtime` crate (`code/packages/rust/wolfram-runtime`) is the W-4
deliverable. It takes a parsed `GrammarASTNode` from `wolfram-parser` and:

1. **Lowers** the surface tree to canonical [`symbolic-ir`](../packages/rust/symbolic-ir)
   `IRNode`s. This is the *desugaring* step §1/§3 describe.
2. **Evaluates** the lowered IR with a [`symbolic-vm`](../packages/rust/symbolic-vm)
   `VM` over the shared `SymbolicBackend` (the same `build_handler_table` Macsyma
   drives), so the whole rewrite engine is reused unchanged.
3. **Replaces** `/.` via [`cas-pattern-matching`](../packages/rust/cas-pattern-matching)'s
   `rewrite`, threading `Blank`/`Pattern`/`Rule` nodes lowered from `_`/`x_`/`->`.
4. **Pretty-prints** the result back to Wolfram *surface* notation (infix
   operators, `f[…]` application, `{…}` lists), so the string-out side reads like
   Mathematica even though the engine speaks `Add`/`Mul`/`Pow`.

### §7.1 The head-name bridge (surface → IR)

The single subtlety is that Wolfram's *surface* head names are **not** the IR's
canonical head names. The IR/VM speaks `Add`/`Sub`/`Mul`/`Div`/`Pow`/`Neg`; the
Wolfram surface (and its `Plus[…]`/`Times[…]`/`Power[…]` head-applications)
speaks the Mathematica vocabulary. The lowering bridges them in **both**
directions of entry — the infix operators *and* an explicit head-application
like `Plus[1, 2, 3]` map to the same canonical IR head, so `1 + 2` and
`Plus[1, 2]` evaluate identically.

| Wolfram surface / head | IR head | Notes |
|------------------------|---------|-------|
| `a + b`, `Plus[…]` | `Add` | n-ary `Plus[…]` lowers to a left-folded `Add` chain |
| `a - b`, `Subtract[a,b]` | `Sub` | |
| `a b` / `a*b`, `Times[…]` | `Mul` | explicit `*` required (see §4) |
| `a / b`, `Divide[a,b]` | `Div` | |
| `a ^ b`, `Power[a,b]` | `Pow` | right-associative |
| `-a`, `Minus[a]` | `Neg` | |
| `Sin[x]`, `Cos`, `Exp`, `Log`, `Sqrt`, `Tan`, … | `Sin`/`Cos`/… | already canonical — passed through |
| `a == b`, `Equal[…]`, `Less`, `Greater`, `…` | `Equal`/`Less`/… | |
| `a && b`, `And[…]`, `Or`, `Not` | `And`/`Or`/`Not` | |
| `{a, b, c}`, `List[…]` | `List` | |
| `x = e`, `Set[x,e]` | `Assign` | held head; binds `x` in the backend env |
| `x := e`, `SetDelayed[…]` | `Define` | held head; user-function definition |
| `_`, `x_`, `_h`, `x_h` | `Blank[]` / `Pattern[x, Blank[]]` / `Blank[h]` / `Pattern[x, Blank[h]]` | the `cas-pattern-matching` node shapes |
| `a -> b`, `Rule[a,b]` | `Rule` | `cas-pattern-matching` rule head |
| `a :> b`, `RuleDelayed[a,b]` | `RuleDelayed` | |
| `expr /. rules` | *(handled in the runtime)* | `cas-pattern-matching::rewrite(expr, [rules])` |
| any other `f[…]` | `f[…]` | unknown heads pass through unevaluated (Mathematica semantics) |

An unbound symbol stays a free symbol (`SymbolicBackend::on_unresolved`), exactly
matching Mathematica, where `x` with no value is just `x`.

### §7.2 Robustness at the trust boundary

`WolframSession::feed` takes arbitrary user source, so it is the trust boundary
for the whole reused symbolic stack. Following the Maxima precedent, three
layered guards stop a single crafted input from crashing or wedging a session:

1. **Input-size cap** ([`MAX_INPUT_LEN`], 64 KiB) — a cheap first gate on memory
   and time.
2. **Per-statement token cap** ([`MAX_STATEMENT_TOKENS`]) — counted from the
   *real* `wolfram-lexer` token stream (the iterative lexer cannot itself
   overflow). A parse tree's depth is bounded by its token count, so capping
   tokens caps recursion depth in the grammar parser, the lowering, the VM, and
   the later `Drop` of the tree — closing the stack-overflow-on-deep-nesting
   vector (`((((…))))`, `------…x`, `1+1+1+…`) that `catch_unwind` cannot catch.
3. **Big bounded worker stack + `catch_unwind` + session rebuild** — evaluation
   and pretty-printing run on a dedicated thread with a large bounded stack, any
   unwinding panic from the reused stack is converted to a clean `Err(String)`,
   and the session is rebuilt after a caught panic so the next call is always
   usable.

### §7.3 The REPL (shipped with W-4)

`wolfram-repl` wraps a persistent `WolframSession` with the Mathematica console
contract: `In[n]:= ` / `Out[n]= ` prompts, line-continuation while brackets are
open or a string/comment is unterminated, `Quit`/`Exit`/Ctrl-D to leave, and a
size-capped accumulation buffer. The `wolfram` (alias `math`) binary drives it
over stdin/stdout. This mirrors `maxima-repl`'s driver; the only Wolfram-specific
part is that a *newline* (not `;`/`$`) terminates a complete statement.

## §8 W-5 built-ins & evaluation (implemented)

W-4 gave `wolfram-runtime` arithmetic, comparison, logic, lists-as-data,
patterns/`/.`, `Set`/`SetDelayed`, and the elementary functions inherited from
the shared `SymbolicBackend`. W-5 adds the **list, functional, control, and
numeric built-ins** every introductory Wolfram session reaches for — and adds
them the *same* way W-4 added everything else: by lowering onto the shared
symbolic substrate, never a bespoke evaluator (convention: reuse shared infra).

### §8.1 What W-5 adds

| Surface form | Result | Notes |
|--------------|--------|-------|
| `Length[{a, b, c}]` | `3` | element count of a `List`; `Length` of a non-list (an atom or other head) is `0`, matching Wolfram |
| `First[{x, y}]` | `x` | first element; `First[{}]` is left **unevaluated** (`First[{}]`), never a panic — empty-list access has no value |
| `Last[{x, y}]` | `y` | last element; `Last[{}]` likewise unevaluated |
| `Part[{a, b, c}, 2]` | `b` | **1-based** `i`-th element (`Part[expr, 0]` is the head); negative `i` counts from the end (`Part[{a,b,c}, -1]` is `c`); an out-of-range or non-integer index is left unevaluated |
| `Append[{a, b}, c]` | `{a, b, c}` | a new list with `c` appended (the original is unchanged — values are immutable) |
| `Range[3]` | `{1, 2, 3}` | `Range[n]` is `{1, …, n}`; `Range[a, b]` is `{a, …, b}`; `Range[a, b, d]` steps by `d`. **DoS-capped**: a span that would produce more than `MAX_RANGE_LENGTH` elements is left unevaluated rather than allocated |
| `Map[f, {a, b}]` | `{f[a], f[b]}` | apply `f` to each element; the results are re-evaluated, so `Map[Sin, {0}]` is `{0}` |
| `Apply[f, {a, b}]` | `f[a, b]` | replace the list's `List` head with `f`; `Apply[Plus, {1, 2, 3}]` is `6` because the `Plus`→`Add` bridge then folds it |
| `If[cond, t, f]` | `t` or `f` | already worked in W-4 via the shared held `If` handler; W-5 pins it with tests. `If[cond, t]` with a false `cond` yields `False`; a non-boolean `cond` leaves the `If` unevaluated |
| `N[1/2]` | `0.5` | numeric coercion: an exact `Integer`/`Rational` becomes a `Float`; already-float and symbolic values pass through; `N` maps over a list element-wise |

`Length`, `First`, `Last`, `Part`, `Append`, `Range`, `Map`, `Apply`, and `N` are
**function-call heads** the existing `head[args]` grammar already parses — no
grammar/lexer change is needed for W-5. The comparison heads (`==`, `!=`, `<`,
`>`, `<=`, `>=`), the logical operators (`&&`, `||`, `!`), and `If` were already
wired through evaluation in W-4 (the grammar carries `LE`/`GE`/`AND`/`OR` tokens);
W-5 only adds end-to-end tests for them.

### §8.2 Where the handlers live — a decorator backend

W-5 does **not** edit `symbolic-vm`'s shared `build_handler_table`. These
built-ins are dispatched by a thin `WolframBackend` *decorator* that wraps the
stock `SymbolicBackend`: it answers `handler_for` from its own small table of
Wolfram list/functional/numeric handlers and **delegates everything else** —
`lookup`, `bind`, `on_unresolved`, `on_unknown_head`, `rules`, `hold_heads`, and
every arithmetic/elementary head — straight to the inner `SymbolicBackend`. This
keeps the new surface local to the Wolfram lane (no 50-dependent rebuild of the
shared crate) while still reusing the entire evaluation engine, the `Plus`→`Add`
bridge, user-defined functions, and `/.`. Argument evaluation is unchanged: the
VM eagerly evaluates a built-in's arguments before the handler runs (so
`Length[Append[{1}, 2]]` sees the already-built `{1, 2}`), exactly as for `Sin`.

### §8.3 DoS surface

`Range` is the one new built-in that turns a *small* input into a *large*
allocation, so it is explicitly capped: any `Range[…]` whose element count would
exceed `MAX_RANGE_LENGTH` (a generous bound, far beyond interactive use) is left
unevaluated instead of allocated, so `Range[10^9]` cannot exhaust memory. The
other built-ins are size-preserving or shrinking (`Map`/`Append` are bounded by
their already-materialised input list, which the W-4 per-statement token cap and
input-size cap already bound). `Part`/`First`/`Last` reject out-of-range and
empty-list access by leaving the expression unevaluated rather than indexing out
of bounds. All of this runs inside the W-4 worker-thread `catch_unwind`, so even
an unforeseen panic in a handler becomes a clean `Err` and the session is rebuilt.

## §9 W-6 operator sugar `/@`, `@@`, `[[ ]]` (implemented)

W-5 (§8) shipped the *head* forms `Map[f, x]`, `Apply[f, x]`, and `Part[x, i]`.
W-6 adds the **operator-sugar surface forms** every real Wolfram session uses
instead, each desugaring to its existing W-5 head — no new evaluation logic, no
new handler, just three new ways to *spell* the same three calls.

### §9.1 What W-6 adds

| Surface form | Desugars to | Identical to |
|--------------|-------------|--------------|
| `f /@ x` | `Map[f, x]` | `Map[f, x]` head form |
| `f @@ x` | `Apply[f, x]` | `Apply[f, x]` head form |
| `x[[i]]` | `Part[x, i]` | `Part[x, i]` head form |

So `Plus @@ {1, 2, 3}` is `6` (via the `Apply` → `Plus[1,2,3]` → `Add` fold),
`f /@ {1, 2}` is `{f[1], f[2]}`, and `{a, b, c}[[2]]` is `b`. Because the sugar
lowers to the exact same IR head the W-5 handler answers, every behaviour W-5
documented (re-evaluation under `Map`, the `Apply`/`Plus` fold, 1-based and
negative `Part` indexing, out-of-range left unevaluated) carries over unchanged.
`[[ ]]` chains and nests like application: `{{1,2},{3,4}}[[1]][[2]]` is
`Part[Part[{{1,2},{3,4}}, 1], 2]` = `2`.

### §9.2 The grammar/lexer change

Unlike W-5 (pure head built-ins, no grammar work), W-6 is a **lexer + grammar**
change because the tokens `/@`, `@@`, `[[`, `]]` did not exist:

- **Tokens** (`code/grammars/wolfram.tokens`): `MAP` (`/@`), `APPLY` (`@@`),
  `LDBRACKET` (`[[`), `RDBRACKET` (`]]`), added under the longest-match-first
  convention — `/@` is listed before `/.`/`/`, `@@` before any `@`, and
  `[[`/`]]` before `[`/`]` — so the multi-char operator always wins.
- **Grammar** (`code/grammars/wolfram.grammar`): `[[ … ]]` is folded into the
  `postfix` rule alongside `f[…]` (a postfix that binds as tightly as
  application); `/@` and `@@` get a new `mapapply` precedence level between
  `power` and `postfix` (infix, left-associative).
- **Regeneration**: the lexer/parser embed the compiled grammar as
  `src/_grammar.rs`; both were regenerated with the Rust `grammar-tools` CLI
  (`compile-tokens` / `compile-grammar`) — never hand-edited.

### §9.3 Lowering

`wolfram-runtime`'s `lower.rs` maps the new parser nodes onto the W-5 heads:
`postfix`'s `[[ arglist ]]` segment builds `Part[base, idx]` (one `Part` per
index when the bracket carries several), and the `mapapply` rule builds
`Map[f, x]` / `Apply[f, x]`. These flow straight into the `WolframBackend`
built-in table (§8.2), so `f /@ x` and `Map[f, x]` produce byte-identical IR.

### §9.4 DoS surface

W-6 adds no new allocation source: `/@`/`@@` desugar to `Map`/`Apply`, whose
DoS profile (bounded by the already-materialised list) §8.3 covers, and `[[ ]]`
to `Part`, which only ever *reads* one element. The one new shape is
**arbitrarily nested `[[…]]`** (e.g. `x[[1]][[1]]…`); each level is one bounded
postfix step parsed iteratively (not by grammar recursion) and one `Part` apply,
so a deeply-chained part expression is linear in source length and bounded by the
W-4 per-statement token cap — it cannot trigger unbounded parser recursion.

## §10 W-7 iteration constructs `Table`, `Do`, `Sum`, `Product` (implemented)

W-4 through W-6 evaluate expressions that have **no local binder** — every
symbol is either a global binding or a free variable. W-7 adds the first
constructs that introduce a *scoped local index*: the iteration heads bind a
fresh variable `i` to each value of a range and evaluate a body once per value.
They are the Wolfram analogue of a `for` loop, but lowered onto the *same*
symbolic substrate — no bespoke loop opcode, no new evaluator.

### §10.1 What W-7 adds

| Form | Result | Notes |
| --- | --- | --- |
| `Table[expr, {i, imax}]` | `{expr[i→1], …, expr[i→imax]}` | a list; `i` ranges `1..imax` |
| `Table[expr, {i, imin, imax}]` | `{expr[i→imin], …, expr[i→imax]}` | explicit lower bound |
| `Table[expr, {i, imin, imax, di}]` | stepped by `di` | reuses the W-5 `Range` 3-bound form |
| `Do[expr, {i, imax}]` | `Null` | evaluates `expr` `imax` times for side effects |
| `Sum[expr, {i, imin, imax}]` | `expr[i→imin] + … + expr[i→imax]` | folds `Plus`; empty range → `0` |
| `Product[expr, {i, imin, imax}]` | `expr[i→imin] × … × expr[i→imax]` | folds `Times`; empty range → `1` |

Worked examples (the W-7 acceptance tests):
`Table[i^2, {i, 3}]` → `{1, 4, 9}`; `Table[i, {i, 2, 4}]` → `{2, 3, 4}`;
`Sum[i, {i, 1, 10}]` → `55`; `Product[i, {i, 1, 4}]` → `24`;
`Do[…, {i, 3}]` → `Null` (body runs 3×); nested
`Table[Table[i*j, {j, 2}], {i, 2}]` → `{{1, 2}, {2, 4}}` (`i*j` for `i∈{1,2}`,
`j∈{1,2}`).

### §10.2 Binding the local index — held heads + `substitute`

The four iteration heads are **held** (added to the backend's `hold_heads` set
via the W-5 `WolframBackend` decorator — it now returns the union of the inner
held set and `{Table, Do, Sum, Product}`). Holding is essential: the body `expr`
must *not* be evaluated before `i` is bound (otherwise `i` evaluates to a free
symbol and the per-iteration substitution has nothing to replace), and the
iterator spec `{i, …}` must stay a literal `List` so the binder name `i` is
readable rather than evaluated.

Inside the handler, for each value `v` in the range the body is bound by the
**same `substitute`** the VM already uses for user-function parameters
(`vm.rs::substitute`): `substitute(expr, {i → v})` produces a fresh body with `i`
replaced, which is then evaluated through `vm.eval`. This matches "how the
runtime already binds symbols" (function-parameter substitution) rather than
mutating the global environment — so a `Table` never leaks `i` into the session
and nested `Table`s each bind their own index cleanly (the inner `substitute`
runs over a body whose outer index was already replaced).

The iterator-spec **bounds** are sub-expressions that *do* need evaluating
(`{i, n}` where `n` is a bound variable, `{i, 2+1}`). Because the head is held,
the handler evaluates each bound through `vm.eval` itself before reading it as an
integer — the body stays held, the bounds get evaluated. A spec with a
non-integer bound, the wrong arity (`{i}` with no bound, `{}`), or a non-symbol
binder is left **unevaluated** (the Wolfram "I can't reduce this" convention),
never a panic.

### §10.3 DoS surface — the per-iteration cap composes

Iteration is the second allocation source after `Range` (§8.3): a tiny input
(`Table[0, {i, 10^9}]`) would otherwise build a billion-element list or spin a
billion-iteration loop. W-7 caps the iteration count with the **same**
`MAX_RANGE_LENGTH` bound `Range` uses, computed from the spec bounds *before* any
allocation or looping — an over-large iterator leaves the whole form unevaluated
rather than hanging or exhausting memory. `Do` is capped identically even though
it allocates nothing, because the cap bounds *wall-clock work*, not just memory.

The cap **composes** for nested iteration: a nested `Table[Table[…, {j, m}], {i,
n}]` builds the inner `Table` `n` times, and each inner build is itself capped at
`MAX_RANGE_LENGTH`, so the outer count `n` and inner count `m` are each bounded —
there is no multiplicative blow-up past `n · MAX_RANGE_LENGTH` body evaluations
for an `n`-row table, and `n` itself is capped. All bound arithmetic
(`imax − imin`, the count, the running index) is done in `i128` with the same
overflow-safe pattern as `Range`, so a crafted `i64::MIN`/`i64::MAX` bound cannot
overflow — it simply falls outside the valid count range and the form stays
unevaluated.

### §10.4 No grammar change

`Table[…]`, `Do[…]`, `Sum[…]`, `Product[…]` are ordinary `Head[args]`
applications and `{i, …}` is an ordinary list literal — both already parse under
the W-1 grammar. W-7 touches only `wolfram-runtime` (the builtin handler table
and the decorator's held set); the lexer, grammar, and `_grammar.rs` are
untouched.

## §11 W-8 local scoping `With`, `Module`, `Block` (implemented)

W-7 (§10) introduced the first *scoped local binder* — the iteration index `i`,
bound per step via `substitute`. W-8 generalises that idea into Wolfram's three
**local-scoping heads**, which bind one or more named locals over a body and
evaluate the body in that augmented scope. Like W-7 they are lowered onto the
*same* substrate — held heads + the `vm.rs::substitute` primitive — with no new
evaluator, no opcode, and no grammar change.

### §11.1 What W-8 adds

All three are `Head[{decls}, body]` forms: a literal `List` of declarations and a
body. They differ only in how a local is *initialised* and how its scope relates
to the surrounding session.

| Form | Result | Notes |
| --- | --- | --- |
| `With[{x = e}, body]` | `body` with `x → eval(e)` | lexical constant; the value is substituted *immediately*. Multiple decls: `With[{x = e1, y = e2}, body]` |
| `Module[{x, y = e}, body]` | `body` with locals bound; an *uninitialised* local (`x`) stays undefined | lexically-scoped locals; an initialised local (`y = e`) gets `eval(e)`, an uninitialised one (`x`) is α-renamed to a fresh `x$nnn` so it is undefined and cannot capture a global |
| `Block[{x = e}, body]` | `body` evaluated with `x` temporarily set to `eval(e)` | dynamically-scoped: shadows a global `x` for the duration of `body`, then restores it |

Worked examples (the W-8 acceptance tests):
`With[{x = 3}, x^2]` → `9`; `With[{a = 1, b = 2}, a + b]` → `3`;
`Module[{a = 1, b = 2}, a + b]` → `3`; `Block[{x = 5}, x + 1]` → `6`;
nested `With[{x = 1}, With[{y = 2}, x + y]]` → `3`; a decl referring to an
outer binding `With[{x = 1}, With[{y = x + 1}, y]]` → `2`.

### §11.2 Binding the locals — held heads + `substitute`

The three scoping heads are **held** (added to the backend's `hold_heads` set
via the `WolframBackend` decorator — it now returns the union of the inner held
set and `{Table, Do, Sum, Product, With, Module, Block}`). Holding is essential
for the same reason as W-7: the `body` must *not* be evaluated before the locals
are bound (otherwise a local symbol evaluates to its free/global meaning and the
per-binding substitution has nothing to replace), and the declaration list
`{x = e, …}` must stay a literal `List` of literal assignments (`x = e` parses as
a `Set`/`Assign` node) so the binder names and their RHS expressions are readable
rather than evaluated against the session.

Inside the handler, each declaration's **RHS is evaluated** through `vm.eval`
(`x = e` → `x → eval(e)`), then the collected mapping is applied to the body with
the **same `substitute`** the VM uses for user-function parameters and W-7's
iteration index. `substitute(body, {x → v, …})` produces a fresh body with the
locals replaced; the result is evaluated through `vm.eval`. Using `substitute`
rather than mutating the session environment is what keeps the locals **local**:

- **`With`** evaluates every RHS *first* (against the surrounding scope, so a
  decl may reference an outer binding) and substitutes all values simultaneously.
  Multiple decls bind in parallel like Wolfram's `With` (each RHS sees the outer
  scope, not its sibling decls).
- **`Module`** treats an initialised decl `y = e` exactly like `With`; an
  *uninitialised* decl `x` is **α-renamed to a fresh gensym `x$nnn`** (mirroring
  real Wolfram) and `x → x$nnn` is substituted into the body. The renamed symbol
  is one the session env has never bound, so the body sees an undefined local
  that cannot resolve to — or be captured by — a same-named global `x`. (Mapping
  `x → x` would *not* shadow a global, since the surviving symbol `x` would still
  be resolved against the env at eval time; the gensym is what makes the local
  genuinely fresh.)
- **`Block`** evaluates each RHS and substitutes into the body identically. The
  *semantic* difference from `With`/`Module` (dynamic vs lexical scope) is only
  observable when the body calls a *separately-defined* function that reads the
  shadowed global; for the substitution-based subset shipped here, a `Block` over
  a self-contained body behaves like `With` over the same body, which is correct
  for every tested case. See §11.3 for the simplification this entails.

Because the binding is by substitution into a held body, **no local leaks into
the session**: after `With[{x = 3}, x]`, a bare `x` is still the free symbol `x`
(the session environment was never touched). Nested scopes each substitute over a
body whose outer locals were already replaced, so `With[{x = 1}, With[{y = 2},
x + y]]` correctly yields `3`.

### §11.3 Substitution-based scoping — the documented simplification

Real Wolfram renames `Module` locals (`x` → `x$nnn`) to guarantee no variable
*capture*: if the substituted value itself mentions the local name, or the body
contains a nested binder of the same name, a naïve textual substitution could
capture the wrong occurrence. W-8 uses a **capture-avoiding-by-construction**
substitution instead of renaming:

- **No global mutation, so no leak and no clobber.** `substitute` rewrites a
  *copy* of the body; the session environment is never written, so a local can
  neither escape the body nor overwrite a same-named global. This is the property
  the leak tests pin (`x` is still free after `With[{x = 3}, x]`).
- **Inner binders shadow correctly under `substitute`.** `substitute` only
  replaces *free* symbol occurrences in the body; an inner `With[{x = …}, …]`
  re-binds `x` for its own (already-substituted) sub-body, so the outer
  substitution and the inner binding compose without capture in the nested cases
  tested. (A fully general implementation would still need α-renaming for the
  adversarial case where a substituted *value* contains a name that a deeper body
  binder shadows; W-8 documents this boundary rather than implementing renaming,
  matching the brief's "substitution-based binding is acceptable IF documented
  and capture-guarded in the tested cases".)
- **`Block`'s dynamic scope is approximated by lexical substitution.** Because
  the subset has no separately-stored closures that close over a *dynamic* `x`,
  substituting `x`'s temporary value directly into the body is observationally
  identical to dynamic shadowing for self-contained bodies. The divergence
  (a `Block` body calling an out-of-line function that reads the global `x`) is
  documented and out of scope for the tested forms.

### §11.4 Robustness — malformed declarations stay unevaluated

The handlers follow the same fail-soft convention as every W-5/W-7 head: a
malformed form is **left unevaluated** (echoed back), never a panic.

- Wrong arity (`With[{x = 1}]`, `With[{x = 1}, b, c]`) → unevaluated.
- A first argument that is **not a `List`** (`With[x, body]`) → unevaluated.
- A declaration that is neither a bare symbol nor a `name = value` assignment
  (`With[{1 + 1}, body]`, `With[{f[x] = 1}, body]`) → the whole form unevaluated.
- For `With`/`Block`, a bare-symbol decl with **no value** (`With[{x}, body]`) is
  rejected (a value is required); for `Module` it is the valid "uninitialised
  local" case. This asymmetry matches Wolfram (`With` requires every local to be
  initialised; `Module` does not).

There is no new allocation source: the body is substituted once per scope entry
and the declaration count is bounded by the (token/-input-capped) source size, so
W-8 adds no DoS surface beyond what W-4 already bounds. Deeply nested scopes are
bounded by the evaluator's existing recursion handling (each nested head is an
ordinary `vm.eval` call over a strictly smaller body).

### §11.5 No grammar change

`With[…]`, `Module[…]`, `Block[…]` are ordinary `Head[args]` applications, the
declaration list `{x = e, …}` is an ordinary list literal, and `x = e` inside it
is the ordinary `Set` infix already parsed since W-1. W-8 touches only
`wolfram-runtime` (the builtin handler table and the decorator's held set); the
lexer, grammar, and `_grammar.rs` are untouched.

## §6 References

Internal: [`HML00`](HML00-historical-math-languages-roadmap.md),
[`MA03`](MA03-maxima-language.md) (the Maxima reuse precedent),
`symbolic-ir`, `symbolic-vm`, `cas-pattern-matching`, `cas-simplify`,
`grammar-tools`.

External: Stephen Wolfram, *The Mathematica Book* / *An Elementary Introduction
to the Wolfram Language*; the Wolfram Language operator-precedence tables.
