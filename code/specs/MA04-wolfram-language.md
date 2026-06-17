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
- **W-6 — the `cas-*` function surface under Wolfram names** (`Expand`,
  `Factor`, `Solve`, `D`, `Integrate`, …) wired to the existing `cas-*` crates,
  plus the operator sugar deferred from W-5 (`/@`, `@@`, `[[ ]]`, and the `&&`,
  `||`, `<=`, `>=` *long-name* heads if any further grammar work is needed).

## §3 The supported surface (the grammar)

The W-1 grammar captures this subset of Wolfram syntax. Everything is desugared
to a `head[args]` form (shown in the right column) in W-4.

| Surface | Meaning | Lowers to |
|---------|---------|-----------|
| `123`, `1.5` | integer / real literal | `Integer` / `Float` |
| `"text"` | string literal | `Str` |
| `Sin`, `x`, `foo` | symbol (case-sensitive; built-ins are Capitalized) | `Symbol` |
| `f[a, b]` | function application (square brackets) | `f[a, b]` |
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
→ comparison → additive → multiplicative → unary minus → `Power` → application
`f[…]` → atoms. This matches Wolfram's operator precedences for the subset (e.g.
`x /. a -> b` is `ReplaceAll[x, Rule[a, b]]`; `a && b -> c` is
`Rule[And[a, b], c]`; `-x^2` is `Times[-1, Power[x, 2]]`).

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
- **The `[[ … ]]` `Part` *sugar*, `;;` `Span`**, `@`/`@@`/`/@`
  (prefix/apply/map), `&` pure functions and `#`/`#1` slots, `~f~` infix, `|`
  `Alternatives`, — the `Part[expr, i]`, `Map[f, list]`, and `Apply[f, list]`
  *head* forms ship in W-5 (§8); only the operator sugar (`[[ ]]`, `/@`, `@@`)
  is deferred to W-6.
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

## §6 References

Internal: [`HML00`](HML00-historical-math-languages-roadmap.md),
[`MA03`](MA03-maxima-language.md) (the Maxima reuse precedent),
`symbolic-ir`, `symbolic-vm`, `cas-pattern-matching`, `cas-simplify`,
`grammar-tools`.

External: Stephen Wolfram, *The Mathematica Book* / *An Elementary Introduction
to the Wolfram Language*; the Wolfram Language operator-precedence tables.
