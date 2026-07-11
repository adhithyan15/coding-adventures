# wolfram-runtime

The **W-4** runtime of the Wolfram-language lane: it takes the parsed
M-expression AST from
[`wolfram-parser`](../wolfram-parser), **lowers** it to the shared
[`symbolic-ir`](../symbolic-ir) term representation, and **evaluates** it with
[`symbolic-vm`](../symbolic-vm) — reusing the same symbolic substrate that
Macsyma/Maxima drive rather than writing a bespoke evaluator.

See the spec: [`code/specs/MA04-wolfram-language.md`](../../../specs/MA04-wolfram-language.md)
§7 (W-4 runtime), §8 (W-5 built-ins), §9 (W-6 operator sugar), §10 (W-7
iteration constructs), §11 (W-8 local scoping), §12 (W-9 list-manipulation
builtins), §13 (W-10 functional-iteration combinators), §14 (W-11 pure
functions), §15 (W-12 string builtins), §16 (W-13 list set operations), §17
(W-14 conditionals & predicates), §18 (W-15 numeric & integer math), §19
(W-16 nested/structured list operations + W-18 pattern-matching predicates), and
§21 (W-19 named patterns & replacement rules — `ReplaceAll`/`/.`, `Replace`,
`Rule`/`RuleDelayed`).

## What it does

