# DT18 Dart hash-map profile

This profile freezes the public contract for the portable Dart DT18 package.
It specializes the general concepts and pseudocode in
[DT18-hash-map.md](DT18-hash-map.md). Where the general example leaves a
boundary open, this profile is normative for the Dart package.

The matching Swift package at `code/packages/swift/hash-map` is a reference for
the shared behavior described below. Its source and tests are not proof of the
Dart implementation. The Dart package has no native Warrant or SAT binding.
Acceptance is based on direct Dart public-API execution against independent
fixtures and a separate association-list model.

## Public surface

The package barrel is
`package:coding_adventures_hash_map/coding_adventures_hash_map.dart`.
The public surface is closed to these types and operations:

```dart
// Public API signatures; private implementation classes are omitted.
enum CollisionStrategy { separateChaining, linearProbing }

sealed class HashMapKey {
  factory HashMapKey.bytes(List<int> bytes);
  factory HashMapKey.string(String value);
  List<int> get encodedBytes;
}

sealed class HashAlgorithm {
  const factory HashAlgorithm.fnv1a32() = _Fnv1a32;
  factory HashAlgorithm.murmur3({required int seed}) = _Murmur3;
  factory HashAlgorithm.sipHash24({required List<int> key}) = _SipHash24;
  BigInt validatedHash(List<int> bytes);
}

sealed class HashMapError implements Exception {
  const HashMapError();
}

final class InvalidCapacity extends HashMapError {
  const InvalidCapacity(this.capacity);
  final int capacity;
}

final class CapacityLimitExceeded extends HashMapError {
  const CapacityLimitExceeded();
}

final class InvalidSipHashKeyLength extends HashMapError {
  const InvalidSipHashKeyLength(this.actualLength);
  final int actualLength;
}

final class InvalidMurmurSeed extends HashMapError {
  const InvalidMurmurSeed(this.seed);
  final int seed;
}

final class InvalidByte extends HashMapError {
  const InvalidByte(this.index, this.value);
  final int index;
  final int value;
}

final class MalformedUtf16 extends HashMapError {
  const MalformedUtf16(this.codeUnitOffset);
  final int codeUnitOffset;
}

final class KeyTooLong extends HashMapError {
  const KeyTooLong(this.actual, this.maximum);
  final int actual;
  final int maximum;
}

final class HashMapConfiguration {
  HashMapConfiguration._({
    required this.capacity,
    required this.strategy,
    required this.hashAlgorithm,
  });

  final int capacity;
  final CollisionStrategy strategy;
  final HashAlgorithm hashAlgorithm;
}

abstract final class HashMapLimits {
  static const int maximumCapacity;
  static const int maximumKeyByteCount;
  static const List<int> deterministicSipHashKey;
}

final class HashMap<V> {
  HashMap({
    int capacity = 16,
    CollisionStrategy strategy = CollisionStrategy.separateChaining,
    HashAlgorithm? hashAlgorithm,
  });

  factory HashMap.fromEntries(
    Iterable<MapEntry<HashMapKey, V>> entries, {
    int capacity = 16,
    CollisionStrategy strategy = CollisionStrategy.separateChaining,
    HashAlgorithm? hashAlgorithm,
  });

  HashMap<V> setting(V value, {required HashMapKey forKey});
  V? valueForKey(HashMapKey key);
  bool has(HashMapKey key);
  HashMap<V> removingValueForKey(HashMapKey key);
  HashMap<V> merging(HashMap<V> other);

  int get count;
  int get capacity;
  double get loadFactor;
  CollisionStrategy get strategy;
  HashAlgorithm get hashAlgorithm;
  HashMapConfiguration get configuration;
  List<MapEntry<HashMapKey, V>> get entries;
  List<HashMapKey> get keys;
  List<V> get values;
}
```

The hash algorithms are FNV-1a-32, seeded MurmurHash3 x86-32, and SipHash-2-4.
`validatedHash` accepts byte values in `0..255` and returns an unsigned
`BigInt`. The 32-bit results are zero-extended. It has no DT18 key-size limit
when called directly. BigInt keeps SipHash and bucket reduction exact on both
the Dart VM and JavaScript runtime. No caller-defined hash callback or host
`hashCode` participates in table placement.

`HashAlgorithm.murmur3` stores the supplied seed. The SipHash constructor
snapshots its key bytes. Both constructors defer argument-domain validation
until `validatedHash` or `HashMap` construction. This lets map construction
apply its documented capacity-first error order. `HashMapConfiguration` is an
immutable read-only snapshot of an already-valid map configuration. Algorithm
configurations compare by algorithm and parameters. Key equality uses type tag
and payload bytes, with matching equality/hashCode behavior. Configuration
equality uses capacity, strategy, and algorithm configuration.

## Keys and encoding

