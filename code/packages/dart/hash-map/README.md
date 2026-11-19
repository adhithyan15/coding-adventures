# Dart hash-map

Pure Dart implementation of the persistent DT18 hash-map profile. The map
uses the sibling `coding_adventures_hash_functions` package and provides
separate chaining and linear probing with immutable updates.

The public API, key encoding, validation rules, growth thresholds, iteration
order, and merge behavior follow
[`DT18-hash-map-dart-profile.md`](../../../specs/DT18-hash-map-dart-profile.md).

Run `BUILD` or `BUILD_windows` from this directory. The front prepares a
Node-loadable snapshot from the shared v1 corpus, executes public tests on the
Dart VM and Node, measures production line and branch coverage separately,
and runs source-mutation controls. Snapshot generation and coverage tooling
remain outside the production library.
