# CLOC30 — resolving an optional chain against an object/array literal

Status: implemented
Issue: CCR-068 (#15837)
Depends on: CLOC28 (literal property propagation)

## What this adds

The CLOC28 resolver walked `MemberExpression` spines only. It now walks
optional ones too, so every mixture of `.` and `?.` resolves the same way:

```js
var o = { a: { b: 1 } };
console.log(o?.a?.b);     // => console.log(1)
console.log(o.a?.b);      // => console.log(1)
console.log(o?.a.b);      // => console.log(1)
console.log(o?.["a"]);    // (with var o={a:1})  => console.log(1)

var a = [1, 2, 3];
console.log(a?.[1]);      // => console.log(2)
```

One ladder rung closes (`ladder_t7_optional_chain_advanced`), 60 → 59. The
other four shapes were gaps the ladder had no rung for; they were found by
probing the oracle around the rung rather than by the rung itself.

## Why `?.` needs no new soundness argument

`?.` short-circuits to `undefined` when its object is `null` or `undefined`.
On a path this pass accepts, that can never happen:

* the **root** is a candidate whose initializer `is_structured_literal` — an
  object or array literal, never nullish;
* every **intermediate** step is resolved by `resolve` against that literal,
  and `resolve` only continues through a nested object or array literal.

So wherever the walker produces a path that `resolve` accepts, no `?.` on it
could have short-circuited, and `?.` reads exactly as `.` would. Every CLOC28
guard — escape, write, absent key, non-scalar result, spread, hole — carries
over unchanged, because they all sit in `resolve` and in the candidate
eligibility proof rather than in the spine walk.

The cases where a `?.` **would** short-circuit decline instead of folding:

```js
var o = { a: null };
console.log(o?.a?.b);     // stops at the NullLiteral; we leave the chain
```

Upstream folds that to `void 0`. Declining is a gap in the safe direction —
behind the oracle, never ahead of it.

## Implementation

`chain_of` kept its `MemberExpression` signature; a new `chain_of_expr` takes
any expression and accepts `MemberExpression`, `OptionalMemberExpression` and
`ChainExpression` at every step. `key_of` became `key_of_parts(computed,
property)` because both member kinds carry the same pair and read the same
way.

`ChainExpression` is a transparent wrapper around the spine, so it is
unwrapped at entry and again mid-spine — the latter cannot occur in
well-formed input, but handling it keeps the walker total rather than
relying on a parser invariant.

## Not in scope, and deliberately so

Two neighbouring behaviours were measured against `v20260915` and left alone:

| input | upstream | us | why |
|---|---|---|---|
| `var o=null; o?.a` | `console.log(void 0)` | unchanged | short-circuit folding of a nullish root — needs CLOC29 scalar propagation plus a nullish fold, not chain resolution |
| `var o={a:1}; o?.b` | `console.log({a:1}?.b)` | unchanged | upstream inlines the object but does **not** fold the absent key, exactly as CLOC28 records for `o.b`: an absent key can resolve up the prototype chain |

The second is worth naming explicitly: it looks like the same gap and is not.
"Fixing" it would fold a read that upstream deliberately leaves, which is the
direction that ships miscompiles.