`HashMapKey.bytes` snapshots its input list. `HashMapKey.string` accepts only
well-formed UTF-16 and stores the exact UTF-8 representation. A high surrogate
must be followed immediately by a low surrogate. An isolated low surrogate or
a high surrogate without its low partner raises `MalformedUtf16` with the
first offending code-unit offset. Dart does not
replace malformed UTF-16 during encoding.

Key identity is the complete type tag and payload byte sequence:

| Key constructor | Tag | Payload |
| --- | ---: | --- |
| `HashMapKey.bytes(bytes)` | `0x00` | The copied bytes |
| `HashMapKey.string(value)` | `0x01` | Strict UTF-8 bytes |

`encodedBytes` is the tag, followed by the payload length as an unsigned
64-bit little-endian integer, followed by the payload. The length counts only
payload bytes. A byte key and a string key with equal payload bytes are
different keys. Strings are case-sensitive and are not Unicode-normalized, so
composed and decomposed spellings remain distinct.

Byte-valued lists must contain only integers in `0..255`. The first invalid
element raises `InvalidByte` with its list index and value. `HashMapKey`
snapshots bytes before exposing them, and its encoded-byte
getter returns a detached snapshot. SipHash keys are also copied when their
configuration is created. Mutating either caller-owned list later cannot
change a key or a map's hash configuration.

## Hash configuration and defaults

`HashMapLimits.maximumCapacity` and
`HashMapLimits.maximumKeyByteCount` are both `1_048_576`. The deterministic
default SipHash key is the 16 ASCII bytes for `codex-dt18-key!!`, with hex
`636f6465782d647431382d6b65792121`. It is public compatibility material. The
package does not generate secrets or read system entropy, and the default key
does not promise hash-flood resistance. Callers may supply their own 16-byte
key.

When `hashAlgorithm` is omitted or null, the map uses SipHash-2-4 with the
default key. The other defaults are capacity 16 and separate chaining.
Capacities may be any integer in `1..1_048_576`; construction does not round
them to powers of two. Key payloads may contain at most `1_048_576` bytes.
Framing bytes do not count toward this payload limit.

The map reduces each unsigned algorithm result modulo the positive capacity.
SipHash reduction is performed with exact BigInt arithmetic. The map does not
use `String.hashCode`, object identity, randomized host hash tables, or a
caller-supplied serializer.

## Collision, growth, and iteration

Separate chaining stores entries in bucket arrays. It grows before inserting a
new key when `count == capacity`. Thus a table may reach load factor exactly
1.0. Replacing an existing key never grows the table.

Linear probing starts at `hash(key) % capacity` and visits
`(start + offset) % capacity` for offsets from zero up to, but not including,
capacity. Lookup and insertion inspect at most capacity slots. Lookup passes
through tombstones and stops at the first empty slot or after a full cycle.
Insertion remembers the earliest tombstone encountered, but continues until
it finds an equal key, an empty slot, or completes a full cycle. Before
inserting a new key it grows when `(count + 1) * 4 > capacity * 3`; replacement
never grows. The threshold uses the live count and is checked before reusing a
tombstone. At the first empty slot, insertion uses the remembered tombstone if
present, otherwise that empty slot. After a full cycle, it uses the earliest
tombstone if one was found. Deletion marks a tombstone and never shrinks the
table.

Growth chooses `min(capacity * 2, maximumCapacity)`. If that value cannot
increase the capacity, insertion raises the `CapacityLimitExceeded` error
type. Rehashing inserts the existing entries in their previous physical
iteration order. Chaining iterates buckets by ascending index and each bucket
in insertion order. Linear probing iterates slots by ascending index and
omits empty and tombstone slots. This physical iteration order is normative.
Replacing a key changes its value without changing its physical position.

## Persistent operations and values

`setting`, `removingValueForKey`, and `merging` return maps with independent
logical state. Their input maps remain unchanged. Missing-key deletion is a
no-op. `fromEntries` accepts an `Iterable<MapEntry<HashMapKey, V>>` and
consumes it once in source order. It validates map configuration before
requesting the iterable's iterator. It stops at the first insertion failure
and propagates an exception raised by the iterable. No partially built map is
published. A duplicate replaces the earlier value without increasing count
or moving its physical position. Swift's corresponding API accepts finite
arrays; lazy iteration and iterator-error behavior are Dart-only contract
cases.

`merging` starts with the left map's capacity, strategy, and hash algorithm,
then processes the right map's entries in right physical order. Right-side
values replace equal left-side keys. New keys follow normal insertion and
growth rules. Both inputs remain unchanged if merging fails.

An implementation may use a private transient builder inside `fromEntries` or
`merging` to avoid copying the full table for every pair. It must preserve
sequential insertion, duplicate replacement, resize order, and first-error
behavior, then publish an immutable map only after successful completion. No
mutable builder escapes through the public API. A persistent `setting` may
copy table structure proportional to capacity. The API makes no O(1) claim
for those copies. Values are shallow: caller-owned reference values remain
shared between input values and map versions.

