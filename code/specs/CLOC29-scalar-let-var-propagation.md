# CLOC29 — propagating a scalar `let`/`var` to its use sites

Status: implemented
Issue: CCR-068 (#15837), CCR-022
Depends on: CLOC28 (literal property propagation)

## What this adds

`closure-pass-inline-variables` propagates a top-level **scalar** `let` or
`var` binding into its reads, under the closed-world (ADVANCED) gate:

```js
var x = 1;
if (x) { console.log(1); } else { console.log(2); }   // => console.log(1);

var a = null;
console.log(a ?? 2);                                   // => console.log(2);
```

No new folding is involved. Constant folding already reduces `1 ? A : B` and
`null ?? 2`; the pass only ever failed to put the literal there, because the
scalar path admitted `const` and nothing else. Closing that one gate closes
four ladder rungs.

## Why the `const`-only gate existed, and why lifting it is not simply safe

The scalar path decides how many sites to rewrite with `count_uses_*`. That
counter deliberately does **not** count a bare identifier in assignment
position, and for a `const` that is correct: a `const` cannot be assigned, so
the case cannot arise. Admit `var` on that counter and:

```js
var X = 1;
X = 2;
console.log(X);          // prints 2; the tally sees ONE use and folds to 1
```

This is the same trap CLOC28 hit from the other side — there the write was
invisible because `AssignmentTarget` is `#[serde(untagged)]`, so a scan keyed
on `"type":"Identifier"` never saw it. Two different mechanisms, one failure.

So CLOC29 does not lift the gate; it **routes around the counter**. A scalar
`let`/`var` candidate is marked `strong_proof` and goes through the same
whole-program proof CLOC28 introduced:

1. count every mention of the name over the serialized AST;
2. refuse if any of them is a write (assignment, update, `delete`, for-head);
3. refuse if the initializer mentions the name itself;
4. rewrite on a **clone**, and commit only if exactly one mention survives —
   the declaration's own binding target.

Over-counting only ever declines a candidate, so an AST shape nobody modelled
fails closed. `const` keeps the cheap counter route it has always used.

## The gate is closed-world, and that is a measured fact

Probed against `closure-compiler-v20260915`:

```text
                                  SIMPLE                      ADVANCED
var x=1; console.log(x)           var x=1;console.log(x);     console.log(1);
let x=1; console.log(x)           let x=1;console.log(x);     console.log(1);
```

At SIMPLE upstream does not substitute the value into a read. A top-level
binding is a property of the global object, and another script may observe or
replace it. So the propagation rides the same `closed_world` flag as CLOC28.

A subtlety worth recording, because it looks like a counter-example and is
not. At SIMPLE upstream *does* use a known value to pick a branch:

```text
var x=1; x?console.log(1):console.log(2)   SIMPLE =>  var x=1;console.log(1);
```

That is branch selection from a known condition, not value substitution — the
binding survives and no `x` is replaced anywhere. It is a separate capability
(`ladder_t4_if_else_simple` still records it as a gap) and CLOC29 does not
attempt it.

## Where we remain deliberately behind, and one shared blind spot

`eval` can write a binding from inside a string, where no AST scan can see it:

```js
var x = 1;
eval("x = 9");
console.log(x);        // node prints 9; we emit eval("x=9");console.log(1);
```

Upstream v20260915 emits **byte-identical** output, so this is exact parity
rather than a divergence — Closure at ADVANCED documents that it does not
support `eval` modifying local variables. It is recorded here so the next
reader does not rediscover it as a bug.

`with` is a different story, and an earlier draft of this spec got it wrong.
The oracle does reject `with` outright (`JSC_USE_OF_WITH`, exit 1) — but that
protects *upstream*, not us, because **closurec compiles `with`**. So the
exposure was ours alone:

```js
var x = 1;
function mk() { return JSON.parse('{"x":9}'); }
with (mk()) { console.log(x); }      // node prints 9; we folded x to 1
```

A `with` block resolves identifiers against a runtime object, so a name that
looks like a read of the binding may be a property of whatever `with` was
handed, and no static scan can tell. The pass now refuses to propagate
anything in a program containing a `WithStatement`. That guard covers the
`const` path too, where the same hole predated CLOC29.

The lesson generalises: "the oracle rejects this input" only removes exposure
when *we* reject it too. Parity arguments need both halves checked.

Function-*local* scalars are still not propagated (`ladder_t6_local_const_*`,
`ladder_t6_local_let_*` remain in the ledger): the pass collects only
top-level declarations. Note that widening it to locals would also need the
`closed_world` gate revisited, since upstream folds a function-local binding
at SIMPLE too — a local is closed-world wherever it appears.

## Rungs closed

| rung | before | after |
|---|---|---|
| `ladder_t4_if_else_advanced` | `var x=1;x?…` | `console.log(1);` |
| `ladder_t4_switch_advanced` | switch kept | `console.log(1);` |
| `ladder_t6_let_const_advanced` | `let a=1;…a+2` | `console.log(3);` |
| `ladder_t7_nullish_advanced` | `var a=null;a??2` | `console.log(2);` |

Ledger 64 → 60. `ladder_t6_let_const_advanced` was CCR-022's only sole-cause
rung, so that issue now has none.
