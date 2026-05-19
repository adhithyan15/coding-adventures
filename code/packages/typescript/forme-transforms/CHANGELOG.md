# Changelog — @coding-adventures/forme-transforms

## 0.1.0 — 2026-05-18

Initial release.  Fourth FM00 v0 stage package — foundational
sequence-helper layer providing filter / map / sort / limit /
partition / group / unique / pipe over arrays of anything.

Companion to `forme-feeds` / `forme-opengraph` /
`forme-index-renderer`; consumed by renderers, collectors, and
the orchestrator as the building block they all need.

### Added

- `filter(items, predicate)` — keep matching items.
- `map(items, mapper)` — project each item.
- `flatMap(items, mapper)` — project + one-level flatten.
- `take(items, n)` — first N items.  Clamps negative / NaN /
  Infinity to 0.
- `drop(items, n)` — skip first N.  Clamps negative / NaN /
  Infinity to 0 (returns copy).
- `slice(items, start?, end?)` — `items[start..end)` with
  clamped, never-negative-from-end indices.
- `chunk(items, size)` — split into batches of `size`.  Throws
  `RangeError` on non-positive or non-integer `size`.
- `sortBy(items, keyFn, dir?)` — stable sort.  `null` / `undefined`
  keys sort last regardless of direction.  Index tiebreaker
  locks in stability.
- `sortBy2(items, primary, secondary, primaryDir?, secondaryDir?)`
  — primary-then-secondary stable sort in one pass.
- `partition(items, predicate)` — `{ yes, no }` split in one pass.
- `groupBy(items, keyFn)` — bucket into `Map<K, T[]>`.
- `unique(items, keyFn?)` — dedupe by identity or by extracted
  key; first occurrence wins.
- `pipe(items, ...steps)` — left-to-right function composition
  with type inference up to 5 steps.
- Type definitions: `Predicate`, `Mapper`, `FlatMapper`, `KeyFn`,
  `SortDirection`, `Partition`, `PipeStep`.

### Spec adherence

Extends the FM00 v0 stage surface area with a foundational
sequence layer.  The spec's §5.3 lists individual `transform-*`
packages by name; this package is the building block they (and
collectors, renderers, the orchestrator) all use.  No spec
divergences.

### Behavioural notes

- **Every helper is pure** — `readonly T[]` in, fresh array (or
  `Map`) out.  Input arrays are never mutated.
- **`sortBy` puts null / undefined keys LAST regardless of
  direction.**  Asking for "newest first" puts undated items at
  the end, not at the top.
- **`sortBy` uses an index tiebreaker** so output is deterministic
  even on JS engines whose `Array.prototype.sort` stability we
  haven't independently verified.  Cost is one integer compare
  per tied pair.
- **`sortBy` memoises keys** — `keyFn` is invoked exactly once
  per item, not O(N log N) times.  Side effects (if any) fire
  predictably.
- **`groupBy` returns a `Map`, not a plain object** — protection
  against `__proto__` injection from caller-controlled keyFn
  output, plus support for non-string keys.
- **`chunk` is the only helper that throws** — `RangeError` on
  bad `size`.  Every other helper clamps out-of-range inputs to
  array bounds and returns an empty / copy result, so they're
  safe to chain on caller-controlled lengths.
- **`pipe` returns input unchanged when no steps supplied** —
  useful in conditional pipeline-building code where some steps
  are toggled off.

### Security posture

Three concerns explicitly addressed (pre-push review):

- **Prototype pollution via key functions.**  `groupBy` and
  `unique` use `Map` / `Set` internally, never plain objects.
  A hostile `keyFn` returning `"__proto__"` lands as a normal
  Map / Set key with no `Object.prototype` reach.  Pinned by
  `group.test.ts` (`__proto__` key doesn't pollute `{}`).
- **No URL / HTML / shell surface.**  This is pure data
  transformation; there's no string-rendering path that could
  emit XSS, no fs/shell that could leak data, no network that
  could exfiltrate.
- **Bounded computation.**  `chunk` throws on non-positive size
  rather than looping forever / dividing by zero; `take` /
  `drop` / `slice` clamp to array bounds rather than dispatching
  arbitrary-length allocations.

### Capabilities

`[]` — pure transform.  No I/O, no network, no shell, no env, no
fs.

### Tests

101 tests across 5 files:

- `filter-map.test.ts` (18) — filter / map / flatMap purity,
  index passthrough, type-changes, no-mutation, empty inputs,
  one-level-only flatten verification.
- `slice.test.ts` (32) — take / drop / slice / chunk on every
  edge: negative, NaN, Infinity, fractional, zero, out-of-range;
  input-not-mutated; fresh-array guarantee (`drop(input, 0)`
  returns a copy, not the input itself).
- `sort.test.ts` (18) — `sortBy` / `sortBy2` with asc + desc
  parameterisation, null-last on both directions, NaN ties
  preserve input order, stable index tiebreaker, keyFn-called-
  once memoisation, numeric and bigint keys, FM00 archive idiom
  (pubDate-desc + title-asc).
- `group.test.ts` (24) — partition halves preserve input order,
  groupBy returns Map (not Object) with `__proto__`-safety,
  insertion-order iteration, within-bucket order preservation,
  numeric keys, both unique overloads (identity + keyFn), object
  identity for reference dedupe.
- `pipe.test.ts` (9) — composition order verification (each
  step's output → next step's input), type-changes between
  steps (T → U → V), no-mutation through full pipelines,
  real-world Forme pipeline shapes ("recent published posts",
  "IDs of all non-drafts"), empty input survives.

Coverage: **100% line / 100% branch** across all source files
with logic (`types.ts` is type-only declarations).

### v0 simplifications (documented)

- **No streaming / generator variants.**  Array-in, array-out.
  Forme datasets are small enough (single-machine blog scale)
  that materialising intermediates is cheaper than iterator-
  protocol overhead.
- **No partial application / curried forms.**  `pipe` callers
  write arrows directly (`(xs) => filter(xs, p)`) rather than
  `filter(p)`.  Reduces TypeScript inference weight and API
  surface.
- **No reduce / fold.**  Too shape-dependent to wrap usefully —
  native `for` loop wins.
- **No async variants.**  All helpers are synchronous; the Forme
  stages handle their own async boundaries before/after the
  transform layer.
