# HyperLogLog for Swift

A pure Swift implementation of the probabilistic cardinality sketch from
[DT21](../../../specs/DT21-hyperloglog.md). It composes only with the reviewed
DT17 `HashFunctions` package and uses deterministic FNV-1a 64-bit hashing plus
a fixed avalanche finalizer.

## Example

```swift
import HyperLogLog

var visitors = try HyperLogLog(precision: 14)
visitors = visitors.adding("alice")
visitors = visitors.adding("bob")
visitors = visitors.adding("alice")

print(visitors.count())
```

`adding` and `merged(with:)` return new values, so earlier sketches remain
unchanged. Inputs are exact `[UInt8]` values or exact UTF-8 `String` bytes;
the package never uses Swift's randomized `Hasher` or an executable encoder.

Precision is bounded to 4...16. Merging sketches with different precision
returns a stable `HyperLogLogError.precisionMismatch` failure. Helper methods
report the standard error, packed-register byte count, and the bounded optimal
precision for a requested error rate.

## Testing

```sh
swift test --enable-code-coverage
swift build -c release
```

The package is deterministic and declares no filesystem, process, network,
environment, entropy, clock, credential, or other host capability.
