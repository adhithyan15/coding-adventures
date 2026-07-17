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
- **W-10 — functional-iteration combinators `Nest`, `NestList`, `Fold`,
  `FoldList`.** *(implemented — see §13.)* The point-free iteration heads every
  functional-programming session reaches for, lowered onto the *same* substrate:
  `Nest[f, x, n]` applies `f` to `x` `n` times, `NestList` collects the `n+1`
  intermediate results, `Fold[f, x0, list]` left-folds `f` over a list, and
  `FoldList` collects the running accumulations. The function-application reuses
  the **exact** `Map`/`Apply` path (`build_canonical_application` + `vm.eval`) and
  the W-5 `list_elements` accessor; the iteration count `n` and the result-list
  length are DoS-capped exactly like `Range`/the list ops (§13.3). No grammar
  change — these are ordinary `Head[args]` forms the existing grammar already
  parses.
- **W-16 — nested/structured list operations: `Transpose`, `Dimensions`,
  `Partition`, `Take`, `Drop`, `ConstantArray`.** *(implemented — see §19.)* The
  *shape* vocabulary for nested (matrix-like) lists: transpose a rectangular list
  of lists, read its dimensions, cut a list into consecutive blocks, take/drop a
  prefix or suffix, and build a constant-filled list or matrix. All are ordinary
  `Head[args]` forms reusing the W-9 `list_elements` accessor and the
  `MAX_LIST_LENGTH` cap; `ConstantArray`/`Partition` outputs are size-capped
  (with `checked_mul`) before allocation (§19.5). No grammar change.
- **W-22 — the `cas-*` function surface under Wolfram names.** *(in
  progress — see §24.)* Wires the existing `cas-*` crates in one head at a
  time: `Simplify` (a thin call into `cas-simplify`'s `simplify()`),
  `Expand` (a thin call into `cas-simplify`'s `expand()`), `Factor` (a
  direct call into `symbolic-vm`'s own `factor_handler`), `D` (a direct
  call into `symbolic-vm`'s own `differentiate`), and `Integrate` (a direct
  call into `symbolic-vm`'s own `integrate_expr`, indefinite form only) are
  all delivered — each the same function its Macsyma counterpart calls, so
  the two languages agree on every result this crate can produce.
  Remaining: `Solve`, each its own item — no grammar change (an ordinary
  `Head[args]` form the existing grammar already parses).

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

## §12 W-9 list-manipulation builtins `Sort`, `Reverse`, `Join`, `Flatten`, `Select`, `Count`, `Total` (implemented)

W-5 gave the *structural* list built-ins (`Length`, `First`, `Part`, `Append`,
`Range`, `Map`, `Apply`). W-9 adds the *manipulation* heads every list-processing
session reaches for — reordering, concatenating, flattening, filtering, counting,
and summing — lowered onto the **same substrate** (the W-5 list machinery, the
`Map`/`Apply` re-evaluation path, and the canonical `Add` fold). Like W-5 these
are plain `Head[args]` applications, so **there is no grammar change**: W-9 touches
only `wolfram-runtime` (the builtin handler table).

### §12.1 What W-9 adds

| Surface form                | Result                                | Reuses |
|-----------------------------|---------------------------------------|--------|
| `Sort[list]`                | ascending in canonical order          | a total order over `IRNode` |
| `Reverse[list]`             | the list reversed                     | the W-5 `list_elements` accessor |
| `Join[a, b, …]`             | the lists concatenated (2+ args)      | the W-5 `list_elements` accessor |
| `Flatten[list]`             | all sub-lists spliced in, **all levels** | recursive splice |
| `Flatten[list, n]`          | flatten the top `n` levels only       | depth-bounded recursive splice |
| `Select[list, pred]`        | elements where `pred[e]` → `True`     | the `Map`/`Apply` application path |
| `Count[list, pred]`         | count of elements where `pred[e]` → `True` | the `Map`/`Apply` application path |
| `Total[list]`               | sum of the elements                   | the canonical `Add` fold (as W-7 `Sum`) |
| `EvenQ[n]` / `OddQ[n]`      | `True`/`False` integer-parity predicate | a minimal predicate primitive (see §12.3) |

Worked examples (the W-9 acceptance tests):

```wolfram
Sort[{3, 1, 2}]          (* {1, 2, 3} *)
Reverse[{1, 2, 3}]       (* {3, 2, 1} *)
Join[{1}, {2, 3}]        (* {1, 2, 3} *)
Join[{1}, {2}, {3}]      (* {1, 2, 3} *)
Flatten[{{1, 2}, {3}}]   (* {1, 2, 3} *)
Flatten[{1, {2, {3}}}]   (* {1, 2, 3}  — all levels *)
Flatten[{1, {2, {3}}}, 1](* {1, 2, {3}}  — one level only *)
Select[{1, 2, 3, 4}, EvenQ]  (* {2, 4} *)
Count[{1, 2, 3, 4}, EvenQ]   (* 2 *)
Total[{1, 2, 3}]         (* 6 *)
```

### §12.2 `Sort` — a total canonical order over `IRNode`

Real Wolfram's canonical order is an elaborate cross-type comparison. The subset
ships a **documented simplification**: a deterministic total order over the
`IRNode` variants that agrees with Wolfram for the common cases an introductory
session sorts — pure-numeric lists sort numerically (`{3, 1, 2}` → `{1, 2, 3}`),
and mixed/symbolic lists sort by a stable, well-defined key (numbers before
symbols before strings before compound expressions; within numbers by value;
within symbols/strings lexicographically; within compound expressions by head
then by arguments). The order is *total* (every pair compares), so `Sort` never
panics and is deterministic across runs. It is **not** bit-for-bit Wolfram
canonical order for every exotic mix — that is out of scope and documented here.
Numeric comparison coerces integers, rationals, and floats to a common `f64`
magnitude so `{2, 1/2, 1.5}` orders sensibly; equal magnitudes fall back to the
type/structure key so the order stays total and stable.

### §12.3 `Select`/`Count` — the predicate application path, and `EvenQ`/`OddQ`

`Select[list, pred]` and `Count[list, pred]` apply `pred` to each element and keep
(or tally) those where the result is the symbol `True`. The application reuses the
**exact** `Map`/`Apply` path: `build_canonical_application(pred, [e])` builds
`pred[e]` and `vm.eval` re-evaluates it through the shared engine, so any callable
— a built-in predicate, a user-defined `SetDelayed` function `f[x_] := …`, or a
bridged head — works as the predicate. Only a result that evaluates to the literal
`True` symbol counts; anything else (`False`, an unevaluated `pred[e]`, a number)
is treated as "not selected". This is the **documented simplification** versus
full Wolfram pattern-matching `Count` (where the second argument may be a pattern
like `_Integer`): W-9 supports a *function* predicate, which is the common
introductory case and reuses the existing application machinery.

Because the W-5/W-6 surface offers no parity predicate, and `Select`/`Count` need
a *testable* one, W-9 adds two minimal predicate primitives:

- `EvenQ[n]` → `True` if `n` is an even integer, else `False`.
- `OddQ[n]` → `True` if `n` is an odd integer, else `False`.

A non-integer argument yields `False` (matching Wolfram: `EvenQ[x]` is `False`,
not unevaluated, for a non-integer). Even-ness is tested as `n.rem_euclid(2) == 0`
so a negative `n` is handled correctly (`EvenQ[-4]` → `True`). These are eager
(non-held) heads like every other W-9 built-in.

### §12.4 DoS surface — bounded outputs, bounded recursion

W-9's heads either **shrink or preserve** their input size, or grow it by
concatenation/flattening, so all are bounded by the W-4 input/token caps that
already bound the materialised input lists:

- `Sort`/`Reverse`/`Select`/`Count`/`Total` are size-non-increasing — their output
  is at most as large as the input list, which is itself bounded by the source
  size. `Sort` allocates one comparison key vector; the sort is `O(n log n)`.
- `Join` concatenates its already-materialised argument lists; the result length is
  the sum of the inputs, each of which is source-bounded. As a defensive guard the
  combined length is capped at [`MAX_LIST_LENGTH`] (= `MAX_RANGE_LENGTH`,
  1,000,000); an over-cap `Join` is left unevaluated rather than allocated.
- `Flatten` splices nested lists into one flat list. Two bounds keep it safe: the
  **recursion depth** is bounded (full-flatten recurses on structure, which is
  bounded by the token-capped input nesting; the explicit-`n` form additionally
  stops after `n` levels), and the **output length** is capped at
  `MAX_LIST_LENGTH`, so a crafted deeply/widely nested list cannot exhaust memory.

Every malformed form (`Sort` of a non-list, `Select` with a non-list or a
non-callable predicate, `Join` of a non-list argument, `Flatten` with a negative/
non-integer depth) follows the W-5 convention and is **left unevaluated** — echoed
back, never a panic.

### §12.5 No grammar change

`Sort[…]`, `Reverse[…]`, `Join[…]`, `Flatten[…]`, `Select[…]`, `Count[…]`,
`Total[…]`, `EvenQ[…]`, `OddQ[…]` are all ordinary `Head[args]` applications. W-9
touches only `wolfram-runtime`'s builtin handler table; the lexer, grammar,
`_grammar.rs`, and the decorator's held set are untouched (none of these heads is
held — their arguments are eagerly evaluated before the handler runs).

## §13 W-10 functional-iteration combinators `Nest`, `NestList`, `Fold`, `FoldList` (implemented)

W-5 gave the *structural* and *higher-order* list built-ins (`Map`, `Apply`); W-7
gave *iterator-bound* evaluation over an index (`Table`, `Sum`); W-9 gave the
*manipulation* heads (`Sort`, `Select`, `Total`). W-10 adds the four **functional
combinators** that iterate a *function* — the point-free idioms (`Nest`,
`NestList`, `Fold`, `FoldList`) that functional-programming sessions reach for —
lowered onto the **same substrate**: each function application reuses the *exact*
`Map`/`Apply` path (`build_canonical_application(f, args)` then `vm.eval`), so any
callable works (a built-in like `Plus`, a user-defined `SetDelayed` function
`g[a_] := …`, or a bridged head). Like W-5/W-9 these are plain `Head[args]`
applications, so **there is no grammar change**: W-10 touches only
`wolfram-runtime` (the builtin handler table). All four are eager (non-held) heads
— `f`, the seed, and the list arrive already-evaluated — so the `WolframBackend`
held set is untouched.

### §13.1 What W-10 adds

| Surface form            | Result                                              | Reuses |
|-------------------------|-----------------------------------------------------|--------|
| `Nest[f, x, n]`         | `f` applied to `x` `n` times: `f[f[…f[x]…]]`         | the `Map`/`Apply` application path |
| `NestList[f, x, n]`     | `{x, f[x], f[f[x]], …}` — the `n+1` intermediates    | the `Map`/`Apply` application path |
| `Fold[f, x0, list]`     | left fold: `f[…f[f[x0, l₁], l₂]…, lₙ]`               | the application path + `list_elements` |
| `FoldList[f, x0, list]` | `{x0, f[x0,l₁], f[f[x0,l₁],l₂], …}` — running accums | the application path + `list_elements` |

Worked examples (the W-10 acceptance tests):

```wolfram
Nest[f, x, 3]            (* f[f[f[x]]]  — symbolic f *)
Nest[f, x, 0]            (* x  — zero applications is the identity *)
NestList[f, x, 2]        (* {x, f[x], f[f[x]]} *)
Fold[Plus, 0, {1, 2, 3}] (* 6 *)
FoldList[Plus, 0, {1, 2, 3}]  (* {0, 1, 3, 6} *)
g[a_] := a + 1; NestList[g, 0, 3]  (* {0, 1, 2, 3} — a user function *)
```

### §13.2 The application path — reusing `Map`/`Apply`

Every combinator iterates by building `f[acc]` (unary, for `Nest`/`NestList`) or
`f[acc, element]` (binary, for `Fold`/`FoldList`) and re-evaluating it through the
VM. The construction is the **same** `build_canonical_application(f, args)` that
`Map`/`Apply`/`Select` use, so the surface→IR head bridge (`Plus`→`Add`, …) and
user-defined functions both resolve. A symbolic `f` (no definition) leaves each
`f[acc]` unevaluated, so `Nest[f, x, 3]` returns the literal nested expression
`f[f[f[x]]]` — exactly Wolfram's behaviour for an undefined head. Pure-function
syntax (`#`/`&`) is **not** required here (it is the planned W-11 grammar-change
item); a named `SetDelayed` function is the canonical way to supply a non-trivial
`f` in this lane, and the worked example above tests exactly that.

`Nest[f, x, 0]` is the identity (`x`); `Fold`/`FoldList` over the empty list `{}`
return the seed (`x0`) / the single-element list `{x0}` respectively — the
mathematically correct degenerate cases.

### §13.3 DoS surface — bounded iteration count, bounded result size

W-10 introduces two growth surfaces, both capped with the **same** machinery the
existing list/iteration ops use:

- **Iteration count `n`** (`Nest`/`NestList`). A tiny input (`Nest[f, x, 10^9]`)
  would otherwise drive a billion `vm.eval` calls. `n` is read as an exact
  non-negative integer and capped at [`MAX_LIST_LENGTH`] (= `MAX_RANGE_LENGTH`,
  1,000,000) **before** the loop runs; an over-cap `n` (or a negative / non-integer
  `n`) leaves the whole form unevaluated rather than iterating. This mirrors the
  `Range`/`plan_iterator` count cap exactly.
- **Result-list size** (`NestList`/`FoldList`). `NestList` materialises `n+1`
  elements and `FoldList` materialises `len+1`; both are bounded — `NestList` by
  the capped `n`, `FoldList` by the (source-bounded) input list length, with a
  defensive `MAX_LIST_LENGTH` check so the `n+1`/`len+1` allocation can never
  exceed the cap.

`Nest`/`Fold` (the scalar-result forms) hold only the running accumulator, so they
add no result-size surface beyond the iteration count. Each step re-evaluates
through `vm.eval`, so a numeric fold (`Fold[Plus, 0, {1,2,3}]`) collapses as it
accumulates rather than building a giant unevaluated tree; a symbolic one
(`Nest[f, x, 3]`) grows linearly with `n`, which the `n` cap bounds.

Every malformed form follows the W-5 convention and is **left unevaluated** —
echoed back, never a panic: a negative or non-integer `n`, a non-list third
argument to `Fold`/`FoldList`, the wrong arity, or an over-cap `n`. A
non-callable `f` is *not* an error — each `f[acc]` simply stays unevaluated (as
`Nest[f, x, 3]` → `f[f[f[x]]]` demonstrates), exactly as an undefined head does
everywhere else in the subset.

### §13.4 No grammar change

`Nest[…]`, `NestList[…]`, `Fold[…]`, `FoldList[…]` are all ordinary `Head[args]`
applications. W-10 touches only `wolfram-runtime`'s builtin handler table; the
lexer, grammar, `_grammar.rs`, and the decorator's held set are untouched (none of
these heads is held — `f`, the seed, and the list are eagerly evaluated before the
handler runs).

