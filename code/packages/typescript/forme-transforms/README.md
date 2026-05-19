# @coding-adventures/forme-transforms

Pure-data sequence helpers — `filter` / `map` / `flatMap` / `take`
/ `drop` / `slice` / `chunk` / `sortBy` / `sortBy2` / `partition` /
`groupBy` / `unique` / `pipe` — for the Forme pipeline (FM00 v0).

Fourth FM00 v0 stage package — sits alongside
[`forme-feeds`](../forme-feeds),
[`forme-opengraph`](../forme-opengraph), and
[`forme-index-renderer`](../forme-index-renderer).  Renderers and
collectors compose these helpers; the pipeline never has to reach
for `Array.prototype` discipline directly.

## Quick start

```ts
import { pipe, filter, sortBy, take } from "@coding-adventures/forme-transforms";

// Recent published posts, newest-first, top 10.
const recent = pipe(posts,
  (xs) => filter(xs, (p) => !p.draft),
  (xs) => sortBy(xs, (p) => p.pubDate, "desc"),
  (xs) => take(xs, 10),
);
```

## Why this package exists

The Forme stages all touch arrays of stuff — `Document[]` (pages),
`IndexItem[]` (archive entries), `ContentNode[]` (AST children).
Each stage would otherwise grow its own ad-hoc sort/filter logic
with subtle bugs:

- `Array.prototype.sort` is stable on modern V8 but undefined on
  ancient JS engines that some embedded runtimes still ship — and
  our reproducibility contract requires "same input → byte-
  identical output."
- Plain-object `groupBy` (`{ [key]: [...] }`) is a `__proto__`
  injection vector when keys come from user-authored frontmatter.
- `Array.prototype.filter` / `.map` accept `readonly T[]` but the
  callback can mutate by reference, so the no-mutation invariant
  has to be enforced by convention.

This package solves all three by exposing a single uniform
algebra: every helper takes `readonly T[]`, returns a fresh array
(or `Map`), and has documented behaviour for ties / nulls / empty
inputs.

## API

### `filter(items, predicate): T[]`
### `map(items, mapper): U[]`
### `flatMap(items, mapper): U[]`

Thin wrappers over the obvious `Array.prototype` operations.  Why
re-export?  See the package docstring — uniform `readonly`
discipline, pipe-shaped signatures, single documentation surface.

### `take(items, n): T[]`
### `drop(items, n): T[]`
### `slice(items, start?, end?): T[]`
### `chunk(items, size): T[][]`

Windows over a sequence.  All clamp out-of-range indices to array
bounds (so `take(posts, options.max)` is safe whether `max` is 0,
5, or a million).  `chunk` is the only one that throws — `RangeError`
on non-positive or non-integer `size`, since the operation is
undefined there.

### `sortBy(items, keyFn, dir?): T[]`
### `sortBy2(items, primary, secondary, primaryDir?, secondaryDir?): T[]`

Stable sort with two non-default behaviours:

1. **`null` / `undefined` keys sort LAST regardless of direction.**
   Asking for "newest first" puts undated items at the end, not
   the top.
2. **Deterministic index tiebreaker.**  Locks in stability even on
   engines whose `Array.prototype.sort` stability we haven't
   independently verified.

`sortBy2` adds a secondary key — common Forme use:
`sortBy2(posts, p => p.pubDate, p => p.id, "desc", "asc")` gives
reverse-chrono with `id` as the within-same-date tiebreaker.

### `partition(items, predicate): { yes, no }`

Single-pass split into the two halves.  Both arrays preserve
input order.

### `groupBy(items, keyFn): Map<K, T[]>`

Buckets items by an extracted key.  Returns a `Map` (not a plain
`Object`):

- **Prototype-pollution defence** — hostile keys like `"__proto__"`
  go into the `Map`'s internal slot, not `Object.prototype`.
- **Non-string keys** round-trip without `String(...)` coercion.
- **Insertion-order iteration** — buckets appear in the order
  their first item appeared in the input.

