# HashSet for Swift

This package implements the persistent DT19 hash-set API by composing the
sibling Swift [HashMap](../hash-map) package. It stores each member as a DT18
key with a private sentinel value, so collision handling, finite capacity
limits, key validation, and DT17 hashing stay in one reviewed implementation.

## Element and hash profile

The element domain is `HashMapKey`: exact UTF-8 strings and raw byte arrays.
The two forms are type-tagged and remain distinct. String equality is exact
UTF-8 byte equality, so composed and decomposed Unicode spellings can coexist.

Callers choose separate chaining or linear probing and one of the closed DT17
hash profiles: FNV-1a-32, seeded MurmurHash3-32, or SipHash-2-4 with an exact
16-byte key. The convenience initializer uses HashMap's public deterministic
compatibility key. It is reproducible, not secret; callers facing adversarial
keys should provide a secret caller-owned SipHash key.

```swift
import HashMap
import HashSet

let empty = try HashSet(
  capacity: 8,
  strategy: .linearProbing,
  hashAlgorithm: .fnv1a32
)
let names = try empty
  .adding(.string("alice"))
  .adding(.string("bob"))

empty.contains(.string("alice")) // false
names.contains(.string("alice")) // true
```

## Persistence and algebra

`adding`, `removing`, `discarding`, `union`, `intersection`, `difference`, and
`symmetricDifference` return new values. Their inputs remain unchanged.
Algebraic results retain the receiver's collision strategy and hash algorithm,
even when the other set uses a different profile.

```swift
let staff = try HashSet.fromElements([
  .string("alice"), .string("bob"),
])
let admins = try HashSet.fromElements([
  .string("bob"), .string("carol"),
])

let everyone = try staff.union(admins)
let staffAdmins = try staff.intersection(admins)

everyone.count // 3
staffAdmins.elements // contains only "bob"
```

The relational helpers are `isSubset(of:)`, `isSuperset(of:)`,
`isDisjoint(with:)`, and `equals(_:)`. Iteration through `elements` follows the
underlying map's physical storage order. It is neither sorted nor insertion
order and is deterministic only for the same complete table profile and
operation history.

## Validation and authority

Insertion and algebra can throw the stable `HashMapError` values when caller
parameters, key size, or finite growth limits are invalid. Membership and
removal reject oversized keys without allocating or changing the set.

This package performs pure in-memory computation and declares no host
capabilities. It reads no filesystem, network, process, environment, entropy,
clock, or credentials.

```sh
swift test --enable-code-coverage
swift build -c release
```

`BUILD` selects `xcrun swift` when available so XCTest follows the chosen Xcode
toolchain. `BUILD_windows` requires Swift and propagates test or release-build
failures.
