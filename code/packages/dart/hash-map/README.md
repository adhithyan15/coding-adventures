# Dart hash-map

This package implements the persistent DT18 hash-map profile in pure Dart. It
uses the sibling `coding_adventures_hash_functions` package for FNV-1a,
Murmur3, and SipHash-2-4, and provides separate chaining and linear probing.

## Stack and use

The hash map sits above the hash-functions package. A `HashMapKey` gives
strings and byte lists separate type tags and stores its own copied payload.
`HashMap` keeps table structure persistent: updates return a new map, while
values remain shallow references.

```dart
import 'package:coding_adventures_hash_map/coding_adventures_hash_map.dart';

void main() {
  final empty = HashMap<String>();
  final key = HashMapKey.string('compiler');
  final populated = empty.setting('ready', forKey: key);

  print(populated.valueForKey(key)); // ready
  print(empty.has(key)); // false
}
```

`HashMap.fromEntries` builds a map from a one-pass iterable. `merging` applies
right-side entries in their physical iteration order and keeps the left map's
configuration. `has` distinguishes a stored `null` from an absent key.

## Profile and validation

The defaults are capacity 16, separate chaining, and SipHash-2-4 with the
public compatibility key `codex-dt18-key!!`. Capacities range from 1 through
1,048,576; map keys are limited to 1 MiB. Chaining grows when full. Linear
probing grows before a distinct insertion would exceed a 75% live load, even
when a tombstone is available. Replacements do not grow either strategy.

Run `BUILD` or `BUILD_windows` from this directory. They run the shared corpus
and boundary tests on the Dart VM and Node, measure production line and branch
coverage, and execute source-mutation controls. Snapshot generation and
coverage tooling stay outside the production library, which has no filesystem,
clock, random, network, process, or environment dependency.

The public API, key encoding, validation rules, growth thresholds, iteration
order, and merge behavior follow
[`DT18-hash-map-dart-profile.md`](../../../specs/DT18-hash-map-dart-profile.md).