## §14 W-11 pure functions — `Function`, `#`/`#n`/`##` slots, `&` postfix (implemented)

W-4..W-10 gave the M-expression core, the arithmetic/`Plus`→`Add` bridge, operator
sugar (`/@`, `@@`, `[[ ]]`), iteration (`Table`, `Sum`), scoping (`With`, `Module`),
list manipulation (`Sort`, `Select`, `Total`), and the functional-iteration
combinators (`Map`, `Nest`, `Fold`). W-11 adds Wolfram's **anonymous (pure)
functions** — the single most-used functional idiom — so a higher-order builtin can
take an inline lambda instead of a named definition:

```wolfram
Map[#^2 &, {1, 2, 3}]            (* {1, 4, 9}  — square each *)
Select[{1, 2, 3, 4}, EvenQ]     (* already worked since W-9   *)
Select[{1, 2, 3, 4}, Mod[#, 2] == 0 &]   (* {2, 4} — inline predicate *)
Nest[# + 1 &, 0, 3]             (* 3 *)
```

### §14.1 Surface forms

Pure functions have **two interchangeable spellings**, both of which lower to the
*same* IR head `Function`:

| Surface                       | Meaning                                              |
|-------------------------------|------------------------------------------------------|
| `Function[x, body]`           | one named parameter `x`                              |
| `Function[{x, y}, body]`      | named parameters `x`, `y`                            |
| `body &`                      | a slot-based pure function (the `&` postfix)         |
| `#`  ≡  `#1`                  | the first argument slot                              |
| `#1`, `#2`, …                 | the *n*-th argument slot                             |
| `##`                          | `SlotSequence` — *all* arguments, spliced            |

`(#^2)&` and `Function[x, x^2]` are two ways to write "square the argument"; applied
to `5` both give `25`. `(#1 + #2)&` and `Function[{x, y}, x + y]` both add two args.

### §14.2 Grammar + lexer change (mirrors W-6)

Unlike every head since W-5, pure functions **need new surface syntax**, so W-11
edits `wolfram.tokens` and `wolfram.grammar` and **regenerates** the embedded
`_grammar.rs` for the lexer and parser via the Rust `grammar-tools` CLI — exactly as
W-6 did for `/@`, `@@`, `[[`. The `_grammar.rs` files are GENERATED; they are never
hand-edited (lessons.md).

**Tokens** (`wolfram.tokens`, longest-match-first — `##` *before* `#`):

| Token     | Lexeme | Note                                                       |
|-----------|--------|------------------------------------------------------------|
| `SLOTSEQ` | `##`   | listed first so `##` wins over two `#`                      |
| `HASH`    | `#`    | the slot opener; a following `NUMBER` (`#2`) is read in the parser |
| `AMP`     | `&`    | the postfix "make a pure function" marker                  |

A slot *number* is **not** a dedicated token: `#2` lexes as `HASH` followed by the
existing `NUMBER` token, and the grammar's `slot` rule consumes the optional number.
This reuses the lexer untouched (no new number-suffix regex) and keeps the lexer
change to three plain string tokens — the minimal, W-6-shaped diff. (`#` and the
multiply `*` never collide; `&` is unrelated to the existing `&&` `AND`, which
remains a separate longest-match-first multi-char token.)

**Grammar** (`wolfram.grammar`):

- A new `slot` atom: `slot = HASH [ NUMBER ] | SLOTSEQ` — `#`, `#n`, or `##` — added
  as an alternative in `atom`.
- A new **postfix** level `amp` for the trailing `&`. The `&` must bind **looser
  than `^`** so it captures the *whole* `#^2` (giving `(#^2)&`, not `#^(2&)`), so
  `amp` sits *above* `power` in the cascade and the existing `mapapply` layer now
  calls `amp` instead of `power`:

  ```text
  mapapply = amp { ( MAP | APPLY ) amp }   # /@, @@ now operate on amp operands
  amp      = power { AMP }                  # trailing & wraps the preceding expr
  power    = postfix [ POWER unary ]        # ^ unchanged, binds tighter than &
  ```

  A trailing run of `&` (`expr &`) wraps the preceding `power` expression into a
  `Function` node with no explicit parameter list (a slot-based pure function);
  `expr & &` wraps twice (rare but well-defined). The precedence is **tested
  directly**: `#^2 &` must lower to `Function[Pow[Slot[1], 2]]`, *not*
  `Pow[Slot[1], Function[2]]`.

The opener-only / longest-match conventions follow W-6: no `&&`-vs-`&` ambiguity (the
two-char `AND` is matched first), and `##`-before-`#` guarantees the sequence slot is
never mis-lexed as two single slots.

### §14.3 Lowering — `Function`, slots, and `&` to a callable

`wolfram-runtime` lowers the three forms to a single canonical IR shape the
evaluator recognises:

| Surface                  | IR                                                        |
|--------------------------|----------------------------------------------------------|
| `#` / `#1`               | `Slot[1]`                                                 |
| `#n`                     | `Slot[n]`                                                 |
| `##`                     | `SlotSequence[1]`                                         |
| `Function[x, body]`      | `Function[List[x], body]`  (named params normalised to a list) |
| `Function[{x,y}, body]`  | `Function[List[x, y], body]`                              |
| `body &`                 | `Function[body]`  (a one-argument `Function`, slot-based) |

So a `Function` node is either **two-argument** (`Function[params, body]`, named) or
**one-argument** (`Function[body]`, slot-based). Both are *inert values* until
applied: `Function[…]` on its own evaluates to itself (it is the Wolfram "function
object"). Application is `Function[…][args]` — an `Apply` whose **head is itself a
`Function` apply**.

### §14.4 Application — a backend rewrite rule, reusing `vm.rs::substitute`

The application `Function[…][args]` is intercepted by a **backend rewrite rule** (the
`Backend::rules()` seam the VM already consults in `eval_apply`, *before* head
dispatch, on the already-arg-evaluated `IRApply`). The predicate matches an `Apply`
whose head is `Apply(Function, …)`; the transform:

1. **Named** `Function[List[p1, …, pn], body]`: bind `p1 → arg1, …` and run the
   **same `vm.rs::substitute`** user functions (W-4 `Define`), W-7 `Table`, and W-8
   scoping already use. Arity is checked; a mismatch leaves the form unevaluated.
2. **Slot-based** `Function[body]`: substitute `Slot[k] → argk` and splice
   `SlotSequence[k] → argk, argk+1, …` into any enclosing application's argument
   list, then return the body for the VM to re-evaluate.

Because the rule fires *inside* `vm.eval` on `Apply(Function[…], [args])`, it
**composes for free** with every W-5/W-9/W-10 combinator: `map_handler` already does
`vm.eval(build_canonical_application(f, [x]))`; when `f` is a `Function[…]` that is
exactly `Apply(Function[…], [x])`, so `Map[#^2 &, {1,2,3}]` → `{1,4,9}` with **no new
code in `Map`**. Likewise `Select[{1,2,3,4}, Mod[#,2]==0 &]` → `{2,4}` and
`Nest[#+1 &, 0, 3]` → `3`.

Slot substitution is **non-capturing by construction**: slots are looked up only
against the *current* application's arguments, and a nested `Function` re-binds its
own slots when *it* is applied, so an outer `#` cannot leak into an inner pure
function's body before the inner one is applied. (Wolfram's true nested-slot scoping
via `Function` levels is out of subset scope and documented as such; the common
single-level idioms all behave correctly.)

### §14.5 DoS surface — bounded substitution, bounded recursion

A pure function substitutes its body **once** per application (one `substitute` walk
over a body whose size is bounded by the token/-input-capped source), so a single
application is linear in the body size. Self-referential recursion — e.g. a pure
function that re-applies itself — is bounded by the evaluator's existing recursion
handling exactly as a self-referential `Define` is (each re-application is an ordinary
`vm.eval` over a strictly *non-growing* body; an unbounded fixpoint diverges no worse
than `f[x_] := f[x]`, which W-4 already tolerates). The slot/`SlotSequence` splice
produces an argument list bounded by the *call's* argument count, never amplified.
W-11 therefore adds **no new unbounded growth source** beyond what W-4 already bounds.

## §15 W-12 string builtins — `StringJoin`, `StringLength`, `StringTake`/`Drop`/`Split`/`Replace`, `ToString`, `Characters` (implemented)

W-4..W-11 gave the M-expression core, the arithmetic bridge, operator sugar,
iteration, scoping, list manipulation, functional combinators, and pure functions
— but every one of those heads operated over *numbers, symbols, and lists*. W-12
adds the **string** builtins every introductory session reaches for, lowered onto
the *same* substrate: the string atom is already `IRNode::Str(String)` (W-4's
lexer produces it and the printer renders it), and the list machinery from W-9
(`StringSplit`/`Characters` build a `List(...)`; the W-9 `MAX_LIST_LENGTH` cap is
reused) is reused verbatim. Like every head since W-5 these are plain `Head[args]`
applications, so **there is no grammar change**: W-12 touches only
`wolfram-runtime`'s builtin handler table (`builtins.rs`), and — for `ToString`'s
unquoted rendering of a bare string — adds one small consideration to the printer
reuse. The `<>` infix sugar for `StringJoin` is **deferred** (it would need a
`wolfram.tokens`/`wolfram.grammar` regen, out of scope for a no-grammar-change
lane item); the `StringJoin[…]` head form ships instead.

### §15.1 What W-12 adds

| Head                        | Meaning                                                          |
|-----------------------------|------------------------------------------------------------------|
| `StringJoin[a, b, …]`       | concatenate string arguments                                     |
| `StringLength[s]`           | number of **characters** (not bytes)                             |
| `StringTake[s, n]`          | first `n` chars; `n < 0` → last `|n|`                            |
| `StringTake[s, {m, n}]`     | 1-based inclusive character range                                |
| `StringDrop[s, n]`          | drop first `n` chars; `n < 0` → drop last `|n|`                  |
| `StringSplit[s]`            | split on runs of whitespace → list of strings                    |
| `StringSplit[s, sep]`       | split on a literal string separator → list of strings            |
| `StringReplace[s, a -> b]`  | replace every literal occurrence of `a` with `b`                 |
| `StringReplace[s, {r, …}]`  | apply a list of literal rules left-to-right                      |
| `ToString[expr]`            | the Wolfram surface form of `expr` as a string                   |
| `Characters[s]`             | list of single-character strings                                 |

Worked examples (the W-12 acceptance tests):

```wolfram
StringLength["abc"]              (* 3 *)
StringJoin["a", "b", "c"]        (* "abc" *)
StringTake["hello", 3]           (* "hel" *)
StringTake["hello", {2, 4}]      (* "ell" *)
StringTake["hello", -2]          (* "lo" *)
StringDrop["hello", 2]           (* "llo" *)
StringSplit["a,b,c", ","]        (* {"a", "b", "c"} *)
StringSplit["a b  c"]            (* {"a", "b", "c"} *)
StringReplace["banana", "a"->"o"](* "bonono" *)
ToString[123]                    (* "123" *)
Characters["ab"]                 (* {"a", "b"} *)
StringLength["héllo"]            (* 5  — multi-byte char counts as 1 *)
StringTake["héllo", 2]           (* "hé" — never splits a char *)
```

