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

Where a `?.` **would** short-circuit, the walk stops — but "stops" is not
"leaves the program alone", and an earlier draft of this spec said the wrong
one. The walker descends and resolves the longest prefix it can, so the
binding still disappears and a shorter chain is left behind:

```js
var o = { a: null };
console.log(o?.a?.b);     // => console.log(null?.b)
```

That is correct — `null?.b` is `undefined`, exactly what the source computes —
and it is still behind the oracle, which folds the whole thing to `void 0`.
The `ChainExpression` wrapper survives the prefix rewrite, which is what keeps
a later `?.` short-circuiting to the end of the chain rather than throwing.

The distinction matters for review: a reader told "we decline" would not think
to check that the residual chain still short-circuits correctly.

## Implementation

`chain_of` is **deleted** and replaced by `chain_of_expr`, which takes any
expression and accepts `MemberExpression`, `OptionalMemberExpression` and
`ChainExpression` at every step. (An earlier draft said the two coexist; they
do not — clippy flagged the old one as dead the moment nothing called it.) `key_of` became `key_of_parts(computed,
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
| `var o=null; o?.a` | `console.log(void 0)` | `console.log(null?.a)` | CLOC29 already places the scalar; what is missing is a *fold* of a nullish-based optional chain, which belongs in `closure-pass-constant-fold` |
| `var o={a:1}; o?.b` | `console.log({a:1}?.b)` | unchanged | upstream inlines the object but does **not** fold the absent key, exactly as CLOC28 records for `o.b`: an absent key can resolve up the prototype chain |

The second is worth naming explicitly: it looks like the same gap and is not.
"Fixing" it would fold a read that upstream deliberately leaves, which is the
direction that ships miscompiles.
