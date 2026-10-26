# HashFunctions for Swift

Pure Swift implementations of the foundational hashes from
[DT17](../../../specs/DT17-hash-functions.md). The package is the hashing layer
for later Swift hash maps, sets, Bloom filters, HyperLogLog structures, and
storage engines.

## What is included

| Algorithm | Output | Purpose |
| --- | ---: | --- |
| FNV-1a | 32 or 64 bits | Small, fast byte and string keys |
| DJB2 | 64 bits | The classic `hash * 33 + byte` teaching hash |
| MurmurHash3 | 32 bits | Strong non-cryptographic mixing with an optional seed |
| SipHash-2-4 | 64 bits | Keyed hash-flooding resistance with a 16-byte key |
| Polynomial rolling | 64 bits | Compatibility helper used by repository data structures |

FNV-1a, DJB2, MurmurHash3, and polynomial rolling are not cryptographic hashes.
SipHash is keyed and suitable for protecting hash-table keys, but it is not a
password hash or a replacement for a general-purpose message digest.

## Direct functions

```swift
import HashFunctions

let fnv = fnv1a32("hello")
let murmur = murmur3_32("hello", seed: 7)
let polynomial = try polynomialRolling("hello")

let key = Array(UInt8(0)...UInt8(15))
let sip = try sipHash24(Array("hello".utf8), key: key)
```

Every hash accepts `[UInt8]`; the non-keyed hashes also have UTF-8 `String`
overloads. `sipHash24` rejects any key that is not exactly 16 bytes, and
`polynomialRolling` rejects a zero modulus.

## Choosing an algorithm at runtime

Concrete strategy values share the `HashFunction` protocol:

```swift
let strategies: [any HashFunction] = [
    FNV1a32(),
    FNV1a64(),
    DJB2Hash(),
    PolynomialRollingHash(),
    Murmur3Hash(seed: 7),
    try SipHash24(key: [UInt8](repeating: 0, count: 16)),
]

for strategy in strategies {
    print(strategy.outputBits, strategy.hash([0x61, 0x62, 0x63]))
}
```

## Deterministic quality analysis

The analysis helpers do not read operating-system entropy. Callers provide the
exact samples so a test, benchmark, and production diagnostic all observe the
same computation:

```swift
let samples = [
    Array("abcdefgh".utf8),
    Array("ABCDEFGH".utf8),
]

let avalanche = try avalancheScore(
    hashFunction: { UInt64(murmur3_32($0)) },
    outputBits: 32,
    samples: samples
)

let chiSquared = try distributionTest(
    hashFunction: fnv1a64,
    inputs: samples,
    numBuckets: 8
)
```

`avalancheScore` flips every bit of every non-empty sample and returns the
fraction of the declared low-order output bits that changed; higher bits are
ignored. `distributionTest` returns the chi-squared statistic for the requested
buckets.

## Design and safety

- All arithmetic is explicit fixed-width wrapping or checked modular arithmetic.
- Polynomial rolling keeps the full 128-bit product with Swift's
  `multipliedFullWidth`/`dividingFullWidth` pair.
- SipHash parses its key and message words explicitly as little-endian bytes.
- The package is deterministic and declares no filesystem, process, network,
  environment, entropy, clock, credential, or other host capability.

## Testing

```sh
swift test --enable-code-coverage
swift build -c release
```

On macOS, use `xcrun swift` so XCTest is resolved from the selected Xcode
toolchain. `BUILD` and `BUILD_windows` encode these native front doors.
