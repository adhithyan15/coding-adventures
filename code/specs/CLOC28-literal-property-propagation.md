# CLOC28 — propagating an object/array literal's properties to its use sites

> **Status:** Proposed (conservative v1). Closes the four `tier 5` ladder rungs
> where a single-use object or array literal's scalar property is read and
> upstream folds the read to the value. Deliberately narrower than upstream's
> mechanism; see "How upstream does it, and why we do not (yet)".

## The gap

`closurec` emits the input for the shape upstream reduces to a constant:

| rung | input | upstream (ADVANCED) | `closurec` |
|---|---|---|---|
| `object` | `var o = { a: 1, b: 2 };console.log(o.a);` | `console.log(1);` | `var o={a:1,b:2};console.log(o.a);` |
| `nested_obj` | `var o = { a: { b: { c: 1 } } };console.log(o.a.b.c);` | `console.log(1);` | `var o={a:{b:{c:1}}};console.log(o.a.b.c);` |
| `array` | `var a = [1, 2, 3];console.log(a[0]);` | `console.log(1);` | `var a=[1,2,3];console.log(a[0]);` |
| `quoted_key` | `var o = { "a-b": 1 };console.log(o["a-b"]);` | `console.log(1);` | `var o={"a-b":1};console.log(o["a-b"]);` |

