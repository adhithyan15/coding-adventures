# wolfram-runtime

The **W-4** runtime of the Wolfram-language lane: it takes the parsed
M-expression AST from
[`wolfram-parser`](../wolfram-parser), **lowers** it to the shared
[`symbolic-ir`](../symbolic-ir) term representation, and **evaluates** it with
[`symbolic-vm`](../symbolic-vm) — reusing the same symbolic substrate that
Macsyma/Maxima drive rather than writing a bespoke evaluator.

See the spec: [`code/specs/MA04-wolfram-language.md`](../../../specs/MA04-wolfram-language.md)
§7 (W-4 runtime), §8 (W-5 built-ins), §9 (W-6 operator sugar), §10 (W-7
iteration constructs), and §11 (W-8 local scoping).

## What it does

```text
  Wolfram source
       │  wolfram-parser::parse            (W-3)
  GrammarASTNode  (additive, power, postfix, list, …)
       │  this crate: lower
  symbolic_ir::IRNode  (Add, Mul, Pow, List, Rule, …)
       │  ├─ ReplaceAll? → cas-pattern-matching::rewrite
       │  symbolic_vm::VM over SymbolicBackend
  symbolic_ir::IRNode  (evaluated)
       │  this crate: print
  Wolfram surface string  (infix, f[…], {…})
```