```text
  Wolfram source
       │  wolfram-parser::parse            (W-3)
  GrammarASTNode  (additive, power, postfix, list, …)
       │  this crate: lower
  symbolic_ir::IRNode  (Add, Mul, Pow, List, Rule, …)
       │  ├─ ReplaceAll? → single top-down pass (cas-pattern-matching matcher)
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
[`cas-pattern-matching`](../cas-pattern-matching) node shapes; `expr /. rules` is
applied by a single top-down pass over that crate's binding matcher (W-19).

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

// W-9 list manipulation — reorder, concatenate, flatten, filter, count, sum:
assert_eq!(eval("Sort[{3, 1, 2}]\n").unwrap(), "Out[1]= {1, 2, 3}\n");
assert_eq!(eval("Reverse[{1, 2, 3}]\n").unwrap(), "Out[1]= {3, 2, 1}\n");
assert_eq!(eval("Join[{1}, {2, 3}]\n").unwrap(), "Out[1]= {1, 2, 3}\n");
assert_eq!(eval("Flatten[{{1, 2}, {3}}]\n").unwrap(), "Out[1]= {1, 2, 3}\n");
assert_eq!(eval("Select[{1, 2, 3, 4}, EvenQ]\n").unwrap(), "Out[1]= {2, 4}\n");
assert_eq!(eval("Count[{1, 2, 3, 4}, EvenQ]\n").unwrap(), "Out[1]= 2\n");
assert_eq!(eval("Total[{1, 2, 3}]\n").unwrap(), "Out[1]= 6\n");

// W-10 functional-iteration combinators — iterate a function:
assert_eq!(eval("Nest[f, x, 3]\n").unwrap(), "Out[1]= f[f[f[x]]]\n");
assert_eq!(eval("NestList[f, x, 2]\n").unwrap(), "Out[1]= {x, f[x], f[f[x]]}\n");
assert_eq!(eval("Fold[Plus, 0, {1, 2, 3}]\n").unwrap(), "Out[1]= 6\n");
assert_eq!(eval("FoldList[Plus, 0, {1, 2, 3}]\n").unwrap(), "Out[1]= {0, 1, 3, 6}\n");

// W-11 pure (anonymous) functions — named, or slot-based with the `&` postfix:
assert_eq!(eval("Function[x, x^2][5]\n").unwrap(), "Out[1]= 25\n");
assert_eq!(eval("Function[{x, y}, x + y][3, 4]\n").unwrap(), "Out[1]= 7\n");
assert_eq!(eval("(#^2)&[5]\n").unwrap(), "Out[1]= 25\n");      // # ≡ #1
assert_eq!(eval("(#1 + #2)&[3, 4]\n").unwrap(), "Out[1]= 7\n");
// …and they slot straight into the higher-order builtins:
assert_eq!(eval("Map[#^2 &, {1, 2, 3}]\n").unwrap(), "Out[1]= {1, 4, 9}\n");
assert_eq!(eval("Select[{1, 2, 3, 4}, Mod[#, 2] == 0 &]\n").unwrap(), "Out[1]= {2, 4}\n");
assert_eq!(eval("Nest[# + 1 &, 0, 3]\n").unwrap(), "Out[1]= 3\n");

// W-12 string builtins — concatenate, measure, slice, split, replace, render:
assert_eq!(eval("StringJoin[\"a\", \"b\", \"c\"]\n").unwrap(), "Out[1]= \"abc\"\n");
assert_eq!(eval("StringLength[\"héllo\"]\n").unwrap(), "Out[1]= 5\n"); // by char, not byte
assert_eq!(eval("StringTake[\"hello\", {2, 4}]\n").unwrap(), "Out[1]= \"ell\"\n");
assert_eq!(eval("StringSplit[\"a,b,c\", \",\"]\n").unwrap(), "Out[1]= {\"a\", \"b\", \"c\"}\n");
assert_eq!(eval("StringReplace[\"banana\", \"a\" -> \"o\"]\n").unwrap(), "Out[1]= \"bonono\"\n");
assert_eq!(eval("ToString[123]\n").unwrap(), "Out[1]= \"123\"\n");
assert_eq!(eval("Characters[\"ab\"]\n").unwrap(), "Out[1]= {\"a\", \"b\"}\n");

// W-13 list set operations — union, intersection, complement, dedup, membership, tally:
assert_eq!(eval("Union[{3, 1, 2, 1}]\n").unwrap(), "Out[1]= {1, 2, 3}\n"); // sorted + unique
assert_eq!(eval("Intersection[{1, 2, 3}, {2, 3, 4}]\n").unwrap(), "Out[1]= {2, 3}\n");
assert_eq!(eval("Complement[{1, 2, 3, 4}, {2, 4}]\n").unwrap(), "Out[1]= {1, 3}\n");
assert_eq!(eval("DeleteDuplicates[{3, 1, 1, 2, 3}]\n").unwrap(), "Out[1]= {3, 1, 2}\n"); // order kept
assert_eq!(eval("MemberQ[{1, 2, 3}, 2]\n").unwrap(), "Out[1]= True\n");

// W-14 conditionals & predicates — Which/Switch (held), Boole, type tests:
assert_eq!(eval("Which[False, 1, True, 2]\n").unwrap(), "Out[1]= 2\n");
assert_eq!(eval("Switch[2, 1, \"a\", 2, \"b\", _, \"z\"]\n").unwrap(), "Out[1]= \"b\"\n");
assert_eq!(eval("Boole[2 > 1]\n").unwrap(), "Out[1]= 1\n");
assert_eq!(eval("IntegerQ[3]\n").unwrap(), "Out[1]= True\n");
assert_eq!(eval("ListQ[{1, 2}]\n").unwrap(), "Out[1]= True\n");

// W-15 numeric & integer math — Abs/Sign, Min/Max, rounding, Quotient, GCD/LCM, Sqrt:
assert_eq!(eval("Abs[-3]\n").unwrap(), "Out[1]= 3\n");        // exact for integers
assert_eq!(eval("Sign[-2]\n").unwrap(), "Out[1]= -1\n");
assert_eq!(eval("Min[3, 1, 2]\n").unwrap(), "Out[1]= 1\n");   // also Min[{3, 1, 2}]
assert_eq!(eval("Round[2.5]\n").unwrap(), "Out[1]= 2\n");     // half-to-even: 2.5 → 2
assert_eq!(eval("Round[3.5]\n").unwrap(), "Out[1]= 4\n");     // …and 3.5 → 4
assert_eq!(eval("GCD[12, 18, 24]\n").unwrap(), "Out[1]= 6\n");
assert_eq!(eval("LCM[4, 6]\n").unwrap(), "Out[1]= 12\n");
assert_eq!(eval("Quotient[-7, 2]\n").unwrap(), "Out[1]= -4\n"); // toward −∞
assert_eq!(eval("Sqrt[16]\n").unwrap(), "Out[1]= 4\n");       // exact perfect square
// Sqrt[2] stays symbolic; the float is on demand via N:
assert_eq!(eval("Sqrt[2]\n").unwrap(), "Out[1]= Sqrt[2]\n");

// W-16 nested/structured list ops — transpose, dimensions, partition, take/drop, fill:
assert_eq!(eval("Transpose[{{1, 2}, {3, 4}}]\n").unwrap(), "Out[1]= {{1, 3}, {2, 4}}\n");
assert_eq!(eval("Dimensions[{{1, 2, 3}, {4, 5, 6}}]\n").unwrap(), "Out[1]= {2, 3}\n");
assert_eq!(eval("Partition[{1, 2, 3, 4}, 2]\n").unwrap(), "Out[1]= {{1, 2}, {3, 4}}\n");
assert_eq!(eval("Partition[{1, 2, 3, 4, 5}, 2, 1]\n").unwrap(),
    "Out[1]= {{1, 2}, {2, 3}, {3, 4}, {4, 5}}\n");  // step d = 1, overlapping
assert_eq!(eval("Take[{1, 2, 3, 4, 5}, -2]\n").unwrap(), "Out[1]= {4, 5}\n"); // last 2
assert_eq!(eval("Drop[{1, 2, 3}, 1]\n").unwrap(), "Out[1]= {2, 3}\n");        // drop first
assert_eq!(eval("ConstantArray[5, {2, 2}]\n").unwrap(), "Out[1]= {{5, 5}, {5, 5}}\n");

// W-18 pattern matching — MatchQ/Cases/FreeQ (held: the pattern stays literal):
assert_eq!(eval("MatchQ[2, _Integer]\n").unwrap(), "Out[1]= True\n");
assert_eq!(eval("MatchQ[2.0, _Integer]\n").unwrap(), "Out[1]= False\n"); // a float is _Real
assert_eq!(eval("Cases[{1, 2.0, 3}, _Integer]\n").unwrap(), "Out[1]= {1, 3}\n");
assert_eq!(eval("FreeQ[f[g[2]], g]\n").unwrap(), "Out[1]= False\n");     // g occurs nested
assert_eq!(eval("FreeQ[{1, 2, 3}, 5]\n").unwrap(), "Out[1]= True\n");

// W-19 named patterns & replacement — x_ binds, /. and Replace rewrite:
assert_eq!(eval("MatchQ[2, x_]\n").unwrap(), "Out[1]= True\n");           // a named blank matches anything
assert_eq!(eval("f[2] /. f[x_] -> x\n").unwrap(), "Out[1]= 2\n");        // bind x=2, substitute the RHS
assert_eq!(eval("g[1, 2] /. g[a_, b_] -> a + b\n").unwrap(), "Out[1]= 3\n");
assert_eq!(eval("ReplaceAll[{1, 2, 3}, x_Integer -> x^2]\n").unwrap(), "Out[1]= {1, 4, 9}\n");
assert_eq!(eval("Replace[5, x_ -> x + 1]\n").unwrap(), "Out[1]= 6\n");    // Replace matches the whole expr
assert_eq!(eval("h[3] /. h[n_] :> n + 1\n").unwrap(), "Out[1]= 4\n");     // RuleDelayed: RHS held

// W-20 advanced pattern constructs — head forms:
assert_eq!(eval("MatchQ[2, Alternatives[1, 2, 3]]\n").unwrap(), "Out[1]= True\n");
assert_eq!(eval("Cases[{1, 2, 3, 4}, Condition[Pattern[x, Blank[]], x > 2]]\n").unwrap(), "Out[1]= {3, 4}\n");
assert_eq!(eval("MatchQ[4, PatternTest[Blank[], EvenQ]]\n").unwrap(), "Out[1]= True\n");
assert_eq!(eval("ReplaceRepeated[{1, 2, 3}, Rule[2, 99]]\n").unwrap(), "Out[1]= {1, 99, 3}\n"); // fixed point

// W-21 operator sugar — the SAME four constructs written the Wolfram way
// (each lowers to the W-20 head above, so they evaluate identically):
assert_eq!(eval("MatchQ[2, 1 | 2 | 3]\n").unwrap(), "Out[1]= True\n");          // a | b   = Alternatives
assert_eq!(eval("Cases[{1, 2, 3, 4}, x_ /; x > 2]\n").unwrap(), "Out[1]= {3, 4}\n"); // p /; t = Condition
assert_eq!(eval("MatchQ[4, _?EvenQ]\n").unwrap(), "Out[1]= True\n");            // p ? fn  = PatternTest
assert_eq!(eval("{1, 2, 3} //. 2 -> 99\n").unwrap(), "Out[1]= {1, 99, 3}\n");   // e //. r = ReplaceRepeated

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

**W-9** adds the list-manipulation heads — reorder, concatenate, flatten, filter,
count, sum — lowered onto the same W-5 substrate (the list accessor, the
`Map`/`Apply` application path, the `Add` fold). All are eager `Head[args]` forms
(no grammar change, nothing held):

| Head | Example | Result |
|------|---------|--------|
| `Sort` | `Sort[{3, 1, 2}]` | `{1, 2, 3}` |
| `Reverse` | `Reverse[{1, 2, 3}]` | `{3, 2, 1}` |
| `Join` | `Join[{1}, {2, 3}]` | `{1, 2, 3}` |
| `Flatten` | `Flatten[{{1, 2}, {3}}]` | `{1, 2, 3}` |
| `Flatten` | `Flatten[{1, {2, {3}}}, 1]` | `{1, 2, {3}}` |
| `Select` | `Select[{1, 2, 3, 4}, EvenQ]` | `{2, 4}` |
| `Count` | `Count[{1, 2, 3, 4}, EvenQ]` | `2` |
| `Total` | `Total[{1, 2, 3}]` | `6` |
| `EvenQ` / `OddQ` | `EvenQ[4]` | `True` |

`Sort` uses a documented total canonical order over `IRNode` (numbers by
magnitude < symbols < strings < compound; stable, panic-free); pure-numeric lists
sort numerically. `Select`/`Count` apply `pred[e]` through the **same** path as
`Map`/`Apply` and keep/tally where it evaluates to `True`, so a built-in `EvenQ`, a
user `f[x_] := …` predicate, or any bridged head all work (function-predicate
`Count` is the documented simplification versus full pattern matching). `Total`
folds onto the canonical `Add` head, consistent with W-7 `Sum`. `Flatten` defaults
to flattening **all** levels; `Flatten[list, n]` flattens only the top `n` levels.
`Join`/`Flatten` outputs are DoS-capped at `MAX_LIST_LENGTH` (= `MAX_RANGE_LENGTH`,
1,000,000); the minimal `EvenQ`/`OddQ` parity predicates exist so `Select`/`Count`
are testable. Every malformed form (non-list, non-callable predicate, bad depth,
wrong arity) is left unevaluated rather than panicking.

**W-10** adds the functional-iteration combinators — the point-free heads that
iterate a *function*, lowered onto the same `Map`/`Apply` application path
(`build_canonical_application` + `vm.eval`) and the W-5 list accessor. All are
eager `Head[args]` forms (no grammar change, nothing held):

| Head | Example | Result |
|------|---------|--------|
| `Nest` | `Nest[f, x, 3]` | `f[f[f[x]]]` |
| `Nest` | `Nest[f, x, 0]` | `x` |
| `NestList` | `NestList[f, x, 2]` | `{x, f[x], f[f[x]]}` |
| `Fold` | `Fold[Plus, 0, {1, 2, 3}]` | `6` |
| `FoldList` | `FoldList[Plus, 0, {1, 2, 3}]` | `{0, 1, 3, 6}` |

`Nest[f, x, n]` applies `f` `n` times; `NestList` collects the `n + 1`
intermediates (seed first); `Fold` is a left fold seeded at `x0`; `FoldList`
collects the running accumulations (seed first). Each re-applies `f` through the
**same** path as `Map`/`Apply`, so a built-in (`Plus`), a bridged head, or a user
`SetDelayed` function (`g[a_] := a + 1; NestList[g, 0, 3]` → `{0, 1, 2, 3}`) all
work; a symbolic `f` builds the literal nest, and a non-callable `f` is *not* an
error (`Fold[f, 0, {1, 2}]` → `f[f[0, 1], 2]`). The iteration count `n` is
DoS-capped at `MAX_LIST_LENGTH` *before* iterating (so `Nest[f, x, 10^9]` cannot
drive a billion evals), and the `NestList`/`FoldList` result allocations are
bounded by that cap. Every malformed form (negative/non-integer/over-cap `n`,
non-list fold target, wrong arity) is left unevaluated rather than panicking.

**W-11** adds Wolfram's **pure (anonymous) functions** — the first runtime change
since W-5 to require a grammar + lexer change (new tokens `#`/`##`/`&`, a `slot`
atom, and a low-binding `amp` postfix level; the embedded `_grammar.rs` is
regenerated, not hand-edited). Three interchangeable spellings lower to one IR
shape:

