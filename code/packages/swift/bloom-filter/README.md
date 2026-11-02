# Bloom Filter for Swift

A pure Swift implementation of the probabilistic membership filter from
[DT22](../../../specs/DT22-bloom-filter.md). It composes only with the reviewed
DT17 `HashFunctions` package and derives deterministic positions from FNV-1a
and DJB2 with fixed avalanche mixing.

```swift
import BloomFilter

var visited = try BloomFilter(expectedItems: 10_000, falsePositiveRate: 0.01)
visited = visited.adding("alice")

print(visited.contains("alice"))
print(visited.contains("bob"))
```

`adding` returns a new value, so earlier filters remain unchanged. Inputs are
exact `[UInt8]` values or exact UTF-8 `String` bytes; the package never uses
Swift's randomized `Hasher` or an executable encoder. A negative result is
definitive, while a positive result may be a false positive.

Construction validates every parameter and caps allocation at 134,217,728
bits (16 MiB) and double hashing at 64 positions per value. Helpers expose the
optimal bit and hash counts, capacity for a fixed memory budget, fill ratio,
estimated false-positive rate, and over-capacity status.

## Testing

```sh
swift test --enable-code-coverage
swift build -c release
```

The package is deterministic and declares no filesystem, process, network,
environment, entropy, clock, credential, or other host capability.
