# Changelog — HashSet (Swift)

All notable changes to this package are documented here.

## 0.1.0 — Initial release

- Add a persistent DT19 hash set composed over the Swift DT18 hash map.
- Preserve exact byte/string identity, closed DT17 hash profiles, collision
  strategies, and finite work limits from the underlying package.
- Implement membership, persistent insertion/removal, complete set algebra,
  subset/superset/disjoint/equality relations, and deterministic physical-order
  element export.
- Add package-native tests for persistence, both collision strategies, Unicode
  identity, algebra laws, stable failures, release builds, truthful Unix/macOS
  and Windows fronts, and an empty capability profile.