| Surface | Lowers to | Applied → |
|---------|-----------|-----------|
| `Function[x, body]` | `Function[List[x], body]` | substitutes `x`→arg |
| `Function[{x,y}, body]` | `Function[List[x,y], body]` | substitutes both |
| `body &` | `Function[body]` (slot-based) | substitutes `Slot[k]`→argk |
| `#` ≡ `#1`, `#n` | `Slot[n]` | the n-th argument |
| `##` | `SlotSequence[1]` | splices *all* args |

The `&` has a **low precedence** (looser than every arithmetic/comparison
operator, tighter than `,`), so `#^2 &`, `# + 1 &`, and `Mod[#,2]==0 &` are all
pure functions of the *whole* body. Application is a **rewrite rule** on the
backend: it matches a *reducible* `Function[…][args]` and substitutes args →
params/slots via the **same `vm.rs::substitute`** user functions / `Table` /
scoping already use, then re-evaluates. Because the rule fires inside `vm.eval`,
it composes for free with `Map`/`Select`/`Nest` — they already re-apply `f`
through `build_canonical_application` + `vm.eval`, so `Map[#^2 &, {1,2,3}]` →
`{1, 4, 9}` with no special code in `Map`. Gating *reducibility in the predicate*
keeps an arity-mismatched/malformed form from re-matching and looping (it falls
through to `on_unknown_head` and stays unevaluated). The only new builtin W-11
needs is a minimal integer `Mod` (for the canonical `Mod[#,2]==0 &` predicate).

