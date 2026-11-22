# coding_adventures_skip_list (Dart)

Sorted skip-list collection: the Dart lane of
[DT20](../../../specs/DT20-skip-list.md).

A skip list keeps elements sorted with O(log n) expected operations — plus
range queries and rank access that hash maps cannot do. This lane stores
entries in a balanced ordered map (like the reference lanes), so behavior is
fully deterministic, and preserves the skip-list tuning surface
(`maxLevel`/`probability`/`currentMax`).

## API

```dart
import 'package:coding_adventures_skip_list/coding_adventures_skip_list.dart';

final scores = SkipList<String, int>();
scores.insert('ada', 1815);
int? v = scores.search('ada');            // 1815
bool gone = scores.delete('ada');         // true
bool has = scores.contains('grace');      // containsKey alias too

int? r = scores.rank('grace');            // 0-based, null when absent
String? k = scores.byRank(0);             // null when out of range
List<(String, int)> w = scores.rangeQuery('a', 'm', true);  // range alias too

List<String> keys = scores.toList();
List<(String, int)> pairs = scores.entries();  // entriesList alias too
String? lo = scores.min(); String? hi = scores.max();
int n = scores.length; int m = scores.size();  // same value
bool empty = scores.isEmpty; bool full = scores.isNotEmpty;

// Tuning (defaults 32 levels, p = 0.5; sanitized like the reference lanes)
final tuned = SkipList<int, int>.withParams(8, 0.9);  // newWithParams alias too
int ml = tuned.maxLevel(); double p = tuned.probability();
int est = tuned.currentMax();             // ceil(log(n)/log(1/p)), clamped
```

* Keys need `Comparable`; values are unrestricted (nullable values round-trip
  and delete cleanly).
* Rank is 0-based with null for missing keys, matching the established lanes.
  (The DT20 pseudocode sketches 1-based ranks; reconciling that stale sketch
  is logged follow-up work, not part of this port.)
* Snapshots (`toList`, `entries`) copy: later mutations never affect them.
* Zero dependencies. Pure computation: no clock, no I/O, no randomness.

## Tests

```bash
dart pub get
dart test
```

The suite mirrors the reference tests (empty behavior, insert/search/delete,
sorted order, rank, inclusive/exclusive ranges, params, tuning), runs a
seeded 2,000-operation model-based sweep against a map oracle with
per-step order/size/rank/range agreement, and covers nullable values,
string keys, and snapshot isolation. Coverage gate: 95% branch+function.
