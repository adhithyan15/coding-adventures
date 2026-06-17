# Changelog

## 0.1.0 — initial release

First release of the SIR range runtime for TypeScript/JavaScript, per
`code/specs/sir-runtime.md`. A Ruby range (`1..5`, `1...5`, `1..`, `..5`) is a
first-class value, not a loop; JavaScript has no range type at all, so the SIR
`Range` ships here as a dedicated per-concern runtime (mirroring
`@coding-adventures/sir-runtime-pairs`).

### Added

- `class Range` — an immutable range with readonly `start` / `stop` /
  `exclusive` fields. Iterable (`[Symbol.iterator]`), value membership via
  `includes`, and a Ruby-notation `toString` (`1..5` / `1...5` / `1..` / `..5`).
- `range(start, stop, exclusive): Range` — the constructor the TypeScript
  backend targets (`__SirRange.range(...)`). Either bound may be `null` for the
  begin/endless forms.
- `includes(r, v)` / `Range.includes` — membership (Ruby `include?`), correct
  for inclusive, exclusive, beginless, and endless forms.
- `toList(r)` / `Range.toList` — materialise as an array (Ruby `to_a`); throws
  `TypeError` for an unbounded (beginless or endless) range.
- `isRange(v): boolean` — range predicate. `Val` — the universal SIR value type
  alias at this boundary.
- Full standard layout: `package.json`, `tsconfig.json`, `vitest.config.ts`,
  `BUILD`, `BUILD_windows`, `required_capabilities.json` (no capabilities),
  README. vitest suite at 100% coverage; `tsc --strict` clean.

### Semantics (v0)

- Iteration walks integers upward from `start`. An endless range yields forever
  (consume lazily); a beginless range throws on iteration (no first element) —
  matching Ruby's behaviour for `(..5).each`.
- Membership and the begin/endless forms are fully supported; non-integer
  stride / float ranges are out of scope for v0.
