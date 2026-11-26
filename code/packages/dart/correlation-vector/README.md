# coding_adventures_correlation_vector (Dart)

Provenance log for tracked entities. The Dart lane of the Rust
`correlation-vector` crate — every entity's IDs, origins,
contributions, tombstones, and ancestry, with budgeted checked
mode.

```dart
import 'package:coding_adventures_correlation_vector/correlation_vector.dart';

final log = CVLog();
final id = log.create(Origin(source: 'app.ts', location: '5:12'));
log.contribute(id, 'parser', 'created', const {});
print('$id ${log.history(id).length}');
// 9fcf3313.1 1
```

## API (slice 1: in-memory log)

- Mint: `create` / `tryCreate` (`{base}.{n}` hierarchical IDs
  from SHA-256 origins, `cv1.{seq}` in `CVLog.compact()`).
- Mutate: `contribute`, `derive` / `tryDerive`, `merge` /
  `tryMerge` (order- and duplicate-preserving parents), `delete` /
  `tryDelete` (permanent tombstones), `passthrough` /
  `tryPassthrough`.
- Query: `get`, `ancestors` (BFS nearest-first), `descendants`
  (reverse index), `history`, `lineage` (deterministic topo),
  `entries` / `passOrder` views, `isEnabled` / `setEnabled`.
- Checked: `CVLog.newCheckedCompact(GraphLimits)` with
  transaction budgets; failures throw `CvError` with the
  reference's exact messages, and leave state unchanged.
- Slice 2 (queued): serialization, `validateGraph`, fallible
  `tryAncestors` / `tryDescendants` / `tryLineage`, checked
  export, and the journal.

## Deliberate divergences from the reference

- `Result<T, String>` maps to throwing `CvError` (same messages);
  the panicking twins (`create`, `derive`, `merge`, `delete`,
  checked `passthrough`) throw `StateError` with the `expect`
  text, unquoted (Dart doesn't Debug-format payloads).
- `try*` here means "throws `CvError`" (never null): the errors
  carry messages the nullable-twin convention would discard.
- Stored collections are deep-frozen on the way in (the reference
  moves ownership, which Dart cannot express): `parentIds` /
  `contributions` become unmodifiable lists and meta maps become
  unmodifiable deep copies, so reads (`get`, `lineage`, `entries`)
  can safely hand out the live entries. Cyclic meta and
  non-string keys — both unrepresentable upstream — fail closed
  with `CvError`; the freeze is iterative and memoized, so deep
  inputs can't overflow the stack and shared substructure stays
  linear.
- No `cleanup` module: stack-safe disposal of rejected JSON is
  subsumed by garbage collection.
- Traversal queries run direct walks; validation wrappers arrive
  with graph validation in slice 2. Dangling parents (only
  constructible via unchecked derive-from-missing) are skipped by
  `lineage` rather than erroring.

## Validation

- **Unit** (`test/correlation_vector_test.dart`): 37 of the
  reference lib suite's 41 tests (the 4 serialization tests await
  slice 2) plus 13 extras (compact IDs, checked budgets with
  exact byte counts, second-delete permanence, unknown/deleted
  identities, ceiling validation, twin wrapping, meta isolation,
  closed meta vocabulary, diamond orders, unmodifiable views).
  ID bases are pinned against hashlib (`9fcf3313`, `1c2137e9`,
  `cf7ce668`).
- **Property** (`test/properties_test.dart`): 10 seeded properties
  over the in-lane `pbt` package — ID shapes, uniqueness,
  counter monotonicity, disabled-log laws, history fidelity,
  deletion blocking, traversal consistency over random DAGs,
  exact checked budgets, merge fidelity, determinism.
- **Fuzz** (`test/fuzz_test.dart`): 2,000-iteration hostile campaign
  over in-lane `fuzz`, driving op scripts across all 5 log
  configurations; only `CvError` (and the specified checked
  `StateError`) escapes, and id/get/history/lineage/topo/budget
  invariants hold after every campaign.
- **Coverage**: 100% lines (unreachable-without-import branches
  carry `coverage:ignore` markers that slice-2 import tests
  remove).