"Everything is `head[args]`" (Wolfram's defining idea) makes this a *lowering*,
not a translation: `2 + 3` is `Plus[2, 3]` is `Add(2, 3)`, which the
`SymbolicBackend` folds to `5`. The whole rewrite engine — numeric folding,
algebraic identities, the elementary-function handlers, user-defined functions —
is the *same* handler table Macsyma uses.

### The head-name bridge

The one subtlety: Wolfram's **surface** head names are not the IR's **canonical**
head names. The VM is keyed on `Add`/`Sub`/`Mul`/`Div`/`Pow`/`Neg`; Wolfram speaks
`Plus`/`Subtract`/`Times`/`Divide`/`Power`/`Minus`. The lowering bridges them in
both directions of entry — the infix operators *and* an explicit head-application
like `Plus[1, 2, 3]` map to the same IR head — so `1 + 2` and `Plus[1, 2]`
evaluate identically.

| Surface | IR head | | Surface | IR head |
|---------|---------|-|---------|---------|
| `+` `Plus` | `Add` | | `==` `Equal` | `Equal` |
| `-` `Subtract` | `Sub` | | `<` `Less` | `Less` |
| `*` `Times` | `Mul` | | `&&` `And` | `And` |
| `/` `Divide` | `Div` | | `\|\|` `Or` | `Or` |
| `^` `Power` | `Pow` | | `!` `Not` | `Not` |
| unary `-` | `Neg` | | `{…}` `List` | `List` |
| `=` `Set` | `Assign` | | `:=` `SetDelayed` | `Define` |

`Sin`/`Cos`/`Exp`/`Log`/`Sqrt`/… are already IR head names and pass through; an
unknown `f[…]` also passes through unevaluated (Mathematica semantics). Patterns
(`_`, `x_`, `_h`, `x_h`) and rules (`->`, `:>`) lower to the
[`cas-pattern-matching`](../cas-pattern-matching) node shapes, and `expr /. rules`
is run through that crate's `rewrite`.

## Usage

```rust
use coding_adventures_wolfram_runtime::{eval, WolframSession};

// One-shot:
assert_eq!(eval("1 + 2*3\n").unwrap(), "Out[1]= 7\n");
assert_eq!(eval("Power[2, 10]\n").unwrap(), "Out[1]= 1024\n");
assert_eq!(eval("{1 + 1, 2*3}\n").unwrap(), "Out[1]= {2, 6}\n");
assert_eq!(eval("x /. x -> 5\n").unwrap(), "Out[1]= 5\n");

// W-5 list / functional / control / numeric built-ins:
assert_eq!(eval("Length[{1, 2, 3}]\n").unwrap(), "Out[1]= 3\n");
assert_eq!(eval("Range[3]\n").unwrap(), "Out[1]= {1, 2, 3}\n");
assert_eq!(eval("Map[f, {1, 2}]\n").unwrap(), "Out[1]= {f[1], f[2]}\n");
assert_eq!(eval("Apply[Plus, {1, 2, 3}]\n").unwrap(), "Out[1]= 6\n");
assert_eq!(eval("Part[{a, b, c}, 2]\n").unwrap(), "Out[1]= b\n");
assert_eq!(eval("If[1 > 0, a, b]\n").unwrap(), "Out[1]= a\n");
assert_eq!(eval("N[1/2]\n").unwrap(), "Out[1]= 0.5\n");

// W-6 operator sugar — identical to the head forms above:
assert_eq!(eval("f /@ {1, 2}\n").unwrap(), "Out[1]= {f[1], f[2]}\n"); // Map
assert_eq!(eval("Plus @@ {1, 2, 3}\n").unwrap(), "Out[1]= 6\n");      // Apply
assert_eq!(eval("{a, b, c}[[2]]\n").unwrap(), "Out[1]= b\n");          // Part

// W-7 iteration constructs — bind a local index over a range:
assert_eq!(eval("Table[i^2, {i, 3}]\n").unwrap(), "Out[1]= {1, 4, 9}\n");
assert_eq!(eval("Sum[i, {i, 1, 10}]\n").unwrap(), "Out[1]= 55\n");
assert_eq!(eval("Product[i, {i, 1, 4}]\n").unwrap(), "Out[1]= 24\n");
assert_eq!(eval("Do[i, {i, 3}]\n").unwrap(), "Out[1]= Null\n");

// W-8 local scoping — bind named locals over a body (locals never leak):
assert_eq!(eval("With[{x = 3}, x^2]\n").unwrap(), "Out[1]= 9\n");
assert_eq!(eval("With[{a = 1, b = 2}, a + b]\n").unwrap(), "Out[1]= 3\n");
assert_eq!(eval("Module[{a = 1, b = 2}, a + b]\n").unwrap(), "Out[1]= 3\n");
assert_eq!(eval("Block[{x = 5}, x + 1]\n").unwrap(), "Out[1]= 6\n");

// Stateful (bindings and definitions persist):
let mut s = WolframSession::new();
s.feed("square[x_] := x^2;\n").unwrap();   // `;` suppresses display
assert_eq!(s.feed("square[5]\n").unwrap(), "Out[2]= 25\n");
```

## Built-ins

W-4 inherited arithmetic, comparisons, logic, `If`, lists-as-data, patterns/`/.`,
`Set`/`SetDelayed`, and the elementary functions from the shared
`SymbolicBackend`. **W-5** adds the list/functional/control/numeric built-ins via
a `WolframBackend` *decorator* — it answers those heads from a small table and
delegates everything else to the inner `SymbolicBackend`, so the change touches
only this crate (not `symbolic-vm`'s 50-dependent shared table) while reusing the
whole engine:

| Head | Example | Result |
|------|---------|--------|
| `Length` | `Length[{1,2,3}]` | `3` |
| `First` / `Last` | `First[{9,8}]` | `9` |
| `Part` | `Part[{a,b,c}, 2]` (1-based; `-1` = last; `0` = head) | `b` |
| `Append` | `Append[{1,2}, 3]` | `{1, 2, 3}` |
| `Range` | `Range[1,7,2]` | `{1, 3, 5, 7}` |
| `Map` | `Map[f, {1,2}]` | `{f[1], f[2]}` |
| `Apply` | `Apply[Plus, {1,2,3}]` | `6` |
| `If` | `If[1>0, a, b]` | `a` |
| `N` | `N[1/2]` | `0.5` |

`Map`/`Apply` route the head they build through the same `Plus`→`Add` bridge as
lowering, so `Apply[Plus, …]` sums. `First`/`Last`/`Part` on an empty list or an
out-of-range index, and `Range` of an oversize span (capped at
`MAX_RANGE_LENGTH = 1_000_000` *before* allocation), are left **unevaluated** —
never a panic, never an OOM.

**W-6** adds the operator *sugar* for three of these heads, desugared in lowering
to the exact same head so the results are byte-identical:

| Sugar | ≡ head form | Result |
|-------|-------------|--------|
| `f /@ x` | `Map[f, x]` | `{f[…], …}` |
| `f @@ x` | `Apply[f, x]` | e.g. `Plus @@ {1,2,3}` = `6` |
| `x[[i]]` | `Part[x, i]` | e.g. `{a,b,c}[[2]]` = `b` |

`[[ ]]` chains and nests (`{{1,2},{3,4}}[[1]][[2]]` = `2`, `x[[i, j]]` =
`Part[Part[x,i],j]`) and interleaves with `f[…]` application; `/@` and `@@` share
one left-associative precedence level (parenthesise when mixing them).

**W-7** adds the iteration constructs — the first forms that introduce a *scoped
local index*. Each binds a fresh `i` over a range and evaluates a body per value,
folded onto the same engine:

| Head | Example | Result |
|------|---------|--------|
| `Table` | `Table[i^2, {i, 3}]` | `{1, 4, 9}` |
| `Table` | `Table[i, {i, 2, 4}]` | `{2, 3, 4}` |
| `Do` | `Do[x = i, {i, 3}]` (runs 3×, side effects) | `Null` |
| `Sum` | `Sum[i, {i, 1, 10}]` | `55` (empty range → `0`) |
| `Product` | `Product[i, {i, 1, 4}]` | `24` (empty range → `1`) |

The iterator spec `{i, …}` accepts the same 1-/2-/3-bound forms as `Range`
(`{i, imax}`, `{i, imin, imax}`, `{i, imin, imax, di}`). The four heads are
**held** so the body and spec arrive unevaluated; each iteration binds `i → value`
with the *same* substitution that binds user-function parameters, so the index
stays local (no session leak) and nested `Table`s bind their own index cleanly.
The spec *bounds* are still evaluated (a bound may be `{i, 1+1}` or reference a
session binding). An over-large iterator is capped at `MAX_RANGE_LENGTH` *before*
allocation/looping (so `Table[0, {i, 2000000}]` stays unevaluated, never OOMs or
hangs), and a malformed spec (`{i}` with no bound, a zero step, a non-integer
bound) is left unevaluated rather than panicking. No grammar change — these are
ordinary `Head[args]` forms.

**W-8** adds the local-scoping heads — the generalisation of W-7's local index
into named locals over a body, lowered onto the same held-head + substitution
substrate:

| Head | Example | Result |
|------|---------|--------|
| `With` | `With[{x = 3}, x^2]` | `9` |
| `With` | `With[{a = 1, b = 2}, a + b]` | `3` |
| `Module` | `Module[{a = 1, b = 2}, a + b]` | `3` |
| `Block` | `Block[{x = 5}, x + 1]` | `6` |

All three are `Head[{decls}, body]` forms (no grammar change — `{x = e, …}` is an
ordinary list of `Set` nodes). They are **held** so the decl list and body arrive
unevaluated; the handler evaluates each decl's RHS, then binds the locals into a
*copy* of the body with the *same* `substitute` that binds W-7's index and
user-function parameters. Because the session environment is never touched, a
**local never leaks** (`x` is still free after `With[{x = 3}, x]`) and never
clobbers a same-named global. `With`/`Block` require every local initialised
(`name = value`); `Module` also accepts a bare `name`, which it α-renames to a
fresh gensym `name$nnn` (as real Wolfram does) so an uninitialised local stays
undefined and cannot capture a global. `Block`'s dynamic scope is approximated by
lexical substitution — observably identical to `With` for the self-contained
bodies this subset supports (see MA04 §11.3). A malformed form (a non-list decl
argument, a `With`/`Block` local with no value, a non-symbol assignment target,
the wrong arity) is left unevaluated rather than panicking.

A `;` at the end of a line suppresses that result's display (the notebook
convention) but the statement still runs and still advances the `Out[n]` counter.

## Robustness

`feed` is the trust boundary for the whole reused stack, so — mirroring
`maxima-runtime` — it guards against crafted input: an input-size cap
(`MAX_INPUT_LEN`), a per-statement token cap (`MAX_STATEMENT_TOKENS`, measured on
the real lexer token stream) that bounds parse-tree depth so deep nesting cannot
overflow the stack, and a bounded worker thread with `catch_unwind` plus
session-rebuild so a panic becomes a clean `Err` rather than a crash.

## Where it fits

- **W-1** spec + grammar, **W-2** `wolfram-lexer`, **W-3** `wolfram-parser`
  (all merged) — the frontend.
- **W-4** (this crate) — lowering + evaluation over the shared symbolic engine,
  plus [`wolfram-repl`](../wolfram-repl) (the interactive `wolfram`/`math` binary).
- **W-5** (this crate) — the list/functional/control/numeric built-ins above,
  added via the `WolframBackend` decorator.
- **W-6** (this crate) — the `/@`/`@@`/`[[ ]]` operator sugar (a lexer+grammar
  change), each desugaring to the W-5 `Map`/`Apply`/`Part` head.
- **W-7** (this crate) — the `Table`/`Do`/`Sum`/`Product` iteration constructs,
  iterator-bound evaluation over a local index (held heads + per-step
  substitution), DoS-capped like `Range`. No grammar change.
- **W-8** (this crate) — the `With`/`Module`/`Block` local-scoping heads, named
  locals bound into a held body via substitution (no session leak, no global
  clobber; `Module` gensym-renames uninitialised locals). No grammar change.
- **Future** — the full `cas-*` function surface under Wolfram names
  (`Simplify`, `Expand`, `Factor`, `Solve`, …).

## Testing

```sh
cargo test -p coding-adventures-wolfram-runtime
```
