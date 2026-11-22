# HashSet for Dart

This package implements the persistent DT19 hash-set API in Dart by composing the
sibling Dart `coding_adventures_hash_map` package. It stores each member as a DT18
key with a private sentinel boolean value, delegating collision handling, finite capacity
limits, key validation, and DT17 hashing to one unified, reviewed implementation.

## Element and Hash Profile

The element domain is `HashMapKey`: exact UTF-8 strings and raw byte arrays.
The two forms are type-tagged and remain distinct. String equality is exact
UTF-8 byte equality, so composed and decomposed Unicode spellings can coexist.

Callers choose separate chaining or linear probing and one of the closed DT17
hash profiles: FNV-1a-32, seeded MurmurHash3-32, or SipHash-2-4 with an exact
16-byte key. The convenience initializer uses HashMap's public deterministic
compatibility key. It is reproducible, not secret; callers facing adversarial
keys should provide a secret caller-owned SipHash key.

```dart
import 'package:coding_adventures_hash_set/coding_adventures_hash_set.dart';

final empty = HashSet(
  capacity: 8,
  strategy: CollisionStrategy.linearProbing,
  hashAlgorithm: const HashAlgorithm.fnv1a32(),
);
final names = empty
    .adding(HashMapKey.string('alice'))
    .adding(HashMapKey.string('bob'));

print(empty.contains(HashMapKey.string('alice'))); // false
print(names.contains(HashMapKey.string('alice'))); // true
```

## Persistence and Algebra

`adding`, `removing`, `discarding`, `union`, `intersection`, `difference`, and
`symmetricDifference` return new values. Their inputs remain unchanged.
Algebraic results retain the receiver's collision strategy and hash algorithm,
even when the other set uses a different profile.

```dart
final staff = HashSet.fromElements([
  HashMapKey.string('alice'),
  HashMapKey.string('bob'),
]);
final admins = HashSet.fromElements([
  HashMapKey.string('bob'),
  HashMapKey.string('carol'),
]);

final everyone = staff.union(admins);
final staffAdmins = staff.intersection(admins);

print(everyone.count); // 3
print(staffAdmins.elements); // contains only 'bob'
```

The relational helpers are `isSubsetOf`, `isSupersetOf`, `isDisjointWith`, and
`equals`. Iteration through `elements` follows the underlying map's physical storage
order. It is neither sorted nor insertion order and is deterministic only for the same
complete table profile and operation history.

## 5-Class Security Resilience Showcase

1. **Memory Exhaustion / Allocation Bomb Defense**: Strict capacity bounds (`HashMapLimits.maximumCapacity`) prevent unbounded memory consumption.
2. **Stack Overflow / Recursion Defense**: All set algebra, traversal, and lookups execute iteratively without recursion.
3. **Key Integrity & Encoding Defense**: Exact byte-level tagging ensures raw bytes and strings are distinct, and NFC and NFD strings coexist accurately.
4. **Boundary & State Machine Defense**: Robust handling of empty sets, self-algebra, duplicate additions, and missing element removals.
5. **Diagnostic Payload Blindness (Standing Rule #20)**: Diagnostic errors never echo caller payloads or sensitive tokens.
