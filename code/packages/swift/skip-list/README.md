# SkipList for Swift

This package implements the ordered-map portion of
[DT20](../../../specs/DT20-skip-list.md) as an actual span-augmented skip-list
tower. It does not substitute a sorted array or a standard-library tree.

## Closed behavior profile

- Rank and selection are zero-based and return `nil` for missing or
  out-of-range positions. This matches the established Python, Rust, and
  TypeScript behavior while a separate DT20 prose reconciliation remains
  pending.
- Comparator equivalence defines key identity. Updating an equivalent key
  changes only its value and does not consume a new level or change the shape.
- Range endpoints are independently inclusive or exclusive. Reversed ranges
  return an empty snapshot.
- Structural snapshots are owned arrays. Generic keys and values are retained
  shallowly, so callers must not mutate reference keys in a way that changes
  their ordering.

## Caller-controlled levels

The initializer requires a synchronous `levelSource`. It is called exactly
once for each genuinely new key and must return a height in
`1...maximumLevel`. Updates and rejected capacity growth do not call it. The
package never reads operating-system entropy or a global random generator.

```swift
import SkipList

var scriptedLevels = [1, 3, 2].makeIterator()
let scores = try SkipList<Int, String>(
  comparator: <,
  levelSource: { scriptedLevels.next() ?? 1 }
)

try scores.insert(10, value: "ten")
try scores.insert(20, value: "twenty")
scores.rank(of: 20)       // 1
scores.entry(at: 0)?.key  // 10
```

The caller owns the comparator and level-source behavior and any authority
inside those callbacks. They must be stable for an operation. `SkipList` is
mutable and intentionally not thread-safe.

## Spans and limits

Every forward link stores the number of bottom-level entries it crosses.
Insert and deletion repair those spans, making rank and selection inverse
O(log n) expected operations for a well-distributed caller level source.
Adversarial all-level-one input remains finite but degrades to O(n).

Maximum level is capped at 32 and maximum configured entries at 1,048,576.
Generated levels are validated before mutation. Ordered snapshots and ranges
accept explicit limits capped at 65,536 entries. Invalid configuration,
levels, capacity, and output limits fail with stable `SkipListError` values.

## Authority and validation

The package performs bounded in-memory computation and declares an empty
capability profile. It has no filesystem, network, process, environment,
entropy, clock, credential, or other host authority.

```sh
swift test --enable-code-coverage
swift build -c release
```

`BUILD` chooses `xcrun swift` when available. `BUILD_windows` requires Swift
and propagates native test or release-build failures.