This is CCR-068, tracked on
[#15837](https://github.com/adhithyan15/coding-adventures/issues/15837). Of the
68 divergences in `tests/ladder/divergences.json`, 27 cite CCR-068 and 20 cite
it as their *only* cause — the largest single group. That 20 is not a promise
that one change closes 20 rungs: those 20 span at least four distinct
capabilities (literal-property reads, function inlining with argument
substitution, scalar propagation into a condition, and statement lowering such
as `a ||= 1` and `static {}`). This spec addresses the first group only.

## What v1 does

Extend `closure-pass-inline-variables`. When a binding's initializer is an
object or array literal, resolve member-access *chains* rooted at that binding
against the literal and replace the whole chain with the value it resolves to.

```js
// before
var o = { a: { b: { c: 1 } } };
console.log(o.a.b.c);

// after inline-variables — the chain resolves to the literal 1
var o = { a: { b: { c: 1 } } };   // now unreferenced …
console.log(1);                   // … and removed by remove-unused-vars
```

The binding is left alone; `remove-unused-vars` deletes it once nothing reads
it, exactly as it already does for the scalar `const` case. No new pass and no
new pipeline slot — `inline-variables` already runs between `inline` and
`remove-unused-vars` at both levels.

## Guards — measured, not assumed

Each guard below was checked against the pinned oracle
(`closure-compiler-v20260915`, sha256
`9c8af06056aa06f968b5a457540a85869c7ba2861c211c56d8d4ef6c35ddf36d`) at
`--compilation_level ADVANCED_OPTIMIZATIONS`.

1. **Every reference must be a readable member chain.** If the name appears
   anywhere that is not the object of a member read — passed to a call,
   assigned, returned, compared — the whole binding is skipped. Upstream agrees
   it must not propagate there:

   ```
   var o={a:1};window.f(o);console.log(o.a);   =>  var a={a:1};window.f(a);console.log(a.a);
   ```

   The object escapes, and upstream renames rather than folds.

2. **The resolved value must be a scalar literal.** Substituting an object or
   array literal at a use site would construct a *new* object there, changing
   identity (`o.a === o.a` must stay true). Only numbers, strings, booleans,
   `null` and bigints are substituted. A chain that bottoms out in an object is
   resolved *through*, not substituted.

3. **The key must be an own data property of the literal.** A key that is
   absent is NOT folded to `undefined`, because the read may resolve up the
   prototype chain. Upstream is careful here too:

   ```
   var o={a:1};console.log(o.b);              =>  console.log({}.b);
   var o={a:1};console.log(typeof o.toString);=>  console.log(typeof{}.toString);
   ```

   It empties the literal of unread properties but keeps the access.

4. **No accessors.** A `get`/`set` property (`PropertyKind::Get`/`Set`) makes
   the read a function call. We skip the binding. (Upstream *does* fold
   `{get a(){return 1}}.a` to `1` at ADVANCED; matching that needs
   side-effect analysis of the getter body, which v1 does not have.)

5. **No mutation.** Any assignment to the binding or to a property of it
   (`o.a = 2`, `delete o.a`) skips the binding.

6. **Arrays: integer-literal index, in range, element present.** `a[0]` on
   `[1,2,3]` resolves. `a[i]` does not — the index is not known.

7. **The existing shadow and TDZ guards still apply** — the name must be
   declared exactly once in the whole program, unchanged from the scalar case.

Guards 1-5 are soundness: crossing any of them changes what the program does,
and upstream declines there too (probes above).

## Where v1 stays behind upstream on purpose

These are *not* guards in the same sense. Upstream folds them and v1 does not,
so the rung stays divergent — v1 neither closes nor widens these. Recorded here
so the next person does not read the guard list as a parity claim:

```
var a=[1,2,3];console.log(a.length);        =>  console.log(3);
var a=[1,2];console.log(a[5]);              =>  console.log(void 0);
var a=[1,,3];console.log(a[1]);             =>  console.log(void 0);
var x={b:2},o={...x,a:1};console.log(o.a);  =>  console.log(1);
var o={get a(){return 1}};console.log(o.a); =>  console.log(1);
```

`.length` and an out-of-range or hole index need array-shape reasoning beyond
"read element *n*"; spread needs the spread source resolved first; the getter
needs side-effect analysis of its body. Each is a follow-up, not a v1 guard.

The identity guard (2) is the one place a *wrong* fold is easy and upstream
shows the correct answer plainly:

```
var o={a:{b:1}};console.log(o.a===o.a);     =>  var a={};console.log(a===a);
```

It keeps one binding rather than substituting the inner object twice, because
two substituted literals would be two distinct objects and `===` would flip
from `true` to `false`.

A duplicate key (`{a:1,a:2}`) needs no guard: upstream *refuses* the program at
ADVANCED with `JSC_DUPLICATE_MEMBER`, so no correct output exists to match. We
must merely not crash.

## How upstream does it, and why we do not (yet)

Upstream does not propagate the literal. It runs `CollapseProperties`, which
flattens the object into one scalar binding per property, and the existing
scalar `InlineVariables` then propagates those:

```
var o={a:1};o.a=2;console.log(o.a);   =>  var a=1;a=2;console.log(a);
```

The binding survives as a *scalar* and the assignment with it. Our v1 bails on
that input (guard 5) and leaves it unchanged, so `mutated`-shaped programs stay
divergent — a gap this spec does not close and does not widen.

Two consequences worth recording:

* `closure-pass-collapse-properties` in this repo is **not** upstream's
  `CollapseProperties`. Ours caches a repeated access chain
  (`ns.utils.format`) into one local binding, a CSE-style size win. Upstream's
  flattens a namespace object into separate globals. The names collide; the
  transforms do not. Ours is also not registered in `run.rs`.
* Implementing upstream's version later would subsume this pass's object
  handling. v1 is scoped so that removing it is a deletion, not an unpick: the
  chain-resolution lives behind one entry point and adds no new pipeline slot.

## Acceptance

* `ladder_t5_object_advanced`, `ladder_t5_nested_obj_advanced`,
  `ladder_t5_array_advanced` and `ladder_t5_quoted_key_advanced` match the
  oracle byte-for-byte, and their entries are **deleted** from
  `tests/ladder/divergences.json` — the ladder gate fails if a recorded gap
  starts matching, so deleting the entry is part of the change, not follow-up.
* No other ladder rung changes state in either direction.
* No fixture golden under `tests/diff/` changes.
* `inline-variables` reports `changed = false` when it substitutes nothing.
  Under `IterationPolicy::FixedPoint` a pass that claims a change without
  making one re-runs forever; see the `changed=true` lesson in `lessons.d/`.