**W-12** adds the **string builtins** — concatenate, measure, slice, split,
replace, render — lowered onto the *same* substrate as everything above: the
string atom is already `IRNode::Str`, and `StringSplit`/`Characters` reuse the W-9
list machinery (and its `MAX_LIST_LENGTH` cap). All are eager `Head[args]` forms
(no grammar change, nothing held):

| Head | Example | Result |
|------|---------|--------|
| `StringJoin` | `StringJoin["a", "b", "c"]` | `"abc"` |
| `StringLength` | `StringLength["héllo"]` (by char, not byte) | `5` |
| `StringTake` | `StringTake["hello", 3]` | `"hel"` |
| `StringTake` | `StringTake["hello", {2, 4}]` (1-based inclusive) | `"ell"` |
| `StringTake` | `StringTake["hello", -2]` | `"lo"` |
| `StringDrop` | `StringDrop["hello", 2]` | `"llo"` |
| `StringSplit` | `StringSplit["a b  c"]` (whitespace) | `{"a", "b", "c"}` |
| `StringSplit` | `StringSplit["a,b,c", ","]` (separator) | `{"a", "b", "c"}` |
| `StringReplace` | `StringReplace["banana", "a" -> "o"]` | `"bonono"` |
| `ToString` | `ToString[123]` | `"123"` |
| `Characters` | `Characters["ab"]` | `{"a", "b"}` |

Every length, index, and slice operates on **Unicode by character** — each goes
through `chars().count()` / a `Vec<char>`, never a byte index — so a multi-byte
char (`é`, an emoji) counts as one and `StringTake`/`StringDrop` can never split a
UTF-8 boundary or panic (`StringTake["héllo", 2]` → `"hé"`). `StringJoin` and
`StringReplace` are DoS-capped at `MAX_STRING_LENGTH` (= `MAX_LIST_LENGTH`,
1,000,000); `StringReplace` rejects an **empty pattern** and scans non-overlapping
left-to-right (so `"a" -> "aa"` terminates). `ToString` reuses the `print_wolfram`
printer (a bare string renders unquoted: `ToString["hi"]` → `"hi"`). The `<>`
infix sugar for `StringJoin` is **deferred** to a future grammar-change item.
Every malformed form (non-string arg, out-of-range or `i64::MIN` index, malformed
rule) is left unevaluated rather than panicking — the W-5/W-9 fail-soft contract.