`entries`, `keys`, and `values` expose detached snapshots in physical
iteration order. A caller may change a returned list without changing the map.
`valueForKey` returns null for an absent key and also permits nullable `V`.
`has` distinguishes a stored null value from an absent key.

For a valid key whose payload exceeds the limit, `setting` raises
`KeyTooLong` with the actual payload byte count and the maximum. `valueForKey`
returns null, `has` returns false, and deletion leaves the map unchanged for
such a key. Key construction still validates byte values and UTF-16 before any
map operation.

## Errors and precedence

`HashMapError` is a sealed exception family. Each error type and its associated
data are stable API:

| Error type | Data |
| --- | --- |
| `InvalidCapacity` | Supplied capacity when it is less than or equal to zero |
| `CapacityLimitExceeded` | No additional data |
| `InvalidSipHashKeyLength` | Actual SipHash key length |
| `InvalidMurmurSeed` | Supplied seed outside `0..0xffffffff` |
| `InvalidByte` | First invalid byte's index and value |
| `MalformedUtf16` | First invalid code-unit offset |
| `KeyTooLong` | Actual and maximum payload byte counts |

Human-readable exception text is not stable API. Constructor validation order
is: nonpositive capacity, capacity above the production maximum, then hash
configuration. Hash configuration checks the Murmur seed range. For SipHash,
it checks key length before scanning byte values, then reports the first
invalid byte. `HashAlgorithm.validatedHash` validates its configuration before
its data bytes. `HashMap.fromEntries` validates its map configuration before
reading entries, then follows source order. Each insertion validates key
length, replaces an existing key if present, and considers growth only for a
new key. A capacity-growth failure leaves all published maps unchanged.

The table above is the complete public error family. No API accepts a custom
strategy, hash callback, value copier, or key serializer.

## Cross-language case boundary

Shared Swift/Dart conformance cases are limited to inputs representable by
both APIs: byte payload elements in `0..255`, valid Unicode strings, and
Murmur seeds in the unsigned 32-bit range. A Swift `[UInt8]` cannot express a
negative or greater-than-255 byte. Swift string values cannot represent an
unpaired UTF-16 surrogate as a Unicode scalar, and the Swift seed type cannot
represent an out-of-range unsigned 32-bit seed. These cases therefore do not
count as Swift API conformance. Dart must test malformed UTF-16, out-of-range
byte integers, and invalid seeds through its own public API; fixture adapters
must reject or classify them as Dart-only without prevalidating shared valid
cases.

## Capability and conformance boundary

The package is pure in-memory computation and declares an empty capability
profile. It does not read files, environment variables, a clock, process state,
network, credentials, or entropy.

The fixture gate must compare literal key encodings and hash answers against
the real public Swift hash-map calls and an independent exact-arithmetic
association-list model. The model is a separate oracle, not a projection of
the Swift table layout. Phase2 routes the shared profile and fixture to the
real Swift consumer only. Fixture adapters pass raw cases to public methods
and observe returned maps and errors without pre-validating them. Phase3 adds
the Dart consumer and its real package/BUILD routing together with the Dart
package. A placeholder Dart package or selector is not part of phase1 or
phase2.

Public Dart acceptance must execute the package APIs on both VM and Node. It
must cover both collision strategies, exact resize boundaries, collisions,
tombstones, physical order, snapshots, duplicate entries, merge, nullable
values, failure atomicity, Unicode and byte validation, and error precedence.
For each strategy, exhaust every history of length zero through five over
three keys, two values, and the operations set(key, value) and remove(key).
That is 66,430 histories per strategy. Compare every prefix with an
independent association-list model, and recheck every earlier immutable
version after each prefix. Longer generated histories and byte-mutation fuzz
must record deterministic seeds, iteration counts, and replayable failures.
Report whether the selected engine actually supports shrinking; preserve and
replay the failing input when it does not. Do not call deterministic mutation
testing coverage-guided fuzzing unless it uses coverage feedback.

The production maximum requires public-boundary tests at capacity
`1_048_576`: for chaining, fill all `1_048_576` entries; for probing, fill
`786_432` entries (the largest count before the next distinct insertion
crosses its 75% threshold). At each boundary, a distinct insertion must fail
with `CapacityLimitExceeded`, replacement must succeed without growth,
remove-and-reinsert must follow the threshold, and prior map versions must
remain unchanged. Use the private transient builder to construct these cases
without exposing mutation. Smaller injected limits are internal test cases
and must be labeled separately. Key-size maximum and over-limit behavior also
require public-boundary tests.

Production line and branch coverage must be measured separately and each must
exceed 95% on the actual source. Function coverage may be reported only when
the coverage engine emits those counters. Meaningful source-fault controls
must fail on actual public behavior. Do not claim a SAT proof, coverage-guided
fuzzing, or shrink support unless the selected tool actually provides it.

These are acceptance obligations, not completed validation. The profile and
reference package do not claim a Dart Warrant proof, a native proof binding,
unrun VM or Node results, or cross-language conformance before the shared
fixtures and public tests execute.
