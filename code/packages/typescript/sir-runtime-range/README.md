# @coding-adventures/sir-runtime-range

Range runtime for **Semantic-IR-emitted TypeScript/JavaScript**.

The SIR backends translate most Ruby-surface constructs to *native* code: a
sequence becomes an `Array`, a map becomes a `Map`. A Ruby **range** is a
first-class object — you iterate it, test membership (`r.include?(3)`), or
materialise it (`r.to_a`) — and JavaScript has no range type at all. So the SIR
`Range` value type lives here, exactly like the cons cell lives in
[`@coding-adventures/sir-runtime-pairs`](../sir-runtime-pairs).

## Where it fits in the stack

```
Ruby source ─▶ ruby-to-semantic-ir ─▶ Semantic IR ─▶ semantic-ir-to-typescript ─▶ .ts
                                                                                 │ imports
                                                                                 ▼
                                                       @coding-adventures/sir-runtime-range
```

A Ruby range literal lowers to `BuiltinCall("range", [start, stop, exclusive])`;
the backend emits `__SirRange.range(start, stop, exclusive)` and imports this
package **only** when a module actually uses a range.

## The range forms

| Ruby | `start` | `stop` | `exclusive` | members |
|---|---|---|---|---|
| `1..5`  | `1`    | `5`    | `false` | `1 2 3 4 5` |
| `1...5` | `1`    | `5`    | `true`  | `1 2 3 4` |
| `1..`   | `1`    | `null` | `false` | `1 2 3 …` (endless) |
| `..5`   | `null` | `5`    | `false` | `… 4 5` (beginless) |

Iteration walks integers upward from `start`. An **endless** range yields
forever — consume it lazily (`break` out of the `for…of`). A **beginless** range
has no first element, so iterating one (or calling `toList` on any unbounded
range) throws a `TypeError` rather than hanging — matching Ruby, where
`(..5).each` raises.

## API

| Export | Purpose |
|---|---|
| `class Range` | Immutable range with `start` / `stop` / `exclusive`; iterable, membership via `includes`, Ruby-notation `toString`. |
| `range(start, stop, exclusive): Range` | Construct a range (the backend's `__SirRange.range`). |
| `includes(r, v): boolean` | Membership (Ruby `include?`). |
| `toList(r): Val[]` | Materialise (Ruby `to_a`); throws on an unbounded range. |
| `isRange(v): boolean` | True iff `v` is a `Range`. |
| `Val` | The universal SIR value type alias at this boundary. |

## Usage

```ts
import { range, toList } from "@coding-adventures/sir-runtime-range";

const r = range(1, 5, false);   // the inclusive range 1..5
[...r];                          // [1, 2, 3, 4, 5]
r.includes(3);                   // true
toList(range(1, 5, true));       // [1, 2, 3, 4]  (exclusive)
String(range(1, null, false));   // "1.."         (endless)
```

## Development

```bash
npm install
npx tsc --noEmit
npx vitest run --coverage
```

## License

MIT
