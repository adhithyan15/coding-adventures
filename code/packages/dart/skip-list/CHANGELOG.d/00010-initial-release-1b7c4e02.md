## 0.1.0

- Initial Dart lane of the skip-list identity (closes 15/15): generic
  `SkipList<K extends Comparable, V>` over an ordered map with insert,
  delete, search, contains(+Key), 0-based rank/byRank, rangeQuery/range,
  toList, entries(+List), min/max, length/size/isEmpty, tuning accessors,
  and the `currentMax` height estimate.
- 13 tests: reference mirrors, a seeded 2,000-op model-based sweep with
  per-step oracle agreement, nullable-value and snapshot-isolation cases;
  95% branch+function coverage gate.
