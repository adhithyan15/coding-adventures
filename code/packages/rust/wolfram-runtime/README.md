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
functions), and §15 (W-12 string builtins).

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
- **Future** — the full `cas-*` function surface under Wolfram names
  (`Simplify`, `Expand`, `Factor`, `Solve`, …).

## Testing

```sh
cargo test -p coding-adventures-wolfram-runtime
```