**W-13** adds the **list set / multiset operations** — union, intersection,
complement, dedup, membership, and tally — lowered onto the *same* substrate as
W-9: the list machinery (`list_elements`, `MAX_LIST_LENGTH`) and the W-9
canonical-order comparator `canonical_cmp`, reused both to *sort* the unique
outputs and to define **element-equality** (two nodes are the same element iff
`canonical_cmp` ranks them `Equal`). All are eager `Head[args]` forms (no grammar
change):

| Head | Example | Result |
|------|---------|--------|
| `Union` | `Union[{1, 2}, {2, 3}]` | `{1, 2, 3}` |
| `Union` | `Union[{3, 1, 2, 1}]` (sort + unique) | `{1, 2, 3}` |
| `Intersection` | `Intersection[{1, 2, 3}, {2, 3, 4}]` | `{2, 3}` |
| `Complement` | `Complement[{1, 2, 3, 4}, {2, 4}]` | `{1, 3}` |
| `DeleteDuplicates` | `DeleteDuplicates[{3, 1, 1, 2, 3}]` (order kept) | `{3, 1, 2}` |
| `MemberQ` | `MemberQ[{1, 2, 3}, 2]` | `True` |
| `MemberQ` | `MemberQ[{1, 2, 3}, 9]` | `False` |
| `Tally` | `Tally[{a, a, b, a}]` | `{{a, 3}, {b, 1}}` |

Two **ordering families**: `Union`/`Intersection`/`Complement` always return a
**sorted**, duplicate-free result (canonical order, regardless of input order),
while `DeleteDuplicates`/`Tally` are **order-preserving** — the first occurrence of
each distinct element fixes its position. The contrast is deliberate: on the same
input `{3, 1, 2, 1}`, `Union` gives `{1, 2, 3}` but `DeleteDuplicates` gives
`{3, 1, 2}`. Element-equality is `canonical_cmp`-derived, so it is deterministic,
panic-free for `NaN` (via `f64::total_cmp`), and keeps distinct numeric subtypes of
equal magnitude separate — `2` and `2.0` are distinct elements (`Union[{2, 2.}]`
keeps both), matching Wolfram. Outputs never exceed the sum of the (already-bounded)
input lengths and each head re-asserts the `MAX_LIST_LENGTH` cap. `IRNode` carries
an `f64` and so isn't `Hash`-keyable, but it *is* totally ordered (`canonical_cmp`),
so every head sorts once (O(n log n)) rather than scanning membership per element —
an earlier version of this file did the latter and was worst-case quadratic (fixed
in 0.19.1, see `CHANGELOG.md`). Every malformed form (non-list arg, wrong arity) is
left unevaluated rather than panicking — the W-5/W-9 fail-soft contract.

**W-14** adds the **conditionals** and **type predicates**. `Which` and `Switch`
are **held** (they join the `WolframBackend` held set alongside `If`, the W-7
iteration heads, and the W-8 scoping heads) so that **only the selected branch is
ever evaluated** — a non-taken branch (which might error or have a side effect)
never runs. `Switch` matches its evaluated subject against each **literal** form by
structural equality (reusing the W-13 `same_element` comparator) and treats
`Blank[]` (the lowering of `_`) as the catch-all default. `Boole` and the five
`…Q` predicates are eager thin matches over the `IRNode` kind. All are `Head[args]`
forms (no grammar change); `EvenQ`/`OddQ` (W-9) are unchanged.

| Head | Example | Result |
|------|---------|--------|
| `Which` | `Which[False, 1, True, 2]` | `2` |
| `Which` | `Which[False, 1]` (none true) | `Null` |
| `Switch` | `Switch[2, 1, "a", 2, "b", _, "z"]` | `"b"` |
| `Switch` | `Switch[5, 1, "a", _, "z"]` (Blank default) | `"z"` |
| `Boole` | `Boole[2 > 1]` / `Boole[1 > 2]` | `1` / `0` |
| `NumberQ` | `NumberQ[3]` / `NumberQ["x"]` | `True` / `False` |
| `IntegerQ` | `IntegerQ[3]` / `IntegerQ[2.0]` | `True` / `False` |
| `StringQ` | `StringQ["x"]` | `True` |
| `ListQ` | `ListQ[{1, 2}]` / `ListQ[3]` | `True` / `False` |
| `TrueQ` | `TrueQ[True]` / `TrueQ[5]` | `True` / `False` |

`Which` returns `Null` when no condition is true and is left unevaluated on an
**odd** argument count (a dangling final condition); `Switch` is left unevaluated on
no match or an **even** argument count (a final unpaired form). `TrueQ` is total —
`TrueQ[x]` of a free symbol is `False`, never unevaluated — while `Boole` of a
non-boolean argument stays unevaluated (`Boole[x]` echoes), matching Wolfram. The
held forms evaluate exactly one branch via a single `vm.eval`, so there is no
double-evaluation, no panic on a malformed pair list, and no new unbounded surface.

**W-15** adds the **numeric & integer math** functions. Integer ops stay
**exact** (i64, computed in i128 with overflow guards); real ops use f64 —
mirroring the IR's own `Integer`/`Float` split. `Mod`, `Power`, and `N` already
existed and are **not** duplicated; `Sqrt` is overridden in the Wolfram table
(which precedes the inner backend in `handler_for`) to give Wolfram's
exact-or-symbolic behaviour. All are `Head[args]` forms — no grammar change.