### §15.2 Unicode by character, never by byte

Every length, index, and slice operates on **Unicode scalar values** (`char`),
never bytes. The implementation uses `s.chars().count()` for length and collects
`s.chars().collect::<Vec<char>>()` before any indexing, so a multi-byte character
(`é`, an emoji) counts as exactly **one** position and `StringTake`/`StringDrop`
can never slice through the middle of a UTF-8 sequence — the byte-slicing panic
(`byte index N is not a char boundary`) is structurally impossible because no byte
index is ever taken. `StringLength["héllo"]` is `5`, and `StringTake["héllo", 2]`
is `"hé"` (two characters, three bytes).

### §15.3 The "I can't reduce this" contract — malformed input stays unevaluated

Following the W-5/W-9 convention, every W-12 handler returns the application
**unevaluated** when it cannot reduce: a non-string argument
(`StringLength[123]`), an out-of-range index (`StringTake["hi", 9]`), a
non-integer/non-pair second argument, an `i64::MIN` index, or a `StringReplace`
rule whose pattern/replacement is not a string. No handler panics; this is both
the Wolfram-faithful behaviour and a safety property (a crafted index reduces to
nothing rather than crashing).

`ToString` is the one head that always reduces: it renders *any* expr via the
existing `print_wolfram` printer, except that a bare `IRNode::Str(s)` renders as
its **raw content** `s` (no surrounding quotes), so `ToString["hi"]` is the string
`"hi"` and `ToString[123]` is `"123"`. (Inside a larger structure the quoted form
is kept, matching Wolfram's `ToString[{"a"}]` → `{a}` simplification we
intentionally do *not* chase — only the top-level bare-string case is unquoted.)

### §15.4 DoS surface — bounded outputs, bounded scan

Three heads can produce output *larger* than any single input and are capped at
the W-9 `MAX_LIST_LENGTH` / a mirrored character cap:

- **`StringJoin`** — output length is the sum of input lengths; the running total
  is accumulated with `checked_add` and the join is left unevaluated if it would
  exceed the cap, so a long chain cannot aim for an unbounded allocation.
- **`StringReplace`** — a replacement longer than its pattern grows the string per
  match; the output is bounded by the same cap, and an **empty pattern** (`"" ->
  x`, which would match at every position and between every char, an unbounded /
  quadratic expansion) is rejected and left unevaluated.
- **`Characters`/`StringSplit`** — both build a `List` whose length is bounded by
  the input character count, itself bounded by the W-4 input-size cap; a defensive
  `MAX_LIST_LENGTH` check mirrors the W-9 list builders.

`StringReplace`'s scan is **non-overlapping left-to-right** (advance past each
match by the pattern length), so it is linear in the input and terminates even
when the replacement contains the pattern (`"a" -> "aa"` does not re-scan the
inserted text). The other heads (`StringLength`, `StringTake`, `StringDrop`,
`ToString`) are size-non-increasing or bounded by their already-materialised
input and need no separate cap.

### §15.5 No grammar change

`StringJoin[…]`, `StringLength[…]`, `StringTake[…]`, `StringDrop[…]`,
`StringSplit[…]`, `StringReplace[…]`, `ToString[…]`, and `Characters[…]` are all
ordinary `Head[args]` applications. W-12 touches only `wolfram-runtime`'s builtin
handler table (and a one-line printer consideration for `ToString`); the lexer,
parser, and grammar files are untouched. The `<>` infix operator for `StringJoin`
is **deferred** to a future grammar-change lane item.

## §16 W-13 list set operations — `Union`, `Intersection`, `Complement`, `DeleteDuplicates`, `MemberQ`, `Tally` (implemented)

W-4..W-12 gave the M-expression core, the arithmetic bridge, operator sugar,
iteration, scoping, list *manipulation* (W-9: `Sort`/`Reverse`/`Join`/`Flatten`/
`Select`/`Count`/`Total`), functional combinators, pure functions, and string
builtins. W-13 completes the introductory list vocabulary with the **set / multiset**
operations, lowered onto the *same* substrate the rest of the lane already provides:

- the **W-9 list machinery** — `list_elements` to unwrap a `List(...)`, `apply(sym(LIST), …)` to rebuild one, the `MAX_LIST_LENGTH` DoS cap;
- the **W-9 canonical-order comparator** `canonical_cmp` — the documented total order that `Sort` introduced (numbers < symbols < strings < compound), reused both to *sort* the unique outputs of `Union`/`Intersection`/`Complement` and to define **element-equality** (two nodes are the *same element* iff `canonical_cmp` ranks them `Equal`). Because `canonical_cmp` is built on `f64::total_cmp`, equality is panic-free even for `NaN`, and the type-tag tie-break keeps `2` and `2.0` distinct elements (matching Wolfram, where `Union[{2, 2.}]` keeps both).

Like every head since W-5 these are plain `Head[args]` applications, so **there is
no grammar change**: W-13 touches only `wolfram-runtime`'s builtin handler table
(`builtins.rs`). `Count` (added in W-9 as a *predicate* count, `Count[list, EvenQ]`)
is left as-is; W-13's element membership and multiplicity needs are served by the
new `MemberQ` and `Tally` heads.

### §16.1 What W-13 adds

| Head                       | Meaning                                                                |
|----------------------------|------------------------------------------------------------------------|
| `Union[a, b, …]`           | **sorted**, duplicate-free union of the element lists                  |
| `Intersection[a, b, …]`    | **sorted** elements common to *all* argument lists                     |
| `Complement[all, x, …]`    | **sorted** elements of `all` not in any of `x, …`                      |
| `DeleteDuplicates[list]`   | first-occurrence-order dedup (**order-preserving**, *not* sorted)      |
| `MemberQ[list, elem]`      | `True`/`False` — is `elem` an element of `list`?                       |
| `Tally[list]`              | `{element, count}` pairs in **first-occurrence** order                 |

Worked examples (the W-13 acceptance tests):

```
Union[{1, 2}, {2, 3}]              (* → {1, 2, 3}          *)
Union[{3, 1, 2, 1}]                (* → {1, 2, 3}  sorted+unique *)
Intersection[{1, 2, 3}, {2, 3, 4}] (* → {2, 3}             *)
Complement[{1, 2, 3, 4}, {2, 4}]   (* → {1, 3}             *)
DeleteDuplicates[{3, 1, 1, 2, 3}]  (* → {3, 1, 2}  order kept   *)
MemberQ[{1, 2, 3}, 2]              (* → True               *)
MemberQ[{1, 2, 3}, 9]              (* → False              *)
Tally[{a, a, b, a}]                (* → {{a, 3}, {b, 1}}   *)
```

### §16.2 Ordering semantics — sorted vs order-preserving

The single subtlety of this lane is that the heads split into **two ordering
families**, both of which match Wolfram exactly:

- **Sorted outputs** (`Union`, `Intersection`, `Complement`): the result is
  *always* in `canonical_cmp` order, regardless of the input order, and free of
  duplicates. `Union[{3, 1, 2, 1}]` → `{1, 2, 3}` (re-sorted, deduped) — `Union`
  doubles as "sort-and-unique" of a single list.
- **Order-preserving output** (`DeleteDuplicates`, `Tally`): the *first occurrence*
  of each distinct element fixes its position; later duplicates are dropped (`DeleteDuplicates`)
  or counted (`Tally`). `DeleteDuplicates[{3, 1, 1, 2, 3}]` → `{3, 1, 2}` keeps the
  input order — deliberately *unlike* `Union`, which would sort to `{1, 2, 3}`.

`MemberQ` returns a boolean and has no ordering concern.

### §16.3 Element-equality — reusing the W-9 comparator

Every W-13 head needs to answer "are these two elements the same?". Rather than
introduce a second notion of equality, W-13 derives it from the W-9 comparator:
`same_element(a, b) ≡ canonical_cmp(a, b) == Equal`. This is the natural,
already-tested total order, so the answers are deterministic and consistent with
`Sort`. Consequences worth stating:

- distinct numeric subtypes with equal magnitude (`2` vs `2.0`) are **distinct**
  elements (the type-tag tie-break separates them) — matching Wolfram;
- a `NaN` float compares panic-free (via `total_cmp`), so a crafted `0./0.`-style
  literal can never crash a set operation;
- structural equality on compound elements (`f[1]` vs `f[1]`) is decided
  recursively by `canonical_cmp`, so symbolic elements unify correctly.

### §16.4 DoS surface — bounded outputs, bounded cost

W-13's heads never produce an output larger than the **sum of their input
lengths**, which the W-4 input/token caps already bound; the outputs are, in fact,
size-*non-increasing* relative to that sum (dedup only shrinks). Each head still
re-asserts the **W-9 `MAX_LIST_LENGTH`** cap on its result as a defensive,
mirrored bound:

- `Union` accumulates the deduped union and refuses (leaves the form unevaluated)
  the moment the accumulator would exceed `MAX_LIST_LENGTH` — symmetric with
  `Join`/`Flatten`.
- `Tally` caps its `{element, count}` pair list at `MAX_LIST_LENGTH` distinct
  elements.
- `Intersection`/`Complement`/`DeleteDuplicates` only ever *shrink* a single input,
  so they are bounded by their (already-capped) first argument; the cap is asserted
  anyway for symmetry.

Cost is **quadratic** in the worst case (each candidate is compared against the
running unique/result set via a linear `canonical_cmp` scan — no hashing, because
`IRNode` carries an `f64` and is not `Hash`-keyable by value). With every input
already bounded by `MAX_LIST_LENGTH` this is a deliberate, documented trade
(simplicity over a custom canonical-key index); the worst case is `O(n²)` element
comparisons over a million-element cap, which is acceptable for the interactive
scope and never unbounded.

### §16.5 The "I can't reduce this" contract — malformed input stays unevaluated

Following the W-5/W-9/W-12 convention, every W-13 handler returns the application
**unevaluated** (never panics) when its arguments are malformed: a non-`List`
argument to any head, the wrong arity, or an over-cap result all leave the original
`Head[args]` node untouched, so the caller sees the literal form rather than a
crash or a wrong answer. `MemberQ[3, 2]` (non-list first argument) → unevaluated;
`Union[1, {2}]` (a non-list among the arguments) → unevaluated.

### §16.6 No grammar change

`Union[…]`, `Intersection[…]`, `Complement[…]`, `DeleteDuplicates[…]`,
`MemberQ[…]`, and `Tally[…]` are all ordinary `Head[args]` applications. W-13
touches only `wolfram-runtime`'s builtin handler table; the lexer, parser, and
grammar files are untouched.

## §17 W-14 conditionals & predicates — `Which`, `Switch`, `Boole`, `NumberQ`/`IntegerQ`/`StringQ`/`ListQ`/`TrueQ` (implemented)

W-4..W-13 gave the M-expression core, the arithmetic bridge, operator sugar,
iteration, local scoping, the list/functional/string/set-operation builtins, and
pure functions. The one control structure that shipped early was the held `If`
(inherited unchanged from the shared `SymbolicBackend`). W-14 rounds out the
**conditional** vocabulary every introductory Wolfram tutorial reaches for —
multi-branch `Which`, structural `Switch`, the boolean-to-integer bridge `Boole` —
and the **type-predicate** family (`NumberQ`, `IntegerQ`, `StringQ`, `ListQ`,
`TrueQ`) that those conditionals are usually built on top of.

Like every head since W-5 these are all ordinary `Head[args]` applications, so
**there is no grammar change**: W-14 touches only `wolfram-runtime`'s builtin
handler table (`builtins.rs`) and the `WolframBackend` held set (`backend.rs`).

### §17.1 What W-14 adds

| Head        | Arity        | Held? | Result                                                                 |
|-------------|--------------|-------|------------------------------------------------------------------------|
| `Which`     | `2k` (k≥0)   | yes   | the value paired with the FIRST condition that evaluates to `True`     |
| `Switch`    | `2k+1` (k≥1) | yes   | the value paired with the first `form` that matches `expr` structurally |
| `Boole`     | 1            | no    | `1` for `True`, `0` for `False`, else unevaluated                       |
| `NumberQ`   | 1            | no    | `True` if the argument is a real/integer/rational/float number         |
| `IntegerQ`  | 1            | no    | `True` if the argument is an exact integer                             |
| `StringQ`   | 1            | no    | `True` if the argument is a string literal                             |
| `ListQ`     | 1            | no    | `True` if the argument is a `List[…]`                                   |
| `TrueQ`     | 1            | no    | `True` only if the argument is literally `True`; `False` for anything else |

Worked examples (the W-14 acceptance tests):

```
Which[False, 1, True, 2]            (* → 2  — first True condition's value      *)
Which[False, 1]                     (* → Null — no condition true              *)
Which[2 > 1, "a"]                   (* → "a"                                   *)
Switch[2, 1, "a", 2, "b", _, "z"]   (* → "b" — literal form 2 matches          *)
Switch[5, 1, "a", _, "z"]           (* → "z" — Blank default matches anything  *)
Boole[2 > 1]                        (* → 1                                     *)
Boole[1 > 2]                        (* → 0                                     *)
NumberQ[3]                          (* → True                                  *)
NumberQ["x"]                        (* → False                                 *)
IntegerQ[3]                         (* → True                                  *)
StringQ["x"]                        (* → True                                  *)
ListQ[{1, 2}]                       (* → True                                  *)
ListQ[3]                            (* → False                                 *)
TrueQ[True]                         (* → True                                  *)
TrueQ[5]                            (* → False                                 *)
```