### `unique(items, keyFn?): T[]`

Dedupe.  First occurrence wins, input order preserved among the
survivors.  Without `keyFn`, uses identity (`SameValueZero` via
`Set`).  With `keyFn`, two items are duplicates iff their keys are
equal.

### `pipe(items, ...steps): U[]`

Left-to-right function composition.  Each step takes a
`readonly T[]` and returns a `readonly U[]`.  Type inference
covers chains up to length 5; past that, break into a named
intermediate.

```ts
const recent = pipe(posts,
  (xs) => filter(xs, (p) => !p.draft),
  (xs) => sortBy(xs, (p) => p.pubDate, "desc"),
  (xs) => take(xs, 10),
);
```

## Behavioural contract

| Helper            | Mutates input? | Throws?                          | Empty input behaviour |
|-------------------|----------------|----------------------------------|-----------------------|
| `filter` / `map` / `flatMap` | never  | never                            | empty output          |
| `take` / `drop` / `slice`    | never  | never                            | empty / copy          |
| `chunk`           | never          | `RangeError` on bad `size`       | empty output          |
| `sortBy` / `sortBy2` | never       | never                            | empty output          |
| `partition`       | never          | never                            | `{ yes: [], no: [] }` |
| `groupBy`         | never          | never                            | empty `Map`           |
| `unique`          | never          | never                            | empty output          |
| `pipe`            | never          | only if a step throws            | passes through        |

## Reproducibility (FM03)

Same input → byte-identical output, given a deterministic
`keyFn` / `predicate`.

The index tiebreaker in `sortBy` is what makes two inputs that
differ only in caller-array order *also* produce the same output
*when paired with a deterministic key* (e.g. sort by a unique
`id`).  If you want order-independent output, sort by a unique
stable key.

## Capabilities — `[]`

Pure transforms.  No I/O, no network, no shell, no env, no fs.
Same posture as `forme-feeds` / `forme-opengraph` /
`forme-index-renderer`.

## Tests

101 tests across 5 files:

- `filter-map.test.ts` (18) — filter / map / flatMap purity, index
  passthrough, type-changes, no-mutation, empty inputs.
- `slice.test.ts` (32) — take / drop / slice / chunk including
  every edge case (negative, NaN, Infinity, fractional, out of
  range), input-not-mutated, fresh-array guarantee.
- `sort.test.ts` (18) — sortBy / sortBy2 with asc + desc,
  null-last on both directions, NaN ties, stable index tiebreaker,
  keyFn-called-once memoisation, numeric/bigint keys, FM00
  archive idiom (pubDate-desc + id-asc).
- `group.test.ts` (24) — partition, groupBy (Map-based,
  __proto__-safe, numeric keys, insertion order), unique (identity
  + keyFn overloads).
- `pipe.test.ts` (9) — composition order, type changes between
  steps, no-mutation, real-world Forme pipeline shapes.

Coverage: **100% line / 100% branch** across all source files
with logic (`types.ts` is type-only).

## Spec adherence

The FM00 v0 spec (§5.3) calls out individual `transform-*`
packages (`transform-syntax-highlight`, `transform-typography`,
etc.).  This package is the **foundational sequence layer** that
those transforms — and collectors, renderers, and the
orchestrator — all build on.  No spec divergences; the package
extends the FM00 v0 surface area with a building block that
every higher layer needs.

## v0 simplifications

- **No streaming / generator variants.**  Everything is array-in,
  array-out.  Forme datasets are small enough (single-machine
  blog scale) that materialising intermediate arrays is cheaper
  than the iterator-protocol overhead.
- **No partial application helpers.**  `pipe` callers write
  arrows directly (`(xs) => filter(xs, p)`) rather than a curried
  `filter(p)` form.  Reduces API surface and TypeScript inference
  weight.
- **No reduce / fold.**  If you need fold, use a native `for`
  loop — that operation is too shape-dependent to wrap usefully.