| Head | Example | Result |
|------|---------|--------|
| `Abs` | `Abs[-3]` / `Abs[-2.5]` | `3` / `2.5` |
| `Sign` | `Sign[-2]` / `Sign[0]` | `-1` / `0` |
| `Min` / `Max` | `Min[3, 1, 2]` / `Max[{3, 1, 2}]` | `1` / `3` |
| `Floor` / `Ceiling` | `Floor[-2.1]` / `Ceiling[2.1]` | `-3` / `3` |
| `Round` | `Round[2.5]` / `Round[3.5]` (half-to-even) | `2` / `4` |
| `Quotient` | `Quotient[7, 2]` / `Quotient[-7, 2]` | `3` / `-4` |
| `GCD` / `LCM` | `GCD[12, 18, 24]` / `LCM[4, 6]` | `6` / `12` |
| `Sqrt` | `Sqrt[16]` / `Sqrt[2]` | `4` / `Sqrt[2]` |

Two semantics worth pinning: **`Round` is half-to-even** (banker's rounding), so
`Round[2.5]` → `2` and `Round[3.5]` → `4` — Rust's `f64::round` rounds half away
from zero and is *not* used. **`Sqrt` is exact for perfect squares but otherwise
symbolic** — `Sqrt[2]` stays `Sqrt[2]`, and the float is available on demand via
`N[Sqrt[2]]` → `1.4142…`. Exact-integer ops are computed in **i128 with overflow
guards** so `Abs[i64::MIN]`, `Quotient[i64::MIN, -1]`, and an over-i64 `LCM` of
two large coprime integers all fail **closed** (left unevaluated) rather than
wrapping or panicking. Every malformed form (wrong arity, non-numeric or
non-integer argument, division by zero) is left unevaluated — the fail-soft
contract every head since W-5 follows.

**W-18** adds the **pattern-matching predicates** `MatchQ`, `Cases`, and `FreeQ`.
They are **held** (a new `PATTERN_HEADS` set folded into the `WolframBackend`
held set, alongside the W-7/W-8/W-14 held heads) so the **pattern** argument
arrives **literal** — a pattern is a *form*, not a value, exactly as `Switch`
relies on. Each handler evaluates **only its subject** and matches against the
literal pattern through a single panic-free `pattern_matches` primitive that
extends the W-14 `Switch` matcher by **enforcing** the `Blank[h]` head
constraint, reusing the W-13 `same_element` comparator for literals.

| Head | Example | Result |
|------|---------|--------|
| `MatchQ` | `MatchQ[2, _]` / `MatchQ[2, _Integer]` | `True` / `True` |
| `MatchQ` | `MatchQ[2, 3]` / `MatchQ[2.0, _Integer]` | `False` / `False` |
| `Cases` | `Cases[{1, 2, 3, 4}, _]` | `{1, 2, 3, 4}` |
| `Cases` | `Cases[{1, 2, 3}, 2]` / `Cases[{1, 2.0, 3}, _Integer]` | `{2}` / `{1, 3}` |
| `FreeQ` | `FreeQ[{1, 2, 3}, 2]` / `FreeQ[{1, 2, 3}, 5]` | `False` / `True` |
| `FreeQ` | `FreeQ[f[g[2]], g]` / `FreeQ[f[g[2]], h]` | `False` / `True` |

The **supported pattern subset** is deliberately small: a literal (structural
equality), `_` (`Blank[]`, the catch-all), and a head-typed `_h` (`Blank[h]`,
matching iff the subject's Wolfram head is `h`). The lowerer turns `_Integer` →
`Blank[Integer]`, `_Real` → `Blank[Real]`, `_Symbol` → `Blank[Symbol]`, and the
matcher's head map sends an `Integer` atom to head `Integer`, a `Float` atom to
head `Real`, and a symbol to head `Symbol` — so `MatchQ[2.0, _Integer]` is
`False` (a float is `_Real`, not `_Integer`). `FreeQ` recurses the whole
expression tree (the root, every `Apply` head, every argument) **depth-bounded**
(`FREEQ_MAX_DEPTH`) so a crafted over-deep input yields a safe bounded answer
rather than overflowing the stack; heterogeneous atom comparison is total and
never panics; result lists inherit the input's `MAX_LIST_LENGTH` bound. Wrong
arity, and a non-list first argument to `Cases`, are left **unevaluated**.

**W-19** adds **named patterns** and **replacement**. The matcher gains capture
binding by delegating `pattern_matches` to `cas_pattern_matching::match_pattern`,
so `x_` (`Pattern[x, Blank[]]`) and `x_Integer` bind whatever they match — and
`MatchQ`/`Cases`/`FreeQ` honour them (`MatchQ[2, x_]` → `True`). Replacement
applies `Rule`/`RuleDelayed` rules: `ReplaceAll` (`/.`) does a single **top-down
leftmost-outermost** pass (the new `replace_all_once`, replacing the prior
fixed-point rewriter that looped on rules like `x_Integer -> x^2`), while the new
held `Replace` head matches the **whole expression** only.

| Head / op | Example | Result |
|-----------|---------|--------|
| `MatchQ` (named) | `MatchQ[2, x_]` | `True` |
| `/.` (`ReplaceAll`) | `f[2] /. f[x_] -> x` | `2` |
| `/.` per element | `ReplaceAll[{1,2,3}, x_Integer -> x^2]` | `{1, 4, 9}` |
| `/.` two captures | `g[1, 2] /. g[a_, b_] -> a + b` | `3` |
| `Replace` (whole) | `Replace[5, x_ -> x + 1]` | `6` |
| `:>` (`RuleDelayed`) | `h[3] /. h[n_] :> n + 1` | `4` |

`ReplaceAll` recurses depth-bounded by `REPLACE_MAX_DEPTH`; the single pass cannot
expand unboundedly or loop, an unbound RHS capture is left in place (no panic),
and a non-rule operand returns the subject unchanged.

**W-20** adds the **advanced pattern constructs** (MA04 §22) — **runtime-only**,
shipped as ordinary head applications (`Alternatives[…]`, `Condition[…]`,
`PatternTest[…]`, `ReplaceRepeated[…]`), since the parser already accepts
`NAME[args]`. The operator sugar (`|`, `/;`, `?`, `//.`) needs a grammar change
and is **deferred to W-21**.

| Head | Example | Result |
|------|---------|--------|
| `Alternatives` | `MatchQ[2, Alternatives[1, 2, 3]]` | `True` |
| `Alternatives` | `MatchQ[5, Alternatives[1, 2, 3]]` | `False` |
| `Condition` | `Cases[{1,2,3,4}, Condition[Pattern[x, Blank[]], x > 2]]` | `{3, 4}` |
| `PatternTest` | `MatchQ[4, PatternTest[Blank[], EvenQ]]` | `True` |
| `ReplaceRepeated` | `ReplaceRepeated[{1, 2, 3}, Rule[2, 99]]` | `{1, 99, 3}` |
| `ReplaceRepeated` | `ReplaceRepeated[{1, 2}, {Rule[1, 2], Rule[2, 3]}]` | `{3, 3}` |

`Alternatives` tries each branch once (left to right — no combinatorial blowup);
`Condition`/`PatternTest` evaluate their test through a fresh, stateless VM with
the standard bounded evaluator (anything but `True` fails the match). The big
safety bound is on **`ReplaceRepeated`**: it iterates `ReplaceAll` to a fixed
point but with a **hard cap** (`REPLACE_REPEATED_MAX_ITERATIONS` = `2^16`), so a
self-recursive rule like `x -> f[x]` that never converges stops at the cap and
returns the last form — no hang, no panic, no unbounded memory.

The remaining pattern algebra — the operator sugar above, sequences `__`/`___`
(`BlankSequence`/`BlankNullSequence`), `Repeated`, `Except`, `Longest`/`Shortest`,
and `Replace` **level specs** — is **deferred to W-21** (MA04 §22.7).

**W-16** adds the **nested/structured list operations** — the *shape* vocabulary
for matrix-like nested lists, on top of the W-9 flat-list heads. All reuse the
W-9 list machinery (`list_elements`, `apply(sym(LIST), …)`, `MAX_LIST_LENGTH`).
`Take`/`Drop` here are the **list** heads — distinct from W-12's
`StringTake`/`StringDrop`, which keep operating on strings. `Flatten` already
exists (W-9) and is reused unchanged. All are `Head[args]` forms — no grammar
change.

| Head | Example | Result |
|------|---------|--------|
| `Transpose` | `Transpose[{{1, 2}, {3, 4}}]` | `{{1, 3}, {2, 4}}` |
| `Dimensions` | `Dimensions[{{1, 2, 3}, {4, 5, 6}}]` / `Dimensions[5]` | `{2, 3}` / `{}` |
| `Partition` | `Partition[{1, 2, 3, 4}, 2]` | `{{1, 2}, {3, 4}}` |
| `Partition` | `Partition[{1, 2, 3, 4, 5}, 2, 1]` | `{{1, 2}, {2, 3}, {3, 4}, {4, 5}}` |
| `Take` | `Take[{1, 2, 3, 4, 5}, 2]` / `Take[…, -2]` | `{1, 2}` / `{4, 5}` |
| `Drop` | `Drop[{1, 2, 3}, 1]` / `Drop[…, -1]` | `{2, 3}` / `{1, 2}` |
| `ConstantArray` | `ConstantArray[0, 3]` / `ConstantArray[5, {2, 2}]` | `{0, 0, 0}` / `{{5, 5}, {5, 5}}` |

`Transpose` requires a **rectangular** matrix (a ragged or non-matrix argument is
left unevaluated). `Partition` drops a trailing partial block (Wolfram default —
no padding) and steps the window by `d` (default `d = n`). `ConstantArray` is the
only **output-growing** head: its total element count is guarded *before*
allocation — 1-D `n` and 2-D `m × n` (computed with **`checked_mul`** on i128)
are both capped at `MAX_LIST_LENGTH`, so a tiny spec like
`ConstantArray[0, {10^6, 10^6}]` is refused rather than allocated. `Take`/`Drop`
range-check their (possibly negative) count in `i128` so a crafted `i64::MIN`
cannot overflow, and leave an out-of-range count unevaluated. Every malformed
form is left unevaluated — the fail-soft contract every head since W-5 follows.

A `;` at the end of a line suppresses that result's display (the notebook
convention) but the statement still runs and still advances the `Out[n]` counter.

**W-22** starts closing MA04 §2's previously unnumbered "Future" item — the
`cas-*` algorithm surface under Wolfram names. Each head is a thin call into
the existing shared `cas-*` crate, not a reimplementation:

| Head | Example | Result |
|------|---------|--------|
| `Simplify` | `Simplify[x + 0]` | `x` |
| `Simplify` | `Simplify[2 + 3]` | `5` |
| `Expand` | `Expand[(x + 1)^2]` | `1 + x + x + x*x` |

`Simplify` calls `cas-simplify`'s existing `simplify()`; `Expand` calls its
existing `expand()` (distributes `Mul` over `Add`/`Sub`, expands bounded
non-negative integer `Pow`, does **not** collect like terms — see
`cas-simplify`'s own docs). Both are the exact functions Macsyma's own
`simplify()`/`expand()` surface functions call — so Wolfram and Macsyma
agree on every result this crate can produce. Further heads (`Factor`,
`Solve`, `D`, `Integrate`, …) land one at a time, each its own item.

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
- **W-9** (this crate) — the `Sort`/`Reverse`/`Join`/`Flatten`/`Select`/`Count`/
  `Total` list-manipulation heads (plus `EvenQ`/`OddQ` predicates), lowered onto
  the W-5 list/`Map`/`Apply`/`Add` substrate, DoS-capped on `Join`/`Flatten`
  output. No grammar change.
- **W-10** (this crate) — the `Nest`/`NestList`/`Fold`/`FoldList` functional-
  iteration combinators, iterating a function through the W-5 `Map`/`Apply`
  application path, DoS-capped on the iteration count and result-list size. No
  grammar change.
- **W-11** (this crate) — pure (anonymous) functions: `Function[…]`, the slot
  forms `#`/`#n`/`##`, and the `&` postfix, applied via a backend rewrite rule
  that reuses `vm.rs::substitute`. Required a grammar + lexer change.
- **W-12** (this crate) — the `StringJoin`/`StringLength`/`StringTake`/
  `StringDrop`/`StringSplit`/`StringReplace`/`ToString`/`Characters` string
  builtins, Unicode-by-character, lowered onto the `IRNode::Str` atom + the W-9
  list machinery + the `print_wolfram` printer, DoS-capped on `StringJoin`/
  `StringReplace` output. No grammar change (`<>` infix deferred).
- **W-13** (this crate) — the `Union`/`Intersection`/`Complement`/
  `DeleteDuplicates`/`MemberQ`/`Tally` list set/multiset operations, lowered onto
  the W-9 list machinery and canonical-order comparator (reused for both sorting
  and element-equality), DoS-capped on `Union`/`Tally` output. Two ordering
  families — `Union`/`Intersection`/`Complement` sorted, `DeleteDuplicates`/`Tally`
  order-preserving. No grammar change.
- **W-14** (this crate) — the `Which`/`Switch` conditionals (held — only the
  selected branch evaluates; `Switch` matches literal forms via the W-13
  `same_element` comparator with a `Blank[]` default) plus the eager `Boole` and
  `NumberQ`/`IntegerQ`/`StringQ`/`ListQ`/`TrueQ` type predicates, all thin matches
  over the `IRNode` kind. No grammar change (`EvenQ`/`OddQ` from W-9 unchanged).
- **W-15** (this crate) — the `Abs`/`Sign`/`Min`/`Max`/`Floor`/`Ceiling`/`Round`/
  `Quotient`/`GCD`/`LCM`/`Sqrt` numeric & integer math functions. Integer ops
  stay exact (i64, i128 intermediates with overflow guards that fail closed); real
  ops use f64. `Round` is half-to-even; `Sqrt` is exact for perfect squares,
  otherwise symbolic (`N[Sqrt[2]]` for the float). `Mod`/`Power`/`N` reused
  unchanged; `Sqrt` overrides the inner backend's eager-numericising one. No
  grammar change.
- **W-18** (this crate) — the `MatchQ`/`Cases`/`FreeQ` pattern-matching
  predicates (held — the pattern argument stays literal), built on a single
  panic-free `pattern_matches` primitive that extends the W-14 `Switch` matcher
  to enforce `Blank[h]` head constraints and reuses the W-13 `same_element`
  comparator for literals. Supported subset: literal, `_` (`Blank[]`), head-typed
  `_h` (`Blank[h]`); `FreeQ`'s recursive tree walk is depth-bounded. No grammar
  change.
- **W-19** (this crate) — **named patterns** (`x_`, `x_h`, *binding*) and
  **replacement** (`ReplaceAll`/`/.`, `Replace`, `Rule`/`RuleDelayed`). The
  matcher delegates to `cas-pattern-matching::match_pattern` for capture binding
  (so `MatchQ`/`Cases`/`FreeQ` gain named patterns); `/.` does a single top-down
  leftmost-outermost pass (`replace_all_once`, fixing the prior fixed-point
  rewriter that looped on `x_Integer -> x^2`); the held `Replace` head matches the
  whole expression only. Depth-bounded, loop-free, panic-free. Alternatives /
  conditions / `PatternTest` / sequences / `Repeated` / `Replace` level specs /
  `ReplaceRepeated` (`//.`) deferred to W-20. No grammar change.
- **Future** — the rest of the `cas-*` function surface under Wolfram names
  (`Factor`, `Solve`, `D`, `Integrate`, …) — `Simplify` and `Expand` are
  delivered, see the W-22 section above.

## Testing

```sh
cargo test -p coding-adventures-wolfram-runtime
```