### §17.2 Laziness — `Which`/`Switch` are HELD, only the selected branch evaluates

`Which` and `Switch` join the `WolframBackend` held set (alongside `If`, the W-7
iteration heads, and the W-8 scoping heads). Holding is **load-bearing for
correctness, not just efficiency**: a non-selected branch must never run.

* `Which[c1, v1, c2, v2, …]` — the handler evaluates `c1` through `vm.eval`; the
  first condition that reduces to the `True` symbol returns *its* `vi` (evaluated
  once, through the VM). Conditions are evaluated left-to-right and **stop at the
  first `True`** — later conditions and every non-selected `vi` are never touched.
  A condition that reduces to neither `True` nor `False` (an unresolved symbolic
  relation) is treated as not-yet-true and the scan continues; if no condition is
  `True`, `Which` returns `Null` (matching Wolfram). This mirrors the held `If`
  contract exactly: were `Which` eager, a guard like
  `Which[x > 0, 1/x, True, 0]` would evaluate `1/x` even when the guard is false.

* `Switch[expr, form1, v1, form2, v2, …]` — the handler evaluates `expr` once,
  then compares it against each **literal** `formi` (the forms are NOT evaluated:
  `Switch[2, 1+1, "a"]` does not match because the literal form is `Plus[1, 1]`,
  matching Wolfram's held-form semantics). The first `formi` that is structurally
  equal to the evaluated `expr` — or is a `Blank[]` (`_`), which matches anything —
  selects `vi`, which is then evaluated through the VM and returned. Only the
  selected `vi` is evaluated.

Held-head double-evaluation is avoided by construction: the args arrive
unevaluated, the handler chooses, and it calls `vm.eval` on exactly one branch.

### §17.3 Structural matching in `Switch` — reusing the W-13 element comparator

`Switch`'s form matching needs "is this evaluated `expr` the same as this literal
`form`?". Rather than introduce a third notion of equality, W-14 reuses the W-13
`same_element` comparator (built on the W-9 `canonical_cmp`), so `Switch` agrees
with `MemberQ`/`Union` on what "the same" means: `2` and `2.0` are distinct,
compound forms (`f[1]` vs `f[1]`) match recursively. The one extra rule is the
`Blank[]` default: a form that is `Blank[]` (the lowering of `_`) matches any
`expr`. A `Blank[h]` with a head constraint is treated as a plain catch-all in
this subset (the simple VM does not enforce head types) — the honest subset
behaviour, consistent with how W-4 dropped the blank's type constraint on
`SetDelayed` parameters.

### §17.4 Eager predicates — type tests over the IR node kinds

`Boole` and the five `…Q` predicates are **eager** (non-held): their single
argument is pre-evaluated by the VM before the handler runs, so
`IntegerQ[1 + 2]` sees `Integer(3)` and answers `True`. Each predicate is a thin
match over the `IRNode` kind:

* `NumberQ` — `Integer | Rational | Float` → `True`, else `False`.
* `IntegerQ` — `Integer` → `True`, else `False`.
* `StringQ` — `Str` → `True`, else `False`.
* `ListQ` — an `Apply` whose head is the `List` symbol → `True`, else `False`.
* `TrueQ` — the `True` symbol → `True`; **everything else** (including an
  unevaluated relation or `False`) → `False`. `TrueQ` is the total
  "is this definitely true?" test, so unlike the other predicates it never stays
  unevaluated on a symbol — `TrueQ[x]` is `False`.
* `Boole` — the `True` symbol → `1`, the `False` symbol → `0`, **anything else**
  → unevaluated (so `Boole[x]` echoes, matching Wolfram). This is the one W-14
  head that can stay unevaluated on a well-typed-but-non-boolean argument.

`EvenQ`/`OddQ` already shipped in W-9 and are left unchanged.

### §17.5 The "I can't reduce this" contract — malformed input stays unevaluated

Following the W-5/W-9/W-12/W-13 convention, every W-14 handler returns the
application **unchanged** rather than panicking when it cannot make progress:

* `Which` with an **odd** argument count (a dangling final condition with no
  paired value) is malformed → left unevaluated. (A well-formed `Which` whose
  conditions are all false/unresolved returns `Null`, which is the *evaluated*
  answer, not the unevaluated form.)
* `Switch` with an **even** argument count (a final `form` with no paired value,
  or a missing `expr`) is malformed → left unevaluated. `Switch` needs at least
  `expr` plus one `form, value` pair (arity ≥ 3, odd).
* The predicates and `Boole` with arity ≠ 1 → left unevaluated.

No W-14 head ever indexes past the end of its argument list, recurses without a
structural decrease, or evaluates a branch more than once, so there is no panic,
no unbounded recursion, and no double-evaluation surface. The held forms add no
new growth source beyond the single `vm.eval` of the one selected branch, which
the W-4 fuel/`If` machinery already bounds.

### §17.6 No grammar change

`Which[…]`, `Switch[…]`, `Boole[…]`, `NumberQ[…]`, `IntegerQ[…]`, `StringQ[…]`,
`ListQ[…]`, and `TrueQ[…]` are all ordinary `Head[args]` applications. W-14
touches only `wolfram-runtime`'s builtin handler table and the `WolframBackend`
held set; the lexer, parser, and grammar files are untouched.

## §18 W-15 numeric & integer math — `Abs`, `Sign`, `Min`/`Max`, `Floor`/`Ceiling`/`Round`, `Quotient`, `GCD`/`LCM`, `Sqrt` (implemented)

W-4..W-14 gave the M-expression core, the arithmetic bridge (`Add`/`Sub`/`Mul`/
`Div`/`Pow`/`Mod`/`Power`/`N`, inherited from the shared `SymbolicBackend`),
operator sugar, iteration, scoping, list manipulation and set ops, functional
combinators, pure functions, string functions, and conditionals/predicates.
W-15 adds the **numeric & integer math vocabulary** — the scalar functions a
beginner reaches for first: absolute value and sign, the min/max reductions,
the three rounding functions, integer division and `GCD`/`LCM`, and `Sqrt`.

Every W-15 head is an ordinary `Head[args]` application, so — like W-5/W-9/
W-12/W-13/W-14 — **there is no grammar change**: W-15 touches only
`wolfram-runtime`'s builtin handler table (`builtins.rs`). `Mod`, `Power`, and
`N` already exist (the inner `SymbolicBackend`'s arithmetic and the W-6 `N`
handler); W-15 does **not** duplicate them. `Sqrt` *is* overridden in the
Wolfram table (it precedes the inner backend in `handler_for`) so it gets
Wolfram-exact semantics — see §18.4.

### §18.1 What W-15 adds

| Head | Meaning | Example |
| --- | --- | --- |
| `Abs[x]` | absolute value (exact for integers, f64 for reals) | `Abs[-3]` → `3`, `Abs[-2.5]` → `2.5` |
| `Sign[x]` | −1 / 0 / +1 by sign | `Sign[-2]` → `-1`, `Sign[0]` → `0`, `Sign[2.5]` → `1` |
| `Min[a, b, …]` / `Min[{…}]` | least argument (or least list element) | `Min[3, 1, 2]` → `1`, `Min[{3, 1, 2}]` → `1` |
| `Max[a, b, …]` / `Max[{…}]` | greatest argument (or greatest list element) | `Max[3, 1, 2]` → `3`, `Max[{3, 1, 2}]` → `3` |
| `Floor[x]` | greatest integer ≤ x | `Floor[2.7]` → `2`, `Floor[-2.1]` → `-3` |
| `Ceiling[x]` | least integer ≥ x | `Ceiling[2.1]` → `3` |
| `Round[x]` | nearest integer, **half-to-even** | `Round[2.5]` → `2`, `Round[3.5]` → `4` |
| `Quotient[m, n]` | integer division toward −∞ | `Quotient[7, 2]` → `3`, `Quotient[-7, 2]` → `-4` |
| `GCD[a, b, …]` | greatest common divisor (≥ 0) | `GCD[12, 18]` → `6`, `GCD[12, 18, 24]` → `6` |
| `LCM[a, b, …]` | least common multiple (≥ 0) | `LCM[4, 6]` → `12` |
| `Sqrt[x]` | exact for perfect squares, else symbolic | `Sqrt[16]` → `4`, `Sqrt[2]` → `Sqrt[2]` |

### §18.2 Integer ops stay exact; real ops use f64

The single design rule mirrors the IR's own number representation
(`IRNode::Integer(i64)` vs `IRNode::Float(f64)`):

* If the argument(s) are **integers**, the result is exact (`IRNode::Integer`).
  `Abs[-3]` → `3`, `Min[3, 1, 2]` → `1`, `GCD[12, 18]` → `6` — never a float.
* If any argument is a **real** (`Float`), the result is computed in f64.
  `Abs[-2.5]` → `2.5`, `Min[2.5, 1.0]` → `1.0`.
* `Floor`/`Ceiling`/`Round` always return an **integer** (that is their job),
  whether the input is a real or already an integer; `Floor[5]` → `5`.

`Quotient`, `GCD`, and `LCM` are **integer-only** (Wolfram defines them over
integers in this subset); a non-integer argument leaves the form unevaluated.
`Abs`, `Sign`, `Min`, `Max` accept both integers and reals.

`Sign` is total: negative → `-1`, zero → `0`, positive → `1`. For a `Float`,
`Sign[-0.0]` → `0` (signed zero is treated as zero) and `Sign` of `NaN` (which
the parser cannot produce, but a computed intermediate could) leaves the form
unevaluated rather than guessing.

### §18.3 `Round` is half-to-even (banker's rounding)

Wolfram's `Round` rounds a tie (exactly `.5`) to the **nearest even integer**,
not always up:

```
Round[0.5] → 0     Round[1.5] → 2     Round[2.5] → 2     Round[3.5] → 4
```

This is the IEEE-754 "round half to even" / banker's rounding rule, chosen
because it has no upward bias over a stream of ties. Rust's `f64::round` rounds
half **away from zero** (`2.5_f64.round()` is `3.0`), so it is **not** usable
directly. W-15 implements half-to-even explicitly: round to nearest, and on an
exact tie pick the even neighbour. The implementation and its tests pin the
canonical cases `Round[2.5]` → `2` and `Round[3.5]` → `4`.

### §18.4 `Sqrt` — exact for perfect squares, otherwise symbolic

W-15 chooses the **symbolic** branch for non-perfect-squares (rather than
numericising eagerly):

* `Sqrt[n]` for a non-negative integer `n` that is a perfect square returns the
  exact integer root: `Sqrt[16]` → `4`, `Sqrt[0]` → `0`, `Sqrt[1]` → `1`.
* `Sqrt[n]` for a non-perfect-square non-negative integer is left **symbolic**:
  `Sqrt[2]` → `Sqrt[2]` (unevaluated). The float value is available on demand
  via `N[Sqrt[2]]` → `1.4142…` (the W-6 `N` handler numericises the wrapped
  `Sqrt`, which the inner `SymbolicBackend` then evaluates to a float).
* `Sqrt[x]` for a `Float` argument returns the f64 root (`Sqrt[2.0]` →
  `1.4142…`) — a real input already signals "I want a numeric answer".
* `Sqrt` of a negative integer or any non-numeric argument is left unevaluated
  (this subset has no complex numbers).

This override is necessary because the inner `SymbolicBackend`'s `Sqrt`
numericises `Sqrt[2]` to a float eagerly; the Wolfram table's handler precedes
it (per `handler_for`), restoring the exact-or-symbolic behaviour a Wolfram user
expects. The perfect-square test computes the integer `isqrt` and checks
`r * r == n`, with `r` taken in i128 so the squaring cannot overflow.

### §18.5 Overflow is guarded — exact integer math never wraps or panics

Exact integer ops are computed with **i128 intermediates and overflow guards**
so a crafted pair of large `i64` arguments can never wrap (silent corruption) or
panic (`overflow` in debug builds):

* **`Abs[i64::MIN]`** — `(-i64::MIN)` overflows i64 (its magnitude is one past
  `i64::MAX`). Computed as `(n as i128).abs()`; if the result does not fit back
  in i64 the form is left **unevaluated** rather than wrapping.
* **`Quotient[m, n]`** — floor-division `m.div_euclid` toward −∞, computed in
  i128 (so `Quotient[i64::MIN, -1]`, whose true value overflows i64, is refused
  / unevaluated rather than panicking); `Quotient[m, 0]` is undefined and stays
  unevaluated.
* **`GCD`** — computed in i128 with `unsigned_abs`-style magnitude handling so
  `GCD[i64::MIN, 0]` does not overflow on negation; the Euclidean loop is
  monotone-decreasing and terminates. The result is non-negative; if it does
  not fit in i64 (only `GCD[i64::MIN, …]` can produce a magnitude one past
  `i64::MAX`) the form is left unevaluated.
* **`LCM`** — `lcm(a, b) = |a / gcd(a, b) * b|` is the classic overflow trap
  (`a * b` overflows long before the LCM does). W-15 computes it in **i128**,
  divides by the gcd *first* (`a / g * b`, never `a * b / g`), and checks the
  product against the i64 range: an over-range LCM (`LCM` of two large coprime
  ints) is left **unevaluated** — it never wraps or panics. `LCM[…, 0]` is `0`
  (the Wolfram convention).

Every guard fails **closed**: the offending application echoes unevaluated,
exactly the W-5/W-9/W-12/W-13/W-14 "I can't reduce this" contract.

### §18.6 The "I can't reduce this" contract — malformed input stays unevaluated

Following the established convention, every W-15 handler returns the application
**unchanged** (echoed, never panicking) when its arguments are the wrong shape:
wrong arity (`Abs[1, 2]`, `Quotient[7]`), a non-numeric argument
(`Abs[x]`, `Min[x, 1]`, `GCD[x, 2]`), a non-integer where an integer is required
(`Quotient[2.5, 1]`, `GCD[1.5, 2]`), `Quotient`/`Mod` by zero, `Sqrt` of a
negative, or an overflow guard tripping (§18.5). No W-15 head ever panics,
divides by zero, or relies on debug-mode overflow checks for correctness.

### §18.7 No grammar change

`Abs[…]`, `Sign[…]`, `Min[…]`, `Max[…]`, `Floor[…]`, `Ceiling[…]`, `Round[…]`,
`Quotient[…]`, `GCD[…]`, `LCM[…]`, and `Sqrt[…]` are all ordinary `Head[args]`
applications. W-15 touches only `wolfram-runtime`'s builtin handler table; the
lexer, parser, and grammar files are untouched. `Mod`, `Power`, and `N` are
reused unchanged.

## §19 W-16 nested/structured list operations — `Transpose`, `Dimensions`, `Partition`, `Take`, `Drop`, `ConstantArray` (implemented)

W-9 (§12) gave the *flat* list-manipulation heads (`Sort`, `Reverse`, `Join`,
`Flatten`, …). W-16 adds the **structured/nested** vocabulary — the operations a
user reaches for once a list is treated as a *matrix* (a rectangular list of
rows) or once they need to slice it into pieces: transpose, dimension-reading,
block partitioning, prefix/suffix take and drop, and constant-array construction.

Every W-16 head is an ordinary `Head[args]` application, so — like
W-5/W-9/W-12/W-13/W-14/W-15 — **there is no grammar change**: W-16 touches only
`wolfram-runtime`'s builtin handler table (`builtins.rs`). All six reuse the W-9
`list_elements` accessor, the `apply(sym(LIST), …)` constructor, and the
`MAX_LIST_LENGTH` cap. `Flatten` already exists from W-9 and is **not**
reimplemented.

> **Take/Drop are the *list* heads, distinct from the W-12 string heads.** W-12
> shipped `StringTake`/`StringDrop` (which slice characters of a string). W-16's
> `Take`/`Drop` operate on **lists** and are registered under the bare names
> `Take`/`Drop`; they leave a non-list first argument unevaluated, so the two
> families never collide.

### §19.1 What W-16 adds

| Head | Meaning | Example |
| --- | --- | --- |
| `Transpose[m]` | transpose a rectangular list of lists (rows ↔ columns) | `Transpose[{{1,2},{3,4}}]` → `{{1,3},{2,4}}` |
| `Dimensions[e]` | dimensions of a rectangular nested list, as a list | `Dimensions[{{1,2,3},{4,5,6}}]` → `{2,3}`; `Dimensions[5]` → `{}` |
| `Partition[list, n]` | consecutive non-overlapping length-`n` blocks | `Partition[{1,2,3,4},2]` → `{{1,2},{3,4}}` |
| `Partition[list, n, d]` | length-`n` blocks stepping by `d` | `Partition[{1,2,3,4,5},2,1]` → `{{1,2},{2,3},{3,4},{4,5}}` |
| `Take[list, n]` / `Take[list, -n]` | first `n` / last `n` elements | `Take[{1,2,3,4,5},2]` → `{1,2}`; `Take[{1,2,3,4,5},-2]` → `{4,5}` |
| `Drop[list, n]` / `Drop[list, -n]` | drop first `n` / last `n` elements | `Drop[{1,2,3},1]` → `{2,3}`; `Drop[{1,2,3},-1]` → `{1,2}` |
| `ConstantArray[c, n]` | length-`n` list of `c` | `ConstantArray[0,3]` → `{0,0,0}` |
| `ConstantArray[c, {m, n}]` | `m`×`n` nested list of `c` | `ConstantArray[5,{2,2}]` → `{{5,5},{5,5}}` |

### §19.2 `Transpose` and `Dimensions` — the rectangular-matrix contract

`Transpose[m]` treats `m` as a list of equal-length rows and swaps the two
levels: row `i`, column `j` of the result is row `j`, column `i` of the input.
It requires a **rectangular** matrix — every row must be a list of the *same*
length, and there must be at least one row. A ragged matrix (rows of differing
length), a list whose elements are not all lists, an empty outer list, or a
non-list argument leaves `Transpose` **unevaluated** (the W-5/W-9 "I can't reduce
this" contract). The output element count equals the input element count, so
`Transpose` cannot grow its input and needs no separate cap.

`Dimensions[e]` returns the dimensions of the largest *rectangular* nested
array at the head of `e`, as a list:

* A scalar (non-list) → `{}` (rank 0).
* A flat list of `k` scalars → `{k}`.
* A rectangular `m`×`n` matrix → `{m, n}`; deeper uniform nesting extends the
  list (`{m, n, p, …}`).
* Ragged nesting stops the descent at the first non-uniform level — e.g.
  `Dimensions[{{1,2},{3}}]` → `{2}` (the two rows are not the same length, so
  the column dimension is not added). This matches Wolfram's behaviour of
  reporting only the rectangular prefix of the shape.

`Dimensions` reads structure only; it allocates a list at most as long as the
nesting depth (bounded by the token-capped input), so it has no DoS surface.

### §19.3 `Partition` — consecutive blocks, step `d`, trailing partial dropped

`Partition[list, n]` cuts `list` into consecutive **non-overlapping** sublists
of length `n` (step `d = n`). `Partition[list, n, d]` steps the window start by
`d` between blocks, so overlapping (`d < n`) or gapped (`d > n`) partitions are
expressible:

```
Partition[{1,2,3,4},2]      (* {{1,2},{3,4}}                  step d = n = 2 *)
Partition[{1,2,3,4,5},2,1]  (* {{1,2},{2,3},{3,4},{4,5}}      overlapping, d = 1 *)
Partition[{1,2,3,4,5},2]    (* {{1,2},{3,4}}   — trailing {5} is DROPPED *)
```

**Wolfram default — no padding.** A trailing block shorter than `n` is
**dropped** (the no-overflow form; this subset does not implement the padding
overload `Partition[list, n, d, …]`). `n` and `d` must be **positive integers**;
`n ≤ 0`, `d ≤ 0`, a non-integer `n`/`d`, or a non-list first argument leaves the
form unevaluated. The number of full blocks is `floor((len − n) / d) + 1` when
`len ≥ n`, else `0` (the empty list `{}`).

**Output cap.** The result holds `blocks` sublists of `n` elements each;
`blocks` (and `blocks × n`, the total element count) is checked against
`MAX_LIST_LENGTH` with `checked_mul` *before* allocating, so a tiny input cannot
request an over-cap partition — the form is left unevaluated instead.

### §19.4 `Take` / `Drop` — prefix and suffix slices of a list

`Take[list, n]` returns the first `n` elements; `Take[list, -n]` the last `n`.
`Drop[list, n]` returns the list with the first `n` removed; `Drop[list, -n]`
with the last `n` removed:

```
Take[{1,2,3,4,5},2]   (* {1,2} *)      Take[{1,2,3,4,5},-2]  (* {4,5} *)
Drop[{1,2,3},1]       (* {2,3} *)      Drop[{1,2,3},-1]      (* {1,2} *)
Take[{1,2,3},0]       (* {} *)         Drop[{1,2,3},0]       (* {1,2,3} *)
```

The count is an exact integer; its **magnitude must not exceed the list length**
(Wolfram errors on `Take[{1,2,3},5]` — out of range). An out-of-range count, a
non-integer count, or a non-list first argument leaves the form unevaluated.
Both shrink (or preserve) their input — they never grow it, so no separate cap
is needed. The count is range-checked in `i128` (so a crafted `i64::MIN` count
cannot overflow the front/back index arithmetic) and only converted to `usize`
*after* it is known to lie in `[0, len]`.

### §19.5 `ConstantArray` — the one *growing* head, output-capped before allocation

`ConstantArray[c, n]` builds a length-`n` list of copies of `c`;
`ConstantArray[c, {m, n}]` builds an `m`×`n` nested list (`m` rows, each a
length-`n` list of `c`). This is the only W-16 head whose output is **larger**
than its (tiny) input — the same allocation-amplification surface as `Range`
(§8.3) and `Table` (§10.3), and the **primary DoS concern** of W-16.

The dimensions must be **non-negative integers**. The total element count is
guarded *before* any allocation:

* `ConstantArray[c, n]`: `n` is checked against `MAX_LIST_LENGTH` (`n < 0` →
  unevaluated; `n > MAX_LIST_LENGTH` → unevaluated).
* `ConstantArray[c, {m, n}]`: the total `m × n` is computed with **`checked_mul`
  on i128**, and *both* the row count `m` and the total `m × n` are checked
  against `MAX_LIST_LENGTH`. An overflowing or over-cap product leaves the form
  unevaluated — **nothing is allocated**. This prevents `ConstantArray[0, {10^6,
  10^6}]` (a 12-byte input) from requesting a 10¹²-element array.

Negative or non-integer dimensions, the wrong arity, or a dimension spec that is
neither an integer nor a 2-element integer list leaves the form unevaluated.
(This subset supports rank-1 and rank-2 `ConstantArray`; higher-rank dimension
lists are out of scope and left unevaluated.)

### §19.6 No grammar change

`Transpose[…]`, `Dimensions[…]`, `Partition[…]`, `Take[…]`, `Drop[…]`, and
`ConstantArray[…]` are all ordinary `Head[args]` applications. W-16 touches only
`wolfram-runtime`'s builtin handler table; the lexer, parser, and grammar files
are untouched. `Flatten` is reused unchanged from W-9.

## §20 W-18 pattern-matching builtins — `MatchQ`, `Cases`, `FreeQ` (implemented)

W-14's `Switch` introduced the first taste of pattern matching: a `form_matches`
helper that decides whether an evaluated subject equals a literal form *or* is a
catch-all `Blank[]`. W-18 promotes that idea into three first-class
**pattern-matching predicates** every introductory Wolfram session reaches for:
`MatchQ` (does this expression match this pattern?), `Cases` (keep the list
elements that match), and `FreeQ` (does this form appear *anywhere* inside?).

Like every head since W-5 these are ordinary `Head[args]` applications — **no
grammar change**. W-18 touches only `wolfram-runtime`'s builtin handler table
(`builtins.rs`) and the `WolframBackend` held set (`backend.rs`).

### §20.1 What W-18 adds

| Head      | Arity | Held? | Result                                                                  |
|-----------|-------|-------|------------------------------------------------------------------------|
| `MatchQ`  | 2     | yes   | `True` if `expr` matches `patt`, else `False`                           |
| `Cases`   | 2     | yes   | the `List[…]` of `list`'s elements that match `patt` (non-matches dropped) |
| `FreeQ`   | 2     | yes   | `True` if `form` occurs **nowhere** in `expr` (recursively), else `False` |

Worked examples (the W-18 acceptance tests):

```
MatchQ[2, _]                        (* → True  — _ matches anything           *)
MatchQ[2, _Integer]                 (* → True  — 2 has head Integer           *)
MatchQ[2.0, _Integer]              (* → False — 2.0 has head Real, not Integer*)
MatchQ[2, 3]                        (* → False — literals differ              *)
MatchQ[2, 2]                        (* → True  — literals equal               *)
Cases[{1, 2, 3, 4}, _]              (* → {1, 2, 3, 4} — every element matches  *)
Cases[{1, 2, 3}, 2]                 (* → {2} — only the literal 2 matches      *)
Cases[{1, 2.0, 3}, _Integer]        (* → {1, 3} — 2.0 is Real, dropped         *)
FreeQ[{1, 2, 3}, 2]                 (* → False — 2 occurs                      *)
FreeQ[{1, 2, 3}, 5]                 (* → True  — 5 never occurs                *)
FreeQ[f[g[2]], g]                   (* → False — g occurs as the inner head    *)
FreeQ[f[g[2]], h]                   (* → True  — h occurs nowhere              *)
```

### §20.2 The supported-pattern subset (and the single match primitive)

W-18 deliberately ships a **small, solid** pattern vocabulary — exactly what the
W-14 matcher already understood, promoted to *enforce* the head constraint that
`Switch` treated as a no-op:

* **literal patterns** — any non-`Blank` node; matches iff it is *structurally
  equal* to the subject under the W-13 `same_element` comparator (so `2` and
  `2.0` stay distinct, `f[1]` matches `f[1]` recursively);
* **`_` (`Blank[]`)** — the catch-all; matches any subject;
* **head-typed blanks `_Integer`/`_Real`/`_Symbol`/`_h` (`Blank[h]`)** — matches
  iff the subject's *Wolfram head* is exactly `h`. The head of each atom is
  fixed: `Integer → Integer`, `Float → Real`, `Symbol → Symbol`, `Str → String`,
  `Rational → Rational`; a compound `Apply` reports its own head symbol (so
  `f[2]` matches `_f`). This is the **one divergence from W-14**: `Switch`
  treated `Blank[h]` as a plain catch-all (head ignored); W-18 *enforces* the
  head so `Cases[{1, 2.0, 3}, _Integer]` correctly drops the `Real`.

All three builtins funnel through a single match primitive, `pattern_matches`,
which is the W-14 `form_matches` extended with head-constraint enforcement. There
is no second matcher — `MatchQ` is a one-line wrapper, `Cases` filters a list
through it, and `FreeQ` walks every subnode through it.

### §20.3 `FreeQ` — the recursive walk and its depth bound

`FreeQ[expr, form]` answers "does `form` occur *nowhere* in `expr`?" by checking
`pattern_matches(form, node)` at **every** node of `expr` — the whole expression,
each `Apply` head, and every argument, recursively. The first match short-circuits
to `False`; if the walk completes with no match the answer is `True`.

The recursion is **depth-bounded** (`FREEQ_MAX_DEPTH`, 512) so a crafted deeply
nested input cannot overflow the stack: at the cap the walk reports "not free
here" conservatively rather than recursing further, and the surrounding tree is
already size-bounded by the parser's nesting cap and `MAX_LIST_LENGTH`. No new
allocation beyond the bounded traversal occurs; `Cases`/`MatchQ` iterate
already-materialised, `MAX_LIST_LENGTH`-bounded lists.

### §20.4 Held, with the subject evaluated; the pattern stays literal

`MatchQ`, `Cases`, and `FreeQ` join the `WolframBackend` held set (alongside
`If`, the W-7 iteration heads, the W-8 scoping heads, and the W-14 conditionals).
Holding keeps the **pattern** argument literal — `MatchQ[2, 1 + 1]` must match
against the *form* `Plus[1, 1]`, not the evaluated `2`, exactly as `Switch`
matches held forms (MA04 §17.2). Each handler then evaluates only the *subject*
(or list / expr) itself via `vm.eval`, never the pattern. `Blank[h]` survives
evaluation unchanged regardless (no `Blank` handler exists), but holding makes
the literal-pattern contract explicit and uniform with `Switch`.

### §20.5 The "I can't reduce this" contract

Following the established convention, each W-18 handler echoes the application
**unchanged** (never panicking) on malformed input: wrong arity (`MatchQ[2]`,
`Cases[{1}]`), or — for `Cases` — a non-list first argument (`Cases[5, _]` stays
unevaluated, since "the elements of `5`" is undefined). `MatchQ`/`FreeQ` accept
*any* first argument (an atom is a valid expression to test), so they always
reduce to `True`/`False`. No W-18 head panics on heterogeneous element
comparison — `same_element` is total and panic-free (built on `f64::total_cmp`).

### §20.6 Deferred to W-19 (explicitly out of scope)

W-18 ships the three core predicates over the *existing* matcher capability and
**defers** the rest of Wolfram's pattern algebra — none of the following is
supported, and each is left unevaluated / treated as a literal as appropriate:

* **named patterns** `x_` (`Pattern[x, Blank[]]`) — binding match captures;
* **alternatives** `a | b` (`Alternatives`);
* **conditions** `patt /; test` (`Condition`) and **`PatternTest`** `patt ? fn`;
* **`Repeated`** `patt..`, **`BlankSequence`** `__`, **`BlankNullSequence`** `___`;
* **replacement** `expr /. rules`, `Replace`, `ReplaceAll`, `ReplaceRepeated`.

These build on the same `pattern_matches` primitive but require capture binding,
backtracking, and rule application that balloon the matcher well beyond the W-18
core; they are scheduled for **W-19**.

> **W-19 update.** Named patterns (`x_`, `x_h`) *with binding* and replacement
> (`expr /. rules`, `ReplaceAll`, `Replace`, `Rule`, `RuleDelayed`) are now
> implemented — see **§21**. Alternatives, conditions, `PatternTest`, sequences
> (`__`/`___`), `Repeated`, level specs, and `ReplaceRepeated` (`//.`) remain
> deferred (now to **W-20**, §21.6).

### §20.7 No grammar change

`MatchQ[…]`, `Cases[…]`, and `FreeQ[…]` are ordinary `Head[args]` applications.
W-18 touches only `wolfram-runtime`'s builtin handler table and the held set; the
lexer, parser, and grammar files are untouched. The `Blank`/`Pattern` node
vocabulary is reused unchanged from `cas-pattern-matching`.


## §21 W-19 named patterns & replacement rules — `ReplaceAll` / `/.`, `Replace`, `Rule`/`RuleDelayed` (implemented)

W-18 shipped three *predicates* (`MatchQ`/`Cases`/`FreeQ`) over a deliberately
small matcher that **could not bind names** — a `Pattern[x, Blank[]]` (`x_`) fell
through to the literal branch and simply failed to match. W-19 closes that gap and
adds the half of Wolfram's pattern system everyone reaches for next: **named
capture** (`x_`, `x_h` bind `x` to whatever they match) and **replacement**
(`expr /. rule`, `Replace`, the `Rule`/`RuleDelayed` heads).

The grammar already tokenises `->` (`Rule`), `:>` (`RuleDelayed`), `/.`
(`ReplaceAll`), and `_` (`Blank`), and `lower.rs` already lowers `x_`/`x_h` to
`Pattern[x, Blank[]]`/`Pattern[x, Blank[h]]`, `_`/`_h` to `Blank[]`/`Blank[h]`,
and `a -> b` / `a :> b` to `Rule`/`RuleDelayed`. **No grammar change.** W-19 is
entirely in `wolfram-runtime` (the `builtins.rs` matcher + handler table, the
`ReplaceAll` pre-pass in `lib.rs`) reusing `cas-pattern-matching`'s
`match_pattern` (which already records bindings) and `substitute`.

### §21.1 What W-19 adds

| Head           | Form     | Arity | Held? | Result                                                                          |
|----------------|----------|-------|-------|---------------------------------------------------------------------------------|
| `ReplaceAll`   | `e /. r` | 2     | n/a*  | apply rule(s) to `e`, **top-down leftmost-outermost, single pass**              |
| `Replace`      | —        | 2     | n/a*  | apply rule(s) to `e` **as a whole only** (no sub-part descent)                  |
| `Rule`         | `a -> b` | 2     | —     | an immediate rewrite rule (the data; its RHS substitutes captured names)        |
| `RuleDelayed`  | `a :> b` | 2     | —     | a delayed rewrite rule — RHS held (not pre-evaluated) until substitution        |

*\*Replacement runs as an **IR-level pre-pass** (`apply_replace_all` in `lib.rs`),
before VM evaluation, then the substituted result is evaluated normally — so
`ReplaceAll`/`Replace` are not VM handlers and the "held?" column does not apply.
Holding the LHS pattern is automatic: there is no `Blank`/`Pattern`/`Rule` VM
handler, so those nodes survive to the pre-pass literally.*

W-18's `MatchQ`/`Cases`/`FreeQ` are **upgraded in place** to honour named patterns
(they now route through the binding matcher), so `MatchQ[2, x_] → True` and
`Cases[{1,2,3}, x_Integer] → {1, 2, 3}`.

Worked examples (the W-19 acceptance tests):

```
MatchQ[2, x_]                          (* → True  — a named blank matches anything        *)
f[2] /. f[x_] -> x                     (* → 2     — bind x=2, substitute into the RHS     *)
{1, 2, 3} /. 2 -> 99                   (* → {1, 99, 3} — literal rule, applied per element*)
x /. x -> 5                            (* → 5     — whole-expr literal match               *)
g[1, 2] /. g[a_, b_] -> a + b          (* → 3     — two captures; RHS evaluates (->)        *)
ReplaceAll[{1, 2, 3}, x_Integer -> x^2](* → {1, 4, 9} — typed named pattern, per element  *)
Replace[5, x_ -> x + 1]                (* → 6     — Replace matches the whole expr         *)
Replace[{1, 2, 3}, x_ -> 0]            (* → 0     — the WHOLE list matches x_ (no descent) *)
{a, b} /. {a -> 1, b -> 2}             (* → {1, 2} — a list of rules, first match per node *)
h[3] /. h[n_] :> n + 1                 (* → 4     — RuleDelayed: RHS held until substitute *)
```

### §21.2 Named-pattern binding — reusing `cas-pattern-matching::match_pattern`

The W-18 `pattern_matches(pattern, subject) -> bool` primitive is now a thin
wrapper over `cas_pattern_matching::match_pattern(pattern, subject,
Bindings::empty()).is_some()`. That matcher already implements *exactly* the W-19
vocabulary, with binding:

* `Blank[]` — matches anything (no binding);
* `Blank[h]` — matches iff the subject's effective head is `h`;
* `Pattern[name, inner]` — matches `inner`, then records `name → subject`; a
  repeated name must bind consistently (so `f[x_, x_]` only matches `f[2, 2]`);
* compound — head and args matched recursively (equal arity, no sequence
  wildcards);
* otherwise structural equality (`IRNode`'s `PartialEq`, which keeps `2 ≠ 2.0`
  exactly as W-13's `same_element` did, so every W-18 literal/`_Integer` test is
  preserved).

This is a strict superset of the old behaviour, so `MatchQ`/`Cases`/`FreeQ` gain
named patterns for free and keep all W-18 results.

**One head-name convention is reconciled.** Wolfram's `_Real` lowers to
`Blank[Real]`, but the shared (CAS-native) matcher names a floating-point node's
head `Float`. Before delegating, the wrapper rewrites a pattern's `Blank[Real]`
constraint to `Blank[Float]` (a pure structural copy, `wolfram_to_cas_pattern`),
so `MatchQ[2.0, _Real] → True` still holds. Every other head name — `Integer`,
`Rational`, `String`, `Symbol`, and any compound head `f` — already agrees
between the two crates, so `Real`/`Float` is the only translation.

### §21.3 `ReplaceAll` / `/.` — single top-down leftmost-outermost pass

The crux of W-19's *semantic* fix. Wolfram's `/.` does **one** top-down pass: at
each node it tries the rules in order; the **first** rule whose LHS matches the
*whole* node wins — its RHS (with captures substituted) **replaces** the node and
the pass does **not** descend into the result. If no rule matches the node, the
pass recurses into the head and each argument. This is *leftmost-outermost,
single-pass* — distinct from `ReplaceRepeated` (`//.`), which iterates to a fixed
point (deferred to W-20).

This corrects a real defect: the prior pre-pass called
`cas-pattern-matching::rewrite`, which is a **bottom-up fixed-point** rewriter that
re-walks each replacement. On `{1,2,3} /. x_Integer -> x^2` that loops forever
(`1` → `1^2` → folds to `1`, an `Integer`, which matches `x_Integer` again …),
hitting the iteration cap and erroring "did not converge". The single-pass
semantics replace each element exactly once: `{1^2, 2^2, 3^2}` → `{1, 4, 9}`.

Implementation: `replace_all_once(node, &rules)` tries `apply_rule_once` (match
LHS → substitute bindings into RHS) at the node; on a hit it returns the
substituted RHS **without recursing into it**; on a miss it rebuilds the node from
recursively-replaced children. A `List` of rules is tried left-to-right; the first
match per node wins.

### §21.4 `Replace` — whole-expression only

`Replace[expr, rule]` is `ReplaceAll` restricted to the **root**: it tries the
rule(s) against `expr` as a whole and never descends into parts. The *visible*
difference from `/.` needs a head-constrained pattern, because both are
**outermost-first**: `{1,2,3} /. x_ -> 0` and `Replace[{1,2,3}, x_ -> 0]` both
yield `0` — the whole `List[1,2,3]` matches the unconstrained `x_` at the root, so
`/.` never reaches the elements. With `x_Integer` the difference shows:
`{1,2,3} /. x_Integer -> 0 → {0, 0, 0}` (the list head is not `Integer`, so `/.`
descends and replaces each element) whereas `Replace[{1,2,3}, x_Integer -> 0]`
returns `{1, 2, 3}` unchanged (the whole list's head is `List`, not `Integer`, and
`Replace` does not descend). If no rule matches, `Replace` returns `expr`
unchanged. **Level specifications** (`Replace[expr, rule, levelspec]`) are
deferred to W-20; a third argument leaves the form unevaluated.

### §21.5 `Rule` / `RuleDelayed` and the immediate-vs-held distinction

A `Rule[lhs, rhs]` (`a -> b`) and a `RuleDelayed[lhs, rhs]` (`a :> b`) are both
*data* — they carry a pattern and a template. Because replacement is an IR-level
pre-pass that runs **before** VM evaluation, the RHS of *both* forms is held until
substitution in the same mechanical sense: nothing in the rule is evaluated until
its captures are filled in and the resulting expression is handed to the VM. The
distinction Wolfram draws — `->` evaluates its RHS once when the rule is built,
`:>` re-evaluates per match — does not change any W-19 acceptance result (every
example substitutes then evaluates once), so W-19 treats them identically at the
rewrite layer while preserving the two distinct heads end-to-end (the printer
round-trips `->`/`:>`). A bare symbol on the RHS that names a capture is rewritten
by `lower.rs` into the `Pattern[name, …]` reference shape `substitute` expands.

### §21.6 Safety — bounded single pass, no loops, no panics

* **Bounded recursion.** `replace_all_once` walks the expression tree, which is
  already bounded by the parser's per-statement token cap (so depth is bounded)
  and `MAX_LIST_LENGTH` (so breadth is bounded). The walk is depth-guarded by
  `REPLACE_MAX_DEPTH` (mirroring W-18's `FREEQ_MAX_DEPTH`): at the cap it stops
  descending and returns the sub-node unchanged, turning a crafted
  pathologically-nested input into a safe bounded answer rather than a stack
  overflow. The whole pre-pass also runs inside `lib.rs`'s bounded-stack worker
  thread with `catch_unwind` (MA04 §0 robustness), so any residual panic becomes a
  clean `Err`.
* **No unbounded expansion / no loops.** A *single* top-down pass that does not
  re-descend into substituted RHSes cannot loop the way the old fixed-point
  rewriter did — each node is visited at most once. There is no `MAX_REWRITE_ITERATIONS`
  iteration to exhaust.
* **No panic on unbound names.** `substitute` leaves an unbound `Pattern[name, …]`
  reference in place rather than panicking; a non-rule operand to `/.` yields an
  empty rule set, so the expression is returned unchanged. Heterogeneous compares
  go through total `PartialEq`.

### §21.7 Deferred to W-20 (explicitly out of scope)

W-19 ships named binding + the immediate single-pass replacement core and defers
the richer pattern algebra to **W-20**:

* **alternatives** `a | b` (`Alternatives`);
* **conditions** `patt /; test` (`Condition`) and **`PatternTest`** `patt ? fn`;
* **sequences** `__` (`BlankSequence`) / `___` (`BlankNullSequence`) and
  **`Repeated`** `patt..`;
* **level specifications** for `Replace` (the third argument);
* **`ReplaceRepeated`** `//.` (fixed-point iteration — the bottom-up `rewrite` the
  prior pre-pass used is the natural substrate, but it needs the loop bound and a
  convergence contract);
* **typed-named edge cases** beyond head constraints (e.g. `x_?test`).

### §21.8 No grammar change

Every W-19 surface form (`->`, `:>`, `/.`, `_`, `x_`) was already tokenised and
lowered before this item. W-19 touches only `wolfram-runtime` (the binding matcher
wrapper, the `Replace` handler, the single-pass `ReplaceAll` pre-pass) and reuses
`cas-pattern-matching`'s `match_pattern`/`substitute` unchanged. The lexer, parser,
and compiled grammar are untouched.


## §22 W-20 advanced pattern constructs — `Alternatives`, `Condition`, `PatternTest`, `ReplaceRepeated` (implemented)

W-20 grows the pattern algebra of §21 with four richer constructs. Like W-19, it
is **runtime-only**: the matcher wrapper [`pattern_match_bindings`] learns three
new pattern *heads*, and a new fixed-point replacement pre-pass handles the
fourth. The lexer, parser, and compiled grammar are **untouched** — see §22.6.

### §22.1 What W-20 adds

| construct           | head form (parseable today)        | meaning |
| ------------------- | ---------------------------------- | ------- |
| **Alternatives** `a\|b\|c` | `Alternatives[a, b, c]`     | matches iff the subject matches **any** alternative |
| **Condition** `patt /; test` | `Condition[patt, test]`   | matches iff `patt` matches **and** `test` (with the match's named bindings substituted) evaluates to `True` |
| **PatternTest** `patt ? fn` | `PatternTest[patt, fn]`    | matches iff `patt` matches **and** `fn[subject]` evaluates to `True` |
| **ReplaceRepeated** `expr //. rules` | `ReplaceRepeated[expr, rules]` | apply `ReplaceAll` repeatedly to a **fixed point**, capped at `REPLACE_REPEATED_MAX_ITERATIONS` |

Worked examples (the W-20 acceptance tests):

```
MatchQ[2, Alternatives[1, 2, 3]]                              (* True  *)
MatchQ[5, Alternatives[1, 2, 3]]                              (* False *)
Cases[{1,2,3,4}, Condition[Pattern[x, Blank[]], x > 2]]       (* {3, 4} *)
MatchQ[4, PatternTest[Blank[], EvenQ]]                        (* True  *)
MatchQ[3, PatternTest[Blank[], EvenQ]]                        (* False *)
ReplaceRepeated[{1, 2, 3}, Rule[2, 99]]                       (* {1, 99, 3} *)
ReplaceRepeated[1, Rule[Pattern[x,Blank[Integer]], 99]]       (* terminates at the cap, returns last form — never hangs *)
```

### §22.2 Alternatives — first-match-wins, zero new evaluation

`Alternatives[a, b, …]` matches the subject against each alternative **in order**,
returning the bindings of the first that matches (or `None` if none do). It calls
the same `pattern_match_bindings` recursively, so each alternative may itself be a
named pattern, a blank, a literal, or another advanced construct. No test is
evaluated, so it needs no VM. Backtracking is bounded: each alternative is tried
once, left to right — there is no cross-alternative combinatorial expansion.

### §22.3 Condition / PatternTest — bounded test evaluation through a fresh VM

Both evaluate a *test expression* and accept the match only when it yields the
`True` symbol:

* **Condition** first matches `patt`, then substitutes the captured **named**
  bindings (bare `Symbol("x")` occurrences, not `Pattern[…]` nodes) into `test`
  and evaluates it. `Cases[{1,2,3,4}, Condition[x_, x > 2]]` keeps `{3,4}`.
* **PatternTest** first matches `patt`, then evaluates `fn[subject]` — the
  *original* subject, not a binding. `MatchQ[4, PatternTest[_, EvenQ]]` is `True`
  via the W-9 `EvenQ` predicate.

The test runs through a **fresh, stateless** `WolframBackend`-backed VM
(constructed per evaluation), because these tests are pure (`x > 2`, `EvenQ[4]`)
and must not see or mutate session state. Evaluation goes through the normal
bounded VM, which carries the existing recursion/stack guards, so a crafted test
cannot recurse unboundedly. A test that does not reduce to `True` (anything
else — `False`, an unresolved relation, a free symbol) makes the match **fail**.

### §22.4 ReplaceRepeated — fixed point with a hard iteration cap

`ReplaceRepeated[expr, rules]` applies the §21.3 single-pass `replace_all_once`
**repeatedly**, evaluating between passes, until either (a) a pass produces a
result **identical** to its input (the fixed point — convergence) or (b) the
iteration count reaches `REPLACE_REPEATED_MAX_ITERATIONS`. The cap is the hard
DoS bound: a self-recursive rule like `ReplaceRepeated[x, x -> f[x]]` rewrites
forever, so without the cap it would never terminate / exhaust memory. At the cap
we **stop and return the last form** — no panic, no unbounded growth. Each
individual pass is still depth-guarded by `REPLACE_MAX_DEPTH` (§21.6), so both the
inner (per-pass tree depth) and outer (number of passes) loops are bounded.

### §22.5 Safety — bounded backtracking, bounded iteration, no panics

* **Bounded backtracking.** Alternatives tries each branch once, left to right;
  there is no exponential cross-product. The recursive matcher calls reuse the
  same depth discipline as §21.
* **Bounded test evaluation.** Condition/PatternTest evaluate through a fresh,
  stateless VM (no session state to corrupt). The runtime `VM::eval` has **no**
  intrinsic recursion-depth guard, so before evaluating a `Condition` test we
  bound its *size* with `node_count_within` against `REPLACE_GROWTH_NODE_CAP`:
  substitution can splice a captured (possibly deep) subject into the test once
  per reference (`Condition[x_, f[x, x, …]]`), building a tree deeper than the
  parser would ever have allowed — and an over-deep `VM::eval` is an *uncatchable*
  stack-overflow abort, not an `Err`. An over-cap test makes the condition fail.
* **Hard iteration cap AND growth cap.** `ReplaceRepeated` cannot loop forever
  (the `REPLACE_REPEATED_MAX_ITERATIONS` *pass* cap), **nor** grow without bound:
  each pass's rewritten tree is size-checked with `node_count_within` against
  `REPLACE_GROWTH_NODE_CAP` *before* it is evaluated. A branching rule like
  `x //. x -> f[x, x]` doubles the term per pass and would reach gigabytes / an
  un-evaluably-deep nesting long before the iteration cap; the growth cap stops it
  and returns the last in-bounds form. The size check is itself O(cap) — it stops
  counting the moment the cap is exceeded, never walking the whole exploding tree.
* **No panic on malformed nodes.** The `pattern_tree_well_formed` guard (§21.2)
  still rejects malformed `Pattern[…]` before any indexing; a malformed
  `Alternatives`/`Condition`/`PatternTest` (wrong arity) simply fails to match
  rather than panicking.

### §22.6 No grammar change in W-20 — the operator sugar lands in W-21 (§23)

W-20 ships the four constructs as **ordinary head applications**
(`Alternatives[…]`, `Condition[…]`, `PatternTest[…]`, `ReplaceRepeated[…]`),
which the existing parser already accepts as `NAME[args]`. The surface **operator
sugar** — `a | b` (`Alternatives`), `patt /; test` (`Condition`), `patt ? fn`
(`PatternTest`), and `expr //. rules` (`ReplaceRepeated`) — needs new lexer tokens
(`|`, `/;`, `?`, `//.`), parser precedence rules, lowering, and a regenerated
compiled grammar. To keep W-20 bounded and runtime-only (the W-19 precedent), that
grammar work is done in **W-21 (§23)**, lowering each operator to the W-20 head it
desugars to (the runtime heads are reused unchanged).

### §22.7 Deferred to W-21 (explicitly out of scope)

W-20 ships the core matching algebra (`Alternatives`/`Condition`/`PatternTest`)
plus fixed-point `ReplaceRepeated`, and defers:

* **operator sugar** for all four constructs (`|`, `/;`, `?`, `//.`) — needs a
  grammar change (new tokens + precedence + lowering + regenerated grammar). This
  ships in **W-21 (§23)**;
* **sequence patterns** `__` (`BlankSequence`) / `___` (`BlankNullSequence`) — the
  big one: variable-arity sequence matching is not yet supported by the shared
  `cas-pattern-matching` matcher, so the §22 `ReplaceRepeated` acceptance tests use
  non-sequence rules (e.g. `{1,2,3} //. 2 -> 99`); sequence support is deferred;
* **`Repeated`** `patt..` and **`Except`** `Except[p]`;
* **`Longest` / `Shortest`** sequence disambiguators;
* **level specifications** for `Replace` (the third argument).


## §23 W-21 pattern operator sugar — `a|b`, `patt /; test`, `patt ? fn`, `expr //. rules`

W-21 adds the **infix operator surface** for the four W-20 pattern constructs.
The runtime heads `Alternatives`, `Condition`, `PatternTest`, and
`ReplaceRepeated` already evaluate (§22) as ordinary `Head[args]` applications;
W-21 is **grammar + lowering only** — it lexes the operators, parses them at the
right precedence, and lowers each to the *existing* W-20 head. **No new runtime
evaluation logic is introduced**: the §22 matcher and the fixed-point
`ReplaceRepeated` pre-pass are reused unchanged.

### §23.1 What W-21 adds

| operator surface     | lowers to                       | reuses (§22) |
| -------------------- | ------------------------------- | ------------ |
| `a \| b \| c`         | `Alternatives[a, b, c]`         | §22.2 |
| `patt /; test`       | `Condition[patt, test]`         | §22.3 |
| `patt ? fn`          | `PatternTest[patt, fn]`         | §22.3 |
| `expr //. rules`     | `ReplaceRepeated[expr, rules]`  | §22.4 |

Worked examples (the W-21 acceptance tests):

```
MatchQ[2, 1 | 2 | 3]                 (* True  — Alternatives  *)
MatchQ[5, 1 | 2 | 3]                 (* False                 *)
Cases[{1, 2, 3, 4}, x_ /; x > 2]     (* {3, 4} — Condition    *)
MatchQ[4, _?EvenQ]                    (* True  — PatternTest   *)
MatchQ[3, _?EvenQ]                    (* False                 *)
{1, 2, 3} //. 2 -> 99                 (* {1, 99, 3} — ReplaceRepeated (terminating) *)
```

### §23.2 New tokens (lexer) — longest-match ordering

Four new tokens join `code/grammars/wolfram.tokens`:

| token             | lexeme | placement constraint |
| ----------------- | ------ | -------------------- |
| `REPLACEREPEATED` | `//.`  | **before** `MAP` (`/@`), `REPLACEALL` (`/.`), and `CONDITION` (`/;`) — else `//.` mis-lexes as `/` then `/.` |
| `CONDITION`       | `/;`   | **before** the bare `SLASH` (`/`) and `SEMI` (`;`) — else `/;` mis-lexes as `/` then `;` |
| `ALTERNATIVES`    | `\|`    | distinct from `OR` (`\|\|`, already first) — longest-match keeps `\|\|` winning |
| `PATTERNTEST`     | `?`    | a brand-new lexeme; no overlap with any existing token |

The lexer is longest-match-first by declaration order, so the multi-char
`//.` and `/;` are declared in SECTION 1 *ahead of* the `/`-family they share a
prefix with. `ALTERNATIVES` (`|`) is declared after `OR` (`||`) so `||` still
wins; `PATTERNTEST` (`?`) is a fresh single char in SECTION 2.

### §23.3 Precedence cascade and parser rules

Wolfram's precedence for these operators (loosest binds first → tightest last),
mapped into the existing W-1..W-11 cascade:

```
  replaceall    /.  //.            (left-assoc; //. sits beside /.)
  rule          ->  :>
  condition     /;                 (right-assoc; looser than | )
  alternatives  |                  (left-assoc, n-ary; looser than ||)
  logical_or    ||
  …                                (unchanged W-1..W-11 stack)
  mapapply      /@  @@
  patterntest   ?                  (left-assoc; tighter than /@, looser than postfix)
  postfix       f[…]  x[[…]]
```

So `?` binds **tighter** than `/;` (a `_?EvenQ` test attaches before the
condition), `|` is the alternatives layer between `/;` and `||`, and `//.` is a
low-precedence replace operator alongside `/.`. The grammar rules added (modelled
on the existing `replaceall`→`ReplaceAll` and `rule`→`Rule` rules):

```
replaceall   = rule        { ( REPLACEALL | REPLACEREPEATED ) rule } ;
rule         = condition    [ ( RULE | RULEDELAYED ) rule ] ;
condition    = alternatives  [ CONDITION condition ] ;
alternatives = logical_or   { ALTERNATIVES logical_or } ;
patterntest  = postfix      { PATTERNTEST postfix } ;
```

`replaceall` gains the `REPLACEREPEATED` arm; `rule` now recurses on `condition`;
`condition` (new) wraps `alternatives` (new), which wraps the old `logical_or`;
`patterntest` (new) is inserted between `mapapply` and `postfix` (so `power`'s
`mapapply` child stays, but `mapapply`'s child becomes `patterntest`).

### §23.4 Lowering — to the existing W-20 heads

`wolfram-runtime/src/lower.rs` gains four small handlers mirroring §22:

* `replaceall` folds left into `ReplaceAll` for `/.` and `ReplaceRepeated` for
  `//.` (inspecting the operator token between operands).
* `condition` emits `Condition[patt, test]`; like `rule`, named LHS pattern
  bindings are rewritten in the test so the matcher's `substitute` fills them in.
* `alternatives` folds its operands into one n-ary `Alternatives[a, b, …]`; a
  single operand passes through transparently.
* `patterntest` folds left into `PatternTest[patt, fn]`.

No new IR heads and **no new runtime evaluation** are introduced: each lowers to a
head that §22 already evaluates.

### §23.5 Safety

The new surface is purely grammar/parsing. The parser's existing nesting cap
bounds deeply-nested `a|b|c|…` / `patt /; t /; …` (no new recursion path escapes
it). `ReplaceRepeated` retains its §22.4 hard iteration cap
(`REPLACE_REPEATED_MAX_ITERATIONS`) and §22.5 growth cap
(`REPLACE_GROWTH_NODE_CAP`) — the `//.` operator lowers to the very same head, so
the DoS bounds apply identically whether written as operator or `Head[args]`.

## §24 W-22 the `cas-*` function surface under Wolfram names — `Simplify`, `Expand`, `Factor`, `D` (in progress)

W-22 starts closing §2's previously unnumbered "Future" item: wiring the
existing `cas-*` algorithm crates under Wolfram's own head names. Unlike
every prior W-item, this one lands **one head at a time** rather than as a
single PR, since each head is an independent algorithm with its own `cas-*`
crate dependency — bundling them would violate the one-PR-per-item
discipline for no benefit (they share no code with each other, only with
their respective `cas-*` crate).

### §24.1 `Simplify` (delivered)

`Simplify[expr]` is a thin call into `cas-simplify::simplify(expr,
max_iterations)` — **the exact function** Macsyma's own `simplify()` surface
function calls (`macsyma-runtime`'s `simplify_handler`), with the same
iteration cap (50). No algorithm is reimplemented; a test pins both
languages' call sites to agree on the same input, so this is not merely "the
same crate" but "the same result, verified."

Like every W-5+ built-in, `Simplify` is an ordinary eager `Head[args]` form
— its argument is evaluated before the handler runs — and requires exactly
one argument; any other arity leaves the form unevaluated (the fail-soft
contract every built-in here follows).

```
Simplify[x + 0]     (* x *)
Simplify[2 + 3]     (* 5 *)
```

### §24.2 `Expand` (delivered)

`Expand[expr]` is a thin call into `cas_simplify::expand(expr)` — **the
exact function** Macsyma's own `expand()` surface function calls
(`macsyma-runtime`'s `expand_handler`), including its internal
`EXPAND_MAX_POW`/`EXPAND_MAX_TERMS` DoS guards (a faithful port of the
Python reference's general recursive-distributor path: distributes `Mul`
over `Add`/`Sub`, expands bounded non-negative integer `Pow` via
square-and-multiply, guarded against the doubly-exponential term-count
blowup a multi-term base can hit). No algorithm or guard is reimplemented
for Wolfram; a test pins both languages' call sites to agree on the same
input, exactly like `Simplify`'s §24.1 parity test.

Like `Simplify` and every W-5+ built-in, `Expand` is an ordinary eager
`Head[args]` form requiring exactly one argument; any other arity leaves
the form unevaluated.

```
Expand[(x + 1)^2]           (* 1 + x + x + x*x *)
Expand[(x + 1) * (x + 2)]   (* 2 + x + 2*x + x*x *)
```

**Honest scope note (inherited from `cas-simplify`, not a Wolfram-specific
gap):** `expand` does not collect like terms — e.g. `Expand[(x+1)^2]`
produces `1 + x + x + x*x`, not the fully-collected `1 + 2*x + x^2` —
tracked as a separate follow-up (see `cas-simplify`'s own module docs).

### §24.3 `Factor` (delivered)

`Factor[expr]` is a direct call into `symbolic_vm::handlers::factor_handler`
— **the exact function** Macsyma's own `factor()` surface function calls
(`macsyma-runtime`'s `factor_handler`). Unlike `Simplify`/`Expand` (thin
calls into the standalone `cas-simplify` crate), `Factor`'s implementation
lives directly inside `symbolic-vm` itself, so this wiring reuses that
function directly rather than going through a separate `cas-*` crate;
`factor_handler` was made `pub` in `symbolic-vm` specifically for this
cross-language reuse (see that crate's own changelog). No algorithm is
reimplemented; a test pins both languages' call sites to agree on the same
input, exactly like `Simplify`/`Expand`'s own parity tests.

`factor_handler` itself factors a univariate integer polynomial (via
`cas_factor::factor_integer_polynomial`), or recognises one of a handful of
common multivariate patterns (perfect square/cube, difference of squares,
cubic identities, a common symbolic/integer term to pull out, bivariate and
n-variate Hensel lifting), falling back to the unevaluated form when none
apply — see `symbolic-vm`'s own module docs for the full algorithm
inventory (shared, not reimplemented, with Macsyma).

Like `Simplify` and `Expand`, `Factor` is an ordinary eager `Head[args]`
form requiring exactly one argument; any other arity leaves the form
unevaluated. That arity check lives inside `factor_handler` itself, so
(unlike `simplify_handler`/`expand_handler`, which must unwrap a single
expression argument themselves before calling a function that only takes
one bare expression) the Wolfram wiring needs no arity check of its own.

```
Factor[x^2 - 1]     (* (1 + x) * (-1 + x) *)
Factor[x + y]       (* x + y — unevaluated, no recognised pattern *)
```

### §24.4 `D` (delivered)

`D[expr, x]` is a thin call into `symbolic_vm::handlers::differentiate` —
**the exact pipeline** Macsyma's own `D` already runs via
`derivative_handler` (`macsyma-runtime`'s own dispatch). Unlike
`Simplify`/`Expand` (thin calls into the standalone `cas-simplify` crate),
the differentiation logic lives directly inside `symbolic-vm` itself, same
as `Factor`; `differentiate` was extracted out of `derivative_handler`'s
closure body and made `pub` in `symbolic-vm` specifically for this
cross-language reuse (see that crate's own changelog). No algorithm is
reimplemented; a test pins both languages' call sites to agree on the same
input, exactly like `Simplify`/`Expand`/`Factor`'s own parity tests.

`differentiate` applies the standard sum/product/quotient/power/chain
rules plus the elementary transcendental functions (`Sin`, `Cos`, `Exp`,
`Log`, `Sqrt`, the inverse and hyperbolic trig functions, …), fully
recursing `diff` and then running the result back through the VM's `eval`
so constant folding and any nested rule output collapse into one final
form — see `symbolic-vm`'s own module docs for the full rule inventory
(shared, not reimplemented, with Macsyma).

Unlike `Factor` — whose arity check lives inside `factor_handler` itself,
so the Wolfram wiring needs none of its own — `derivative_handler`'s
existing arity contract *panics* on the wrong argument count (a genuine
internal invariant for `symbolic-vm`'s own dispatch table, not a fail-soft
contract). Since `D` still needs Wolfram's usual "leave it unevaluated"
behaviour, `D`'s wrapper validates the shape itself before calling through:
exactly two arguments, and the second must be a bare symbol.

```
D[x^3, x]      (* 3 * x^2 *)
D[x^2, 2]      (* x^2 unevaluated — second argument isn't a symbol *)
```

### §24.5 `Integrate` (delivered, indefinite form only)

`Integrate[expr, x]` is a thin call into
`symbolic_vm::handlers::integrate_expr` — **the exact indefinite-integral
pipeline** `integrate_handler`'s own 2-argument branch runs, which is in
turn the same pipeline Macsyma's own `integrate` surface function already
calls. Unlike `Simplify`/`Expand` (thin calls into the standalone
`cas-simplify` crate), the integration logic lives directly inside
`symbolic-vm` itself, same as `Factor`/`D`; `integrate_expr` was extracted
out of `integrate_handler`'s closure body and made `pub` in `symbolic-vm`
specifically for this cross-language reuse (see that crate's own
changelog). No algorithm is reimplemented; a test pins both languages'
call sites to agree on the same input, exactly like
`Simplify`/`Expand`/`Factor`/`D`'s own parity tests.

`integrate_expr` tries shape-specific closed forms in sequence (polynomial
power rule, elementary transcendentals, the Weierstrass trig-rational
substitution, incomplete-elliptic recognition, …), falling back to a
generic tabular integration-by-parts sweep when none apply — see
`symbolic-vm`'s own module docs for the full algorithm inventory (shared,
not reimplemented, with Macsyma).

Like `D`, `Integrate`'s underlying handler (`integrate_handler`) has an
existing arity contract that *panics* on the wrong argument count (2 or 4
expected — a genuine internal invariant for `symbolic-vm`'s own dispatch,
not a fail-soft contract). Since `Integrate` still needs Wolfram's usual
"leave it unevaluated" behaviour, `Integrate`'s wrapper validates the shape
itself before calling through: exactly two arguments, and the second must
be a bare symbol.

Only the indefinite (2-argument) form is wired under the Wolfram name here.
The 4-argument definite-integral shape `integrate_handler` also supports
internally (this repo's flat `Integrate[f, x, a, b]` convention for the
complete-elliptic-integral recognisers) is deliberately not exposed yet —
real Wolfram spells a definite integral `Integrate[f, {x, a, b}]`, bounds
wrapped in a `List`, a different shape needing its own `List`-destructuring
wrapper as a follow-up rather than a same-shape passthrough.

```
Integrate[x^2, x]      (* x^3 / 3 *)
Integrate[x^2, 2]      (* x^2 unevaluated — second argument isn't a symbol *)
```

### §24.6 Remaining (not yet delivered)

`Solve`, and the rest of §2's original list — each is a separate future
item, no grammar change required (an ordinary `Head[args]` form the
existing grammar already parses).

### §6 References

Internal: [`HML00`](HML00-historical-math-languages-roadmap.md),
[`MA03`](MA03-maxima-language.md) (the Maxima reuse precedent),
`symbolic-ir`, `symbolic-vm`, `cas-pattern-matching`, `cas-simplify`,
`grammar-tools`.

External: Stephen Wolfram, *The Mathematica Book* / *An Elementary Introduction
to the Wolfram Language*; the Wolfram Language operator-precedence tables.
