---
category: Rust
---

# An untagged serde enum is invisible to a scan that keys on the type tag, so a guard built that way silently passes the case it exists to catch

Implementing CLOC28 (resolving `o.a` against `var o = {a:1}`) I needed a guard
that no *other* reference to `o` exists, because folding the read is only sound
if the binding is never written or escaped. Rather than hand-write a typed AST
walk — easy to leave a variant out of — I counted occurrences over the
serialized AST, which `#[serde(tag = "type")]` makes uniform:

```rust
map.get("type") == Some("Identifier") && map.get("name") == Some(name)
```

Exhaustive by construction, or so it looked. Two enums in this AST are
`#[serde(untagged)]`: `BindingTarget` (a declarator's `id`) and
`AssignmentTarget` (the left of an assignment). Untagged variants serialize
with **no `"type"` field at all**, so both are invisible to that predicate.

Missing the first was harmless. Missing the second was a miscompile:

```js
var o = { a: 1 };
o = { a: 2 };
console.log(o.a);   // prints 2; the fold emitted console.log(1)
```

The write was invisible to the new guard, and also to the crate's existing
`count_uses_*` walk, which deliberately skips a bare identifier assignment
target — correct for the scalar path, which is `const`-only and where the case
cannot arise. Extending the pass to `var` voided that assumption without
voiding the comment that recorded it. Two guards, one blind spot, and the fold
looked perfectly legitimate.

**What to do.**

* **Before keying a scan on `"type"`, grep the AST crate for
  `serde(untagged)`.** Anything listed there is a hole in the scan. In this
  repo: `BindingTarget`, `AssignmentTarget`.
* **Key on the payload field instead when the guard is a refusal.** Counting
  every object carrying `"name": "<name>"` sees the mention whatever the enum
  tagging. It over-counts (a property key sharing the name), and for a guard
  that only ever *declines* a transform, over-counting is free — it fails
  closed, which is the direction you want.
* **When widening a pass from `const` to `var`/`let`, re-derive every guard
  that existed before.** A guard whose comment says "a `const` cannot be
  assigned, so we need not check" is not a guard once the input can be a
  `var`; it is a loaded gun. Search the crate for reasoning that names the old
  restriction.
* **Test the refusals against a runtime, not just the emitted bytes.** The
  wrong output here was *valid, plausible JavaScript* — `console.log(1)` — and
  reads fine in a golden. Running the before and after under `node` and
  comparing what they print is what actually caught it.
