# HashMap for Swift

This package implements the two hash-table strategies from
[DT18](../../../specs/DT18-hash-map.md) as persistent Swift values. It uses the
sibling [HashFunctions](../hash-functions) package for the corrected DT17 hash
algorithms instead of Swift's process-randomized `Hasher`.

## Key and hash profile

`HashMapKey` is a closed key domain:

- `.bytes([UInt8])` stores caller bytes;
- `.string(String)` stores the string's UTF-8 bytes.

The hash input is exact: one type tag, an unsigned 64-bit little-endian payload
length, and the payload. A byte key and a string with the same payload are
therefore distinct. String identity is also exact UTF-8 byte identity: the
package does not normalize Unicode, so canonically equivalent composed and
decomposed spellings remain distinct keys. This makes equality agree with the
bytes supplied to the hash function. The package does not call
`String(describing:)`, Codable, host object getters, or caller-supplied
encoders.

`HashAlgorithm` admits only DT17 algorithms with explicit parameters:
FNV-1a-32, MurmurHash3-32 with a caller seed, and SipHash-2-4 with an exact
16-byte key. The convenience initializer uses the public deterministic
`codex-dt18-key!!` compatibility key. That key makes examples reproducible; it
is not secret and does not provide adversarial hash-flooding protection. Supply
a secret caller-owned SipHash key when that property is required. The package
never reads operating-system entropy.

## Persistent operations

```swift
import HashMap

let empty = try HashMap<Int>(
    capacity: 8,
    strategy: .linearProbing,
    hashAlgorithm: .fnv1a32
)
let one = try empty.setting(1, for: .string("one"))
let two = try one.setting(2, for: .bytes([0x02]))

empty.has(.string("one")) // false
two.value(forKey: .string("one")) // 1
```

`setting`, `removingValue`, and `merging` return new maps and leave their input
maps unchanged. `fromEntries` and `merging` are right-biased: a later value for
the same key wins without increasing `count`.

Keys and table storage have Swift value semantics. Exported `keys`, `values`,
and `entries` arrays do not alias the map's structural storage. Generic values
are retained shallowly, so a reference-typed value remains shared with its
caller and with older map versions. This package does not claim to clone an
arbitrary caller object.

`value(forKey:)` returns `Value?`. When `Value` is itself optional, use `has`
to distinguish a stored `Optional.none` from a missing key.

## Collision and iteration contracts

- Separate chaining stores colliding entries in per-bucket arrays and grows
  before a new key would make the load factor greater than `1.0`.
- Linear probing walks at most `capacity` slots, retains tombstones so deletion
  cannot break a probe chain, and grows before a new key would make the load
  factor greater than `0.75`.
- Capacity doubling is checked and capped. Production maps admit at most
  1,048,576 slots and keys with at most 1,048,576 payload bytes. Operations
  fail with stable `HashMapError` values instead of overflowing or probing
  forever.

Iteration is physical storage order, not insertion order. It is deterministic
only for the same collision strategy, hash parameters, initial capacity, and
operation history. Resizing or selecting another strategy can change it.

## Downstream hash-set composition

`configuration` exposes a copyable value containing current capacity, collision
strategy, and closed hash algorithm. A DT19 hash-set can construct another map
with the same profile and use `setting`, `removingValue`, `has`, `count`, and
`keys` without access to buckets or slots.

## Authority and validation

The package performs pure in-memory computation and declares an empty capability
profile. It has no filesystem, network, process, environment, entropy, clock,
credential, or other host authority.

```sh
swift test --enable-code-coverage
swift build -c release
```

`BUILD` chooses `xcrun swift` when available so XCTest follows the selected
Xcode toolchain. `BUILD_windows` requires Swift and propagates test or release
build failures.
